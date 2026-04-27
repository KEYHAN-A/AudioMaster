use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize structured logging with JSON output and log rotation.
pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = log_dir();

    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("audiomaster")
        .filename_suffix("log")
        .max_log_files(5)
        .build(&log_dir)?;

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,mastering_core=debug,mastering_tauri=debug"));

    // Use a non-blocking writer to avoid blocking the async runtime
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Store the guard so it doesn't get dropped
    Box::leak(Box::new(_guard));

    // JSON format for log files, pretty format for stdout
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout).with_ansi(true))
        .with(
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    tracing::info!("Logging initialized. Log directory: {}", log_dir.display());

    Ok(())
}

/// Initialize Sentry error tracking.
pub fn init_sentry() -> Option<sentry::ClientInitGuard> {
    // Only initialize if DSN is configured
    let dsn = std::env::var("SENTRY_DSN").ok();

    if dsn.is_none() {
        tracing::info!("Sentry DSN not configured, error tracking disabled");
        return None;
    }

    let guard = sentry::init(sentry::ClientOptions {
        dsn: dsn.map(|d| d.parse().unwrap()),
        release: Some(env!("CARGO_PKG_VERSION").into()),
        environment: Some(
            if cfg!(debug_assertions) {
                "development"
            } else {
                "production"
            }
            .into(),
        ),
        traces_sample_rate: 0.2,
        ..Default::default()
    });

    tracing::info!("Sentry error tracking initialized");
    Some(guard)
}

/// Add a breadcrumb for error tracking.
pub fn add_breadcrumb(message: &str, category: &str) {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        ty: "default".into(),
        level: sentry::Level::Info,
        category: Some(category.into()),
        message: Some(message.into()),
        ..Default::default()
    });
}

/// Set user context for error tracking.
pub fn set_user_context(session_id: &str) {
    sentry::configure_scope(|scope| {
        scope.set_user(Some(sentry::User {
            id: Some(session_id.into()),
            ..Default::default()
        }));
    });
}

/// Set processing context for error tracking.
pub fn set_processing_context(backend: &str, preset: &str, file: &str) {
    sentry::configure_scope(|scope| {
        scope.set_tag("backend", backend);
        scope.set_tag("preset", preset);
        scope.set_extra("input_file", file.into());
    });
}

/// Export diagnostic log bundle for support.
pub fn export_diagnostic_bundle() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let log_path = log_dir();
    let output_path = log_path.join("diagnostic_bundle.txt");

    let mut bundle = String::new();
    bundle.push_str(&format!(
        "AudioMaster Diagnostic Bundle\nVersion: {}\n\n",
        env!("CARGO_PKG_VERSION")
    ));

    // Collect recent logs
    let mut log_files: Vec<_> = std::fs::read_dir(&log_path)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "log")
        })
        .collect();
    log_files.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()).unwrap_or(std::time::UNIX_EPOCH));

    for file in log_files.iter().rev().take(3) {
        bundle.push_str(&format!("=== {} ===\n", file.path().display()));
        if let Ok(contents) = std::fs::read_to_string(file.path()) {
            // Last 500 lines
            let lines: Vec<&str> = contents.lines().rev().take(500).collect();
            for line in lines.iter().rev() {
                bundle.push_str(line);
                bundle.push('\n');
            }
        }
        bundle.push('\n');
    }

    // Add system info
    bundle.push_str("=== System Info ===\n");
    bundle.push_str(&format!("OS: {}\n", std::env::consts::OS));
    bundle.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    bundle.push_str(&format!("Python scripts dir: {}\n", mastering_core::config::Config::python_scripts_dir().display()));

    std::fs::write(&output_path, &bundle)?;
    Ok(output_path)
}

fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("AudioMaster")
        .join("logs")
}
