//! The folder-dependency rules from `RESTRUCTURE-PLAN.md`.

use crate::test_support::architecture;

#[test]
fn calendar_has_no_tauri_dependency() {
    assert_eq!(
        architecture::calendar_tauri_imports(),
        Vec::<String>::new(),
        "features/calendar must stay usable without Tauri"
    );
}

#[test]
fn tauri_commands_live_only_in_ipc() {
    assert_eq!(
        architecture::command_attributes_outside_ipc(),
        Vec::<String>::new(),
        "`#[tauri::command]` belongs in src/ipc only"
    );
}

#[test]
fn features_never_import_bootstrap_or_ipc() {
    assert_eq!(
        architecture::forbidden_imports_in("src/features", &["bootstrap", "ipc"]),
        Vec::<String>::new(),
        "features report through return values, not upward imports"
    );
}

#[test]
fn platform_never_imports_upward() {
    assert_eq!(
        architecture::forbidden_imports_in("src/platform", &["bootstrap", "ipc", "features"]),
        Vec::<String>::new(),
        "platform is the desktop shell and knows nothing above it"
    );
}

#[test]
fn cli_imports_only_the_calendar_feature() {
    assert_eq!(
        architecture::forbidden_imports_in(
            "src/cli",
            &["bootstrap", "ipc", "platform", "features::reminders", "features::stopwatch"],
        ),
        Vec::<String>::new(),
        "the headless client depends on features::calendar and settings only"
    );
}
