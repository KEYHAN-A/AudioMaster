//! KeyhanStudio cloud client.
//!
//! This boundary intentionally transports identity, preferences, mastering
//! presets, early-access state, and feedback only. Audio paths and audio bytes
//! are not part of any request type in this module.

use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MAX_SYNC_BYTES: usize = 256 * 1024;
const MAX_FEEDBACK_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct CloudClient {
    base_url: String,
    http: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPair {
    #[serde(alias = "accessToken")]
    pub token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: CloudUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationPoll {
    pub success: bool,
    pub error: Option<String>,
    #[serde(flatten)]
    pub tokens: Option<TokenPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudSyncDocument {
    pub revision: u64,
    #[serde(default)]
    pub settings: serde_json::Value,
    #[serde(default)]
    pub presets: Vec<serde_json::Value>,
    #[serde(default)]
    pub early_access: bool,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloudSyncUpdate<'a> {
    pub base_revision: u64,
    pub settings: &'a serde_json::Value,
    pub presets: &'a [serde_json::Value],
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedbackRequest<'a> {
    pub category: &'a str,
    pub message: &'a str,
    pub app_version: &'a str,
    pub diagnostics_opt_in: bool,
}

impl CloudClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        anyhow::ensure!(!base_url.is_empty(), "KeyhanStudio cloud URL is empty");
        let parsed = reqwest::Url::parse(&base_url).context("Invalid KeyhanStudio cloud URL")?;
        anyhow::ensure!(
            parsed.scheme() == "https"
                || matches!(parsed.host_str(), Some("localhost" | "127.0.0.1")),
            "KeyhanStudio cloud requires HTTPS outside local development"
        );
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(format!("AudioMaster/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { base_url, http })
    }

    pub async fn start_device_login(&self) -> Result<DeviceAuthorization> {
        let response = self
            .http
            .post(self.url("/auth/device/code"))
            .json(&serde_json::json!({
                "client_info": format!("AudioMaster {}", env!("CARGO_PKG_VERSION"))
            }))
            .send()
            .await
            .context("Connecting to KeyhanStudio sign-in")?;
        decode_success(response, "Starting KeyhanStudio sign-in").await
    }

    pub async fn poll_device_login(&self, device_code: &str) -> Result<AuthorizationPoll> {
        anyhow::ensure!(!device_code.is_empty(), "Device code is empty");
        let response = self
            .http
            .post(self.url("/auth/device/token"))
            .json(&serde_json::json!({ "device_code": device_code }))
            .send()
            .await
            .context("Polling KeyhanStudio sign-in")?;
        decode_success(response, "Polling KeyhanStudio sign-in").await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair> {
        anyhow::ensure!(!refresh_token.is_empty(), "Refresh token is empty");
        let response = self
            .http
            .post(self.url("/auth/web/refresh"))
            .json(&serde_json::json!({ "refreshToken": refresh_token }))
            .send()
            .await
            .context("Refreshing KeyhanStudio session")?;
        decode_success(response, "Refreshing KeyhanStudio session").await
    }

    pub async fn revoke(&self, refresh_token: &str) -> Result<()> {
        anyhow::ensure!(!refresh_token.is_empty(), "Refresh token is empty");
        let response = self
            .http
            .post(self.url("/auth/web/revoke"))
            .json(&serde_json::json!({ "refreshToken": refresh_token }))
            .send()
            .await
            .context("Revoking KeyhanStudio session")?;
        decode_empty(response, "Revoking KeyhanStudio session").await
    }

    pub async fn get_sync(&self, access_token: &str) -> Result<CloudSyncDocument> {
        let response = self
            .http
            .get(self.url("/audiomaster/sync"))
            .bearer_auth(access_token)
            .send()
            .await
            .context("Downloading KeyhanStudio settings")?;
        decode_success(response, "Downloading KeyhanStudio settings").await
    }

    pub async fn put_sync(
        &self,
        access_token: &str,
        update: &CloudSyncUpdate<'_>,
    ) -> Result<CloudSyncDocument> {
        let encoded = serde_json::to_vec(update)?;
        anyhow::ensure!(
            encoded.len() <= MAX_SYNC_BYTES,
            "Cloud settings exceed 256 KiB"
        );
        let response = self
            .http
            .put(self.url("/audiomaster/sync"))
            .bearer_auth(access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(encoded)
            .send()
            .await
            .context("Uploading KeyhanStudio settings")?;
        if response.status() == StatusCode::CONFLICT {
            anyhow::bail!("Cloud settings changed on another device; download before retrying");
        }
        decode_success(response, "Uploading KeyhanStudio settings").await
    }

    pub async fn submit_feedback(
        &self,
        access_token: &str,
        feedback: &FeedbackRequest<'_>,
    ) -> Result<()> {
        anyhow::ensure!(
            !feedback.message.trim().is_empty(),
            "Feedback message is empty"
        );
        let encoded = serde_json::to_vec(feedback)?;
        anyhow::ensure!(
            encoded.len() <= MAX_FEEDBACK_BYTES,
            "Feedback exceeds 16 KiB"
        );
        let response = self
            .http
            .post(self.url("/audiomaster/feedback"))
            .bearer_auth(access_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(encoded)
            .send()
            .await
            .context("Sending feedback to KeyhanStudio")?;
        decode_empty(response, "Sending feedback to KeyhanStudio").await
    }

    pub async fn set_early_access(
        &self,
        access_token: &str,
        enabled: bool,
    ) -> Result<CloudSyncDocument> {
        let response = self
            .http
            .put(self.url("/audiomaster/early-access"))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "enabled": enabled }))
            .send()
            .await
            .context("Updating KeyhanStudio early-access enrollment")?;
        decode_success(response, "Updating KeyhanStudio early-access enrollment").await
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

async fn decode_success<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    operation: &str,
) -> Result<T> {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    let bytes = response.bytes().await?;
    if !status.is_success() {
        let detail = serde_json::from_slice::<serde_json::Value>(&bytes)
            .ok()
            .and_then(|value| value.get("error")?.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| {
                status
                    .canonical_reason()
                    .unwrap_or("request failed")
                    .to_string()
            });
        if status == StatusCode::TOO_MANY_REQUESTS {
            anyhow::bail!(
                "{operation} was rate limited; retry after {} seconds",
                retry_after.as_deref().unwrap_or("the server cooldown")
            );
        }
        anyhow::bail!("{operation} failed ({status}): {detail}");
    }
    serde_json::from_slice(&bytes).with_context(|| format!("{operation} returned invalid data"))
}

async fn decode_empty(response: reqwest::Response, operation: &str) -> Result<()> {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    if status.is_success() {
        return Ok(());
    }
    let detail = response
        .json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|value| value.get("error")?.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("request failed")
                .to_string()
        });
    if status == StatusCode::TOO_MANY_REQUESTS {
        anyhow::bail!(
            "{operation} was rate limited; retry after {} seconds",
            retry_after.as_deref().unwrap_or("the server cooldown")
        );
    }
    anyhow::bail!("{operation} failed ({status}): {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_insecure_remote_cloud_urls() {
        assert!(CloudClient::new("http://core.keyhan.info").is_err());
        assert!(CloudClient::new("http://localhost:3000").is_ok());
        assert!(CloudClient::new("https://core.keyhan.info").is_ok());
    }

    #[test]
    fn cloud_sync_contract_has_no_audio_fields() {
        let settings = serde_json::json!({ "target_lufs": -14.0 });
        let update = CloudSyncUpdate {
            base_revision: 1,
            settings: &settings,
            presets: &[],
        };
        let encoded = serde_json::to_string(&update).unwrap();
        assert!(!encoded.contains("audio"));
        assert!(!encoded.contains("path"));
    }

    #[test]
    fn device_token_response_decodes_existing_api_shape() {
        let poll: AuthorizationPoll = serde_json::from_value(serde_json::json!({
            "success": true,
            "token": "access",
            "refreshToken": "refresh",
            "expiresIn": 900,
            "user": { "id": "3fa85f64-5717-4562-b3fc-2c963f66afa6", "email": "master@example.com", "name": "Master", "picture": null }
        }))
        .unwrap();
        let tokens = poll
            .tokens
            .expect("authorized response should contain tokens");
        assert_eq!(tokens.refresh_token, "refresh");
        assert_eq!(tokens.expires_in, 900);
    }
}
