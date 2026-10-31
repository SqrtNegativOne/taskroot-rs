//! Folder-dependency checks for the layered backend layout.

use std::path::{Path, PathBuf};

use super::source_scan::{crate_root, is_test_path, sources};

fn rust_files(relative: &str) -> Vec<(PathBuf, String)> {
    let root = crate_root().join(relative);
    sources(&root, &["rs"])
        .into_iter()
        .map(|path| {
            let source = std::fs::read_to_string(&path).expect("source is readable");
            (path, source)
        })
        .collect()
}

fn files_containing(relative: &str, needle: &str) -> Vec<String> {
    rust_files(relative)
        .into_iter()
        .filter(|(_, source)| source.contains(needle))
        .map(|(path, _)| display_path(&path))
        .collect()
}

fn display_path(path: &Path) -> String {
    path.strip_prefix(crate_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[must_use]
pub(crate) fn calendar_tauri_imports() -> Vec<String> {
    files_containing("src/features/calendar", "use tauri::")
}

#[must_use]
pub(crate) fn command_attributes_outside_ipc() -> Vec<String> {
    rust_files("src")
        .into_iter()
        .filter(|(path, source)| {
            !is_test_path(path)
                && source.contains("#[tauri::command]")
                && !path.starts_with(crate_root().join("src/ipc"))
        })
        .map(|(path, _)| display_path(&path))
        .collect()
}

#[must_use]
pub(crate) fn forbidden_imports_in(relative: &str, prefixes: &[&str]) -> Vec<String> {
    rust_files(relative)
        .into_iter()
        .filter(|(_, source)| {
            prefixes
                .iter()
                .any(|prefix| source.contains(&format!("crate::{prefix}")))
        })
        .map(|(path, _)| display_path(&path))
        .collect()
}
