//! Desktop shell mechanics: data directory, windows and tray.
//!
//! This module never imports `bootstrap`, `ipc` or `features`.

mod data_dir;
mod tray;
mod window_creation;
mod window_lifecycle;
mod windows;

pub use data_dir::{app_data_dir, configure_dev_webview_storage, configure_webview_flags};
pub use tray::build_tray;
pub use windows::{
    WindowLabel, handle_window_event, hide_launcher, hide_screen_effect, quit_app, resize_launcher,
    restore_main, set_minitracker_visible, show_screen_effect,
    start_minitracker_topmost_guard, toggle_launcher_window,
};

#[cfg(test)]
mod tests {
    use super::WindowLabel;

    #[test]
    fn every_configured_window_label_is_represented_in_window_label_enum() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json")).expect("tauri.conf.json");
        let windows = conf["app"]["windows"].as_array().expect("app.windows");
        let mut parsed_labels = Vec::new();

        for window in windows {
            let raw = window["label"].as_str().expect("window label");
            let parsed: WindowLabel = serde_json::from_value(serde_json::json!(raw))
                .unwrap_or_else(|err| panic!("window label `{raw}` missing from WindowLabel: {err}"));
            assert_eq!(parsed.as_str(), raw);
            parsed_labels.push(parsed);
        }

        assert!(parsed_labels.contains(&WindowLabel::Sidebar));
        assert_eq!(WindowLabel::Sidebar.as_str(), "sidebar");
    }
}
