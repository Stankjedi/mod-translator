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

#[cfg_attr(mobile, tauri::mobile_entry_point)]

pub fn run() {
    tauri::Builder::<tauri::Wry>::default()
        .plugin(tauri_plugin_stronghold::Builder::new(|password| {
            // 앱 고유 키로 Stronghold 암호화 (실제 배포시 더 강력한 키 필요)
            // argon2 또는 더 안전한 해시를 사용하는 것이 권장됨
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            password.hash(&mut hasher);
            b"mod-translator-stronghold-key-v1".hash(&mut hasher);
            let hash1 = hasher.finish();
            
            hasher = DefaultHasher::new();
            hash1.hash(&mut hasher);
            password.hash(&mut hasher);
            let hash2 = hasher.finish();
            
            hasher = DefaultHasher::new();
            hash2.hash(&mut hasher);
            b"salt-key".hash(&mut hasher);
            let hash3 = hasher.finish();
            
            hasher = DefaultHasher::new();
            hash3.hash(&mut hasher);
            password.hash(&mut hasher);
            let hash4 = hasher.finish();
            
            // 4개의 u64 해시를 조합하여 32바이트 키 생성
            let mut key = Vec::with_capacity(32);
            key.extend_from_slice(&hash1.to_le_bytes());
            key.extend_from_slice(&hash2.to_le_bytes());
            key.extend_from_slice(&hash3.to_le_bytes());
            key.extend_from_slice(&hash4.to_le_bytes());
            key
        }).build())
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
