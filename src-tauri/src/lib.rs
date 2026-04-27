mod commands;
mod telemetry;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize telemetry before anything else
    let _sentry_guard = telemetry::init_sentry();
    let _ = telemetry::init_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::analyze_file,
            commands::master_file,
            commands::master_batch,
            commands::get_config,
            commands::save_config,
            commands::check_backends,
            commands::diagnose_backends,
            commands::get_presets,
            commands::get_waveform_data,
            commands::lmstudio_status,
            commands::lmstudio_models,
            commands::detect_vram,
        ])
        .setup(|app| {
            // Set project dir env var so mastering-core can find python scripts
            if let Ok(resource_dir) = app.path().resource_dir() {
                std::env::set_var("MASTERING_PROJECT_DIR", &resource_dir);
            }

            telemetry::add_breadcrumb("Application started", "lifecycle");

            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
