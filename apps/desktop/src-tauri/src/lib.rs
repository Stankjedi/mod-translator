use mod_translator_core::{discover_steam_path, scan_library, start_translation_job};

#[cfg_attr(mobile, tauri::mobile_entry_point)]

pub fn run() {
    tauri::Builder::default()
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
            discover_steam_path,
            scan_library,
            start_translation_job
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
