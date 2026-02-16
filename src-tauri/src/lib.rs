mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::analyze_file,
            commands::master_file,
            commands::get_config,
            commands::save_config,
            commands::check_backends,
            commands::get_presets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
