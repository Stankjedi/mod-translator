use crate::ai::{
    hints::{RetryHint, RetryHintSource},
    translate_text, ProviderId, TranslationError,
};
use crate::archive::{self, ArchiveModification};
use crate::backup::backup_and_swap;
use crate::placeholder_validator::{PlaceholderValidator, Segment as ValidatorSegment};
use crate::protector::Protector;
use crate::quality::{validate_segment, SegmentLimits};
use crate::validation_logger::{validation_logger, ValidationOutcome};
use log::warn;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::{DefaultHasher, Entry};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;
use tokio::time::sleep;

const MAX_RETRY_ATTEMPTS: usize = 3;

const RATE_LIMIT_BASE_BACKOFF_MS: u64 = 1_000;
const RATE_LIMIT_MAX_BACKOFF_MS: u64 = 60_000;
const RESUME_DIR_NAME: &str = ".resume";

static ACTIVE_JOBS: Lazy<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static JOB_STATES: Lazy<Mutex<HashMap<String, JobState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static JOB_BACKOFFS: Lazy<Mutex<HashMap<String, Arc<BackoffController>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct ActiveBackoff {
    attempt: u32,
    delay: Duration,
    reason: String,
    used_hint: bool,
    manual_triggered: bool,
}

#[derive(Debug)]
struct BackoffController {
    state: Mutex<Option<ActiveBackoff>>,
    notifier: Notify,
}

impl BackoffController {
    fn new() -> Self {
        Self {
            state: Mutex::new(None),
            notifier: Notify::new(),
        }
    }

