//! Folder-dependency checks for the layered backend layout.

use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_test_path(path: &Path) -> bool {
    let in_test_dir = path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name == "test" || name.starts_with("tests") || name.starts_with("test_")
    });
    let is_test_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with("tests.rs") || name.ends_with(".test.ts") || name.ends_with(".spec.ts")
        });
    in_test_dir || is_test_file
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
            continue;
        }
        files.push(path);
    }
}

fn rust_files(relative: &str) -> Vec<(PathBuf, String)> {
    let root = crate_root().join(relative);
    let mut files = Vec::new();
    collect_files(&root, &mut files);
    files.retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"));
    files.sort();
    files
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
