mod cloud;
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
        .invoke_handler(tauri::generate_handler![
            commands::analyze_file,
            commands::master_file,
            commands::cancel_mastering,
            commands::create_mastering_preview,
            commands::master_batch,
            commands::master_album,
            commands::get_config,
            commands::save_config,
            commands::clear_provider_credential,
            commands::check_backends,
            commands::diagnose_backends,
            commands::get_presets,
            commands::get_waveform_data,
            commands::lmstudio_status,
            commands::lmstudio_models,
            commands::lmstudio_load_model,
            commands::lmstudio_unload_model,
            commands::lmstudio_loaded_models,
            commands::lmstudio_recommend_models,
            commands::detect_vram,
            commands::export_diagnostic_bundle,
            cloud::cloud_begin_login,
            cloud::cloud_poll_login,
            cloud::cloud_status,
            cloud::cloud_logout,
            cloud::cloud_pull_sync,
            cloud::cloud_push_sync,
            cloud::cloud_submit_feedback,
            cloud::cloud_set_early_access,
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
