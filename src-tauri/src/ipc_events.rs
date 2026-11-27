//! Typed Tauri events shared between Rust and the webview via `tauri-specta`.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::features::stopwatch::StopwatchState;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct StopwatchUpdated(pub StopwatchState);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
pub struct AccountChanged;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
pub struct StoreUpdated;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct DeepLink(pub String);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
pub struct ScreenEffect;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct LauncherNavigate(pub String);
