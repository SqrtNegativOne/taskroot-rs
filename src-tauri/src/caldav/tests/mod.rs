//! Unit and HTTP-contract tests for the `CalDAV` layer.
//!
//! The XML tests drive the real Radicale captures in `caldav/fixtures/`; the
//! transport and discovery tests run against a local `wiremock` server so the
//! gate needs no running Radicale. The single test that does need the real
//! server is `#[ignore]`d.

mod account;
mod client;
mod discovery;
mod transport;
mod xml;

use std::path::{Path, PathBuf};

use super::transport::{CalDavClient, HttpClient};

pub(super) const ROOT_PROPFIND: &str = include_str!("../fixtures/root_propfind.xml");
pub(super) const PRINCIPAL_PROPFIND: &str = include_str!("../fixtures/principal_propfind.xml");
pub(super) const HOME_PROPFIND: &str = include_str!("../fixtures/home_propfind.xml");
pub(super) const MULTIGET: &str = include_str!("../fixtures/multiget.xml");
pub(super) const UNAUTHORIZED: &str = include_str!("../fixtures/unauthorized_status.txt");

/// A `CalDavClient` pointed at a wiremock server.
pub(super) fn test_client(server: &wiremock::MockServer) -> CalDavClient {
    let http = HttpClient::new().unwrap();
    CalDavClient::new(&http, &server.uri(), "me", "secret").unwrap()
}

/// A unique, writable directory under `src-tauri/target/`.
///
/// The process sandbox denies writes outside the repository, and `%TEMP%` is
/// not writable, so account tests must stay under `target/`.
pub(super) fn scratch_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("caldav-tests")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
