use std::{path::PathBuf, sync::Arc};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub total: u32,
    pub done: u32,
    pub file: Option<String>,
}

#[derive(Debug)]
pub enum JobMsg {
    Start {
        files: Vec<PathBuf>,
        from: String,
        to: String,
    },
    Cancel,
}

pub struct JobRunner {
    tx: mpsc::Sender<JobMsg>,
}

impl JobRunner {
    pub fn new(app: AppHandle) -> Arc<Self> {
        let (tx, mut rx) = mpsc::channel::<JobMsg>(8);
        let app_handle = app.clone();

        tauri::async_runtime::spawn(async move {
            let mut cancelling = false;
            while let Some(msg) = rx.recv().await {
                match msg {
                    JobMsg::Start { files, from, to } => {
                        cancelling = false;
                        let total = files.len() as u32;
                        let _ = app_handle.emit(
                            "translate:started",
                            &serde_json::json!({ "total": total, "from": from, "to": to }),
                        );

                        let mut done = 0u32;

                        for path in files.into_iter() {
                            if cancelling {
                                let _ = app_handle.emit(
                                    "translate:cancelled",
                                    &serde_json::json!({ "done": done, "total": total }),
                                );
                                break;
                            }

                            let file_disp = path.to_string_lossy().to_string();
                            let path_clone = path.clone();
                            let from_clone = from.clone();
                            let to_clone = to.clone();

                            let res = tauri::async_runtime::spawn_blocking(move || {
                                crate::translate::translate_file(
                                    &path_clone,
                                    &from_clone,
                                    &to_clone,
                                )
                            })
                            .await;

                            match res {
                                Ok(Ok(())) => {
                                    done += 1;
                                    let _ = app_handle.emit(
                                        "translate:progress",
                                        &Progress {
                                            total,
                                            done,
                                            file: Some(file_disp.clone()),
                                        },
                                    );
                                }
                                Ok(Err(e)) => {
                                    let _ = app_handle.emit(
                                        "translate:error",
                                        &serde_json::json!({
                                            "file": file_disp,
                                            "error": e.to_string(),
                                        }),
                                    );
                                }
                                Err(join_err) => {
                                    let _ = app_handle.emit(
                                        "translate:error",
                                        &serde_json::json!({
                                            "file": file_disp,
                                            "error": join_err.to_string(),
                                        }),
                                    );
                                }
                            }
                        }

                        let _ = app_handle.emit(
                            "translate:finished",
                            &serde_json::json!({ "done": done, "total": total }),
                        );
                    }
                    JobMsg::Cancel => {
                        cancelling = true;
                        let _ = app_handle.emit("translate:stopping", &serde_json::json!({}));
                    }
                }
            }
        });

        Arc::new(Self { tx })
    }

    pub async fn start(&self, files: Vec<PathBuf>, from: String, to: String) -> Result<(), String> {
        self.tx
            .send(JobMsg::Start { files, from, to })
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn cancel(&self) -> Result<(), String> {
        self.tx
            .send(JobMsg::Cancel)
            .await
            .map_err(|e| e.to_string())
    }
}
