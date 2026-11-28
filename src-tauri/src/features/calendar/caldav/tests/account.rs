//! `account.json` round-trip, malformed-file and atomic-write tests.
//!
//! These tests never touch the OS keychain: only the JSON half of the account
//! storage is exercised here (the keychain calls are thin wrappers with no
//! testable branch beyond `NoEntry`).

use crate::features::calendar::caldav::account::{
    account_path, delete_account_file, Account, AccountProfile, AccountRegistry, AccountRepository,
    JsonAccountRepository,
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

fn single_registry(account: Account) -> AccountRegistry {
    AccountRegistry {
        active_id: Some("primary".to_string()),
        profiles: vec![AccountProfile {
            id: "primary".to_string(),
            account,
        }],
    }
}

#[test]
fn account_round_trips_and_never_contains_a_password() {
    let dir = tempfile::tempdir().unwrap();
    let repo = JsonAccountRepository::new(dir.path().to_path_buf());
    let account = sample_account();

    repo.write(&single_registry(account.clone())).unwrap();

    let loaded = repo.read().unwrap();
    assert_eq!(loaded.active().map(|p| &p.account), Some(&account));
    assert!(
        !dir.path().join("account.json.tmp").exists(),
        "the temporary file is renamed away"
    );
    let raw = std::fs::read_to_string(account_path(dir.path())).unwrap();
    assert!(!raw.to_lowercase().contains("password"));
    assert!(raw.contains(r#""serverUrl": "http://127.0.0.1:5232""#));
}

#[test]
fn a_missing_account_file_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let repo = JsonAccountRepository::new(dir.path().to_path_buf());

    assert_eq!(repo.read().unwrap(), AccountRegistry::default());
}

#[test]
fn a_malformed_account_file_is_a_store_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(account_path(dir.path()), r#"{"serverUrl": 42}"#).unwrap();
    let repo = JsonAccountRepository::new(dir.path().to_path_buf());

    let error = repo.read().unwrap_err();

    assert_eq!(error.code(), "store");
    assert!(error.to_string().contains("Malformed account file"));
}

#[test]
fn unparsable_json_is_a_store_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(account_path(dir.path()), "not json").unwrap();
    let repo = JsonAccountRepository::new(dir.path().to_path_buf());

    let error = repo.read().unwrap_err();

    assert_eq!(error.code(), "store");
}

#[test]
fn an_incomplete_registry_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(account_path(dir.path()), "{}").unwrap();
    let repo = JsonAccountRepository::new(dir.path().to_path_buf());

    let error = repo.read().unwrap_err();

    assert_eq!(error.code(), "store");
}

#[test]
fn deleting_the_account_file_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let repo = JsonAccountRepository::new(dir.path().to_path_buf());
    repo.write(&single_registry(sample_account())).unwrap();

    delete_account_file(dir.path()).unwrap();
    assert!(!account_path(dir.path()).exists());
    delete_account_file(dir.path()).unwrap();
}
