//! Server URL normalisation and `CalDAV` discovery.
//!
//! Full discovery runs when the account is connected. The cheaper half — the
//! `PROPFIND` of the calendar home — also runs at every startup
//! ([`refresh_collections`]), so `account.json` cannot go stale:
//!
//! `PROPFIND /.well-known/caldav` → `current-user-principal` →
//! `calendar-home-set` → `PROPFIND <home> Depth: 1` → calendars.
//!
//! Radicale answers `/.well-known/caldav` with a `301` to `/`, so redirects are
//! followed by hand (at most [`MAX_REDIRECTS`] times) while preserving the
//! method, body and Basic credentials. When the well-known endpoint cannot
//! produce a principal, discovery falls back to a `PROPFIND` on `/`.

use std::path::Path;
use std::time::Duration;

use reqwest::Url;

use crate::error::AppError;

use super::account::{self, Account};
use super::model::CalendarInfo;
use super::transport::{check_status, CalDavClient, CalDavRequest, HttpClient};
use super::xml::{
    parse_calendar_home_set, parse_collections, parse_current_user_principal, HOME_PROPFIND_BODY,
    PRINCIPAL_PROPFIND_BODY, ROOT_PROPFIND_BODY,
};

/// The local Radicale instance the app ships with.
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:5232";

const WELL_KNOWN_PATH: &str = "/.well-known/caldav";
const MAX_REDIRECTS: usize = 5;

/// The result of a successful discovery run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// The normalised server URL, no trailing slash.
    pub server_url: String,
    pub principal_href: String,
    pub home_href: String,
    pub calendars: Vec<CalendarInfo>,
}

/// Normalise a user-entered server URL.
///
/// - An empty value means [`DEFAULT_SERVER_URL`].
/// - A missing scheme defaults to `https://`, except for loopback hosts, which
///   default to plain `http://`.
/// - Plain `http` is only accepted for `127.0.0.1`, `localhost`, `::1` and
///   `ts.net` / `*.ts.net`; every other host must use `https`.
/// - A trailing slash is stripped.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for an unparsable URL, an unsupported
/// scheme, embedded credentials, or plain `http` on a non-loopback host.
pub fn normalize_server_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let input = if trimmed.is_empty() {
        DEFAULT_SERVER_URL
    } else {
        trimmed
    };
    let has_scheme = input.contains("://");
    let candidate = if has_scheme {
        input.to_string()
    } else {
        format!("http://{input}")
    };
    let mut url = Url::parse(&candidate).map_err(|error| {
        AppError::InvalidInput(format!("`{input}` is not a valid server URL: {error}"))
    })?;

    // A missing scheme defaults to https, except for a loopback host where the
    // app's own Radicale lives.
    if !has_scheme && !is_loopback_host(&url) {
        url.set_scheme("https").map_err(|()| {
            AppError::InvalidInput(format!("`{input}` cannot be used as an https URL"))
        })?;
    }

    match url.scheme() {
        "https" => {}
        "http" if is_plain_http_host(&url) => {}
        "http" => {
            return Err(AppError::InvalidInput(format!(
                "Plain http is only allowed for loopback or tailnet hosts; \
                 use https:// for `{}`",
                url.host_str().unwrap_or_default()
            )));
        }
        other => {
            return Err(AppError::InvalidInput(format!(
                "Unsupported URL scheme `{other}`; use http or https"
            )));
        }
    }

    if !url.username().is_empty() {
        return Err(AppError::InvalidInput(
            "The server URL must not contain a username or password".to_string(),
        ));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(AppError::InvalidInput(format!("`{input}` has no host")));
    }

    url.set_query(None);
    url.set_fragment(None);
    Ok(url.as_str().trim_end_matches('/').to_string())
}

/// Discover the principal, calendar home and calendars for an account.
///
/// # Errors
///
/// Returns [`AppError::InvalidInput`] for a bad server URL and
/// [`AppError::Sync`] when the server is unreachable or not a `CalDAV` server.
pub async fn discover(
    http: &HttpClient,
    server_url: &str,
    username: &str,
    password: &str,
) -> Result<Discovery, AppError> {
    let server_url = normalize_server_url(server_url)?;
    let client = CalDavClient::new(http, &server_url, username, password)?;

    let principal_href = discover_principal(&client).await?;
    let principal_body = fetch_propfind(&client, &principal_href, PRINCIPAL_PROPFIND_BODY).await?;
    let home_href = parse_calendar_home_set(&principal_body, &principal_href)?;
    let calendars = fetch_calendars(&client, &home_href).await?;

    Ok(Discovery {
        server_url,
        principal_href,
        home_href,
        calendars,
    })
}

