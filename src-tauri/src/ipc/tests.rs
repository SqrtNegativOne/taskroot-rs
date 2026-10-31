//! Command-layer contract tests.
//!
//! The handlers cannot be invoked from a test binary (see `test_support`), so
//! the `generate_handler!` table and the frontend call sites are checked against
//! each other by reading the sources. Store behaviour is covered by
//! `store::tests`; settings behaviour by `settings::tests`.

use crate::test_support::{frontend_scan, source_scan};

#[test]
fn the_command_table_lists_every_command_once() {
    let registered = source_scan::registered_commands();

    assert!(
        !registered.is_empty(),
        "generate_handler! lists no commands"
    );
    let mut unique = registered.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        registered.len(),
        "generate_handler! lists a command twice: {registered:?}"
    );
}

#[test]
fn every_command_name_is_snake_case() {
    for name in source_scan::registered_commands() {
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "`{name}` is not a snake_case command name"
        );
    }
}

#[test]
fn every_command_has_a_handler_signature_and_vice_versa() {
    let registered = source_scan::registered_commands();
    let params = source_scan::annotated_command_params();

    for name in &registered {
        assert!(
            params.contains_key(name),
            "`{name}` is registered but no `#[tauri::command]` fn declares it"
        );
    }
    for name in params.keys() {
        assert!(
            registered.contains(name),
            "`{name}` is a `#[tauri::command]` fn but is missing from generate_handler!"
        );
    }
}

#[test]
fn frontend_call_sites_match_the_rust_command_signatures() {
    let calls = frontend_scan::frontend_calls();
    assert!(!calls.is_empty(), "the frontend scan found no call sites");

    assert_keys_match_rust_signatures(&calls);
}

/// Panics on a call site that names an unknown command or passes an argument key
/// its command does not declare.
fn assert_keys_match_rust_signatures(calls: &[frontend_scan::FrontendCall]) {
    let params = source_scan::annotated_command_params();

    for call in calls {
        let Some(declared) = params.get(&call.command) else {
            panic!(
                "the frontend invokes `{}`, which is not a command",
                call.command
            );
        };
        for key in &call.keys {
            assert!(
                declared
                    .iter()
                    .any(|param| source_scan::js_key(param) == *key),
                "`{}` is passed `{key}`, but its Rust parameters are {declared:?}",
                call.command
            );
        }
    }
}

/// A `useTauriQuery(..)` result is usually executed later as
/// `query.execute({..})`; those keys belong to the same command and must be
/// checked, or a renamed argument key passes unnoticed.
#[test]
fn dot_execute_argument_keys_are_attributed_to_the_command() {
    let source = "\
let tasksQuery = useTauriQuery<AppTask[]>('query_tasks');
tasksQuery.execute({ filters: [], sort: [], query: \"\" });";

    let calls = frontend_scan::calls_in(source);

    assert_eq!(calls.len(), 1);
    let call = calls.first().expect("one call site");
    assert_eq!(call.command, "query_tasks");
    assert_eq!(call.keys, ["filters", "sort", "query"]);
    assert_keys_match_rust_signatures(&calls);
}

#[test]
#[should_panic(expected = "is passed `bogusKey`")]
fn a_dot_execute_key_the_command_does_not_declare_fails_the_contract() {
    let source = "\
let tasksQuery = useTauriQuery<AppTask[]>('query_tasks');
tasksQuery.execute({ bogusKey: 1 });";

    assert_keys_match_rust_signatures(&frontend_scan::calls_in(source));
}

#[test]
fn every_frontend_command_literal_belongs_to_a_scanned_call_site() {
    let calls = frontend_scan::frontend_calls();
    for name in source_scan::registered_commands() {
        let quoted = [format!("'{name}'"), format!("\"{name}\"")];
        for source in frontend_scan::frontend_sources() {
            if !quoted.iter().any(|literal| source.contains(literal)) {
                continue;
            }
            assert!(
                calls.iter().any(|call| call.command == name),
                "the frontend mentions `{name}` but the call-site scan missed it"
            );
        }
    }
}
