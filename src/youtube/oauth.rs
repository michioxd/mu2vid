use anyhow::{Context, Result};
use url::Url;

use crate::youtube::token::OAuthToken;

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const YOUTUBE_UPLOAD_SCOPE: &str = "https://www.googleapis.com/auth/youtube.upload";

pub fn build_auth_url(client_id: &str, redirect_uri: &str) -> String {
    let mut url = Url::parse(AUTH_ENDPOINT).expect("valid Google OAuth URL");
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", YOUTUBE_UPLOAD_SCOPE);
    url.into()
}

pub fn parse_code_from_redirect_url(url: &str) -> Result<String> {
    let url = Url::parse(url.trim()).context("Invalid redirect URL")?;

    if let Some(error) = url
        .query_pairs()
        .find_map(|(key, value)| (key == "error").then(|| value.into_owned()))
    {
        anyhow::bail!("YouTube OAuth failed: {error}");
    }

    url.query_pairs()
        .find_map(|(key, value)| (key == "code").then(|| value.into_owned()))
        .filter(|code| !code.trim().is_empty())
        .context("Redirect URL does not contain an authorization code")
}

pub async fn exchange_code_for_token(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<OAuthToken> {
    crate::youtube::token::exchange_code_for_token(client_id, client_secret, redirect_uri, code)
        .await
}

pub async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<OAuthToken> {
    crate::youtube::token::refresh_access_token(client_id, client_secret, refresh_token).await
}
