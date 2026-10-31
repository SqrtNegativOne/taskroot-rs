//! Server-URL normalisation, discovery against `wiremock`, and the one
//! `#[ignore]`d check against the real Radicale.

use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::caldav::account::{read_account, write_account, Account};
use crate::caldav::discovery::{
    discover, normalize_server_url, refresh_collections, DEFAULT_SERVER_URL,
};
use crate::caldav::model::{CalendarComponent, CalendarInfo};
use crate::caldav::transport::HttpClient;
use crate::test_support::caldav::{start_http_server, start_server};

use super::{HOME_PROPFIND, PRINCIPAL_PROPFIND, ROOT_PROPFIND};

#[test]
fn an_empty_server_url_is_the_loopback_default() {
    assert_eq!(normalize_server_url("").unwrap(), DEFAULT_SERVER_URL);
    assert_eq!(
        normalize_server_url("   ").unwrap(),
        "http://127.0.0.1:5232"
    );
}

#[test]
fn a_missing_scheme_defaults_by_host() {
    assert_eq!(
        normalize_server_url("127.0.0.1:5232").unwrap(),
        "http://127.0.0.1:5232"
    );
    assert_eq!(
        normalize_server_url("localhost:5232/").unwrap(),
        "http://localhost:5232"
    );
    assert_eq!(
        normalize_server_url("sqrt-1.tail03279c.ts.net").unwrap(),
        "https://sqrt-1.tail03279c.ts.net"
    );
}

#[test]
fn trailing_slashes_and_queries_are_stripped() {
    assert_eq!(
        normalize_server_url("http://127.0.0.1:5232///").unwrap(),
        "http://127.0.0.1:5232"
    );
    assert_eq!(
        normalize_server_url("https://dav.example/dav/?x=1#frag").unwrap(),
        "https://dav.example/dav"
    );
}

#[test]
fn plain_http_is_only_allowed_for_loopback_and_tailnet_hosts() {
    for allowed in [
        "http://127.0.0.1:5232",
        "http://localhost:5232",
        "http://[::1]:5232",
        "http://sqrt-1.tail03279c.ts.net",
        "http://ts.net",
    ] {
        assert!(
            normalize_server_url(allowed).is_ok(),
            "`{allowed}` should be allowed"
        );
    }

    for rejected in [
        "http://example.com",
        "http://evilts.net",
        "http://192.168.1.5:5232",
    ] {
        let error = normalize_server_url(rejected).unwrap_err();
        assert_eq!(error.code(), "invalid-input", "`{rejected}` was accepted");
    }
}

#[test]
fn unsupported_schemes_and_embedded_credentials_are_rejected() {
    assert_eq!(
        normalize_server_url("ftp://127.0.0.1:5232")
            .unwrap_err()
            .code(),
        "invalid-input"
    );
    assert_eq!(
        normalize_server_url("http://me:secret@127.0.0.1:5232")
            .unwrap_err()
            .code(),
        "invalid-input"
    );
    assert_eq!(
        normalize_server_url("http://").unwrap_err().code(),
        "invalid-input"
    );
}

#[tokio::test]
async fn discovery_follows_the_well_known_redirect_and_persists_nothing() {
    let (server, http) = start_http_server().await;
    Mock::given(method("PROPFIND"))
        .and(path("/.well-known/caldav"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/"))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/"))
        .and(header("depth", "0"))
        .and(body_string_contains("current-user-principal"))
        .and(header("authorization", "Basic bWU6c2VjcmV0"))
        .respond_with(ResponseTemplate::new(207).set_body_string(ROOT_PROPFIND))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/me/"))
        .and(header("depth", "0"))
        .respond_with(ResponseTemplate::new(207).set_body_string(PRINCIPAL_PROPFIND))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/me/"))
        .and(header("depth", "1"))
        .respond_with(ResponseTemplate::new(207).set_body_string(HOME_PROPFIND))
        .mount(&server)
        .await;

    let discovery = discover(&http, &server.uri(), "me", "secret")
        .await
        .unwrap();

    assert_eq!(discovery.server_url, server.uri());
    assert_eq!(discovery.principal_href, "/me/");
    assert_eq!(discovery.home_href, "/me/");
    assert_eq!(discovery.calendars.len(), 2);
    let sqrt_tasks = discovery
        .calendars
        .iter()
        .find(|calendar| calendar.display_name == "SqrtTasks")
        .expect("the task calendar is discovered");
    assert_eq!(sqrt_tasks.component, CalendarComponent::VTodo);
    assert!(sqrt_tasks.writable);
}

