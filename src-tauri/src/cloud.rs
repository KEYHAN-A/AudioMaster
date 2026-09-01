use mastering_core::cloud::{
    CloudClient, CloudSyncDocument, CloudSyncUpdate, CloudUser, DeviceAuthorization,
    FeedbackRequest, TokenPair,
};
use mastering_core::config::Config;
use serde::Serialize;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const KEYRING_SERVICE: &str = "com.keyhanstudio.audiomaster";
const KEYRING_ACCOUNT: &str = "keyhanstudio-refresh-token";
const PROVIDER_SECRET_ACCOUNTS: &[(&str, &str)] = &[
    ("keyhanstudio", "keyhanstudio-api-key"),
    ("openai", "openai-api-key"),
    ("anthropic", "anthropic-api-key"),
];

#[derive(Debug, Clone)]
struct ActiveSession {
    access_token: String,
    expires_at: u64,
    user: CloudUser,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloudStatus {
    pub signed_in: bool,
    pub user: Option<CloudUser>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginPollResult {
    pub state: String,
    pub user: Option<CloudUser>,
}

static SESSION: OnceLock<Mutex<Option<ActiveSession>>> = OnceLock::new();

fn session() -> &'static Mutex<Option<ActiveSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

fn client() -> Result<CloudClient, String> {
    let config = Config::load().map_err(|error| format!("Cloud configuration error: {error}"))?;
    CloudClient::new(config.cloud.endpoint).map_err(|error| error.to_string())
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|error| format!("Could not access the operating-system credential vault: {error}"))
}

fn provider_secret_entry(provider: &str) -> Result<keyring::Entry, String> {
    let account = PROVIDER_SECRET_ACCOUNTS
        .iter()
        .find_map(|(name, account)| (*name == provider).then_some(*account))
        .ok_or_else(|| format!("Unsupported credential provider: {provider}"))?;
    keyring::Entry::new(KEYRING_SERVICE, account)
        .map_err(|error| format!("Could not access the operating-system credential vault: {error}"))
}

pub(crate) fn load_provider_secret(provider: &str) -> Result<Option<String>, String> {
    match provider_secret_entry(provider)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read {provider} credentials: {error}")),
    }
}

pub(crate) fn store_provider_secret(provider: &str, secret: &str) -> Result<(), String> {
    if secret.trim().is_empty() {
        return Ok(());
    }
    provider_secret_entry(provider)?
        .set_password(secret)
        .map_err(|error| format!("Could not store {provider} credentials securely: {error}"))
}

pub(crate) fn delete_provider_secret(provider: &str) -> Result<(), String> {
    match provider_secret_entry(provider)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not remove {provider} credentials: {error}")),
    }
}

async fn store_refresh_token(token: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        keyring_entry()?
            .set_password(&token)
            .map_err(|error| format!("Could not store cloud credentials securely: {error}"))
    })
    .await
    .map_err(|error| format!("Credential task failed: {error}"))?
}

async fn load_refresh_token() -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(|| match keyring_entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read cloud credentials: {error}")),
    })
    .await
    .map_err(|error| format!("Credential task failed: {error}"))?
}

async fn delete_refresh_token() -> Result<(), String> {
    tokio::task::spawn_blocking(|| match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(format!("Could not remove cloud credentials: {error}")),
    })
    .await
    .map_err(|error| format!("Credential task failed: {error}"))?
}

async fn activate(tokens: TokenPair) -> Result<ActiveSession, String> {
    store_refresh_token(tokens.refresh_token).await?;
    let active = ActiveSession {
        access_token: tokens.token,
        expires_at: now_epoch_seconds().saturating_add(tokens.expires_in),
        user: tokens.user,
    };
    *session().lock().await = Some(active.clone());
    Ok(active)
}

async fn authorized_session() -> Result<ActiveSession, String> {
    if let Some(active) = session().lock().await.clone() {
        if active.expires_at > now_epoch_seconds().saturating_add(30) {
            return Ok(active);
        }
    }

    let refresh_token = load_refresh_token()
        .await?
        .ok_or_else(|| "Sign in to KeyhanStudio first".to_string())?;
    match client()?.refresh(&refresh_token).await {
        Ok(tokens) => activate(tokens).await,
        Err(error) => {
            *session().lock().await = None;
            let _ = delete_refresh_token().await;
            Err(format!("KeyhanStudio session expired: {error}"))
        }
    }
}

pub(crate) async fn cloud_access_token() -> Result<String, String> {
    authorized_session()
        .await
        .map(|session| session.access_token)
}

#[tauri::command]
pub async fn cloud_begin_login() -> Result<DeviceAuthorization, String> {
    client()?
        .start_device_login()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cloud_poll_login(device_code: String) -> Result<LoginPollResult, String> {
    let poll = client()?
        .poll_device_login(&device_code)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(tokens) = poll.tokens {
        let active = activate(tokens).await?;
        return Ok(LoginPollResult {
            state: "authorized".into(),
            user: Some(active.user),
        });
    }
    Ok(LoginPollResult {
        state: poll.error.unwrap_or_else(|| "authorization_pending".into()),
        user: None,
    })
}

#[tauri::command]
pub async fn cloud_status() -> Result<CloudStatus, String> {
    if load_refresh_token().await?.is_none() {
        return Ok(CloudStatus {
            signed_in: false,
            user: None,
        });
    }
    match authorized_session().await {
        Ok(active) => Ok(CloudStatus {
            signed_in: true,
            user: Some(active.user),
        }),
        Err(_) => Ok(CloudStatus {
            signed_in: false,
            user: None,
        }),
    }
}

#[tauri::command]
pub async fn cloud_logout() -> Result<(), String> {
    *session().lock().await = None;
    if let Some(refresh_token) = load_refresh_token().await? {
        // Local logout must still succeed offline; the rotated token family is
        // revoked server-side whenever the cloud is reachable.
        let _ = client()?.revoke(&refresh_token).await;
    }
    delete_refresh_token().await
}

#[tauri::command]
pub async fn cloud_pull_sync() -> Result<CloudSyncDocument, String> {
    let active = authorized_session().await?;
    client()?
        .get_sync(&active.access_token)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cloud_push_sync(
    base_revision: u64,
    settings: serde_json::Value,
    presets: Vec<serde_json::Value>,
) -> Result<CloudSyncDocument, String> {
    let active = authorized_session().await?;
    client()?
        .put_sync(
            &active.access_token,
            &CloudSyncUpdate {
                base_revision,
                settings: &settings,
                presets: &presets,
            },
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cloud_submit_feedback(
    category: String,
    message: String,
    diagnostics_opt_in: bool,
) -> Result<(), String> {
    let active = authorized_session().await?;
    client()?
        .submit_feedback(
            &active.access_token,
            &FeedbackRequest {
                category: &category,
                message: &message,
                app_version: env!("CARGO_PKG_VERSION"),
                diagnostics_opt_in,
            },
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cloud_set_early_access(enabled: bool) -> Result<CloudSyncDocument, String> {
    let active = authorized_session().await?;
    client()?
        .set_early_access(&active.access_token, enabled)
        .await
        .map_err(|error| error.to_string())
}
