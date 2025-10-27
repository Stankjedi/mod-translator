use crate::ai::{ProviderAuth, TranslateOptions, TranslationError, TranslatorKind};
use crate::pipeline::PipelinePlan;
use dirs::data_dir;
use log::warn;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

static WORK_QUEUE: Lazy<Mutex<WorkQueue>> = Lazy::new(|| {
    Mutex::new(WorkQueue::new(
        3,                                               // concurrent workers
        RateLimiter::new(5, Duration::from_millis(750)), // 5 tokens per 750ms bucket
    ))
});

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationJobRequest {
    pub mod_id: String,
    pub mod_name: Option<String>,
    pub translator: TranslatorKind,
    pub source_language: String,
    pub target_language: String,
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
    ) -> Result<TranslationJobStatus, JobError> {
        let job_id = Uuid::new_v4().to_string();
        let mut translator = request.translator.build_with_auth(&request.provider_auth);
        let options = TranslateOptions {
            source_lang: Some(request.source_language.clone()),
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
                "{} 번역 작업이 큐에 등록되었습니다.",
                job_display_name
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
        };

        let job = QueuedJob {
            job_id: job_id.clone(),
            request: request.clone(),
            options: options.clone(),
        };

        let (status, maybe_job_to_start) = {
            let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
            queue.register_job(job, initial_status)
        };

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

#[derive(Debug)]
struct WorkQueue {
    concurrent_workers: usize,
    running: usize,
    waiting: VecDeque<QueuedJob>,
    rate_limiter: RateLimiter,
    statuses: HashMap<String, TranslationJobStatus>,
}

impl WorkQueue {
    fn new(concurrent_workers: usize, rate_limiter: RateLimiter) -> Self {
        Self {
            concurrent_workers,
            running: 0,
            waiting: VecDeque::new(),
            rate_limiter,
            statuses: HashMap::new(),
        }
    }

    fn register_job(
        &mut self,
        job: QueuedJob,
        mut status: TranslationJobStatus,
    ) -> (TranslationJobStatus, Option<QueuedJob>) {
        let mut start_immediately = None;
        if self.running < self.concurrent_workers {
            self.running += 1;
            start_immediately = Some(job);
        } else {
            self.waiting.push_back(job);
        }

        self.rate_limiter.consume();
        status.queue = self.snapshot();
        status.rate_limiter = self.rate_limiter.snapshot();
        self.statuses.insert(status.job_id.clone(), status.clone());

        (status, start_immediately)
    }

    fn update_status<F>(&mut self, job_id: &str, update: F) -> Option<TranslationJobStatus>
    where
        F: FnOnce(&mut TranslationJobStatus),
    {
        let queue_snapshot = self.snapshot();

        if let Some(status) = self.statuses.get_mut(job_id) {
            update(status);
            status.queue = queue_snapshot;
            return Some(status.clone());
        }

        None
    }

    fn finish_job(&mut self, job_id: &str) -> Option<QueuedJob> {
        if self.running > 0 {
            self.running -= 1;
        }

        if let Some(next_job) = self.waiting.pop_front() {
            self.running += 1;
            Some(next_job)
        } else {
            // Job is finished and no queued work.
            let queue_snapshot = self.snapshot();
            self.statuses.get_mut(job_id).map(|status| {
                status.queue = queue_snapshot.clone();
            });
            None
        }
    }

    fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            queued: self.waiting.len(),
            running: self.running,
            concurrent_workers: self.concurrent_workers,
        }
    }

    fn job_status(&self, job_id: &str) -> Option<TranslationJobStatus> {
        self.statuses.get(job_id).cloned()
    }
}

#[derive(Debug)]
struct RateLimiter {
    capacity: u32,
    available: u32,
    refill_interval: Duration,
}

impl RateLimiter {
    fn new(capacity: u32, refill_interval: Duration) -> Self {
        Self {
            capacity,
            available: capacity,
            refill_interval,
        }
    }

    fn consume(&mut self) {
        if self.available == 0 {
            // Simulate backoff by refilling after a virtual interval.
            self.available = self.capacity.saturating_sub(1);
        } else {
            self.available -= 1;
        }
    }

    fn snapshot(&self) -> RateLimiterSnapshot {
        RateLimiterSnapshot {
            bucket_capacity: self.capacity,
            tokens_available: self.available,
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

        {
            let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
            if let Some(status) = queue.update_status(&job.job_id, |status| {
                status.state = JobState::Running;
                status.progress = status.progress.max(0.2);
                status.message = Some(format!("{} 번역을 시작했습니다.", job_display_name));
            }) {
                append_job_log(&status);
            }
        }

        thread::sleep(Duration::from_millis(400));

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

        match result {
            Ok(outputs) => {
                {
                    let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                    if let Some(status) = queue.update_status(&job.job_id, |status| {
                        status.progress = 0.9;
                        if let Some(first) = outputs.first() {
                            status.preview = Some(first.clone());
                        }
                        status.message =
                            Some(format!("{}개의 문자열을 번역했습니다.", outputs.len()));
                    }) {
                        append_job_log(&status);
                    }
                }

                thread::sleep(Duration::from_millis(300));

                let next_job = {
                    let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                    let next_job = queue.finish_job(&job.job_id);
                    if let Some(status) = queue.update_status(&job.job_id, |status| {
                        status.state = JobState::Completed;
                        status.progress = 1.0;
                        status.message =
                            Some(format!("{} 번역이 완료되었습니다.", job_display_name));
                    }) {
                        append_job_log(&status);
                    }
                    next_job
                };

                if let Some(next_job) = next_job {
                    spawn_job_worker(next_job);
                }
            }
            Err(err) => {
                let next_job = {
                    let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
                    let next_job = queue.finish_job(&job.job_id);
                    if let Some(status) = queue.update_status(&job.job_id, |status| {
                        status.state = JobState::Failed;
                        status.progress = 1.0;
                        status.message = Some(format!("번역 실패: {}", err));
                    }) {
                        append_job_log(&status);
                    }
                    next_job
                };

                if let Some(next_job) = next_job {
                    spawn_job_worker(next_job);
                }
            }
        }
    });
}

#[tauri::command]
pub fn start_translation_job(
    request: TranslationJobRequest,
) -> Result<TranslationJobStatus, String> {
    TranslationOrchestrator::new()
        .start_job(request)
        .map_err(|err| err.to_string())
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