#[tokio::test]
async fn discovery_falls_back_to_the_root_when_well_known_fails() {
    let (server, http) = start_http_server().await;
    Mock::given(method("PROPFIND"))
        .and(path("/.well-known/caldav"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(ROOT_PROPFIND))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/me/"))
        .and(header("depth", "0"))
        .respond_with(ResponseTemplate::new(207).set_body_string(PRINCIPAL_PROPFIND))
        .mount(&server)
        .await;
    Mock::given(method("PROPFIND"))
        .and(path("/me/"))
        .and(header("depth", "1"))
        .respond_with(ResponseTemplate::new(207).set_body_string(HOME_PROPFIND))
        .mount(&server)
        .await;

    let discovery = discover(&http, &server.uri(), "me", "secret")
        .await
        .unwrap();

    assert_eq!(discovery.principal_href, "/me/");
    assert_eq!(discovery.calendars.len(), 2);
}

#[tokio::test]
async fn a_redirect_loop_is_reported() {
    let (server, http) = start_http_server().await;
    Mock::given(method("PROPFIND"))
        .and(path("/.well-known/caldav"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/.well-known/caldav"))
        .mount(&server)
        .await;

    let error = discover(&http, &server.uri(), "me", "secret")
        .await
        .unwrap_err();

    assert_eq!(error.code(), "remote");
    assert!(error.to_string().contains("Too many redirects"));
}

#[tokio::test]
async fn bad_credentials_surface_as_auth() {
    let (server, http) = start_http_server().await;
    Mock::given(method("PROPFIND"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let error = discover(&http, &server.uri(), "me", "wrong")
        .await
        .unwrap_err();

    assert_eq!(error.code(), "auth");
}

/// Requires the local Radicale at `http://127.0.0.1:5232` with the `me` user.
/// Run with `cargo nextest run --run-ignored ignored-only` when applicable.
#[tokio::test]
#[ignore = "requires the local Radicale on 127.0.0.1:5232 with user `me`"]
async fn discovery_works_against_the_real_radicale() {
    let password = std::env::var("TASKROOT_CALDAV_PASSWORD")
        .unwrap_or_else(|_| "desperate-seventy-hacks-done".to_string());
    let http = HttpClient::new().unwrap();

    let discovery = discover(&http, DEFAULT_SERVER_URL, "me", &password)
        .await
        .unwrap();

    assert_eq!(discovery.principal_href, "/me/");
    assert_eq!(discovery.home_href, "/me/");
    let sqrt_cal = discovery
        .calendars
        .iter()
        .find(|calendar| calendar.display_name == "SqrtCal")
        .expect("SqrtCal is discovered");
    assert_eq!(sqrt_cal.component, CalendarComponent::VEvent);
    assert_eq!(sqrt_cal.href, "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/");
    let sqrt_tasks = discovery
        .calendars
        .iter()
        .find(|calendar| calendar.display_name == "SqrtTasks")
        .expect("SqrtTasks is discovered");
    assert_eq!(sqrt_tasks.component, CalendarComponent::VTodo);
    assert!(sqrt_cal.writable && sqrt_tasks.writable);
}

#[tokio::test]
async fn a_refresh_persists_the_list_the_server_actually_has() {
    let (server, client) = start_server().await;
    Mock::given(method("PROPFIND"))
        .and(path("/me/"))
        .and(header("depth", "1"))
        .respond_with(ResponseTemplate::new(207).set_body_string(HOME_PROPFIND))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    let mut account = Account {
        server_url: server.uri(),
        username: "me".to_string(),
        principal_href: "/me/".to_string(),
        home_href: "/me/".to_string(),
        calendars: Vec::new(),
    };
    write_account(dir, &account).expect("seed the account file");

    let calendars = refresh_collections(dir, &client, &mut account).await;

    assert_eq!(calendars.len(), 2);
    assert_eq!(
        read_account(dir).unwrap().unwrap().calendars,
        calendars,
        "the fresh list must be written back, not just returned"
    );
}

#[tokio::test]
async fn a_refresh_that_cannot_reach_the_server_keeps_the_cached_list() {
    let (server, client) = start_server().await;
    Mock::given(method("PROPFIND"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let cached = vec![CalendarInfo {
        href: "/me/old/".to_string(),
        display_name: "Old".to_string(),
        component: CalendarComponent::VTodo,
        ..CalendarInfo::default()
    }];
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    let mut account = Account {
        server_url: server.uri(),
        username: "me".to_string(),
        principal_href: "/me/".to_string(),
        home_href: "/me/".to_string(),
        calendars: cached.clone(),
    };
    write_account(dir, &account).expect("seed the account file");

    let calendars = refresh_collections(dir, &client, &mut account).await;

    assert_eq!(calendars, cached, "an offline start must still have lists");
    assert_eq!(read_account(dir).unwrap().unwrap().calendars, cached);
}
