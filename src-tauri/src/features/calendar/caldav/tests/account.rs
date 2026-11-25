//! `account.json` round-trip, malformed-file and atomic-write tests.
//!
//! These tests never touch the OS keychain: only the JSON half of the account
//! storage is exercised here (the keychain calls are thin wrappers with no
//! testable branch beyond `NoEntry`).

use crate::features::calendar::caldav::account::{
    account_path, delete_account_file, read_account, read_client_account, write_account, Account,
    AccountProfile, AccountRegistry, AccountRepository, JsonAccountRepository,
};
use crate::features::calendar::caldav::model::{CalendarComponent, CalendarInfo};

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
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    let account = sample_account();

    write_account(dir, &account).unwrap();

    assert_eq!(read_account(dir).unwrap(), Some(account));
    assert!(
        !dir.join("account.json.tmp").exists(),
        "the temporary file is renamed away"
    );
    let raw = std::fs::read_to_string(account_path(dir)).unwrap();
    assert!(!raw.to_lowercase().contains("password"));
    assert!(raw.contains(r#""serverUrl": "http://127.0.0.1:5232""#));
}

#[test]
fn a_missing_account_file_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();

    assert_eq!(read_account(dir).unwrap(), None);
}

#[test]
fn a_malformed_account_file_is_a_store_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    std::fs::write(account_path(dir), r#"{"serverUrl": 42}"#).unwrap();

    let error = read_account(dir).unwrap_err();

    assert_eq!(error.code(), "store");
    assert!(error.to_string().contains("Malformed account file"));
}

#[test]
fn unparsable_json_is_a_store_error() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    std::fs::write(account_path(dir), "not json").unwrap();

    let error = read_account(dir).unwrap_err();

    assert_eq!(error.code(), "store");
}

#[test]
fn an_incomplete_registry_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    std::fs::write(account_path(dir), "{}").unwrap();

    let account = read_account(dir).unwrap_err();

    assert_eq!(account.code(), "store");
}

#[test]
fn client_metadata_updates_do_not_target_the_newly_active_profile() {
    let dir = tempfile::tempdir().unwrap();
    let mut first = sample_account();
    let second = Account {
        server_url: "http://127.0.0.1:15233".to_string(),
        ..sample_account()
    };
    let repository = JsonAccountRepository::new(dir.path().to_path_buf());
    repository
        .write(&AccountRegistry {
            active_id: Some("second".to_string()),
            profiles: vec![
                AccountProfile {
                    id: "first".to_string(),
                    account: first.clone(),
                },
                AccountProfile {
                    id: "second".to_string(),
                    account: second.clone(),
                },
            ],
        })
        .unwrap();
    let http = crate::features::calendar::HttpClient::new().unwrap();
    let client =
        crate::features::calendar::CalDavClient::from_account(&http, &first, "fake").unwrap();
    first.calendars.first_mut().unwrap().display_name = "Changed on first".to_string();
    write_account(dir.path(), &first).unwrap();
    assert_eq!(read_account(dir.path()).unwrap(), Some(second));
    assert_eq!(
        read_client_account(dir.path(), &client).unwrap(),
        Some(first)
    );
    assert_eq!(repository.read().unwrap().profiles.len(), 2);
}

#[test]
fn deleting_the_account_file_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    write_account(dir, &sample_account()).unwrap();

    delete_account_file(dir).unwrap();
    assert!(!account_path(dir).exists());
    delete_account_file(dir).unwrap();
}

