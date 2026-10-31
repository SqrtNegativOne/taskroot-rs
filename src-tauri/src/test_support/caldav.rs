//! Wiremock-backed `CalDAV` server setup shared by the transport, discovery,
//! collection and store tests.

use wiremock::MockServer;

use crate::caldav::transport::{CalDavClient, HttpClient};

/// The credentials every mocked `CalDAV` request authenticates with.
pub(crate) const TEST_USERNAME: &str = "me";
pub(crate) const TEST_PASSWORD: &str = "secret";

/// Start a mock server and a client authenticated as [`TEST_USERNAME`].
#[must_use]
pub(crate) async fn start_server() -> (MockServer, CalDavClient) {
    let (server, http) = start_http_server().await;
    let client = CalDavClient::new(&http, &server.uri(), TEST_USERNAME, TEST_PASSWORD).unwrap();
    (server, client)
}

/// Start a mock server plus the HTTP client `discover` needs.
#[must_use]
pub(crate) async fn start_http_server() -> (MockServer, HttpClient) {
    let server = MockServer::start().await;
    let http = HttpClient::new().unwrap();
    (server, http)
}
