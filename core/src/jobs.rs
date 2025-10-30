use crate::ai::{translate_text, ProviderId, TranslationError};
use log::warn;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

static ACTIVE_JOBS: Lazy<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const DEFAULT_MAX_RETRY_ATTEMPTS: u32 = 4;
const DEFAULT_RETRY_INITIAL_DELAY_MS: u64 = 1_000;
const DEFAULT_RETRY_MAX_DELAY_MS: u64 = 8_000;
const DEFAULT_RETRY_MULTIPLIER: f64 = 2.0;

#[derive(Clone, Copy, Debug)]
struct TranslationRetryPolicy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    multiplier: f64,
}

impl TranslationRetryPolicy {
    fn delay_after_failure(&self, attempt: u32) -> Duration {
        if attempt >= self.max_attempts {
            return Duration::ZERO;
        }

        let exponent = attempt.saturating_sub(1) as i32;
        let mut delay = if exponent <= 0 {
            self.initial_delay
        } else {
            self.initial_delay.mul_f64(self.multiplier.powi(exponent))
        };

        if delay > self.max_delay {
            delay = self.max_delay;
        }

        delay
    }

    fn has_remaining_attempts(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }

    fn next_attempt_index(&self, attempt: u32) -> Option<u32> {
        if self.has_remaining_attempts(attempt) {
            Some(attempt + 1)
        } else {
            None
        }
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

impl Default for TranslationRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_RETRY_ATTEMPTS,
            initial_delay: Duration::from_millis(DEFAULT_RETRY_INITIAL_DELAY_MS),
            max_delay: Duration::from_millis(DEFAULT_RETRY_MAX_DELAY_MS),
            multiplier: DEFAULT_RETRY_MULTIPLIER,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFileInput {
    pub relative_path: String,
    pub mod_install_path: String,
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

struct FileContext {
    relative_path: String,
    mod_install_path: PathBuf,
    lines: Vec<String>,
    translated_lines: Vec<Option<String>>,
    had_trailing_newline: bool,
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
    };
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
            },
        );
        return Err("번역에 사용할 모델을 선택해 주세요.".into());
    }

    {
        let mut guard = ACTIVE_JOBS
            .lock()
            .map_err(|_| "job registry lock poisoned".to_string())?;
        guard.insert(payload.job_id.clone(), Arc::new(AtomicBool::new(false)));
    }

    let job_id = payload.job_id.clone();
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let cancel_flag = {
            let guard = ACTIVE_JOBS
                .lock()
                .expect("job registry lock poisoned during spawn");
            guard
                .get(&job_id)
                .cloned()
                .expect("cancel flag should exist for active job")
        };

        run_translation_job(
            app_handle.clone(),
            payload,
            provider,
            api_key.trim().to_string(),
            cancel_flag,
        )
        .await;

        if let Ok(mut guard) = ACTIVE_JOBS.lock() {
            guard.remove(&job_id);
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
            },
        );

        Ok(())
    } else {
        drop(guard);

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

async fn run_translation_job(
    app: AppHandle,
    payload: StartTranslationJobPayload,
    provider: ProviderId,
    api_key: String,
    cancel_flag: Arc<AtomicBool>,
) {
    let source_lang = payload.source_lang.as_deref().unwrap_or("auto").to_string();
    let target_lang = payload.target_lang.as_deref().unwrap_or("ko").to_string();
    let override_root = payload
        .output_override_dir
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    let mut file_contexts: Vec<FileContext> = Vec::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut file_errors: Vec<TranslationFileErrorEntry> = Vec::new();

    for file in &payload.files {
        let relative_path = PathBuf::from(&file.relative_path);
        let mod_root_raw = PathBuf::from(&file.mod_install_path);
        let mod_root = mod_root_raw.canonicalize().unwrap_or(mod_root_raw.clone());
        let source_file_path = mod_root.join(&relative_path);

        let content = match fs::read_to_string(&source_file_path) {
            Ok(value) => value,
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
        };

        let had_trailing_newline = content.ends_with('\n');
        let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();

        let mut context = FileContext {
            relative_path: file.relative_path.clone(),
            mod_install_path: mod_root,
            lines,
            translated_lines: Vec::new(),
            had_trailing_newline,
        };
        context.translated_lines = vec![None; context.lines.len()];

        let file_index = file_contexts.len();
        for (line_index, line) in context.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
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
    let mut processed_segments: u32 = 0;
    let mut last_file_name: Option<String> = None;
    let mut last_file_success: Option<bool> = None;

    emit_progress(
        &app,
        TranslationProgressEventPayload {
            job_id: payload.job_id.clone(),
            status: "running".into(),
            progress_pct: Some(0.0),
            cancel_requested: None,
            log: Some("번역을 준비하는 중입니다.".into()),
            translated_count: Some(0),
            total_count: Some(total_segments),
            file_name: None,
            file_success: None,
            file_errors: clone_errors(&file_errors),
            last_written: None,
        },
    );

    if total_segments > 0 {
        let client = match Client::builder().build() {
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
                        translated_count: Some(0),
                        total_count: Some(total_segments),
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: clone_errors(&file_errors),
                        last_written: None,
                    },
                );
                return;
            }
        };

        let retry_policy = TranslationRetryPolicy::default();

        for (index, segment) in segments.iter().enumerate() {
            let processed = index as u32;

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

            let mut attempt = 0;
            let translated = loop {
                attempt += 1;
                match translate_text(
                    &client,
                    provider,
                    &api_key,
                    &payload.model_id,
                    &segment.text,
                    &source_lang,
                    &target_lang,
                )
                .await
                {
                    Ok(value) => break value,
                    Err(error) => {
                        let can_retry =
                            should_retry(&error) && retry_policy.has_remaining_attempts(attempt);
                        if can_retry {
                            let delay = retry_policy.delay_after_failure(attempt);
                            let next_attempt = retry_policy
                                .next_attempt_index(attempt)
                                .unwrap_or(retry_policy.max_attempts());
                            last_file_name = Some(segment.relative_path.clone());
                            last_file_success = Some(false);
                            let log_message = format!(
                                "{} {}행 번역 실패 (cause: {}). Retrying in {:.1}s… (attempt {}/{})",
                                segment.relative_path,
                                segment.line_number,
                                error_code_for(&error),
                                delay.as_secs_f32(),
                                next_attempt,
                                retry_policy.max_attempts()
                            );
                            emit_progress(
                                &app,
                                TranslationProgressEventPayload {
                                    job_id: payload.job_id.clone(),
                                    status: "running".into(),
                                    progress_pct: Some(percentage(processed, total_segments)),
                                    cancel_requested: None,
                                    log: Some(log_message),
                                    translated_count: Some(processed),
                                    total_count: Some(total_segments),
                                    file_name: Some(segment.relative_path.clone()),
                                    file_success: Some(false),
                                    file_errors: clone_errors(&file_errors),
                                    last_written: None,
                                },
                            );

                            if wait_with_cancellation(&cancel_flag, delay).await {
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

                            continue;
                        }

                        let log_message = format_translation_error(segment, &error);
                        let file_message = format_file_error_message(segment, &error);
                        last_file_name = Some(segment.relative_path.clone());
                        last_file_success = Some(false);
                        file_errors.push(TranslationFileErrorEntry {
                            file_path: segment.relative_path.clone(),
                            message: file_message.clone(),
                            code: Some(error_code_for(&error).into()),
                        });
                        emit_progress(
                            &app,
                            TranslationProgressEventPayload {
                                job_id: payload.job_id.clone(),
                                status: "failed".into(),
                                progress_pct: Some(percentage(processed, total_segments)),
                                cancel_requested: None,
                                log: Some(log_message.clone()),
                                translated_count: Some(processed),
                                total_count: Some(total_segments),
                                file_name: last_file_name.clone(),
                                file_success: last_file_success,
                                file_errors: clone_errors(&file_errors),
                                last_written: None,
                            },
                        );
                        return;
                    }
                }
            };

            if let Some(context) = file_contexts.get_mut(segment.file_index) {
                if segment.line_index < context.translated_lines.len() {
                    let replacement = format!("{}{}{}", segment.prefix, translated, segment.suffix);
                    context.translated_lines[segment.line_index] = Some(replacement);
                }
            }

            processed_segments = processed + 1;
            last_file_name = Some(segment.relative_path.clone());
            last_file_success = Some(true);

            emit_progress(
                &app,
                TranslationProgressEventPayload {
                    job_id: payload.job_id.clone(),
                    status: "running".into(),
                    progress_pct: Some(percentage(processed_segments, total_segments)),
                    cancel_requested: None,
                    log: Some(format!(
                        "{} {}행 번역 완료",
                        segment.relative_path, segment.line_number
                    )),
                    translated_count: Some(processed_segments),
                    total_count: Some(total_segments),
                    file_name: last_file_name.clone(),
                    file_success: last_file_success,
                    file_errors: clone_errors(&file_errors),
                    last_written: None,
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

        let output_relative = derive_output_relative_path(&context.relative_path, &target_lang);
        let base_root = override_root
            .as_ref()
            .cloned()
            .unwrap_or_else(|| context.mod_install_path.clone());
        let output_absolute_path = base_root.join(&output_relative);

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
                    },
                );
                continue;
            }
        }

        let contents = render_translated_file(context);
        if let Err(err) = fs::write(&output_absolute_path, contents) {
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
                },
            );
            continue;
        }

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
                log: Some(format!(
                    "{} 번역 결과를 저장했습니다.",
                    context.relative_path
                )),
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
            },
        );
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
    let final_log = if total_segments == 0 && file_errors.is_empty() {
        "번역할 문자열이 없습니다.".to_string()
    } else if final_status == "completed" {
        "번역이 완료되었습니다.".to_string()
    } else if final_status == "partial_success" {
        "일부 파일을 번역하거나 저장하지 못했습니다.".to_string()
    } else {
        "번역을 완료하지 못했습니다.".to_string()
    };

    let mut final_progress = if total_segments == 0 {
        100.0
    } else {
        percentage(processed_segments, total_segments)
    };
    if final_progress < 100.0 && final_status != "failed" {
        final_progress = 100.0;
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
        },
    );
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