    fn begin(&self, attempt: u32, delay: Duration, reason: String, used_hint: bool) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = Some(ActiveBackoff {
                attempt,
                delay,
                reason,
                used_hint,
                manual_triggered: false,
            });
        }
    }

    fn cancel_manual(&self) -> bool {
        match self.state.lock() {
            Ok(mut guard) => {
                if let Some(state) = guard.as_mut() {
                    state.manual_triggered = true;
                    self.notifier.notify_waiters();
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    fn cancel_logic(&self) {
        self.notifier.notify_waiters();
    }

    fn take(&self) -> Option<ActiveBackoff> {
        self.state.lock().ok()?.take()
    }

    async fn notified(&self) {
        self.notifier.notified().await;
    }

    fn is_active(&self) -> bool {
        self.state
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslationCheckpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_line_index: Option<u32>,
    pub translated_count: u32,
    pub total_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSignature {
    modified: Option<SystemTime>,
    hash: u64,
    len: u64,
}

#[derive(Debug, Clone)]
struct FileProgress {
    signature: FileSignature,
    replacements: HashMap<usize, String>,
}

impl FileProgress {
    fn new(signature: FileSignature) -> Self {
        Self {
            signature,
            replacements: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct JobState {
    checkpoint: TranslationCheckpoint,
    files: HashMap<String, FileProgress>,
}

impl JobState {
    fn new() -> Self {
        Self {
            checkpoint: TranslationCheckpoint::default(),
            files: HashMap::new(),
        }
    }
}

fn load_job_state(job_id: &str) -> Option<JobState> {
    JOB_STATES
        .lock()
        .ok()
        .and_then(|guard| guard.get(job_id).cloned())
}

fn save_job_state(job_id: &str, state: JobState) {
    if let Ok(mut guard) = JOB_STATES.lock() {
        guard.insert(job_id.to_string(), state);
    }
}

fn clear_job_state(job_id: &str) {
    if let Ok(mut guard) = JOB_STATES.lock() {
        guard.remove(job_id);
    }
}

fn current_checkpoint(job_id: &str) -> Option<TranslationCheckpoint> {
    JOB_STATES
        .lock()
        .ok()
        .and_then(|guard| guard.get(job_id).map(|state| state.checkpoint.clone()))
}

fn compute_file_signature(path: &Path, content: &str) -> FileSignature {
    let metadata = fs::metadata(path).ok();
    let modified = metadata.as_ref().and_then(|data| data.modified().ok());
    let len = metadata
        .as_ref()
        .map(|data| data.len())
        .unwrap_or_else(|| content.len() as u64);
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let hash = hasher.finish();
    FileSignature {
        modified,
        hash,
        len,
    }
}

fn apply_stored_translations(
    job_state: &JobState,
    file_contexts: &mut [FileContext],
    segments: &[Segment],
) -> u32 {
    let mut processed = 0u32;
    for segment in segments {
        let Some(file_progress) = job_state.files.get(&segment.relative_path) else {
            break;
        };
        let Some(replacement) = file_progress.replacements.get(&segment.line_index) else {
            break;
        };
        if let Some(context) = file_contexts.get_mut(segment.file_index) {
            context.translated_lines[segment.line_index] = Some(replacement.clone());
        }
        processed += 1;
    }
    processed
}

fn update_checkpoint_for_next_segment(
    job_state: &mut JobState,
    segments: &[Segment],
    processed_segments: u32,
) {
    job_state.checkpoint.total_count = segments.len() as u32;
    job_state.checkpoint.translated_count = processed_segments;
    if let Some(next_segment) = segments.get(processed_segments as usize) {
        job_state.checkpoint.current_file_path = Some(next_segment.relative_path.clone());
        job_state.checkpoint.next_line_index = Some(next_segment.line_index as u32);
    } else {
        job_state.checkpoint.current_file_path = None;
        job_state.checkpoint.next_line_index = None;
    }
}

fn set_checkpoint_for_pending_segment(
    job_state: &mut JobState,
    segment: &Segment,
    total_segments: u32,
    processed_segments: u32,
) {
    job_state.checkpoint.total_count = total_segments;
    job_state.checkpoint.translated_count = processed_segments;
    job_state.checkpoint.current_file_path = Some(segment.relative_path.clone());
    job_state.checkpoint.next_line_index = Some(segment.line_index as u32);
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFileInput {
    pub relative_path: String,
    pub mod_install_path: String,
    /// 아카이브 내부 파일인 경우 아카이브의 상대 경로
    #[serde(default)]
    pub archive_path: Option<String>,
    /// 아카이브 내부 엔트리 경로
    #[serde(default)]
    pub archive_entry_path: Option<String>,
}

impl TranslationFileInput {
    /// 아카이브 내부 파일인지 확인
    pub fn is_archive_entry(&self) -> bool {
        self.archive_path.is_some() && self.archive_entry_path.is_some()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartTranslationJobPayload {
    pub job_id: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub model_id: String,
    pub files: Vec<TranslationFileInput>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub output_override_dir: Option<String>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
    #[serde(default)]
    pub reset_resume_state: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationProgressEventPayload {
    pub job_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_requested: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translated_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_errors: Option<Vec<TranslationFileErrorEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_written: Option<LastWrittenInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<TranslationCheckpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryStatusPayload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastWrittenInfo {
    pub source_relative_path: String,
    pub output_absolute_path: String,
    pub output_relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFileErrorEntry {
    pub file_path: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryStatusPayload {
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_seconds: u32,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeHintPayload {
    pub file_path: String,
    pub line_number: u32,
}

#[derive(Debug)]
struct Segment {
    file_index: usize,
    relative_path: String,
    line_index: usize,
    line_number: usize,
    text: String,
    prefix: String,
    suffix: String,
}

#[derive(Clone)]
struct FileContext {
    relative_path: String,
    #[allow(dead_code)]
    mod_install_path: PathBuf,
    lines: Vec<String>,
    translated_lines: Vec<Option<String>>,
    had_trailing_newline: bool,
    output_relative_path: PathBuf,
    output_absolute_path: PathBuf,
    resume_metadata_path: PathBuf,
    resume_line_index: usize,
    /// 아카이브 파일 경로 (아카이브 내부 파일인 경우)
    archive_path: Option<PathBuf>,
    /// 아카이브 내부 엔트리 경로
    archive_entry_path: Option<String>,
}

fn compute_backoff_ms(attempt: u32) -> u64 {
    if attempt == 0 {
        return RATE_LIMIT_BASE_BACKOFF_MS;
    }

    let exponent = attempt.saturating_sub(1).min(10);
    let multiplier = 1u64 << exponent;
    let backoff = RATE_LIMIT_BASE_BACKOFF_MS.saturating_mul(multiplier);
    backoff.min(RATE_LIMIT_MAX_BACKOFF_MS)
}

#[derive(Debug, Clone)]
struct RetryPlan {
    delay: Duration,
    reason: String,
    used_hint: bool,
}

fn compute_retry_plan(error: &TranslationError, attempt: u32) -> RetryPlan {
    if let Some(hint) = error.retry_hint() {
        let delay = hint.clamped_delay();
        let reason = describe_retry_hint(hint);
        RetryPlan {
            delay,
            reason,
            used_hint: true,
        }
    } else {
        let delay = Duration::from_millis(compute_backoff_ms(attempt));
        RetryPlan {
            delay,
            reason: format!("Automatic backoff (attempt {attempt})"),
            used_hint: false,
        }
    }
}

fn describe_retry_hint(hint: &RetryHint) -> String {
    let seconds = hint.clamped_delay().as_secs();
    let pretty_seconds = if seconds == 0 {
        String::from("<1s")
    } else {
        format!("{}s", seconds)
    };

    match hint.source {
        RetryHintSource::RetryAfterHeader => {
            if let Some(raw) = hint.raw_value() {
                format!("Server Retry-After ({raw})")
            } else {
                format!("Server Retry-After (~{pretty_seconds})")
            }
        }
        RetryHintSource::GeminiRetryInfo => {
            if let Some(raw) = hint.raw_value() {
                format!("Gemini retry hint ({raw})")
            } else {
                format!("Gemini retry hint (~{pretty_seconds})")
            }
        }
    }
}

fn duration_to_retry_seconds(duration: Duration) -> u32 {
    if duration.is_zero() {
        return 0;
    }

    let secs = duration.as_secs();
    if secs >= u32::MAX as u64 {
        return u32::MAX;
    }

    if duration.subsec_nanos() > 0 {
        secs.saturating_add(1) as u32
    } else {
        secs as u32
    }
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn start_translation_job(
    app: AppHandle,
    jobId: String,
    provider: String,
    apiKey: Option<String>,
    modelId: String,
    sourceLang: Option<String>,
    targetLang: Option<String>,
    files: Vec<TranslationFileInput>,
    outputOverrideDir: Option<String>,
    resumeFromCheckpoint: Option<bool>,
    resetResumeState: Option<bool>,
) -> Result<(), String> {
    let mut payload = StartTranslationJobPayload {
        job_id: jobId,
        provider,
        api_key: apiKey,
        model_id: modelId,
        files,
        source_lang: sourceLang,
        target_lang: targetLang,
        output_override_dir: outputOverrideDir,
        resume_from_checkpoint: false,
        reset_resume_state: false,
    };
    payload.resume_from_checkpoint = resumeFromCheckpoint.unwrap_or(false);
    payload.reset_resume_state = resetResumeState.unwrap_or(false);
    payload.model_id = payload.model_id.trim().to_string();
    if payload.files.is_empty() {
        return Err("번역할 파일을 하나 이상 선택해야 합니다.".into());
    }

    let provider = match ProviderId::try_from(payload.provider.as_str()) {
        Ok(provider) => provider,
        Err(_) => {
            emit_progress(
                &app,
                TranslationProgressEventPayload {
                    job_id: payload.job_id.clone(),
                    status: "failed".into(),
                    progress_pct: Some(0.0),
                    cancel_requested: None,
                    log: Some(format!("지원하지 않는 번역기: {}", payload.provider)),
                    translated_count: Some(0),
                    total_count: Some(0),
                    file_name: None,
                    file_success: None,
                    file_errors: None,
                    last_written: None,
                    checkpoint: None,
                    retry: None,
                },
            );
            return Err(format!("지원하지 않는 번역기: {}", payload.provider));
        }
    };

    let api_key = payload.api_key.clone().unwrap_or_default();
    if api_key.trim().is_empty() {
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                status: "failed".into(),
                progress_pct: Some(0.0),
                cancel_requested: None,
                log: Some("API 키가 설정되지 않았습니다.".into()),
                translated_count: Some(0),
                total_count: Some(0),
                file_name: None,
                file_success: None,
                file_errors: None,
                last_written: None,
                checkpoint: None,
                retry: None,
            },
        );
        return Err("선택한 번역기의 API 키를 설정해 주세요.".into());
    }

    if payload.model_id.is_empty() {
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                status: "failed".into(),
                progress_pct: Some(0.0),
                cancel_requested: None,
                log: Some("번역 모델이 선택되지 않았습니다.".into()),
                translated_count: Some(0),
                total_count: Some(0),
                file_name: None,
                file_success: None,
                file_errors: None,
                last_written: None,
                checkpoint: None,
                retry: None,
            },
        );
        return Err("번역에 사용할 모델을 선택해 주세요.".into());
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let backoff_controller = Arc::new(BackoffController::new());

    {
        let mut guard = ACTIVE_JOBS
            .lock()
            .map_err(|_| "job registry lock poisoned".to_string())?;
        guard.insert(payload.job_id.clone(), cancel_flag.clone());
    }

    {
        let mut guard = JOB_BACKOFFS.lock().map_err(|_| {
            if let Ok(mut active) = ACTIVE_JOBS.lock() {
                active.remove(&payload.job_id);
            }
            "backoff registry lock poisoned".to_string()
        })?;
        guard.insert(payload.job_id.clone(), backoff_controller.clone());
    }

    let job_id = payload.job_id.clone();
    let app_handle = app.clone();
    tauri::async_runtime::spawn({
        let cancel_flag = cancel_flag.clone();
        let backoff_controller = backoff_controller.clone();
        async move {
            run_translation_job(
                app_handle.clone(),
                payload,
                provider,
                api_key.trim().to_string(),
                cancel_flag,
                backoff_controller.clone(),
            )
            .await;

            if let Ok(mut guard) = ACTIVE_JOBS.lock() {
                guard.remove(&job_id);
            }
            if let Ok(mut guard) = JOB_BACKOFFS.lock() {
                guard.remove(&job_id);
            }
        }
    });

    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn cancel_translation_job(app: AppHandle, jobId: String) -> Result<(), String> {
    let guard = ACTIVE_JOBS
        .lock()
        .map_err(|_| "job registry lock poisoned".to_string())?;

    if let Some(flag) = guard.get(&jobId) {
        flag.store(true, Ordering::SeqCst);
        drop(guard);

        if let Some(controller) = JOB_BACKOFFS
            .lock()
            .map_err(|_| "backoff registry lock poisoned".to_string())?
            .get(&jobId)
            .cloned()
        {
            controller.cancel_logic();
        }

        let checkpoint = current_checkpoint(&jobId);
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: jobId,
                status: "running".into(),
                progress_pct: None,
                cancel_requested: Some(true),
                log: None,
                translated_count: None,
                total_count: None,
                file_name: None,
                file_success: None,
                file_errors: None,
                last_written: None,
                checkpoint,
                retry: None,
            },
        );

        Ok(())
    } else {
        drop(guard);

        if let Some(controller) = JOB_BACKOFFS
            .lock()
            .map_err(|_| "backoff registry lock poisoned".to_string())?
            .get(&jobId)
            .cloned()
        {
            controller.cancel_logic();
        }

        let checkpoint = current_checkpoint(&jobId);
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: jobId,
                status: "canceled".into(),
                progress_pct: Some(0.0),
                cancel_requested: Some(true),
                log: Some("User canceled while preparing.".into()),
                translated_count: Some(0),
                total_count: Some(0),
                file_name: None,
                file_success: None,
                file_errors: None,
                last_written: None,
                checkpoint,
                retry: None,
            },
        );

        Ok(())
    }
}

#[tauri::command]
pub fn open_output_folder(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("경로를 찾을 수 없습니다: {path}"));
    }

    open::that_detached(path_buf).map_err(|error| format!("폴더를 열 수 없습니다: {error}"))
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn retry_translation_now(jobId: String) -> Result<(), String> {
    let controller = JOB_BACKOFFS
        .lock()
        .map_err(|_| "backoff registry lock poisoned".to_string())?
        .get(&jobId)
        .cloned();

    if let Some(controller) = controller {
        controller.cancel_manual();
    }

    Ok(())
}

async fn run_translation_job(
    app: AppHandle,
    payload: StartTranslationJobPayload,
    provider: ProviderId,
    api_key: String,
    cancel_flag: Arc<AtomicBool>,
    backoff_controller: Arc<BackoffController>,
) {
    let source_lang = payload.source_lang.as_deref().unwrap_or("auto").to_string();
    let target_lang = payload.target_lang.as_deref().unwrap_or("ko").to_string();
    let override_root = payload
        .output_override_dir
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let _resume_from_checkpoint = payload.resume_from_checkpoint;
    let _reset_resume_state = payload.reset_resume_state;

    let mut file_contexts: Vec<FileContext> = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut file_errors: Vec<TranslationFileErrorEntry> = Vec::new();
    let qc_limits = SegmentLimits::default();
    let mut rolled_back_segments: Vec<String> = Vec::new();
    let mut job_state = load_job_state(&payload.job_id).unwrap_or_else(JobState::new);
    let mut changed_files: Vec<String> = Vec::new();
    let mut already_processed_segments: u32 = 0;

    for file in &payload.files {
        let relative_path = PathBuf::from(&file.relative_path);
        let mod_root_raw = PathBuf::from(&file.mod_install_path);
        let mod_root = mod_root_raw.canonicalize().unwrap_or(mod_root_raw.clone());
        
        // 아카이브 내부 파일인지 확인
        let (content, archive_path, archive_entry_path, source_file_path) = if file.is_archive_entry() {
            let archive_rel = file.archive_path.as_ref().unwrap();
            let entry_path = file.archive_entry_path.as_ref().unwrap();
            let archive_full_path = mod_root.join(archive_rel);
            
            match archive::read_archive_entry_string(&archive_full_path, entry_path) {
                Ok(content) => (
                    content,
                    Some(archive_full_path.clone()),
                    Some(entry_path.clone()),
                    archive_full_path,
                ),
                Err(err) => {
                    let message = format!(
                        "Failed to read {}!{}: {}",
                        archive_rel, entry_path, err
                    );
                    file_errors.push(TranslationFileErrorEntry {
                        file_path: file.relative_path.clone(),
                        message,
                        code: Some("ARCHIVE_READ_FAILED".into()),
                    });
                    continue;
                }
            }
        } else {
            let source_file_path = mod_root.join(&relative_path);
            match fs::read_to_string(&source_file_path) {
                Ok(value) => (value, None, None, source_file_path),
                Err(err) => {
                    let message = format!(
                        "Failed to read {}: {}",
                        source_file_path.to_string_lossy(),
                        err
                    );
                    file_errors.push(TranslationFileErrorEntry {
                        file_path: file.relative_path.clone(),
                        message,
                        code: Some("READ_FAILED".into()),
                    });
                    continue;
                }
            }
        };

        let had_trailing_newline = content.ends_with('\n');
        let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
        let output_relative_path = derive_output_relative_path(&file.relative_path, &target_lang);
        let base_root = override_root
            .as_ref()
            .cloned()
            .unwrap_or_else(|| mod_root.clone());
        let output_absolute_path = base_root.join(&output_relative_path);
        let resume_metadata_path = build_resume_metadata_path(&output_absolute_path);

        let mut context = FileContext {
            relative_path: file.relative_path.clone(),
            mod_install_path: mod_root,
            lines,
            translated_lines: Vec::new(),
            had_trailing_newline,
            output_relative_path,
            output_absolute_path,
            resume_metadata_path,
            resume_line_index: 0,
            archive_path,
            archive_entry_path,
        };
        context.translated_lines = vec![None; context.lines.len()];

        let signature = compute_file_signature(&source_file_path, &content);
        match job_state.files.entry(context.relative_path.clone()) {
            Entry::Occupied(mut entry) => {
                if entry.get().signature != signature {
                    entry.get_mut().signature = signature.clone();
                    entry.get_mut().replacements.clear();
                    changed_files.push(context.relative_path.clone());
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(FileProgress::new(signature.clone()));
            }
        }

        let file_index = file_contexts.len();
        for (line_index, line) in context.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if line_index < context.resume_line_index {
                already_processed_segments += 1;
                continue;
            }

            let prefix_len = line.find(trimmed).unwrap_or(0);
            let suffix_start = prefix_len + trimmed.len();
            let prefix = line[..prefix_len].to_string();
            let suffix = line[suffix_start..].to_string();

            segments.push(Segment {
                file_index,
                relative_path: context.relative_path.clone(),
                line_index,
                line_number: line_index + 1,
                text: trimmed.to_string(),
                prefix,
                suffix,
            });
        }

        file_contexts.push(context);
    }

    let total_segments = segments.len() as u32;
    let mut processed_segments =
        apply_stored_translations(&job_state, &mut file_contexts, &segments);
    processed_segments = processed_segments.max(already_processed_segments);
    update_checkpoint_for_next_segment(&mut job_state, &segments, processed_segments);
    save_job_state(&payload.job_id, job_state.clone());

    for changed in changed_files {
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                status: "running".into(),
                progress_pct: Some(percentage(processed_segments, total_segments)),
                cancel_requested: None,
                log: Some(format!(
                    "원본 파일이 변경되어 체크포인트를 초기화했습니다: {}",
                    changed
                )),
                translated_count: Some(processed_segments),
                total_count: Some(total_segments),
                file_name: None,
                file_success: None,
                file_errors: clone_errors(&file_errors),
                last_written: None,
                checkpoint: Some(job_state.checkpoint.clone()),
                retry: None,
            },
        );
    }
    let mut last_file_name: Option<String> = None;
    let mut last_file_success: Option<bool> = None;

    emit_progress(
        &app,
        TranslationProgressEventPayload {
            job_id: payload.job_id.clone(),
            status: "running".into(),
            progress_pct: Some(percentage(processed_segments, total_segments)),
            cancel_requested: None,
            log: Some("번역을 준비하는 중입니다.".into()),
            translated_count: Some(processed_segments),
            total_count: Some(total_segments),
            file_name: None,
            file_success: None,
            file_errors: clone_errors(&file_errors),
            last_written: None,
            checkpoint: Some(job_state.checkpoint.clone()),
            retry: None,
        },
    );

    if total_segments > processed_segments {
        let client = match Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
        {
            Ok(client) => client,
            Err(_err) => {
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        status: "failed".into(),
                        progress_pct: Some(0.0),
                        cancel_requested: None,
                        log: Some("HTTP 클라이언트를 초기화하지 못했습니다.".into()),
                        translated_count: Some(processed_segments),
                        total_count: Some(total_segments),
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: clone_errors(&file_errors),
                        last_written: None,
                        checkpoint: Some(job_state.checkpoint.clone()),
                        retry: None,
                    },
                );
                return;
            }
        };

        for (index, segment) in segments
            .iter()
            .enumerate()
            .skip(processed_segments as usize)
        {
            let processed = index as u32;

            set_checkpoint_for_pending_segment(
                &mut job_state,
                segment,
                total_segments,
                processed_segments,
            );
            save_job_state(&payload.job_id, job_state.clone());

            if cancel_flag.load(Ordering::SeqCst) {
                emit_cancelled_progress(
                    &app,
                    &payload,
                    processed,
                    total_segments,
                    &last_file_name,
                    last_file_success,
                    &file_errors,
                );
                return;
            }

            let fragment = Protector::protect(&segment.text);
            let mut attempt: u32 = 0;
            let mut last_error: Option<TranslationError> = None;
            let mut translated_value: Option<String> = None;
            let mut wait_cancelled_by_job = false;
            let mut apply_translation = false;
            let mut qc_messages: Option<Vec<String>> = None;

            loop {
                if cancel_flag.load(Ordering::SeqCst) {
                    break;
                }

                match translate_text(
                    &client,
                    provider,
                    &api_key,
                    &payload.model_id,
                    &fragment,
                    &source_lang,
                    &target_lang,
                )
                .await
                {
                    Ok(value) => {
                        // First, run existing quality validation
                        let validation =
                            validate_segment(segment.text.as_str(), value.as_str(), &qc_limits);

                        // Then, run enhanced placeholder validation with auto-recovery
                        let placeholder_validator = PlaceholderValidator::with_default_config();
                        let validator_segment = ValidatorSegment::new(
                            segment.relative_path.clone(),
                            segment.line_number as u32,
                            format!("line_{}", segment.line_index),
                            segment.text.clone(),
                            fragment.masked_text().to_string(),
                        );

                        let placeholder_result =
                            placeholder_validator.validate(&validator_segment, &value);

                        match placeholder_result {
                            Ok(success) => {
                                // Placeholder validation passed or was auto-recovered
                                let outcome = if success.recovered_with_warning {
                                    ValidationOutcome::RecoveredWithWarn
                                } else {
                                    ValidationOutcome::Clean
                                };
                                validation_logger().log_success(outcome);

                                if validation.is_pass() {
                                    translated_value = Some(success.value.clone());
                                    apply_translation = true;
                                    qc_messages = None;
                                } else {
                                    // Quality validation failed even with recovered placeholders
                                    let mut messages = Vec::new();
                                    if !validation.errors.is_empty() {
                                        messages.extend(validation.errors.clone());
                                    }
                                    if !validation.warnings.is_empty() {
                                        messages.extend(validation.warnings.clone());
                                    }
                                    qc_messages = if messages.is_empty() {
                                        None
                                    } else {
                                        Some(messages)
                                    };
                                    translated_value = Some(segment.text.clone());
                                    apply_translation = false;
                                    rolled_back_segments.push(format!(
                                        "{}:{}",
                                        segment.relative_path, segment.line_number
                                    ));
                                }
                            }
                            Err(failure_report) => {
                                // Placeholder validation failed and auto-recovery didn't help
                                // Log to validation logger
                                validation_logger().log_failure(&failure_report);

                                warn!(
                                    "Placeholder validation failed for {}:{} ({}): {:?}",
                                    segment.relative_path,
                                    segment.line_number,
                                    segment.line_index,
                                    failure_report.code
                                );

                                // Store failure report for UI (could be expanded later)
                                let mut messages = Vec::new();
                                messages.push(format!(
                                    "Placeholder validation failed: {:?}",
                                    failure_report.code
                                ));
                                qc_messages = Some(messages);

                                translated_value = Some(segment.text.clone());
                                apply_translation = false;
                                rolled_back_segments.push(format!(
                                    "{}:{}",
                                    segment.relative_path, segment.line_number
                                ));
                            }
                        }
                        last_error = None;
                        break;
                    }
                    Err(error) => {
                        last_error = Some(error);

                        if !should_retry_error(last_error.as_ref().unwrap()) {
                            break;
                        }

                        attempt = attempt.saturating_add(1);
                        if attempt >= MAX_RETRY_ATTEMPTS as u32 {
                            break;
                        }

                        let plan = compute_retry_plan(last_error.as_ref().unwrap(), attempt);
                        if plan.delay.is_zero() {
                            emit_retry_started(&app, &payload.job_id, attempt);
                            continue;
                        }

                        backoff_controller.begin(
                            attempt,
                            plan.delay,
                            plan.reason.clone(),
                            plan.used_hint,
                        );

                        emit_backoff_started(
                            &app,
                            &payload.job_id,
                            plan.delay,
                            attempt,
                            plan.used_hint,
                            &plan.reason,
                        );

                        let delay_seconds = duration_to_retry_seconds(plan.delay);
                        let log_message = format!(
                            "Retry attempt {}/{} scheduled in {}s ({})",
                            attempt, MAX_RETRY_ATTEMPTS, delay_seconds, plan.reason
                        );

                        emit_progress(
                            &app,
                            TranslationProgressEventPayload {
                                job_id: payload.job_id.clone(),
                                status: "running".into(),
                                progress_pct: Some(percentage(processed_segments, total_segments)),
                                cancel_requested: None,
                                log: Some(log_message),
                                translated_count: Some(processed_segments),
                                total_count: Some(total_segments),
                                file_name: last_file_name.clone(),
                                file_success: last_file_success,
                                file_errors: clone_errors(&file_errors),
                                last_written: None,
                                checkpoint: Some(job_state.checkpoint.clone()),
                                retry: Some(RetryStatusPayload {
                                    attempt,
                                    max_attempts: MAX_RETRY_ATTEMPTS as u32,
                                    delay_seconds,
                                    reason: plan.reason.clone(),
                                }),
                            },
                        );

                        match wait_with_cancellation(
                            &app,
                            &payload.job_id,
                            &cancel_flag,
                            backoff_controller.clone(),
                            plan.delay,
                        )
                        .await
                        {
                            BackoffWaitOutcome::JobCancelled => {
                                wait_cancelled_by_job = true;
                                break;
                            }
                            BackoffWaitOutcome::Manual | BackoffWaitOutcome::Completed => {
                                emit_retry_started(&app, &payload.job_id, attempt);
                                continue;
                            }
                        }
                    }
                }
            }

            if cancel_flag.load(Ordering::SeqCst) || wait_cancelled_by_job {
                emit_cancelled_progress(
                    &app,
                    &payload,
                    processed,
                    total_segments,
                    &last_file_name,
                    last_file_success,
                    &file_errors,
                );
                return;
            }

            let Some(translated_value) = translated_value else {
                // API error occurred - keep original and continue instead of failing
                let error = last_error.expect("translation error must exist on failure");
                
                // Check if this is a fatal error that should stop the entire job
                let is_fatal = matches!(
                    &error,
                    TranslationError::Unauthorized { .. } 
                    | TranslationError::Forbidden { .. }
                    | TranslationError::ModelNotFound { .. }
                );
                
                if is_fatal {
                    // For authentication/authorization errors, stop the job
                    let log_message = format_translation_error(segment, &error);
                    let file_message = format_file_error_message(segment, &error);
                    last_file_name = Some(segment.relative_path.clone());
                    last_file_success = Some(false);
                    file_errors.push(TranslationFileErrorEntry {
                        file_path: segment.relative_path.clone(),
                        message: file_message.clone(),
                        code: Some(error_code_for(&error).into()),
                    });
                    save_job_state(&payload.job_id, job_state.clone());
                    emit_progress(
                        &app,
                        TranslationProgressEventPayload {
                            job_id: payload.job_id.clone(),
                            status: "failed".into(),
                            progress_pct: Some(percentage(processed, total_segments)),
                            cancel_requested: None,
                            log: Some(log_message.clone()),
                            translated_count: Some(processed_segments),
                            total_count: Some(total_segments),
                            file_name: last_file_name.clone(),
                            file_success: last_file_success,
                            file_errors: clone_errors(&file_errors),
                            last_written: None,
                            checkpoint: Some(job_state.checkpoint.clone()),
                            retry: None,
                        },
                    );
                    return;
                }
                
                // For non-fatal errors (transient), keep original and continue
                warn!(
                    "Translation failed for {}:{}, keeping original: {:?}",
                    segment.relative_path, segment.line_number, error
                );
                
                let log_message = format!(
                    "{} {}행 번역 실패, 원본 유지: {}",
                    segment.relative_path, segment.line_number,
                    error.to_string().chars().take(100).collect::<String>()
                );
                
                file_errors.push(TranslationFileErrorEntry {
                    file_path: segment.relative_path.clone(),
                    message: format!("Line {}: {}", segment.line_number, error),
                    code: Some(error_code_for(&error).into()),
                });
                
                rolled_back_segments.push(format!(
                    "{}:{}",
                    segment.relative_path, segment.line_number
                ));
                
                // Continue processing with original text
                processed_segments = processed + 1;
                last_file_name = Some(segment.relative_path.clone());
                last_file_success = Some(false);
                update_checkpoint_for_next_segment(&mut job_state, &segments, processed_segments);
                save_job_state(&payload.job_id, job_state.clone());
                
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        status: "running".into(),
                        progress_pct: Some(percentage(processed_segments, total_segments)),
                        cancel_requested: None,
                        log: Some(log_message),
                        translated_count: Some(processed_segments),
                        total_count: Some(total_segments),
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: clone_errors(&file_errors),
                        last_written: None,
                        checkpoint: Some(job_state.checkpoint.clone()),
                        retry: None,
                    },
                );
                
                continue; // Continue to next segment instead of returning
            };

            if let Some(context) = file_contexts.get_mut(segment.file_index) {
                if segment.line_index < context.translated_lines.len() {
                    if apply_translation {
                        let translated_ref = translated_value.as_str();
                        let replacement =
                            format!("{}{}{}", segment.prefix, translated_ref, segment.suffix);
                        context.translated_lines[segment.line_index] = Some(replacement.clone());
                        if let Some(progress) = job_state.files.get_mut(&segment.relative_path) {
                            progress
                                .replacements
                                .insert(segment.line_index, replacement);
                        }
                    } else {
                        context.translated_lines[segment.line_index] = None;
                        if let Some(progress) = job_state.files.get_mut(&segment.relative_path) {
                            progress.replacements.remove(&segment.line_index);
                        }
                    }
                }
            }

            processed_segments = processed + 1;
            last_file_name = Some(segment.relative_path.clone());
            last_file_success = Some(apply_translation);
            update_checkpoint_for_next_segment(&mut job_state, &segments, processed_segments);
            save_job_state(&payload.job_id, job_state.clone());

            let progress_log = if apply_translation {
                format!(
                    "{} {}행 번역 완료",
                    segment.relative_path, segment.line_number
                )
            } else if let Some(messages) = &qc_messages {
                format!(
                    "QC 검증으로 원본 유지: {} {}행 ({})",
                    segment.relative_path,
                    segment.line_number,
                    messages.join("; ")
                )
            } else {
                format!(
                    "{} {}행 번역 결과 변경 없음",
                    segment.relative_path, segment.line_number
                )
            };

            emit_progress(
                &app,
                TranslationProgressEventPayload {
                    job_id: payload.job_id.clone(),
                    status: "running".into(),
                    progress_pct: Some(percentage(processed_segments, total_segments)),
                    cancel_requested: None,
                    log: Some(progress_log),
                    translated_count: Some(processed_segments),
                    total_count: Some(total_segments),
                    file_name: last_file_name.clone(),
                    file_success: last_file_success,
                    file_errors: clone_errors(&file_errors),
                    last_written: None,
                    checkpoint: Some(job_state.checkpoint.clone()),
                    retry: None,
                },
            );
        }
    }

    if cancel_flag.load(Ordering::SeqCst) {
        emit_cancelled_progress(
            &app,
            &payload,
            processed_segments,
            total_segments,
            &last_file_name,
            last_file_success,
            &file_errors,
        );
        return;
    }

    // 일반 파일 저장 (아카이브 내부 파일은 별도 처리)
    for context in &mut file_contexts {
        if cancel_flag.load(Ordering::SeqCst) {
            emit_cancelled_progress(
                &app,
                &payload,
                processed_segments,
                total_segments,
                &last_file_name,
                last_file_success,
                &file_errors,
            );
            return;
        }

        // 아카이브 내부 파일은 나중에 일괄 처리
        if context.archive_path.is_some() {
            continue;
        }

        let output_absolute_path = context.output_absolute_path.clone();
        let output_relative = context.output_relative_path.clone();

        if let Some(parent_dir) = output_absolute_path.parent() {
            if let Err(err) = fs::create_dir_all(parent_dir) {
                let message = format!(
                    "Failed to write {}: {}",
                    output_absolute_path.to_string_lossy(),
                    err
                );
                last_file_name = Some(context.relative_path.clone());
                last_file_success = Some(false);
                file_errors.push(TranslationFileErrorEntry {
                    file_path: context.relative_path.clone(),
                    message: message.clone(),
                    code: Some("WRITE_FAILED".into()),
                });
                save_job_state(&payload.job_id, job_state.clone());
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        status: "running".into(),
                        progress_pct: Some(percentage(processed_segments, total_segments)),
                        cancel_requested: None,
                        log: Some(message),
                        translated_count: Some(processed_segments),
                        total_count: Some(total_segments),
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: clone_errors(&file_errors),
                        last_written: None,
                        checkpoint: Some(job_state.checkpoint.clone()),
                        retry: None,
                    },
                );
                continue;
            }
        }

        let contents = render_translated_file(context);
        let write_result = if output_absolute_path.exists() {
            match backup_and_swap(&output_absolute_path, contents.as_bytes()) {
                Ok(outcome) => Ok(outcome.backup_path),
                Err(err) => Err(err.to_string()),
            }
        } else {
            match File::create(&output_absolute_path) {
                Ok(mut file) => {
                    if let Err(err) = file.write_all(contents.as_bytes()) {
                        Err(err.to_string())
                    } else if let Err(err) = file.sync_all() {
                        Err(err.to_string())
                    } else {
                        Ok(None)
                    }
                }
                Err(err) => Err(err.to_string()),
            }
        };

        let backup_display = match write_result {
            Ok(backup_path) => backup_path
                .map(|path| path.to_string_lossy().to_string())
                .filter(|path| !path.is_empty()),
            Err(message) => {
                let log_message = format!(
                    "Failed to write {}: {}",
                    output_absolute_path.to_string_lossy(),
                    message
                );
                last_file_name = Some(context.relative_path.clone());
                last_file_success = Some(false);
                file_errors.push(TranslationFileErrorEntry {
                    file_path: context.relative_path.clone(),
                    message: log_message.clone(),
                    code: Some("WRITE_FAILED".into()),
                });
                save_job_state(&payload.job_id, job_state.clone());
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        status: "running".into(),
                        progress_pct: Some(percentage(processed_segments, total_segments)),
                        cancel_requested: None,
                        log: Some(log_message),
                        translated_count: Some(processed_segments),
                        total_count: Some(total_segments),
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: clone_errors(&file_errors),
                        last_written: None,
                        checkpoint: Some(job_state.checkpoint.clone()),
                        retry: None,
                    },
                );
                continue;
            }
        };

        let absolute_display = output_absolute_path
            .canonicalize()
            .unwrap_or_else(|_| output_absolute_path.clone())
            .to_string_lossy()
            .to_string();
        let output_relative_display = normalize_relative_display(&output_relative);

        last_file_name = Some(context.relative_path.clone());
        last_file_success = Some(true);

        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                status: "running".into(),
                progress_pct: Some(percentage(processed_segments, total_segments)),
                cancel_requested: None,
                log: Some(match &backup_display {
                    Some(backup) => format!(
                        "{} 번역 결과를 저장했습니다. (백업: {})",
                        context.relative_path, backup
                    ),
                    None => format!("{} 번역 결과를 저장했습니다.", context.relative_path),
                }),
                translated_count: Some(processed_segments),
                total_count: Some(total_segments),
                file_name: last_file_name.clone(),
                file_success: last_file_success,
                file_errors: clone_errors(&file_errors),
                last_written: Some(LastWrittenInfo {
                    source_relative_path: context.relative_path.clone(),
                    output_absolute_path: absolute_display,
                    output_relative_path: output_relative_display,
                }),
                checkpoint: Some(job_state.checkpoint.clone()),
                retry: None,
            },
        );
    }

    // 아카이브 내부 파일 일괄 저장
    let archive_contexts: Vec<&FileContext> = file_contexts.iter()
        .filter(|c| c.archive_path.is_some())
        .collect();
    
    if !archive_contexts.is_empty() {
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                status: "running".into(),
                progress_pct: Some(percentage(processed_segments, total_segments)),
                cancel_requested: None,
                log: Some("아카이브 파일을 저장하는 중...".into()),
                translated_count: Some(processed_segments),
                total_count: Some(total_segments),
                file_name: None,
                file_success: None,
                file_errors: clone_errors(&file_errors),
                last_written: None,
                checkpoint: Some(job_state.checkpoint.clone()),
                retry: None,
            },
        );

        // 아카이브별로 그룹화하여 저장
        let archive_save_contexts: Vec<FileContext> = file_contexts.iter()
            .filter(|c| c.archive_path.is_some())
            .cloned()
            .collect();
        
        match save_archive_translations(&archive_save_contexts, &target_lang) {
            Ok(results) => {
                for (archive_path, count) in results {
                    let archive_name = archive_path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| archive_path.to_string_lossy().to_string());
                    
                    emit_progress(
                        &app,
                        TranslationProgressEventPayload {
                            job_id: payload.job_id.clone(),
                            status: "running".into(),
                            progress_pct: Some(percentage(processed_segments, total_segments)),
                            cancel_requested: None,
                            log: Some(format!("{} 아카이브에 {}개 파일 저장 완료", archive_name, count)),
                            translated_count: Some(processed_segments),
                            total_count: Some(total_segments),
                            file_name: Some(archive_name),
                            file_success: Some(true),
                            file_errors: clone_errors(&file_errors),
                            last_written: None,
                            checkpoint: Some(job_state.checkpoint.clone()),
                            retry: None,
                        },
                    );
                }
            }
            Err(err) => {
                file_errors.push(TranslationFileErrorEntry {
                    file_path: "archive".into(),
                    message: err.clone(),
                    code: Some("ARCHIVE_WRITE_FAILED".into()),
                });
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        status: "running".into(),
                        progress_pct: Some(percentage(processed_segments, total_segments)),
                        cancel_requested: None,
                        log: Some(format!("아카이브 저장 실패: {}", err)),
                        translated_count: Some(processed_segments),
                        total_count: Some(total_segments),
                        file_name: None,
                        file_success: Some(false),
                        file_errors: clone_errors(&file_errors),
                        last_written: None,
                        checkpoint: Some(job_state.checkpoint.clone()),
                        retry: None,
                    },
                );
            }
        }
    }

    let final_status = if !file_errors.is_empty() {
        if processed_segments > 0 || total_segments > 0 {
            "partial_success"
        } else {
            "failed"
        }
    } else {
        "completed"
    };
    let mut final_log = if total_segments == 0 && file_errors.is_empty() {
        "번역할 문자열이 없습니다.".to_string()
    } else if final_status == "completed" {
        "번역이 완료되었습니다.".to_string()
    } else if final_status == "partial_success" {
        "일부 파일을 번역하거나 저장하지 못했습니다.".to_string()
    } else {
        "번역을 완료하지 못했습니다.".to_string()
    };

    if !rolled_back_segments.is_empty() {
        let preview: Vec<_> = rolled_back_segments.iter().take(3).cloned().collect();
        let remainder = rolled_back_segments.len().saturating_sub(preview.len());
        if !preview.is_empty() {
            final_log.push(' ');
            let detail = if remainder > 0 {
                format!("{} 외 {}건", preview.join(", "), remainder)
            } else {
                preview.join(", ")
            };
            final_log.push_str(&format!(
                "자동 롤백 {}건 ({detail}).",
                rolled_back_segments.len()
            ));
        }
    }

    let mut final_progress = if total_segments == 0 {
        100.0
    } else {
        percentage(processed_segments, total_segments)
    };
    if final_progress < 100.0 && final_status != "failed" {
        final_progress = 100.0;
    }

    if final_status == "completed" {
        clear_job_state(&payload.job_id);
    } else {
        save_job_state(&payload.job_id, job_state.clone());
    }

    emit_progress(
        &app,
        TranslationProgressEventPayload {
            job_id: payload.job_id,
            status: final_status.into(),
            progress_pct: Some(final_progress),
            cancel_requested: None,
            log: Some(final_log),
            translated_count: Some(processed_segments),
            total_count: Some(total_segments),
            file_name: last_file_name,
            file_success: last_file_success,
            file_errors: clone_errors(&file_errors),
            last_written: None,
            checkpoint: if final_status == "completed" {
                None
            } else {
                Some(job_state.checkpoint.clone())
            },
            retry: None,
        },
    );

    if final_status == "completed" {
        for context in &file_contexts {
            if let Err(err) = clear_resume_metadata(&context.resume_metadata_path) {
                warn!(
                    "failed to clean resume metadata for {}: {}",
                    context.relative_path, err
                );
            }
        }
    }
}

