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
            mod_translator_core::start_translation_job
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
