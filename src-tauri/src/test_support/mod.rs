//! Shared support for the backend tests (see `docs/testing.md`).
//!
//! [`caldav`] starts wiremock servers with an authenticated client, [`ics`]
//! serves resource fixtures so the `CalDAV` and store tests share one setup,
//! and [`architecture`] enforces the layered module dependency rules.

pub mod architecture;
pub mod caldav;
pub mod ics;