fn clone_errors(errors: &[TranslationFileErrorEntry]) -> Option<Vec<TranslationFileErrorEntry>> {
    if errors.is_empty() {
        None
    } else {
        Some(errors.to_vec())
    }
}

fn derive_output_relative_path(relative_path: &str, target_lang: &str) -> PathBuf {
    let path = Path::new(relative_path);
    let mut result = PathBuf::new();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            result.push(parent);
        }
    }

    let output_name = path
        .file_name()
        .map(|name| build_output_filename(name, target_lang))
        .unwrap_or_else(|| {
            let lang = sanitized_language_tag(target_lang);
            format!("translated.{lang}")
        });

    result.push(output_name);
    result
}

fn sanitized_language_tag(target_lang: &str) -> String {
    let trimmed = target_lang.trim();
    if trimmed.is_empty() {
        "translated".to_string()
    } else {
        trimmed.to_lowercase()
    }
}

fn build_output_filename(source_name: &OsStr, target_lang: &str) -> String {
    let lang = sanitized_language_tag(target_lang);
    let temp = PathBuf::from(source_name);
    let stem = temp.file_stem().and_then(|s| s.to_str());
    let extension = temp.extension().and_then(|s| s.to_str());

    match (stem, extension) {
        (Some(stem), Some(ext)) => format!("{stem}.{lang}.{ext}"),
        (Some(stem), None) => format!("{stem}.{lang}"),
        (None, Some(ext)) => format!("translated.{lang}.{ext}"),
        (None, None) => format!("translated.{lang}"),
    }
}

