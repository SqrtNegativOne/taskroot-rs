//! The Rust half of the IPC source contract used by the command-layer tests.
//!
//! The `generate_handler!` table in `lib.rs` and the `#[tauri::command]` signatures
//! under `src`, plus the file-walking helpers the sibling
//! [`super::frontend_scan`] shares.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The command names listed by the `generate_handler!` table in `lib.rs`.
pub(crate) fn registered_commands() -> Vec<String> {
    between(include_str!("../lib.rs"), "generate_handler![", ']')
        .split(',')
        .filter_map(|entry| entry.rsplit("::").next())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

/// Every `#[tauri::command]` handler under `src`, keyed by function name.
///
/// The value is the handler's parameter list minus the `AppHandle`, i.e. exactly
/// the arguments the webview has to supply.
pub(crate) fn annotated_command_params() -> BTreeMap<String, Vec<String>> {
    let mut commands = BTreeMap::new();
    for source in rust_sources() {
        for chunk in source.split("#[tauri::command]").skip(1) {
            if let Some((name, params)) = split_signature(chunk) {
                commands.insert(name, params);
            }
        }
    }
    commands
}

/// The argument key `#[tauri::command]` looks up for a Rust parameter
/// (`heck::ToLowerCamelCase`, e.g. `start_time` → `startTime`).
pub(crate) fn js_key(param: &str) -> String {
    let mut parts = param.split('_');
    let mut key = parts.next().unwrap_or_default().to_string();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            key.extend(first.to_uppercase());
        }
        key.push_str(chars.as_str());
    }
    key
}

/// The function name and non-`AppHandle` parameters that follow a
/// `#[tauri::command]` attribute.
///
/// `None` for a mention of the attribute inside a comment: nothing but the
/// attribute itself may sit between it and the signature.
fn split_signature(chunk: &str) -> Option<(String, Vec<String>)> {
    let (before_fn, after_fn) = chunk.split_once("fn ")?;
    if before_fn.contains("//") {
        return None;
    }

    let (name, after_name) = after_fn
        .split_once('(')
        .expect("a command function takes parameters");
    let (params, _) = after_name
        .split_once(')')
        .expect("a command function closes its parameter list");

    let params = params
        .split(',')
        .filter_map(|param| param.split(':').next())
        .map(|param| param.trim().trim_start_matches("mut ").trim())
        .filter(|param| !param.is_empty() && *param != "app")
        .map(str::to_string)
        .collect();

    Some((name.trim().to_string(), params))
}

fn rust_sources() -> Vec<String> {
    sources(&crate_root().join("src"), &["rs"])
        .into_iter()
        .filter(|path| !is_test_path(path))
        .map(|path| std::fs::read_to_string(&path).expect("rust source is readable"))
        .collect()
}

/// Test files, test modules and test infra describe themselves, not the contract.
pub(crate) fn is_test_path(path: &Path) -> bool {
    let in_test_dir = path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name == "test" || name.starts_with("tests") || name.starts_with("test_")
    });
    let is_test_file = path.file_name().and_then(|name| name.to_str()).is_some_and(
        |name| {
            name.ends_with("tests.rs") || name.ends_with(".test.ts") || name.ends_with(".spec.ts")
        },
    );
    in_test_dir || is_test_file
}

/// Every file with one of `extensions` under `dir`, path-sorted.
pub(crate) fn sources(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    files.retain(|path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.contains(&ext))
    });
    files.sort();
    files
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

pub(crate) fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The text between the first `open` marker and the next `close` after it.
fn between<'a>(source: &'a str, open: &str, close: char) -> &'a str {
    let (_, rest) = source.split_once(open).expect("the opening marker is present");
    let (body, _) = rest.split_once(close).expect("the closing marker is present");
    body
}
