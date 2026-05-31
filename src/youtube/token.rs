use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const EXPIRY_SKEW_SECONDS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<u64>,
    pub token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    token_type: Option<String>,
}

pub async fn exchange_code_for_token(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<OAuthToken> {
    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
            ("code", code),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .context("Failed to exchange YouTube authorization code")?
        .error_for_status()
        .context("YouTube token endpoint rejected authorization code")?
        .json::<TokenResponse>()
        .await
        .context("Failed to parse YouTube token response")?;

    Ok(response.into_token(None))
}

pub async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<OAuthToken> {
    let response = reqwest::Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("Failed to refresh YouTube access token")?
        .error_for_status()
        .context("YouTube token endpoint rejected refresh token")?
        .json::<TokenResponse>()
        .await
        .context("Failed to parse YouTube refresh response")?;

    Ok(response.into_token(Some(refresh_token.to_string())))
}

pub fn is_access_token_expired(expires_at: Option<u64>) -> bool {
    let Some(expires_at) = expires_at else {
        return true;
    };

    now_unix_seconds().saturating_add(EXPIRY_SKEW_SECONDS) >= expires_at
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

impl TokenResponse {
    fn into_token(self, fallback_refresh_token: Option<String>) -> OAuthToken {
        OAuthToken {
            access_token: self.access_token,
            refresh_token: self.refresh_token.or(fallback_refresh_token),
            expires_at: self
                .expires_in
                .map(|expires_in| now_unix_seconds().saturating_add(expires_in)),
            token_type: self.token_type,
        }
    }
}
