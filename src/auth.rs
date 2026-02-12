use crate::config::Config;
use crate::error::{RelayError, Result};
use crate::types::TokenPair;
use chrono::{Duration, Utc};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, DeviceAuthorizationUrl, RefreshToken, Scope,
    TokenResponse, TokenUrl,
};
use parking_lot::RwLock;
use std::sync::Arc;

const KEYCHAIN_SERVICE: &str = "com.supervaize.skill-cookbook-relay";
const KEYCHAIN_REFRESH_TOKEN_KEY: &str = "refresh_token";
const KEYCHAIN_ACCESS_TOKEN_KEY: &str = "access_token";

pub struct AuthManager {
    config: Config,
    oauth_client: BasicClient,
    current_token: Arc<RwLock<Option<TokenPair>>>,
    _device_id: String,
}

impl AuthManager {
    pub fn new(config: Config) -> Result<Self> {
        let oauth_client = BasicClient::new(
            ClientId::new(config.oauth_client_id.clone()),
            None,
            AuthUrl::new(config.oauth_auth_url.clone())
                .map_err(|e| RelayError::Auth(format!("Invalid auth URL: {}", e)))?,
            Some(
                TokenUrl::new(config.oauth_token_url.clone())
                    .map_err(|e| RelayError::Auth(format!("Invalid token URL: {}", e)))?,
            ),
        )
        .set_device_authorization_url(
            DeviceAuthorizationUrl::new(format!("{}/device", config.oauth_auth_url))
                .map_err(|e| RelayError::Auth(format!("Invalid device auth URL: {}", e)))?,
        );

        let device_id = Config::device_id()?;

        Ok(Self {
            config,
            oauth_client,
            current_token: Arc::new(RwLock::new(None)),
            _device_id: device_id,
        })
    }

    /// Initialize auth manager and load existing tokens from keychain
    pub async fn initialize(&self) -> Result<bool> {
        match self.load_tokens_from_keychain() {
            Ok(token_pair) => {
                *self.current_token.write() = Some(token_pair.clone());

                // Check if token is expired and refresh if needed
                if token_pair.expires_at <= Utc::now() {
                    tracing::info!("Access token expired, refreshing...");
                    self.refresh_access_token().await?;
                }

                Ok(true)
            }
            Err(e) => {
                tracing::debug!("No existing tokens found: {}", e);
                Ok(false)
            }
        }
    }

    /// Start OAuth device flow for user login
    pub async fn start_device_flow(&self) -> Result<DeviceAuthorizationResponse> {
        use oauth2::StandardDeviceAuthorizationResponse;

        let device_auth_response: StandardDeviceAuthorizationResponse = self
            .oauth_client
            .exchange_device_code()
            .map_err(|e| RelayError::Auth(format!("Failed to start device flow: {}", e)))?
            .add_scope(Scope::new("read:libraries".to_string()))
            .add_scope(Scope::new("read:skills".to_string()))
            .add_scope(Scope::new("read:artifacts".to_string()))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| RelayError::Auth(format!("Device authorization request failed: {}", e)))?;

