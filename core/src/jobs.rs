use crate::ai::{ProviderAuth, TranslateOptions, TranslationError, TranslatorKind};
use crate::pipeline::PipelinePlan;
use dirs::data_dir;
use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use uuid::Uuid;

static WORK_QUEUE: Lazy<Mutex<WorkQueue>> = Lazy::new(|| {
    Mutex::new(WorkQueue::new(
        3,                                               // concurrent workers
        RateLimiter::new(5, Duration::from_millis(750)), // 5 tokens per 750ms bucket
    ))
});

#[derive(Debug, Serialize, Clone)]
struct JobStatusEventPayload {
    job_id: String,
    mod_id: String,
    status: TranslationJobStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueSnapshot {
    pub queued: usize,
    pub running: usize,
    pub concurrent_workers: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RateLimiterSnapshot {
    pub bucket_capacity: u32,
    pub tokens_available: u32,
    pub refill_interval_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityGateSnapshot {
    pub placeholder_guard: bool,
    pub format_validator: bool,
    pub sample_rate: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationJobStatus {
    pub job_id: String,
    pub translator: String,
    pub state: JobState,
    pub progress: f32,
    pub preview: Option<String>,
    pub message: Option<String>,
    pub queue: QueueSnapshot,
    pub rate_limiter: RateLimiterSnapshot,
    pub quality_gates: QualityGateSnapshot,
    pub pipeline: PipelinePlan,
    pub cancel_requested: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationJobRequest {
    pub mod_id: String,
    pub mod_name: Option<String>,
    pub translator: TranslatorKind,
    pub source_language_guess: String,
    pub target_language: String,
    pub selected_files: Vec<String>,
    #[serde(default)]
    pub provider_auth: ProviderAuth,
}

#[derive(Debug, Error)]
pub enum JobError {
    #[error("translator failed: {0}")]
    Translation(String),
}

impl From<TranslationError> for JobError {
    fn from(value: TranslationError) -> Self {
        JobError::Translation(value.to_string())
    }
}

#[derive(Debug, Default)]
pub struct TranslationOrchestrator;

impl TranslationOrchestrator {
    pub fn new() -> Self {
        Self
    }

    pub fn start_job(
        &self,
        request: TranslationJobRequest,
        app_handle: Option<AppHandle>,
    ) -> Result<TranslationJobStatus, JobError> {
        let job_id = Uuid::new_v4().to_string();
        let mut translator = request.translator.build_with_auth(&request.provider_auth);
        let options = TranslateOptions {
            source_lang: Some(request.source_language_guess.clone()),
            target_lang: request.target_language.clone(),
            domain: Some(crate::ai::TranslationDomain::Ui),
            style: Some(crate::ai::TranslationStyle::Game),
        };

        let preview = translator
            .translate_preview(
                "This is a synthetic preview of a mod description.",
                &options,
            )
            .map(Some)
            .map_err(JobError::from)?;

        let job_display_name = request
            .mod_name
            .as_deref()
            .unwrap_or(request.mod_id.as_str())
            .to_string();

        let initial_status = TranslationJobStatus {
            job_id: job_id.clone(),
            translator: translator.name().to_string(),
            state: JobState::Queued,
            progress: 0.05,
            preview,
            message: Some(format!(
                "{} 번역 작업이 큐에 등록되었습니다 ({}개 파일).",
                job_display_name,
                request.selected_files.len()
            )),
            queue: QueueSnapshot {
                queued: 0,
                running: 0,
                concurrent_workers: 0,
            },
            rate_limiter: RateLimiterSnapshot {
                bucket_capacity: 0,
                tokens_available: 0,
                refill_interval_ms: 0,
            },
            quality_gates: QualityGateSnapshot {
                placeholder_guard: true,
                format_validator: true,
                sample_rate: 0.05,
            },
            pipeline: PipelinePlan::default_for(&job_display_name),
            cancel_requested: false,
        };

        let job = QueuedJob {
            job_id: job_id.clone(),
            request: request.clone(),
            options: options.clone(),
        };

        let (status, maybe_job_to_start, event_payload, event_handle) = {
            let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
            if let Some(ref handle) = app_handle {
                queue.set_event_handle(handle.clone());
            }
            let (status, maybe_job_to_start, event_payload) =
                queue.register_job(job, initial_status);
            let event_handle = queue.event_handle.clone();
            (status, maybe_job_to_start, event_payload, event_handle)
        };

        if let Some(payload) = event_payload {
            emit_job_status(event_handle, payload);
        }

        append_job_log(&status);

        if let Some(job_to_start) = maybe_job_to_start {
            spawn_job_worker(job_to_start);
        }

        Ok(status)
    }
}

fn append_job_log(job: &TranslationJobStatus) {
    let Some(mut base_dir) = data_dir() else {
        warn!("data_dir unavailable; skipping job log persistence");
        return;
    };

    base_dir.push("mod-translator");
    base_dir.push("logs");

    if let Err(err) = create_dir_all(&base_dir) {
        warn!("failed to prepare log directory {:?}: {}", base_dir, err);
        return;
    }

    let log_file = base_dir.join("jobs.log");
    let serialized = match serde_json::to_string(job) {
        Ok(value) => value,
        Err(err) => {
            warn!("failed to serialize job log: {}", err);
            return;
        }
    };

    match OpenOptions::new().create(true).append(true).open(&log_file) {
        Ok(mut file) => {
            if let Err(err) = writeln!(file, "{}", serialized) {
                warn!("failed to write job log {:?}: {}", log_file, err);
            }
        }
        Err(err) => {
            warn!("failed to open job log {:?}: {}", log_file, err);
        }
    }
}

#[derive(Debug, Clone)]
struct QueuedJob {
    job_id: String,
    request: TranslationJobRequest,
    options: TranslateOptions,
}

struct WorkQueue {
    concurrent_workers: usize,
    running: usize,
    waiting: VecDeque<QueuedJob>,
    rate_limiter: RateLimiter,
    statuses: HashMap<String, TranslationJobStatus>,
    event_handle: Option<AppHandle>,
    job_mod_index: HashMap<String, String>,
    cancel_requests: HashSet<String>,
}

impl WorkQueue {
    fn new(concurrent_workers: usize, rate_limiter: RateLimiter) -> Self {
        Self {
            concurrent_workers,
            running: 0,
            waiting: VecDeque::new(),
            rate_limiter,
            statuses: HashMap::new(),
            event_handle: None,
            job_mod_index: HashMap::new(),
            cancel_requests: HashSet::new(),
        }
    }

    fn register_job(
        &mut self,
        job: QueuedJob,
        mut status: TranslationJobStatus,
    ) -> (
        TranslationJobStatus,
        Option<QueuedJob>,
        Option<JobStatusEventPayload>,
    ) {
        let job_id = job.job_id.clone();
        let mod_id = job.request.mod_id.clone();
        let mut start_immediately = None;
        if self.running < self.concurrent_workers {
            self.running += 1;
            start_immediately = Some(job);
        } else {
            self.waiting.push_back(job);
        }

        self.job_mod_index.insert(job_id.clone(), mod_id);
        status.queue = self.queue_snapshot();
        status.rate_limiter = self.rate_limiter.snapshot();
        status.cancel_requested = false;
        self.statuses.insert(job_id.clone(), status.clone());

        let payload = self.build_payload(&status);

        (status, start_immediately, payload)
    }

    fn request_cancel(&mut self, job_id: &str) -> Result<Option<JobStatusEventPayload>, String> {
        let status = self
            .statuses
            .get(job_id)
            .cloned()
            .ok_or_else(|| format!("job {job_id} not found"))?;

        if matches!(
            status.state,
            JobState::Completed | JobState::Failed | JobState::Canceled
        ) {
            return Err(format!("job {job_id} is not cancellable"));
        }

        self.cancel_requests.insert(job_id.to_string());

        let (_, payload) = self.update_status(job_id, |status| {
            status.cancel_requested = true;
            status.message = Some("사용자가 작업 중단을 요청했습니다.".to_string());
        });

        Ok(payload)
    }

    fn take_cancel_request(&mut self, job_id: &str) -> bool {
        self.cancel_requests.remove(job_id)
    }

    fn update_status<F>(
        &mut self,
        job_id: &str,
        update: F,
    ) -> (Option<TranslationJobStatus>, Option<JobStatusEventPayload>)
    where
        F: FnOnce(&mut TranslationJobStatus),
    {
        let queue_snapshot = self.queue_snapshot();
        let rate_snapshot = self.rate_limiter.snapshot();

        let (status_clone, is_terminal) = {
            let status = match self.statuses.get_mut(job_id) {
                Some(status) => status,
                None => return (None, None),
            };

            update(status);
            status.queue = queue_snapshot.clone();
            status.rate_limiter = rate_snapshot.clone();
            let is_terminal = matches!(
                status.state,
                JobState::Completed | JobState::Failed | JobState::Canceled
            );
            let status_clone = status.clone();

            (status_clone, is_terminal)
        };

        if is_terminal {
            self.job_mod_index.remove(job_id);
        }

        let payload = self.build_payload(&status_clone);

        (Some(status_clone), payload)
    }

    fn finish_job(&mut self, job_id: &str) -> Option<QueuedJob> {
        if self.running > 0 {
            self.running -= 1;
        }

        self.cancel_requests.remove(job_id);

        if let Some(next_job) = self.waiting.pop_front() {
            self.running += 1;
            Some(next_job)
        } else {
            // Job is finished and no queued work.
            let queue_snapshot = self.queue_snapshot();
            self.statuses.get_mut(job_id).map(|status| {
                status.queue = queue_snapshot.clone();
            });
            None
        }
    }

    fn queue_snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            queued: self.waiting.len(),
            running: self.running,
            concurrent_workers: self.concurrent_workers,
        }
    }

    fn reserve_tokens(
        &mut self,
        job_id: &str,
        tokens: u32,
    ) -> (Duration, Option<JobStatusEventPayload>) {
        let wait = self.rate_limiter.reserve(tokens);
        let rate_snapshot = self.rate_limiter.snapshot();

        let status_clone = {
            if let Some(status) = self.statuses.get_mut(job_id) {
                status.rate_limiter = rate_snapshot.clone();
                if wait > Duration::from_millis(0) {
                    status.message = Some(format!(
                        "API 제한으로 인해 {}ms 대기 중입니다.",
                        wait.as_millis()
                    ));
                }
                Some(status.clone())
            } else {
                None
            }
        };

        let payload = status_clone
            .as_ref()
            .and_then(|status| self.build_payload(status));

        (wait, payload)
    }

    fn set_event_handle(&mut self, handle: AppHandle) {
        self.event_handle = Some(handle);
    }

    fn build_payload(&self, status: &TranslationJobStatus) -> Option<JobStatusEventPayload> {
        let mod_id = self.job_mod_index.get(&status.job_id)?.clone();
        Some(JobStatusEventPayload {
            job_id: status.job_id.clone(),
            mod_id,
            status: status.clone(),
        })
    }

    fn job_status(&self, job_id: &str) -> Option<TranslationJobStatus> {
        self.statuses.get(job_id).cloned()
    }
}

fn emit_job_status(handle: Option<AppHandle>, payload: JobStatusEventPayload) {
    if let Some(handle) = handle {
        if let Err(err) = handle.emit("job-status-updated", payload) {
            warn!("failed to emit job status update: {}", err);
        }
    }
}

#[derive(Debug)]
struct RateLimiter {
    capacity: u32,
    available: f64,
    refill_interval: Duration,
    last_refill: Instant,
}

impl RateLimiter {
    fn new(capacity: u32, refill_interval: Duration) -> Self {
        Self {
            capacity,
            available: capacity as f64,
            refill_interval,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last_refill);
        if elapsed.is_zero() {
            return;
        }

        let interval_ms = self.refill_interval.as_millis();
        if interval_ms == 0 {
            self.available = self.capacity as f64;
            self.last_refill = now;
            return;
        }

        let tokens_per_ms = self.capacity as f64 / interval_ms as f64;
        if tokens_per_ms <= 0.0 {
            return;
        }

        let gained = tokens_per_ms * elapsed.as_millis() as f64;
        if gained > 0.0 {
            self.available = (self.available + gained).min(self.capacity as f64);
            self.last_refill = now;
        }
    }

    fn reserve(&mut self, tokens: u32) -> Duration {
        self.refill();

        if tokens == 0 {
            return Duration::from_millis(0);
        }

        let requested = tokens as f64;
        let used_now = self.available.min(requested);
        self.available -= used_now;
        let remaining = requested - used_now;

        if remaining <= 0.0 {
            return Duration::from_millis(0);
        }

        let interval_ms = self.refill_interval.as_millis();
        if interval_ms == 0 {
            return Duration::from_millis(0);
        }

        let tokens_per_ms = self.capacity as f64 / interval_ms as f64;
        if tokens_per_ms <= 0.0 {
            return Duration::from_millis(interval_ms as u64);
        }

        let wait_ms = (remaining / tokens_per_ms).ceil() as u64;
        Duration::from_millis(wait_ms.max(1))
    }

    fn snapshot(&mut self) -> RateLimiterSnapshot {
        self.refill();
        RateLimiterSnapshot {
            bucket_capacity: self.capacity,
            tokens_available: self.available.floor().clamp(0.0, self.capacity as f64) as u32,
            refill_interval_ms: self.refill_interval.as_millis() as u64,
        }
    }
}

fn spawn_job_worker(job: QueuedJob) {
    thread::spawn(move || {
        let job_display_name = job
            .request
            .mod_name
            .as_deref()
            .unwrap_or(job.request.mod_id.as_str())
            .to_string();
        let job_id = job.job_id.clone();

        let (maybe_status, initial_event, initial_handle) = {
            let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
            let (status, event_payload) = queue.update_status(&job.job_id, |status| {
                status.state = JobState::Running;
                status.progress = status.progress.max(0.2);
                status.message = Some(format!("{} 번역을 시작했습니다.", job_display_name));
            });
            let event_handle = queue.event_handle.clone();
            (status, event_payload, event_handle)
        };

        if let Some(status) = maybe_status {
            append_job_log(&status);
        }

        if let Some(payload) = initial_event {
            emit_job_status(initial_handle, payload);
        }

        let cancel_message = format!("{} 작업이 사용자에 의해 중단되었습니다.", job_display_name);
        let mut finalize_cancel = || -> bool {
            let (maybe_status, event_payload, event_handle, next_job) = {
                let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                if !queue.take_cancel_request(&job_id) {
                    return false;
                }
                let next_job = queue.finish_job(&job_id);
                let (status, event_payload) = queue.update_status(&job_id, |status| {
                    status.state = JobState::Canceled;
                    status.progress = status.progress.max(0.0);
                    status.cancel_requested = true;
                    status.message = Some(cancel_message.clone());
                });
                let event_handle = queue.event_handle.clone();
                (status, event_payload, event_handle, next_job)
            };

            if let Some(status) = maybe_status {
                append_job_log(&status);
            }

            if let Some(payload) = event_payload {
                emit_job_status(event_handle, payload);
            }

            if let Some(next_job) = next_job {
                spawn_job_worker(next_job);
            }

            true
        };

        if finalize_cancel() {
            return;
        }

        loop {
            let (wait_duration, throttle_event, throttle_handle) = {
                let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                let (wait, event_payload) = queue.reserve_tokens(&job.job_id, 1);
                let event_handle = queue.event_handle.clone();
                (wait, event_payload, event_handle)
            };

            if let Some(payload) = throttle_event {
                emit_job_status(throttle_handle, payload);
            }

            if finalize_cancel() {
                return;
            }

            if wait_duration.is_zero() {
                break;
            }

            thread::sleep(wait_duration);
        }

        if finalize_cancel() {
            return;
        }

        thread::sleep(Duration::from_millis(400));

        if finalize_cancel() {
            return;
        }

        let mut translator = job
            .request
            .translator
            .build_with_auth(&job.request.provider_auth);

        let sample_inputs = vec![
            format!("{} — UI 문자열 샘플", job_display_name),
            format!("{} — 시스템 로그 문장", job_display_name),
            "Placeholder string {0} 테스트".to_string(),
        ];

        let result = translator.translate_batch(&sample_inputs, &job.options);

        if finalize_cancel() {
            return;
        }

        match result {
            Ok(outputs) => {
                let (maybe_status, event_payload, event_handle) = {
                    let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                    let (status, event_payload) = queue.update_status(&job.job_id, |status| {
                        status.progress = 0.9;
                        if let Some(first) = outputs.first() {
                            status.preview = Some(first.clone());
                        }
                        status.message =
                            Some(format!("{}개의 문자열을 번역했습니다.", outputs.len()));
                    });
                    let event_handle = queue.event_handle.clone();
                    (status, event_payload, event_handle)
                };

                if let Some(status) = maybe_status {
                    append_job_log(&status);
                }

                if let Some(payload) = event_payload {
                    emit_job_status(event_handle, payload);
                }

                if finalize_cancel() {
                    return;
                }

                thread::sleep(Duration::from_millis(300));

                if finalize_cancel() {
                    return;
                }

                let (maybe_status, event_payload, event_handle, next_job) = {
                    let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                    let next_job = queue.finish_job(&job.job_id);
                    let (status, event_payload) = queue.update_status(&job.job_id, |status| {
                        status.state = JobState::Completed;
                        status.progress = 1.0;
                        status.message =
                            Some(format!("{} 번역이 완료되었습니다.", job_display_name));
                    });
                    let event_handle = queue.event_handle.clone();
                    (status, event_payload, event_handle, next_job)
                };

                if let Some(status) = maybe_status {
                    append_job_log(&status);
                }

                if let Some(payload) = event_payload {
                    emit_job_status(event_handle, payload);
                }

                if let Some(next_job) = next_job {
                    spawn_job_worker(next_job);
                }
            }
            Err(err) => {
                if finalize_cancel() {
                    return;
                }

                let (maybe_status, event_payload, event_handle, next_job) = {
                    let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                    let next_job = queue.finish_job(&job.job_id);
                    let (status, event_payload) = queue.update_status(&job.job_id, |status| {
                        status.state = JobState::Failed;
                        status.progress = 1.0;
                        status.message = Some(format!("번역 실패: {}", err));
                    });
                    let event_handle = queue.event_handle.clone();
                    (status, event_payload, event_handle, next_job)
                };

                if let Some(status) = maybe_status {
                    append_job_log(&status);
                }

                if let Some(payload) = event_payload {
                    emit_job_status(event_handle, payload);
                }

                if let Some(next_job) = next_job {
                    spawn_job_worker(next_job);
                }
            }
        }
    });
}

#[tauri::command]
pub fn start_translation_job(
    app_handle: AppHandle,
    request: TranslationJobRequest,
) -> Result<TranslationJobStatus, String> {
    if request.selected_files.is_empty() {
        return Err("번역할 파일을 하나 이상 선택해야 합니다.".into());
    }

    TranslationOrchestrator::new()
        .start_job(request, Some(app_handle))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn cancel_translation_job(job_id: String) -> Result<(), String> {
    let (event_payload, event_handle) = {
        let mut queue = WORK_QUEUE
            .lock()
            .map_err(|_| "queue lock poisoned".to_string())?;
        let payload = queue.request_cancel(&job_id)?;
        let event_handle = queue.event_handle.clone();
        (payload, event_handle)
    };

    if let Some(payload) = event_payload {
        emit_job_status(event_handle, payload);
    }

    Ok(())
}

#[tauri::command]
pub fn get_translation_job_status(job_id: String) -> Result<TranslationJobStatus, String> {
    let queue = WORK_QUEUE
        .lock()
        .map_err(|_| "queue lock poisoned".to_string())?;

    queue
        .job_status(&job_id)
        .ok_or_else(|| format!("job {job_id} not found"))
}
