//! HTTP transport for the `CalDAV` client.
//!
//! One long-lived [`reqwest::Client`] is built once and registered in Tauri's
//! managed state as [`HttpClient`]; every [`CalDavClient`] clones that client
//! instead of building a new connection pool per request.
//!
//! [`CalDavClient::send`] maps the `CalDAV`-relevant failure statuses onto the
//! application's error contract (`401` → auth, `412` → conflict, `410` → gone,
//! any other non-`2xx` → sync). [`CalDavClient::send_raw`] skips that mapping
//! so discovery can follow a redirect and so a delete can treat `404` as
//! success.

use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, ETAG, IF_MATCH, IF_NONE_MATCH, LOCATION};
use reqwest::{Method, Url};

use crate::error::AppError;

use super::xml::{CALENDAR_CONTENT_TYPE, XML_CONTENT_TYPE};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SNIPPET_LENGTH: usize = 160;

/// The process-wide HTTP client, cloneable and cheap to hand out per request.
#[derive(Clone)]
pub struct HttpClient(pub reqwest::Client);

impl HttpClient {
    /// Build the shared client.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when the TLS backend cannot initialise.
    pub fn new() -> Result<Self, AppError> {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map(Self)
            .map_err(|error| AppError::Internal(format!("Failed to build HTTP client: {error}")))
    }
}

/// A `CalDAV` request, resolved against the client's server URL.
pub struct CalDavRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub depth: Option<&'a str>,
    pub body: Option<&'a str>,
    pub if_match: Option<&'a str>,
    pub create_only: bool,
    pub content_type: &'a str,
}

impl<'a> CalDavRequest<'a> {
    #[must_use]
    pub const fn new(method: &'a str, path: &'a str) -> Self {
        Self {
            method,
            path,
            depth: None,
            body: None,
            if_match: None,
            create_only: false,
            content_type: XML_CONTENT_TYPE,
        }
    }

    #[must_use]
    pub const fn propfind(path: &'a str) -> Self {
        Self::new("PROPFIND", path)
    }

    #[must_use]
    pub const fn report(path: &'a str) -> Self {
        Self::new("REPORT", path)
    }

    #[must_use]
    pub const fn put(path: &'a str, body: &'a str) -> Self {
        Self::new("PUT", path)
            .with_body(body)
            .with_content_type(CALENDAR_CONTENT_TYPE)
    }

    #[must_use]
    pub const fn delete(path: &'a str) -> Self {
        Self::new("DELETE", path)
    }

    #[must_use]
    pub const fn mkcalendar(path: &'a str, body: &'a str) -> Self {
        Self::new("MKCALENDAR", path).with_body(body)
    }

    #[must_use]
    pub const fn proppatch(path: &'a str, body: &'a str) -> Self {
        Self::new("PROPPATCH", path).with_body(body)
    }

    #[must_use]
    pub const fn with_depth(mut self, depth: &'a str) -> Self {
        self.depth = Some(depth);
        self
    }

    #[must_use]
    pub const fn with_body(mut self, body: &'a str) -> Self {
        self.body = Some(body);
        self
    }

    #[must_use]
    pub const fn with_content_type(mut self, content_type: &'a str) -> Self {
        self.content_type = content_type;
        self
    }

    #[must_use]
    pub const fn with_if_match(mut self, etag: &'a str) -> Self {
        self.if_match = Some(etag);
        self
    }

    #[must_use]
    pub const fn create_only(mut self) -> Self {
        self.create_only = true;
        self
    }
}

/// What every request returns: the status, the response `ETag` (quotes kept) and
/// the decoded body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub etag: Option<String>,
    pub location: Option<String>,
    pub body: String,
}

impl Response {
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    #[must_use]
    pub const fn is_redirect(&self) -> bool {
        matches!(self.status, 301 | 302 | 303 | 307 | 308)
    }
}

/// A `CalDAV` client bound to one server and one set of Basic credentials.
pub struct CalDavClient {
    http: reqwest::Client,
    base_url: Url,
    username: String,
    password: String,
}