fn render_translated_file(context: &FileContext) -> String {
    if context.lines.is_empty() {
        return if context.had_trailing_newline {
            "\n".to_string()
        } else {
            String::new()
        };
    }

    let mut buffer = String::new();
    for (index, original_line) in context.lines.iter().enumerate() {
        if index > 0 {
            buffer.push('\n');
        }

        if let Some(replacement) = &context.translated_lines[index] {
            buffer.push_str(replacement);
        } else {
            buffer.push_str(original_line);
        }
    }

    if context.had_trailing_newline {
        buffer.push('\n');
    }

    buffer
}

/// 아카이브 파일에 번역된 내용 저장
/// 
/// 같은 아카이브에 속한 모든 번역된 파일을 한 번에 처리합니다.
fn save_archive_translations(
    contexts: &[FileContext],
    target_lang: &str,
) -> Result<Vec<(PathBuf, usize)>, String> {
    // 아카이브별로 컨텍스트 그룹화
    let mut archive_groups: HashMap<PathBuf, Vec<&FileContext>> = HashMap::new();
    
    for context in contexts {
        if let Some(archive_path) = &context.archive_path {
            archive_groups
                .entry(archive_path.clone())
                .or_default()
                .push(context);
        }
    }
    
    let mut results = Vec::new();
    
    for (archive_path, group_contexts) in archive_groups {
        let mut modifications = ArchiveModification::new();
        
        for context in &group_contexts {
            if let Some(entry_path) = &context.archive_entry_path {
                let translated_content = render_translated_file(context);
                
                // 대상 언어로 경로 변환 (예: en_us.json -> ko_kr.json)
                let target_entry_path = derive_archive_entry_output_path(entry_path, target_lang);
                
                // 원본 경로와 다른 경우 새 파일로 추가, 같으면 업데이트
                if target_entry_path != *entry_path {
                    modifications.add_file_string(&target_entry_path, &translated_content);
                } else {
                    modifications.update_file_string(entry_path, &translated_content);
                }
            }
        }
        
        if modifications.is_empty() {
            continue;
        }
        
        // 백업 디렉토리 생성
        let backup_dir = archive_path.parent()
            .map(|p| p.join(".backup"))
            .unwrap_or_else(|| PathBuf::from(".backup"));
        
        // 아카이브 수정 적용
        archive::update_archive_with_translations(
            &archive_path,
            modifications.updates.into_iter()
                .chain(modifications.additions.into_iter())
                .map(|(k, v)| (k, String::from_utf8_lossy(&v).to_string()))
                .collect(),
            Some(&backup_dir),
        ).map_err(|e| format!("아카이브 수정 실패: {}", e))?;
        
        results.push((archive_path, group_contexts.len()));
    }
    
    Ok(results)
}

