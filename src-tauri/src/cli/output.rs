//! JSON and date output for the headless CLI.

use chrono::{Duration, Local};
use serde::Serialize;

use crate::error::AppError;

/// # Errors
///
/// Returns [`AppError::Internal`] when the value cannot be encoded.
pub fn print_json<T: Serialize>(value: &T) -> Result<(), AppError> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| AppError::Internal(format!("could not encode JSON: {error}")))?;
    println!("{text}");
    Ok(())
}

#[must_use]
pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

#[must_use]
pub fn plus_days(days: i64) -> String {
    let later = Local::now()
        .checked_add_signed(Duration::days(days))
        .unwrap_or_else(Local::now);
    later.format("%Y-%m-%d").to_string()
}
