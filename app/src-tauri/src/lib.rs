// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;

// Imports
mod music;
mod music_db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = music_db::initialize_database(&handle)
                    .await
                    .expect("[soitin]: Failed to initialize database");
                handle.manage(pool);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            music::commands::scan_library,
            music::commands::get_album,
            music::commands::get_albums,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
