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

pub(super) const ROOT_PROPFIND: &str = include_str!("../fixtures/root_propfind.xml");
pub(super) const PRINCIPAL_PROPFIND: &str = include_str!("../fixtures/principal_propfind.xml");
pub(super) const HOME_PROPFIND: &str = include_str!("../fixtures/home_propfind.xml");
pub(super) const MULTIGET: &str = include_str!("../fixtures/multiget.xml");
pub(super) const UNAUTHORIZED: &str = include_str!("../fixtures/unauthorized_status.txt");
