//! `CalDAV` transport, discovery, account setup and resource client.
//!
//! The module is split into:
//!
//! - [`transport`] — the shared [`reqwest::Client`](transport::HttpClient) and
//!   the Basic-auth [`transport::CalDavClient`] that maps status codes onto
//!   [`crate::error::AppError`].
//! - [`xml`] — request bodies and `roxmltree` parsers for the captured Radicale
//!   responses.
//! - [`discovery`] — server URL normalisation and the
//!   `well-known` → principal → home → collections chain.
//! - [`account`] — the saved-profile `account.json` registry (password-free),
//!   per-profile OS keychain/env credentials, session leases and collection
//!   lifecycle.
//! - [`client`] — `sync-collection`, `calendar-multiget`, `PUT`, `DELETE`,
//!   `MKCALENDAR` and `PROPPATCH`.
//! - [`model`] — the discovered calendar metadata shared with the frontend.

pub mod account;
pub mod client;
pub mod discovery;
pub mod model;
pub mod transport;
pub mod xml;

#[cfg(test)]
mod tests;
