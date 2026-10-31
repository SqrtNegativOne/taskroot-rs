//! In-memory store and JSON persistence.
//!
//! The resource map is filled from `CalDAV` at startup and on Refresh; edits
//! merge into the stored raw body and `PUT` it back immediately.

pub mod model;
pub mod ops;
pub mod persist;
pub mod query;
pub mod state;

pub use model::{Resource, ResourceItems, Store};
pub use state::StoreState;

#[cfg(test)]
mod tests;
