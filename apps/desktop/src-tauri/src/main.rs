#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use log::LevelFilter;
use mod_translator_core::job::runner::JobRunner;
use tauri::{Manager, State};

struct Shared(Arc<JobRunner>);

#[tauri::command]
async fn cmd_start(
    state: State<'_, Shared>,
    files: Vec<String>,
    from: String,
    to: String,
) -> Result<(), String> {
    let files = files.into_iter().map(Into::into).collect::<Vec<_>>();
    state.0.start(files, from, to).await
}

#[tauri::command]
async fn cmd_cancel(state: State<'_, Shared>) -> Result<(), String> {
    state.0.cancel().await
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(LevelFilter::Info)
                        .build(),
                )?;
            }

            let runner = JobRunner::new(app.handle().clone());
            app.manage(Shared(runner));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_start,
            cmd_cancel,
            mod_translator_core::detect_steam_path,
            mod_translator_core::scan_steam_library,
            mod_translator_core::list_mod_files,
            mod_translator_core::start_translation_job,
            mod_translator_core::cancel_translation_job,
            mod_translator_core::retry_translation_now,
            mod_translator_core::open_output_folder,
            mod_translator_core::validate_api_key_and_list_models,
            mod_translator_core::get_validation_metrics,
            mod_translator_core::reset_validation_metrics,
            mod_translator_core::export_validation_metrics,
            mod_translator_core::get_validation_log_file_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
