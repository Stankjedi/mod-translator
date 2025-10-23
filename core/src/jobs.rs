use crate::ai::{TranslateOptions, TranslationError, Translator, TranslatorKind};
use crate::pipeline::PipelinePlan;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

static WORK_QUEUE: Lazy<Mutex<WorkQueue>> = Lazy::new(|| {
    Mutex::new(WorkQueue::new(
        3,                                               // concurrent workers
        RateLimiter::new(5, Duration::from_millis(750)), // 5 tokens per 750ms bucket
    ))
});

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Clone)]
pub struct QueueSnapshot {
    pub queued: usize,
    pub running: usize,
    pub concurrent_workers: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct RateLimiterSnapshot {
    pub bucket_capacity: u32,
    pub tokens_available: u32,
    pub refill_interval_ms: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct QualityGateSnapshot {
    pub placeholder_guard: bool,
    pub format_validator: bool,
    pub sample_rate: f32,
}

#[derive(Debug, Serialize, Clone)]
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
        let mut translator = request.translator.build();
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

        let mut queue = WORK_QUEUE.lock().expect("queue lock poisoned");
        let queue_snapshot = queue.register_job(job_id.clone());
        let rate_limiter_snapshot = queue.rate_limiter_snapshot();

        Ok(TranslationJobStatus {
            job_id,
            translator: translator.name().to_string(),
            state: if queue_snapshot.queued > 0 {
                JobState::Queued
            } else {
                JobState::Completed
            },
            progress: if queue_snapshot.queued > 0 { 0.1 } else { 1.0 },
            preview,
            message: Some(format!(
                "Translation job prepared for {}",
                request.mod_name.unwrap_or_else(|| request.mod_id.clone())
            )),
            queue: queue_snapshot,
            rate_limiter: rate_limiter_snapshot,
            quality_gates: QualityGateSnapshot {
                placeholder_guard: true,
                format_validator: true,
                sample_rate: 0.05,
            },
            pipeline: PipelinePlan::default_for(
                request
                    .mod_name
                    .as_deref()
                    .unwrap_or(request.mod_id.as_str()),
            ),
        })
    }
}

#[derive(Debug)]
struct WorkQueue {
    concurrent_workers: usize,
    running: usize,
    waiting: VecDeque<String>,
    rate_limiter: RateLimiter,
}

impl WorkQueue {
    fn new(concurrent_workers: usize, rate_limiter: RateLimiter) -> Self {
        Self {
            concurrent_workers,
            running: 0,
            waiting: VecDeque::new(),
            rate_limiter,
        }
    }

    fn register_job(&mut self, job_id: String) -> QueueSnapshot {
        if self.running < self.concurrent_workers {
            self.running += 1;
        } else {
            self.waiting.push_back(job_id);
        }

        QueueSnapshot {
            queued: self.waiting.len(),
            running: self.running,
            concurrent_workers: self.concurrent_workers,
        }
    }

    fn rate_limiter_snapshot(&mut self) -> RateLimiterSnapshot {
        self.rate_limiter.consume();
        self.rate_limiter.snapshot()
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
