//! Desktop shell mechanics: data directory, windows and tray.
//!
//! This module never imports `bootstrap`, `ipc` or `features`.

mod data_dir;
mod tray;
mod windows;

pub use data_dir::{app_data_dir, configure_dev_webview_storage};
pub use tray::build_tray;
pub use windows::{
    WindowLabel, configure_effect_window, handle_window_event, hide_launcher, hide_screen_effect,
    quit_app, resize_launcher, restore_main, show_minitracker, show_screen_effect,
    toggle_launcher_window,
};
