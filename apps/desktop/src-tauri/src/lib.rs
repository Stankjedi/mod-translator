#[cfg_attr(mobile, tauri::mobile_entry_point)]

pub fn run() {
    tauri::Builder::<tauri::Wry>::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