/// 아카이브 내부 파일 경로를 대상 언어 경로로 변환
fn derive_archive_entry_output_path(entry_path: &str, target_lang: &str) -> String {
    // 마인크래프트 언어 파일 패턴 처리 (en_us.json -> ko_kr.json)
    if let Some(new_path) = archive::minecraft_lang_target_path(entry_path, target_lang) {
        return new_path;
    }
    
    // 일반적인 언어 코드 패턴 처리
    let path = Path::new(entry_path);
    if let (Some(stem), Some(ext)) = (path.file_stem(), path.extension()) {
        let stem_str = stem.to_string_lossy();
        let ext_str = ext.to_string_lossy();
        
        // 파일명에 언어 코드가 있는 경우 (예: messages_en.json)
        let lang_patterns = ["_en", "_eng", "_english", ".en", ".eng", ".english"];
        for pattern in lang_patterns {
            if stem_str.to_lowercase().ends_with(pattern) {
                let base = &stem_str[..stem_str.len() - pattern.len()];
                let new_name = format!("{}_{}.{}", base, target_lang, ext_str);
                if let Some(parent) = path.parent() {
                    return parent.join(new_name).to_string_lossy().replace('\\', "/");
                }
                return new_name;
            }
        }
    }
    
    // 변환할 수 없는 경우 원본 경로 반환 (덮어쓰기)
    entry_path.to_string()
}

