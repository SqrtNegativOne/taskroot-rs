use crate::db;
use crate::error::AppError;
use crate::events;
use color_eyre::eyre::{eyre, Result};
use chrono::{Duration, Utc};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use sqlx::SqlitePool;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const OAUTH_LOOPBACK_PORT: u16 = 14265;

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

macro_rules! get_client {
    ($redirect_uri:expr, $err_mapper:expr) => {{
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map_err(|_| $err_mapper("Missing GOOGLE_CLIENT_ID in .env".to_string()))?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default();

        BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(
                AuthUrl::new(GOOGLE_AUTH_URL.to_string())
                    .map_err(|e| $err_mapper(e.to_string()))?,
            )
            .set_token_uri(
                TokenUrl::new(GOOGLE_TOKEN_URL.to_string())
                    .map_err(|e| $err_mapper(e.to_string()))?,
            )
            .set_redirect_uri(
                RedirectUrl::new($redirect_uri.to_string())
                    .map_err(|e| $err_mapper(e.to_string()))?,
            )
    }};
}

fn db_pool(app: &AppHandle) -> Result<tauri::State<'_, SqlitePool>, AppError> {
    app.try_state::<SqlitePool>()
        .ok_or_else(|| AppError::NotReady("Database not initialized yet".to_string()))
}

/// # Errors
/// Returns an error if the OAuth flow fails.
#[tauri::command]
pub async fn login_with_google(app: AppHandle) -> Result<(), AppError> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", OAUTH_LOOPBACK_PORT))
        .map_err(|e| AppError::Internal(format!("Failed to bind OAuth loopback listener: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| AppError::Internal(format!("Failed to resolve OAuth listener address: {e}")))?
        .port();
    let redirect_uri = format!("http://localhost:{port}");

    let client = get_client!(&redirect_uri, |e: String| AppError::Auth(e));

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.events".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/calendar.readonly".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/tasks".to_string(),
        ))
        .add_extra_param("access_type", "offline")
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Emit the URL instead of opening it automatically so the user can copy it
    app.emit(events::OAUTH_URL, auth_url.to_string())
        .map_err(|e| AppError::Internal(format!("Failed to emit {}: {}", events::OAUTH_URL, e)))?;

    // Block a worker thread to wait for the HTTP callback
    let (mut stream, _) = tokio::task::spawn_blocking(move || listener.accept())
        .await
        .map_err(|e| AppError::Auth(format!("OAuth callback task failed: {e}")))?
        .map_err(|e| AppError::Auth(format!("Failed to receive OAuth callback: {e}")))?;

    let mut buffer = [0; 4096];
    stream
        .read(&mut buffer)
        .map_err(|e| AppError::Auth(format!("Failed to read OAuth callback: {e}")))?;
    let request = String::from_utf8_lossy(&buffer);

    let parsed_params = (|| -> Option<(String, String)> {
        let line = request.lines().next()?;
        let path = line.strip_prefix("GET ")?.split(' ').next()?;
        if !path.starts_with("/?") {
            return None;
        }

        let parsed_url = url::Url::parse(&format!("http://localhost{path}")).ok()?;

        let mut parsed_code = None;
        let mut parsed_state = None;

        for (key, value) in parsed_url.query_pairs() {
            match key.as_ref() {
                "code" => parsed_code = Some(value.into_owned()),
                "state" => parsed_state = Some(value.into_owned()),
                _ => {}
            }
        }

        Some((parsed_code?, parsed_state?))
    })();

    let response = "HTTP/1.1 200 OK\r\n\r\n<html><body><h1>Login successful! You can close this window and return to Taskroot.</h1><script>window.close()</script></body></html>";
    let _ = stream.write_all(response.as_bytes());

    let (code, state) = parsed_params.ok_or_else(|| {
        AppError::InvalidInput("No code or state received from Google".to_string())
    })?;

    if state != *csrf_token.secret() {
        return Err(AppError::Auth("Invalid state token".to_string()));
    }

    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&oauth2::reqwest::Client::new())
        .await
        .map_err(|e| AppError::Auth(format!("Token exchange failed: {e}")))?;

    let pool = db_pool(&app)?;

    let access_token = token_result.access_token().secret();
    db::set_setting(&pool, "google_access_token", access_token).await?;

    if let Some(refresh_token) = token_result.refresh_token() {
        db::set_setting(&pool, "google_refresh_token", refresh_token.secret()).await?;
    }

    if let Some(expires_in) = token_result.expires_in() {
        let expires_at =
            Utc::now().checked_add_signed(Duration::from_std(expires_in).unwrap_or_else(|_| Duration::seconds(3600))).unwrap_or_else(Utc::now);
        db::set_setting(&pool, "google_token_expires_at", &expires_at.to_rfc3339()).await?;
    }

    Ok(())
}

/// # Errors
/// Returns an error if the operation fails.
pub async fn get_valid_access_token(pool: &SqlitePool) -> Result<String, color_eyre::eyre::Error> {
    let access_token = db::get_setting(pool, "google_access_token").await?;
    let expires_at_str = db::get_setting(pool, "google_token_expires_at").await?;

    let needs_refresh = expires_at_str.is_none_or(|exp_str| {
        chrono::DateTime::parse_from_rfc3339(&exp_str).map_or(true, |expires_at| {
            Utc::now().checked_add_signed(Duration::minutes(5)).unwrap_or_else(Utc::now) > expires_at.with_timezone(&Utc)
        })
    });

    if !needs_refresh {
        if let Some(token) = access_token {
            return Ok(token);
        }
    }

    let refresh_token_str = db::get_setting(pool, "google_refresh_token")
        .await?
        .ok_or_else(|| eyre!("No refresh token found. User needs to log in again."))?;

    let client = get_client!("http://localhost", |e: String| eyre!(e));

    let token_result = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token_str))
        .request_async(&oauth2::reqwest::Client::new())
        .await?;

    let new_access_token = token_result.access_token().secret();
    db::set_setting(pool, "google_access_token", new_access_token).await?;

    if let Some(expires_in) = token_result.expires_in() {
        let expires_at =
            Utc::now().checked_add_signed(Duration::from_std(expires_in).unwrap_or_else(|_| Duration::seconds(3600))).unwrap_or_else(Utc::now);
        db::set_setting(pool, "google_token_expires_at", &expires_at.to_rfc3339()).await?;
    }

    if let Some(new_refresh_token) = token_result.refresh_token() {
        db::set_setting(pool, "google_refresh_token", new_refresh_token.secret()).await?;
    }

    Ok(new_access_token.clone())
}

/// # Errors
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn is_logged_in(app: tauri::AppHandle) -> Result<bool, AppError> {
    let pool = db_pool(&app)?;
    let access_token = db::get_setting(&pool, "google_access_token").await?;
    Ok(access_token.is_some())
}

/// # Errors
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn reset_auth(app: tauri::AppHandle) -> Result<(), AppError> {
    let pool = db_pool(&app)?;
    db::delete_setting(&pool, "google_access_token").await?;
    db::delete_setting(&pool, "google_refresh_token").await?;
    db::delete_setting(&pool, "google_token_expires_at").await?;
    Ok(())
}
