// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;

pub mod commands;
pub mod config;
pub mod error;
pub mod storage;
pub use error::AppError;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the main window when a second instance is launched
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            commands::list_configs,
            commands::get_config,
            commands::create_config,
            commands::update_config,
            commands::delete_config,
            commands::duplicate_config,
            commands::apply_config,
            commands::import_from_opencode,
            commands::auto_import_from_opencode,
            commands::import_config_file,
            commands::read_role_json_file,
            commands::export_config,
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            commands::get_active_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