fn normalize_relative_display(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct ResumeMetadata {
    next_line_index: usize,
}

fn build_resume_metadata_path(output_path: &Path) -> PathBuf {
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let metadata_dir = parent.join(RESUME_DIR_NAME);
    let file_name = output_path
        .file_name()
        .map(|name| format!("{}.resume.json", name.to_string_lossy()))
        .unwrap_or_else(|| "resume.json".to_string());
    metadata_dir.join(file_name)
}

#[allow(dead_code)]
fn load_resume_metadata(path: &Path) -> Option<ResumeMetadata> {
    if !path.exists() {
        return None;
    }

    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<ResumeMetadata>(&contents) {
            Ok(metadata) => Some(metadata),
            Err(error) => {
                warn!(
                    "failed to parse resume metadata {}: {}",
                    path.to_string_lossy(),
                    error
                );
                None
            }
        },
        Err(error) => {
            warn!(
                "failed to read resume metadata {}: {}",
                path.to_string_lossy(),
                error
            );
            None
        }
    }
}

#[allow(dead_code)]
fn save_resume_metadata(path: &Path, metadata: &ResumeMetadata) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("resume metadata directory error: {err}"))?;
    }
    let serialized = serde_json::to_vec(metadata)
        .map_err(|err| format!("resume metadata serialization error: {err}"))?;
    fs::write(path, serialized).map_err(|err| format!("resume metadata write error: {err}"))
}