/// Re-read the collections under `home_href` without rerunning principal
/// discovery. Used after creating/renaming/deleting a collection.
///
/// # Errors
///
/// Propagates transport errors and returns [`AppError::Sync`] for a malformed
/// `PROPFIND` response.
pub async fn fetch_calendars(
    client: &CalDavClient,
    home_href: &str,
) -> Result<Vec<CalendarInfo>, AppError> {
    let home_body = client
        .send(
            &CalDavRequest::propfind(home_href)
                .with_depth("1")
                .with_body(HOME_PROPFIND_BODY),
        )
        .await?
        .body;
    parse_collections(&home_body, home_href)
}

/// How long a startup refresh may take before the cached list is used instead.
/// The home `PROPFIND` is one request and normally tens of milliseconds, so this
/// only bites when the server is wedged.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(3);

/// Re-read the account's collection list, persisting it when it changed.
///
/// `account.json` is a cache of what the server has, not the truth: a list
/// created or deleted from another client would otherwise stay invisible until
/// the next full discovery, and a *deleted* one would fail every sync with a
/// `404`. Called at startup and by the CLI, so one `PROPFIND` per process.
///
/// When the server cannot be asked (offline, slow, malformed) the cached list is
/// returned unchanged; the caller's offline path then runs as before.
pub async fn refresh_collections(
    dir: &Path,
    client: &CalDavClient,
    account: &mut Account,
) -> Vec<CalendarInfo> {
    let fetched = tokio::time::timeout(
        REFRESH_TIMEOUT,
        fetch_calendars(client, &account.home_href),
    )
    .await;

    let Ok(Ok(calendars)) = fetched else {
        return account.calendars.clone();
    };

    if calendars != account.calendars {
        account.calendars.clone_from(&calendars);
        if let Err(error) = account::write_account(dir, account) {
            eprintln!("Could not persist the refreshed list of calendars: {error}");
        }
    }
    calendars
}

async fn discover_principal(client: &CalDavClient) -> Result<String, AppError> {
    match principal_from_well_known(client).await {
        Ok(principal) => Ok(principal),
        // Bad credentials will fail at `/` as well; keep the auth error instead
        // of wrapping it in a generic discovery failure.
        Err(error @ AppError::Auth(_)) => Err(error),
        Err(well_known_error) => {
            let body = match fetch_propfind(client, "/", ROOT_PROPFIND_BODY).await {
                Ok(body) => body,
                Err(error @ AppError::Auth(_)) => return Err(error),
                Err(root_error) => {
                    return Err(AppError::Sync(format!(
                        "CalDAV discovery failed at `{WELL_KNOWN_PATH}` ({well_known_error}) \
                         and at `/` ({root_error})"
                    )));
                }
            };
            parse_current_user_principal(&body, "/")
        }
    }
}

async fn principal_from_well_known(client: &CalDavClient) -> Result<String, AppError> {
    let mut target = WELL_KNOWN_PATH.to_string();
    for _ in 0..=MAX_REDIRECTS {
        let response = client
            .send_raw(
                &CalDavRequest::propfind(&target)
                    .with_depth("0")
                    .with_body(ROOT_PROPFIND_BODY),
            )
            .await?;
        if response.is_redirect() {
            let Some(location) = response.location else {
                return Err(AppError::Sync(
                    "The CalDAV server redirected without a Location header".to_string(),
                ));
            };
            target = location;
            continue;
        }
        let response = check_status(response)?;
        return parse_current_user_principal(&response.body, &target);
    }
    Err(AppError::Sync(
        "Too many redirects while discovering the CalDAV principal".to_string(),
    ))
}

async fn fetch_propfind(client: &CalDavClient, href: &str, body: &str) -> Result<String, AppError> {
    let response = client
        .send(
            &CalDavRequest::propfind(href)
                .with_depth("0")
                .with_body(body),
        )
        .await?;
    Ok(response.body)
}

fn normalized_host(url: &Url) -> Option<String> {
    let host = url.host_str()?;
    Some(
        host.trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_lowercase(),
    )
}

fn is_loopback_host(url: &Url) -> bool {
    normalized_host(url)
        .is_some_and(|host| matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1"))
}

fn is_plain_http_host(url: &Url) -> bool {
    is_loopback_host(url)
        || normalized_host(url).is_some_and(|host| host == "ts.net" || host.ends_with(".ts.net"))
}
