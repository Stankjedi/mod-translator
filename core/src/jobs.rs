use crate::ai::{translate_text, ProviderId, TranslationError};
use log::warn;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

static ACTIVE_JOBS: Lazy<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranslationFileInput {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartTranslationJobPayload {
    pub job_id: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub files: Vec<TranslationFileInput>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationProgressEventPayload {
    pub job_id: String,
    pub state: String,
    pub progress: f32,
    pub log: Option<String>,
    pub translated_count: u32,
    pub total_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_errors: Option<Vec<TranslationFileErrorEntry>>,
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
    file_path: String,
    line_index: usize,
    text: String,
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn start_translation_job(
    app: AppHandle,
    jobId: String,
    provider: String,
    apiKey: Option<String>,
    sourceLang: Option<String>,
    targetLang: Option<String>,
    files: Vec<TranslationFileInput>,
) -> Result<(), String> {
    let payload = StartTranslationJobPayload {
        job_id: jobId,
        provider,
        api_key: apiKey,
        files,
        source_lang: sourceLang,
        target_lang: targetLang,
    };
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
                    state: "failed".into(),
                    progress: 0.0,
                    log: Some(format!("지원하지 않는 번역기: {}", payload.provider)),
                    translated_count: 0,
                    total_count: 0,
                    file_name: None,
                    file_success: None,
                    file_errors: None,
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
                state: "failed".into(),
                progress: 0.0,
                log: Some("API 키가 설정되지 않았습니다.".into()),
                translated_count: 0,
                total_count: 0,
                file_name: None,
                file_success: None,
                file_errors: None,
            },
        );
        return Err("선택한 번역기의 API 키를 설정해 주세요.".into());
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
    let _ = app;
    let guard = ACTIVE_JOBS
        .lock()
        .map_err(|_| "job registry lock poisoned".to_string())?;

    if let Some(flag) = guard.get(&jobId) {
        flag.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err(format!("job {jobId} not found"))
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

    let mut segments = Vec::new();
    let mut last_file_name: Option<String> = None;
    let mut last_file_success: Option<bool> = None;
    for file in &payload.files {
        let path = PathBuf::from(&file.path);
        let content = match fs::read_to_string(&path) {
            Ok(value) => value,
            Err(err) => {
                let message = format!("{} 파일을 읽는 중 오류가 발생했습니다: {}", file.path, err);
                last_file_name = Some(file.path.clone());
                last_file_success = Some(false);
                let error_entry = TranslationFileErrorEntry {
                    file_path: file.path.clone(),
                    message: message.clone(),
                    code: None,
                };
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        state: "failed".into(),
                        progress: 0.0,
                        log: Some(message),
                        translated_count: 0,
                        total_count: 0,
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: Some(vec![error_entry]),
                    },
                );
                return;
            }
        };

        for (index, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            segments.push(Segment {
                file_path: file.path.clone(),
                line_index: index + 1,
                text: trimmed.to_string(),
            });
        }
    }

    let total_segments = segments.len();
    emit_progress(
        &app,
        TranslationProgressEventPayload {
            job_id: payload.job_id.clone(),
            state: "running".into(),
            progress: 0.0,
            log: Some("번역을 준비하는 중입니다.".into()),
            translated_count: 0,
            total_count: total_segments as u32,
            file_name: last_file_name.clone(),
            file_success: last_file_success,
            file_errors: None,
        },
    );

    if total_segments == 0 {
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                state: "completed".into(),
                progress: 100.0,
            log: Some("번역할 문자열이 없습니다.".into()),
            translated_count: 0,
            total_count: 0,
            file_name: last_file_name.clone(),
            file_success: last_file_success,
            file_errors: None,
        },
    );
    return;
}

    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(err) => {
            emit_progress(
                &app,
                TranslationProgressEventPayload {
                    job_id: payload.job_id.clone(),
                    state: "failed".into(),
                    progress: 0.0,
                log: Some("HTTP 클라이언트를 초기화하지 못했습니다.".into()),
                translated_count: 0,
                total_count: 0,
                file_name: last_file_name.clone(),
                file_success: last_file_success,
                file_errors: None,
            },
        );
        return;
    }
    };

    let total_segments = total_segments as u32;
    for (index, segment) in segments.iter().enumerate() {
        let processed = index as u32;
        if cancel_flag.load(Ordering::SeqCst) {
            emit_progress(
                &app,
                TranslationProgressEventPayload {
                    job_id: payload.job_id.clone(),
                    state: "canceled".into(),
                    progress: percentage(processed, total_segments),
                log: Some("사용자가 작업을 중단했습니다.".into()),
                translated_count: processed,
                total_count: total_segments,
                file_name: last_file_name.clone(),
                file_success: last_file_success,
                file_errors: None,
            },
        );
        return;
    }

        let translated = match translate_text(
            &client,
            provider,
            &api_key,
            &segment.text,
            &source_lang,
            &target_lang,
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                let message = format_translation_error(segment, error);
                let progress = percentage(processed, total_segments);
                last_file_name = Some(segment.file_path.clone());
                last_file_success = Some(false);
                let error_entry = TranslationFileErrorEntry {
                    file_path: segment.file_path.clone(),
                    message: message.clone(),
                    code: None,
                };
                emit_progress(
                    &app,
                    TranslationProgressEventPayload {
                        job_id: payload.job_id.clone(),
                        state: "failed".into(),
                        progress,
                        log: Some(message),
                        translated_count: processed,
                        total_count: total_segments,
                        file_name: last_file_name.clone(),
                        file_success: last_file_success,
                        file_errors: Some(vec![error_entry]),
                    },
                );
                return;
            }
        };

        last_file_name = Some(segment.file_path.clone());
        last_file_success = Some(true);

        let progress = percentage(processed + 1, total_segments);
        emit_progress(
            &app,
            TranslationProgressEventPayload {
                job_id: payload.job_id.clone(),
                state: "running".into(),
                progress,
                log: Some(format!(
                    "{} {}행 번역 완료",
                    segment.file_path, segment.line_index
                )),
                translated_count: processed + 1,
                total_count: total_segments,
                file_name: last_file_name.clone(),
                file_success: last_file_success,
                file_errors: None,
            },
        );

        if cancel_flag.load(Ordering::SeqCst) {
            emit_progress(
                &app,
                TranslationProgressEventPayload {
                    job_id: payload.job_id.clone(),
                    state: "canceled".into(),
                    progress,
                    log: Some("사용자가 작업을 중단했습니다.".into()),
                translated_count: processed + 1,
                total_count: total_segments,
                file_name: last_file_name.clone(),
                file_success: last_file_success,
                file_errors: None,
            },
        );
        return;
    }

        let _ = translated; // Placeholder for future persistence.
    }

    emit_progress(
        &app,
        TranslationProgressEventPayload {
            job_id: payload.job_id,
            state: "completed".into(),
            progress: 100.0,
            log: Some("번역이 완료되었습니다.".into()),
            translated_count: total_segments,
            total_count: total_segments,
            file_name: last_file_name,
            file_success: last_file_success,
            file_errors: None,
        },
    );
}

fn emit_progress(app: &AppHandle, payload: TranslationProgressEventPayload) {
    if let Err(error) = app.emit("translation-progress", payload) {
        warn!("failed to emit translation progress: {}", error);
    }
}

fn percentage(processed: u32, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    ((processed as f32) / (total as f32) * 100.0).clamp(0.0, 100.0)
}

fn format_translation_error(segment: &Segment, error: TranslationError) -> String {
    match error {
        TranslationError::PlaceholderMismatch(missing) => {
            if missing.is_empty() {
                format!(
                    "{} 번역 중 자리표시자 검증에 실패했습니다.",
                    segment.file_path
                )
            } else {
                format!(
                    "{} 번역 중 자리표시자 누락: {}",
                    segment.file_path,
                    missing.join(", ")
                )
            }
        }
        TranslationError::Provider(message) | TranslationError::Http(message) => {
            format!("{} 번역 중 오류 발생: {}", segment.file_path, message)
        }
    }
}
