use crate::db;
use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use sqlx::SqlitePool;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

macro_rules! get_client {
    ($err_mapper:expr) => {{
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| $err_mapper("Missing GOOGLE_CLIENT_ID in .env".to_string()))?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();
        
        BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).map_err(|e| $err_mapper(e.to_string()))?)
            .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).map_err(|e| $err_mapper(e.to_string()))?)
            .set_redirect_uri(RedirectUrl::new("taskroot://auth-callback".to_string()).map_err(|e| $err_mapper(e.to_string()))?)
    }};
}

pub struct AuthState {
    pub pkce_verifier: Mutex<Option<PkceCodeVerifier>>,
    pub csrf_token: Mutex<Option<CsrfToken>>,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            pkce_verifier: Mutex::new(None),
            csrf_token: Mutex::new(None),
        }
    }
}

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn get_google_auth_url(state: State<'_, AuthState>) -> Result<String, String> {
    let client = get_client!(|e: String| e);

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.events".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/tasks".to_string(),
        ))
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(pkce_challenge)
        .url();

    *state.pkce_verifier.lock().map_err(|e| e.to_string())? = Some(pkce_verifier);
    *state.csrf_token.lock().map_err(|e| e.to_string())? = Some(csrf_token);

    Ok(auth_url.to_string())
}

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn exchange_google_auth_code(
    app: AppHandle,
    state: State<'_, AuthState>,
    code: String,
    state_token: String,
) -> Result<(), String> {
    let csrf_token = state.csrf_token.lock().map_err(|e| e.to_string())?.take().ok_or("No CSRF token found")?;
    let pkce_verifier = state.pkce_verifier.lock().map_err(|e| e.to_string())?.take().ok_or("No PKCE verifier found")?;

    if state_token != *csrf_token.secret() {
        return Err("Invalid state token".to_string());
    }

    let client = get_client!(|e: String| e);

    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&oauth2::reqwest::Client::new())
        .await
        .map_err(|e| format!("Token exchange failed: {e}"))?;

    let pool = app.try_state::<SqlitePool>().ok_or("Database not initialized")?;

    let access_token = token_result.access_token().secret();
    db::set_setting(&pool, "google_access_token", access_token).await.map_err(|e| e.to_string())?;

    if let Some(refresh_token) = token_result.refresh_token() {
        db::set_setting(&pool, "google_refresh_token", refresh_token.secret())
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Some(expires_in) = token_result.expires_in() {
        let expires_at = Utc::now() + Duration::from_std(expires_in).unwrap_or_else(|_| Duration::seconds(3600));
        db::set_setting(&pool, "google_token_expires_at", &expires_at.to_rfc3339())
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_valid_access_token(pool: &SqlitePool) -> Result<String, anyhow::Error> {
    let access_token = db::get_setting(pool, "google_access_token").await?;
    let expires_at_str = db::get_setting(pool, "google_token_expires_at").await?;

    let needs_refresh = expires_at_str.is_none_or(|exp_str| {
        chrono::DateTime::parse_from_rfc3339(&exp_str).map_or(true, |expires_at| Utc::now() + Duration::minutes(5) > expires_at.with_timezone(&Utc))
    });

    if !needs_refresh {
        if let Some(token) = access_token {
            return Ok(token);
        }
    }

    let refresh_token_str = db::get_setting(pool, "google_refresh_token")
        .await?
        .ok_or_else(|| anyhow!("No refresh token found. User needs to log in again."))?;

    let client = get_client!(|e: String| anyhow!(e));

    let token_result = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token_str))
        .request_async(&oauth2::reqwest::Client::new())
        .await?;

    let new_access_token = token_result.access_token().secret();
    db::set_setting(pool, "google_access_token", new_access_token).await?;

    if let Some(expires_in) = token_result.expires_in() {
        let expires_at = Utc::now() + Duration::from_std(expires_in).unwrap_or_else(|_| Duration::seconds(3600));
        db::set_setting(pool, "google_token_expires_at", &expires_at.to_rfc3339()).await?;
    }

    if let Some(new_refresh_token) = token_result.refresh_token() {
        db::set_setting(pool, "google_refresh_token", new_refresh_token.secret()).await?;
    }

    Ok(new_access_token.clone())
}

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn is_logged_in(app: tauri::AppHandle) -> Result<bool, String> {
    let pool = app.try_state::<SqlitePool>().ok_or("Database not initialized")?;
    let access_token = db::get_setting(&pool, "google_access_token")
        .await
        .map_err(|e| e.to_string())?;
    Ok(access_token.is_some())
}