fn clear_resume_metadata(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove resume metadata: {err}")),
    }
}

#[allow(dead_code)]
fn persist_partial_translation(context: &FileContext) -> Result<(), String> {
    if let Some(parent) = context.output_absolute_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("output directory creation failed: {err}"))?;
    }

    let contents = render_translated_file(context);
    fs::write(&context.output_absolute_path, contents)
        .map_err(|err| format!("partial output write failed: {err}"))?;

    let metadata = ResumeMetadata {
        next_line_index: context.resume_line_index,
    };
    save_resume_metadata(&context.resume_metadata_path, &metadata)
}

fn should_retry_error(error: &TranslationError) -> bool {
    matches!(
        error,
        TranslationError::RateLimited { .. }
            | TranslationError::NetworkTransient { .. }
            | TranslationError::ServerTransient { .. }
    )
}

fn emit_progress(app: &AppHandle, payload: TranslationProgressEventPayload) {
    if let Err(error) = app.emit("translation-progress", payload) {
        warn!("failed to emit translation progress: {}", error);
    }
}

fn emit_cancelled_progress(
    app: &AppHandle,
    payload: &StartTranslationJobPayload,
    processed: u32,
    total: u32,
    last_file_name: &Option<String>,
    last_file_success: Option<bool>,
    file_errors: &[TranslationFileErrorEntry],
) {
    emit_progress(
        app,
        TranslationProgressEventPayload {
            job_id: payload.job_id.clone(),
            status: "canceled".into(),
            progress_pct: Some(percentage(processed, total)),
            cancel_requested: Some(true),
            log: Some("사용자가 작업을 중단했습니다.".into()),
            translated_count: Some(processed),
            total_count: Some(total),
            file_name: last_file_name.clone(),
            file_success: last_file_success,
            file_errors: clone_errors(file_errors),
            last_written: None,
            checkpoint: None,
            retry: None,
        },
    );
}