impl CalDavClient {
    /// Bind a client to `base_url` (already normalised by discovery).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] for an unparsable or hostless URL.
    pub fn new(
        http: &HttpClient,
        base_url: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, AppError> {
        let base_url = Url::parse(base_url).map_err(|error| {
            AppError::InvalidInput(format!("`{base_url}` is not a valid server URL: {error}"))
        })?;
        if base_url.host_str().is_none_or(str::is_empty) {
            return Err(AppError::InvalidInput(format!("`{base_url}` has no host")));
        }
        Ok(Self {
            http: http.0.clone(),
            base_url,
            username: username.to_string(),
            password: password.to_string(),
        })
    }

    #[must_use]
    pub const fn base_url(&self) -> &Url {
        &self.base_url
    }

    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Send a request and map a non-`2xx` status onto [`AppError`].
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Auth`] for `401`, [`AppError::Conflict`] for `412`,
    /// [`AppError::Gone`] for `410` and [`AppError::Sync`] for any other
    /// non-`2xx` status or transport failure.
    pub async fn send(&self, request: &CalDavRequest<'_>) -> Result<Response, AppError> {
        check_status(self.send_raw(request).await?)
    }

    /// Send a request without interpreting its status.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Sync`] when the request cannot be completed.
    pub async fn send_raw(&self, request: &CalDavRequest<'_>) -> Result<Response, AppError> {
        let url = self.resolve(request.path)?;
        let method = Method::from_bytes(request.method.as_bytes()).map_err(|error| {
            AppError::Internal(format!("Invalid HTTP method `{}`: {error}", request.method))
        })?;

        let mut builder = self
            .http
            .request(method, url)
            .basic_auth(&self.username, Some(&self.password));

        if let Some(depth) = request.depth {
            builder = builder.header("Depth", depth);
        }
        if let Some(etag) = request.if_match {
            builder = builder.header(IF_MATCH, etag);
        }
        if request.create_only {
            builder = builder.header(IF_NONE_MATCH, "*");
        }
        if let Some(body) = request.body {
            builder = builder
                .header(CONTENT_TYPE, request.content_type)
                .body(body.to_string());
        }

        let response = builder
            .send()
            .await
            .map_err(|error| AppError::Sync(format!("CalDAV request failed: {error}")))?;
        let status = response.status().as_u16();
        let etag = header_string(&response, ETAG);
        let location = header_string(&response, LOCATION);
        let body = response.text().await.map_err(|error| {
            AppError::Sync(format!("CalDAV response could not be read: {error}"))
        })?;

        Ok(Response {
            status,
            etag,
            location,
            body,
        })
    }

    fn resolve(&self, target: &str) -> Result<Url, AppError> {
        self.base_url.join(target).map_err(|error| {
            AppError::InvalidInput(format!("Invalid CalDAV path `{target}`: {error}"))
        })
    }
}

/// Turn a non-`2xx` response into the matching [`AppError`].
///
/// # Errors
///
/// See [`CalDavClient::send`].
pub fn check_status(response: Response) -> Result<Response, AppError> {
    if response.is_success() {
        return Ok(response);
    }
    Err(match response.status {
        401 => AppError::Auth("The CalDAV server rejected the credentials (401)".to_string()),
        412 => AppError::Conflict(
            "The CalDAV resource changed on the server; re-fetch and retry (412)".to_string(),
        ),
        410 => AppError::Gone("The CalDAV resource or sync token is gone (410)".to_string()),
        status => AppError::Sync(format!(
            "CalDAV request failed with status {status}: {}",
            snippet(&response.body)
        )),
    })
}

fn header_string(
    response: &reqwest::Response,
    header: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(header)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn snippet(body: &str) -> String {
    let snippet: String = body.trim().chars().take(SNIPPET_LENGTH).collect();
    if snippet.is_empty() {
        "(empty response)".to_string()
    } else {
        snippet
    }
}