        Ok(DeviceAuthorizationResponse {
            device_code: device_auth_response.device_code().secret().clone(),
            user_code: device_auth_response.user_code().secret().clone(),
            verification_uri: device_auth_response.verification_uri().url().to_string(),
            verification_uri_complete: device_auth_response
                .verification_uri_complete()
                .map(|u| u.secret().to_string()),
            expires_in: device_auth_response.expires_in().as_secs(),
            interval: device_auth_response.interval().as_secs(),
        })
    }

    /// Poll for device flow completion and exchange for tokens
    pub async fn poll_device_flow(
        &self,
        device_code: String,
        interval_secs: u64,
    ) -> Result<TokenPair> {
        use oauth2::DeviceCode;

        let device_code = DeviceCode::new(device_code);
        let interval = std::time::Duration::from_secs(interval_secs);

        loop {
            tokio::time::sleep(interval).await;

            // In oauth2 v4, we need to build the full device flow manually
            // For simplicity, we'll use the basic token endpoint directly
            let token_url = self
                .oauth_client
                .token_url()
                .ok_or_else(|| RelayError::Auth("No token URL configured".to_string()))?;

            let params = [
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device_code.secret()),
                ("client_id", self.config.oauth_client_id.as_str()),
            ];

            let client = reqwest::Client::new();
            let response = client
                .post(token_url.as_str())
                .form(&params)
                .send()
                .await
                .map_err(|e| RelayError::Auth(format!("Token request failed: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                if status.as_u16() == 400 {
                    // Check for pending/slow_down/expired
                    tokio::time::sleep(interval).await;
                    continue;
                }
                return Err(RelayError::Auth(format!(
                    "Token request failed with status: {}",
                    status
                )));
            }

            match response.json::<serde_json::Value>().await {
                Ok(json) if json.get("access_token").is_some() => {
                    let access_token = json["access_token"]
                        .as_str()
                        .ok_or_else(|| RelayError::Auth("Invalid access token".to_string()))?
                        .to_string();
                    let refresh_token = json
                        .get("refresh_token")
                        .and_then(|t| t.as_str())
                        .ok_or_else(|| RelayError::Auth("No refresh token received".to_string()))?
                        .to_string();
                    let expires_in = json
                        .get("expires_in")
                        .and_then(|e| e.as_u64())
                        .unwrap_or(3600);
                    let token_type = json
                        .get("token_type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Bearer")
                        .to_string();

                    let expires_at = Utc::now() + Duration::seconds(expires_in as i64);

                    let token_pair = TokenPair {
                        access_token,
                        refresh_token,
                        expires_at,
                        token_type,
                    };

                    // Save to keychain
                    self.save_tokens_to_keychain(&token_pair)?;

                    // Update in-memory token
                    *self.current_token.write() = Some(token_pair.clone());

                    return Ok(token_pair);
                }
                Ok(_) => {
                    // No access_token, might be error response
                    tokio::time::sleep(interval).await;
                    continue;
                }
                Err(e) => {
                    return Err(RelayError::Auth(format!(
                        "Failed to parse token response: {}",
                        e
                    )));
                }
            }
        }
    }

    /// Refresh the access token using the refresh token
    pub async fn refresh_access_token(&self) -> Result<TokenPair> {
        let current_token = self
            .current_token
            .read()
            .clone()
            .ok_or_else(|| RelayError::Auth("No refresh token available".to_string()))?;

        let refresh_token = RefreshToken::new(current_token.refresh_token.clone());

        let token_response = self
            .oauth_client
            .exchange_refresh_token(&refresh_token)
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| RelayError::Auth(format!("Token refresh failed: {}", e)))?;

        let expires_in = token_response
            .expires_in()
            .unwrap_or(std::time::Duration::from_secs(3600));
        let expires_at = Utc::now() + Duration::seconds(expires_in.as_secs() as i64);

        let new_token_pair = TokenPair {
            access_token: token_response.access_token().secret().clone(),
            refresh_token: token_response
                .refresh_token()
                .map(|t| t.secret().clone())
                .unwrap_or(current_token.refresh_token), // Use old refresh token if not provided
            expires_at,
            token_type: token_response.token_type().as_ref().to_string(),
        };

        // Save to keychain
        self.save_tokens_to_keychain(&new_token_pair)?;

        // Update in-memory token
        *self.current_token.write() = Some(new_token_pair.clone());

        tracing::info!("Access token refreshed successfully");

        Ok(new_token_pair)
    }

    /// Get current access token, refreshing if needed
    pub async fn get_access_token(&self) -> Result<String> {
        let token = self
            .current_token
            .read()
            .clone()
            .ok_or_else(|| RelayError::Auth("Not authenticated".to_string()))?;

        // Check if token is expired or will expire soon (within 5 minutes)
        if token.expires_at <= Utc::now() + Duration::minutes(5) {
            tracing::debug!("Token expired or expiring soon, refreshing...");
            let refreshed_token = self.refresh_access_token().await?;
            Ok(refreshed_token.access_token)
        } else {
            Ok(token.access_token)
        }
    }

    /// Check if user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.current_token.read().is_some()
    }

    /// Logout and clear all tokens
    pub fn logout(&self) -> Result<()> {
        // Clear in-memory token
        *self.current_token.write() = None;

        // Clear keychain
        self.clear_tokens_from_keychain()?;

        tracing::info!("Logged out successfully");

        Ok(())
    }

    /// Save tokens to OS keychain
    fn save_tokens_to_keychain(&self, token_pair: &TokenPair) -> Result<()> {
        let keyring = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_REFRESH_TOKEN_KEY)
            .map_err(|e| RelayError::Keychain(format!("Failed to access keychain: {}", e)))?;

        keyring
            .set_password(&token_pair.refresh_token)
            .map_err(|e| RelayError::Keychain(format!("Failed to save refresh token: {}", e)))?;

        let keyring_access = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCESS_TOKEN_KEY)
            .map_err(|e| RelayError::Keychain(format!("Failed to access keychain: {}", e)))?;

        let token_data = serde_json::to_string(&token_pair)
            .map_err(|e| RelayError::Keychain(format!("Failed to serialize token: {}", e)))?;

        keyring_access
            .set_password(&token_data)
            .map_err(|e| RelayError::Keychain(format!("Failed to save access token: {}", e)))?;

        tracing::debug!("Tokens saved to keychain");

        Ok(())
    }

    /// Load tokens from OS keychain
    fn load_tokens_from_keychain(&self) -> Result<TokenPair> {
        let keyring = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCESS_TOKEN_KEY)
            .map_err(|e| RelayError::Keychain(format!("Failed to access keychain: {}", e)))?;

        let token_data = keyring
            .get_password()
            .map_err(|e| RelayError::Keychain(format!("Failed to read tokens: {}", e)))?;

        let token_pair: TokenPair = serde_json::from_str(&token_data)
            .map_err(|e| RelayError::Keychain(format!("Failed to deserialize token: {}", e)))?;

        tracing::debug!("Tokens loaded from keychain");

        Ok(token_pair)
    }

    /// Clear tokens from OS keychain
    fn clear_tokens_from_keychain(&self) -> Result<()> {
        let keyring_refresh = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_REFRESH_TOKEN_KEY)
            .map_err(|e| RelayError::Keychain(format!("Failed to access keychain: {}", e)))?;

        let keyring_access = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCESS_TOKEN_KEY)
            .map_err(|e| RelayError::Keychain(format!("Failed to access keychain: {}", e)))?;

        // Ignore errors if tokens don't exist
        let _ = keyring_refresh.delete_password();
        let _ = keyring_access.delete_password();

        tracing::debug!("Tokens cleared from keychain");

        Ok(())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DeviceAuthorizationResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}