fn normalize_relative_display(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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
        },
    );
}

fn percentage(processed: u32, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    ((processed as f32) / (total as f32) * 100.0).clamp(0.0, 100.0)
}

fn should_retry(error: &TranslationError) -> bool {
    matches!(
        error,
        TranslationError::NetworkOrHttp { .. } | TranslationError::RateLimited { .. }
    )
}

async fn wait_with_cancellation(cancel_flag: &Arc<AtomicBool>, duration: Duration) -> bool {
    if duration.is_zero() {
        return cancel_flag.load(Ordering::SeqCst);
    }

    let mut elapsed = Duration::ZERO;
    let poll_interval = Duration::from_millis(200);

    while elapsed < duration {
        if cancel_flag.load(Ordering::SeqCst) {
            return true;
        }

        let remaining = duration.saturating_sub(elapsed);
        let sleep_for = if remaining <= poll_interval {
            remaining
        } else {
            poll_interval
        };

        if sleep_for.is_zero() {
            break;
        }

        tauri::async_runtime::sleep(sleep_for).await;
        elapsed += sleep_for;
    }

    cancel_flag.load(Ordering::SeqCst)
}

fn error_code_for(error: &TranslationError) -> &'static str {
    match error {
        TranslationError::InvalidApiKey { .. } => "INVALID_API_KEY",
        TranslationError::RateLimited { .. } => "RATE_LIMITED",
        TranslationError::ModelForbiddenOrNotFound { .. } => "MODEL_FORBIDDEN",
        TranslationError::QuotaOrPlanError { .. } => "QUOTA_OR_PLAN",
        TranslationError::NetworkOrHttp { .. } => "NETWORK_ERROR",
        TranslationError::PlaceholderMismatch(_) => "PLACEHOLDER_MISMATCH",
    }
}

