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
//! - [`account`] — `account.json` (password-free) and the OS keychain password.
//! - [`client`] — `sync-collection`, `calendar-multiget`, `PUT`, `DELETE`,
//!   `MKCALENDAR` and `PROPPATCH`.
//! - [`calendars`] — collection lifecycle (create, rename, recolour, delete).
//! - [`connect`] — shared account/password/client session setup.
//! - [`model`] — the discovered calendar metadata shared with the frontend.

pub mod account;
pub mod calendars;
pub mod client;
pub mod connect;
pub mod discovery;
pub mod model;
pub mod transport;
pub mod xml;

#[cfg(test)]
mod tests;