fn percentage(processed: u32, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    ((processed as f32) / (total as f32) * 100.0).clamp(0.0, 100.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackoffWaitOutcome {
    Completed,
    Manual,
    JobCancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackoffCancelSource {
    User,
    Logic,
}

impl BackoffCancelSource {
    fn as_str(&self) -> &'static str {
        match self {
            BackoffCancelSource::User => "user",
            BackoffCancelSource::Logic => "logic",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackoffStartedEventPayload {
    job_id: String,
    delay_ms: u64,
    attempt: u32,
    max_attempts: u32,
    reason: String,
    used_hint: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackoffCancelledEventPayload {
    job_id: String,
    by: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RetryStartedEventPayload {
    job_id: String,
    attempt: u32,
}

fn emit_backoff_started(
    app: &AppHandle,
    job_id: &str,
    delay: Duration,
    attempt: u32,
    used_hint: bool,
    reason: &str,
) {
    let delay_ms = delay.as_millis().min(u64::MAX as u128) as u64;
    let payload = BackoffStartedEventPayload {
        job_id: job_id.to_string(),
        delay_ms,
        attempt,
        max_attempts: MAX_RETRY_ATTEMPTS as u32,
        reason: reason.to_string(),
        used_hint,
    };

    if let Err(error) = app.emit("translation-backoff-started", payload) {
        warn!("failed to emit translation-backoff-started: {}", error);
    }
}

fn emit_backoff_cancelled(app: &AppHandle, job_id: &str, source: BackoffCancelSource) {
    let payload = BackoffCancelledEventPayload {
        job_id: job_id.to_string(),
        by: source.as_str().to_string(),
    };

    if let Err(error) = app.emit("translation-backoff-cancelled", payload) {
        warn!("failed to emit translation-backoff-cancelled: {}", error);
    }
}

fn emit_retry_started(app: &AppHandle, job_id: &str, attempt: u32) {
    let payload = RetryStartedEventPayload {
        job_id: job_id.to_string(),
        attempt,
    };

    if let Err(error) = app.emit("translation-retry-started", payload) {
        warn!("failed to emit translation-retry-started: {}", error);
    }
}

async fn wait_with_cancellation(
    app: &AppHandle,
    job_id: &str,
    cancel_flag: &Arc<AtomicBool>,
    controller: Arc<BackoffController>,
    duration: Duration,
) -> BackoffWaitOutcome {
    if duration.is_zero() {
        if controller.is_active() {
            if let Some(state) = controller.take() {
                let source = if state.manual_triggered {
                    BackoffCancelSource::User
                } else {
                    BackoffCancelSource::Logic
                };
                emit_backoff_cancelled(app, job_id, source);
            }
        }
        return BackoffWaitOutcome::Completed;
    }

    if cancel_flag.load(Ordering::SeqCst) {
        if controller.is_active() {
            if let Some(state) = controller.take() {
                let source = if state.manual_triggered {
                    BackoffCancelSource::User
                } else {
                    BackoffCancelSource::Logic
                };
                emit_backoff_cancelled(app, job_id, source);
            }
        }
        return BackoffWaitOutcome::JobCancelled;
    }

    let sleep_future = sleep(duration);
    tokio::pin!(sleep_future);

    let cancel_future = wait_for_cancel_flag(cancel_flag.clone());
    tokio::pin!(cancel_future);

    let manual_future = controller.notified();
    tokio::pin!(manual_future);

    let outcome = tokio::select! {
        _ = &mut sleep_future => BackoffWaitOutcome::Completed,
        _ = &mut manual_future => BackoffWaitOutcome::Manual,
        _ = &mut cancel_future => BackoffWaitOutcome::JobCancelled,
    };

    let source = controller.take().map(|state| {
        if state.manual_triggered {
            BackoffCancelSource::User
        } else {
            BackoffCancelSource::Logic
        }
    });

    if let Some(source) = source {
        emit_backoff_cancelled(app, job_id, source);
    }

    outcome
}

async fn wait_for_cancel_flag(cancel_flag: Arc<AtomicBool>) {
    if cancel_flag.load(Ordering::SeqCst) {
        return;
    }

    loop {
        sleep(Duration::from_millis(50)).await;
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }
    }
}

fn error_code_for(error: &TranslationError) -> &'static str {
    match error {
        TranslationError::RateLimited { .. } => "RATE_LIMITED",
        TranslationError::NetworkTransient { .. } => "NETWORK_TRANSIENT",
        TranslationError::ServerTransient { .. } => "SERVER_TRANSIENT",
        TranslationError::Unauthorized { .. } => "UNAUTHORIZED",
        TranslationError::Forbidden { .. } => "FORBIDDEN",
        TranslationError::ModelNotFound { .. } => "MODEL_NOT_FOUND",
        TranslationError::PlaceholderMismatch(_) => "PLACEHOLDER_MISMATCH",
        TranslationError::IoError { .. } => "IO_ERROR",
    }
}

fn format_translation_error(segment: &Segment, error: &TranslationError) -> String {
    let location = format!("{} {}행", segment.relative_path, segment.line_number);
    match error {
        TranslationError::Unauthorized { message, .. } => {
            format!("{location} 번역 중 API 키가 거부되었습니다: {message}")
        }
        TranslationError::Forbidden { message, .. } => {
            format!("{location} 번역 중 요청이 거부되었습니다: {message}")
        }
        TranslationError::ModelNotFound {
            model_id, message, ..
        } => {
            format!("{location} 번역 중 모델 '{model_id}'을(를) 사용할 수 없습니다: {message}")
        }
        TranslationError::RateLimited { message, .. } => {
            format!("{location} 번역 중 429 응답으로 제한되었습니다: {message}")
        }
        TranslationError::NetworkTransient { message, .. } => {
            format!("{location} 번역 중 네트워크 오류가 발생했습니다: {message}")
        }
        TranslationError::ServerTransient {
            status, message, ..
        } => {
            let status_text = status
                .map(|code| format!(" (상태 {code})"))
                .unwrap_or_default();
            format!("{location} 번역 중 서버 오류가 발생했습니다{status_text}: {message}")
        }
        TranslationError::IoError { message, .. } => {
            format!("{location} 번역 파일 처리 중 I/O 오류가 발생했습니다: {message}")
        }
        TranslationError::PlaceholderMismatch(missing) => {
            if missing.is_empty() {
                format!("{location} 번역 중 자리표시자 검증에 실패했습니다.")
            } else {
                format!("{location} 번역 중 자리표시자 누락: {}", missing.join(", "))
            }
        }
    }
}

fn format_file_error_message(segment: &Segment, error: &TranslationError) -> String {
    match error {
        TranslationError::ModelNotFound {
            model_id, message, ..
        } => {
            format!("The selected model '{model_id}' is not available: {message}")
        }
        TranslationError::Unauthorized { message, .. } => {
            format!("The API key was rejected by the provider: {message}")
        }
        TranslationError::Forbidden { message, .. } => {
            format!("The provider rejected the request: {message}")
        }
        TranslationError::RateLimited { message, .. } => {
            format!("Translation was rate limited by the provider: {message}")
        }
        TranslationError::NetworkTransient { message, .. } => {
            format!("Translation request failed due to a transient network error: {message}")
        }
        TranslationError::ServerTransient {
            status, message, ..
        } => {
            let status_text = status
                .map(|code| format!(" (status {code})"))
                .unwrap_or_default();
            format!("Translation request failed due to a server-side error{status_text}: {message}")
        }
        TranslationError::IoError { message, .. } => {
            format!("A local I/O error occurred while processing the file: {message}")
        }
        TranslationError::PlaceholderMismatch(_) => format_translation_error(segment, error),
    }
}