fn format_translation_error(segment: &Segment, error: &TranslationError) -> String {
    let location = format!("{} {}행", segment.relative_path, segment.line_number);
    match error {
        TranslationError::InvalidApiKey { message, .. } => {
            format!("{location} 번역 중 API 키가 거부되었습니다: {message}")
        }
        TranslationError::ModelForbiddenOrNotFound {
            model_id, message, ..
        } => {
            format!("{location} 번역 중 모델 '{model_id}'을(를) 사용할 수 없습니다: {message}")
        }
        TranslationError::RateLimited { message, .. } => {
            format!("{location} 번역 중 너무 많은 요청이 감지되었습니다: {message}")
        }
        TranslationError::QuotaOrPlanError { message, .. } => {
            format!("{location} 번역 중 요금제/할당량 제한으로 실패했습니다: {message}")
        }
        TranslationError::NetworkOrHttp { message, .. } => {
            format!("{location} 번역 중 네트워크 오류가 발생했습니다: {message}")
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
        TranslationError::ModelForbiddenOrNotFound {
            model_id, message, ..
        } => {
            format!("The selected model '{model_id}' is not available for this API key: {message}",)
        }
        TranslationError::InvalidApiKey { message, .. } => {
            format!("The API key was rejected by the provider: {message}")
        }
        TranslationError::RateLimited { message, .. } => {
            format!("Translation request was throttled by the provider: {message}")
        }
        TranslationError::QuotaOrPlanError { message, .. } => {
            format!("Translation failed due to plan or quota limits: {message}")
        }
        TranslationError::NetworkOrHttp { message, .. } => {
            format!("Translation request failed due to a network or HTTP error: {message}")
        }
        TranslationError::PlaceholderMismatch(_) => format_translation_error(segment, error),
    }
}
