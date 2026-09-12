//! The frontend half of the IPC source contract used by the command-layer tests.
//!
//! Every `safeInvoke`/`useAutoQuery`/`useTauriQuery`/`invoke` call site under
//! `../src`, with the literal command name it passes and the argument keys it
//! supplies.

use super::source_scan::{crate_root, is_test_path, sources};

/// The wrappers whose first argument is a literal Tauri command name.
const CALL_SITES: [&str; 4] = ["safeInvoke", "useAutoQuery", "useTauriQuery", "invoke"];

/// One frontend `invoke` call site: the command name and the argument keys.
pub(crate) struct FrontendCall {
    pub(crate) command: String,
    pub(crate) keys: Vec<String>,
}

/// Every literal-command call site in the production frontend sources.
pub(crate) fn frontend_calls() -> Vec<FrontendCall> {
    let mut calls = Vec::new();
    for source in frontend_sources() {
        for callee in CALL_SITES {
            calls.extend(call_sites(&source, callee));
        }
    }
    calls
}

/// The production frontend sources: `../src` without test files and test infra.
pub(crate) fn frontend_sources() -> Vec<String> {
    sources(&crate_root().join("../src"), &["ts", "svelte"])
        .into_iter()
        .filter(|path| !is_test_path(path))
        .map(|path| std::fs::read_to_string(&path).expect("frontend source is readable"))
        .collect()
}

fn call_sites(source: &str, callee: &str) -> Vec<FrontendCall> {
    source
        .match_indices(callee)
        .filter_map(|(offset, _)| {
            let (before, after) = source.split_at(offset);
            let is_identifier = before
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
            if is_identifier {
                return None;
            }

            let (_, tail) = after.split_at(callee.len());
            let (generics, args) = tail.split_once('(')?;
            if !generics.trim().is_empty() && !generics.trim().starts_with('<') {
                return None;
            }

            let (command, rest) = first_literal(args)?;
            if !matches!(rest.trim_start().chars().next(), Some(',' | ')')) {
                return None;
            }

            Some(FrontendCall {
                command,
                keys: arg_keys(rest),
            })
        })
        .collect()
}

/// The first string literal in `text`, and the text after it.
///
/// `None` when a statement boundary comes first (which is how function
/// declarations are skipped) or when the literal is part of an expression rather
/// than the call's first argument — a command name is followed by `,` or `)`.
fn first_literal(text: &str) -> Option<(String, &str)> {
    let (index, _) = text.char_indices().find(|(_, c)| matches!(c, '\'' | '"'))?;
    let (before, quoted) = text.split_at(index);
    if before.contains([';', '{', '}']) {
        return None;
    }

    let (quote, tail) = quoted.split_at(1);
    let (literal, rest) = tail.split_once(quote)?;
    Some((literal.to_string(), rest))
}

/// The argument keys of the object literal that follows a command name:
/// `, { key, value })` and `, () => ({ key, value }))`.
fn arg_keys(mut text: &str) -> Vec<String> {
    let Some(after_comma) = text.trim_start().strip_prefix(',') else {
        return Vec::new();
    };
    text = after_comma.trim_start();
    for prefix in ["()", "=>", "("] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.trim_start();
        }
    }

    text.strip_prefix('{').map_or_else(Vec::new, object_members)
}

/// The top-level member names of an object literal body (its `{` already stripped).
fn object_members(body: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut member = String::new();
    let mut depth = 0_u8;

    for ch in body.chars() {
        match ch {
            '{' | '[' | '(' => {
                depth = depth.saturating_add(1);
                member.push(ch);
            }
            '}' | ']' | ')' if depth == 0 => break,
            '}' | ']' | ')' => {
                depth = depth.saturating_sub(1);
                member.push(ch);
            }
            ',' if depth == 0 => {
                record_member(&mut keys, &member);
                member.clear();
            }
            _ => member.push(ch),
        }
    }
    record_member(&mut keys, &member);
    keys
}

fn record_member(keys: &mut Vec<String>, member: &str) {
    let key = member.split(':').next().unwrap_or_default().trim();
    if !key.is_empty() {
        keys.push(key.to_string());
    }
}
