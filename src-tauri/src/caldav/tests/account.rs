//! `account.json` round-trip, malformed-file and atomic-write tests.
//!
//! These tests never touch the OS keychain: only the JSON half of the account
//! storage is exercised here (the keychain calls are thin wrappers with no
//! testable branch beyond `NoEntry`).

use crate::caldav::account::{
    account_path, delete_account_file, read_account, write_account, Account,
};
use crate::caldav::model::{CalendarComponent, CalendarInfo};

use super::scratch_dir;

fn sample_account() -> Account {
    Account {
        server_url: "http://127.0.0.1:5232".to_string(),
        username: "me".to_string(),
        principal_href: "/me/".to_string(),
        home_href: "/me/".to_string(),
        calendars: vec![CalendarInfo {
            href: "/me/cal/".to_string(),
            display_name: "SqrtCal".to_string(),
            color: Some("#9966a7ff".to_string()),
            component: CalendarComponent::VEvent,
            writable: true,
        }],
    }
}

#[test]
fn account_round_trips_and_never_contains_a_password() {
    let dir = scratch_dir("account-round-trip");
    let account = sample_account();

    write_account(&dir, &account).unwrap();

    assert_eq!(read_account(&dir).unwrap(), Some(account));
    assert!(
        !dir.join("account.json.tmp").exists(),
        "the temporary file is renamed away"
    );
    let raw = std::fs::read_to_string(account_path(&dir)).unwrap();
    assert!(!raw.to_lowercase().contains("password"));
    assert!(raw.contains(r#""serverUrl": "http://127.0.0.1:5232""#));
}

#[test]
fn a_missing_account_file_is_not_an_error() {
    let dir = scratch_dir("account-missing");

    assert_eq!(read_account(&dir).unwrap(), None);
}

#[test]
fn a_malformed_account_file_is_a_store_error() {
    let dir = scratch_dir("account-malformed");
    std::fs::write(account_path(&dir), r#"{"serverUrl": 42}"#).unwrap();

    let error = read_account(&dir).unwrap_err();

    assert_eq!(error.code(), "store");
    assert!(error.to_string().contains("Malformed account file"));
}

#[test]
fn unparsable_json_is_a_store_error() {
    let dir = scratch_dir("account-unparsable");
    std::fs::write(account_path(&dir), "not json").unwrap();

    let error = read_account(&dir).unwrap_err();

    assert_eq!(error.code(), "store");
}

#[test]
fn an_empty_object_uses_the_serde_defaults() {
    let dir = scratch_dir("account-defaults");
    std::fs::write(account_path(&dir), "{}").unwrap();

    let account = read_account(&dir).unwrap().expect("an empty object parses");

    assert_eq!(account, Account::default());
}

#[test]
fn deleting_the_account_file_is_idempotent() {
    let dir = scratch_dir("account-delete");
    write_account(&dir, &sample_account()).unwrap();

    delete_account_file(&dir).unwrap();
    assert!(!account_path(&dir).exists());
    delete_account_file(&dir).unwrap();
}
