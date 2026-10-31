#![warn(clippy::pedantic, clippy::nursery)]
#![warn(unreachable_pub)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

pub mod bootstrap;
pub mod cli;
pub mod error;
pub mod features;
pub mod ipc;
pub mod ipc_events;
pub mod platform;
pub mod settings;
#[cfg(test)]
pub mod test_support;

pub use bootstrap::run;

#[cfg(test)]
mod architecture_tests;
