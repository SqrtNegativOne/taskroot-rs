//! HTTP contract tests for the `CalDAV` transport, backed by `wiremock`.

use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::caldav::transport::CalDavRequest;
use crate::test_support::caldav::start_server;

use super::{MULTIGET, UNAUTHORIZED};

#[tokio::test]
async fn a_multiget_response_keeps_its_etag_and_body() {
    let (server, client) = start_server().await;
    Mock::given(method("REPORT"))
        .and(path("/me/cal/"))
        .and(header("depth", "1"))
        .and(body_string_contains("calendar-multiget"))
        .respond_with(
            ResponseTemplate::new(207)
                .insert_header("ETag", r#""abc123""#)
                .set_body_string(MULTIGET),
        )
        .mount(&server)
        .await;

    let response = client
        .send(
            &CalDavRequest::report("/me/cal/")
                .with_depth("1")
                .with_body("<c:calendar-multiget/>"),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 207);
    assert_eq!(response.etag.as_deref(), Some(r#""abc123""#));
    assert!(response.body.contains("<multistatus"));
    assert!(response.location.is_none());
}

#[tokio::test]
async fn requests_use_basic_auth() {
    let (server, client) = start_server().await;
    Mock::given(method("PROPFIND"))
        .and(path("/me/"))
        .and(header("authorization", "Basic bWU6c2VjcmV0"))
        .respond_with(ResponseTemplate::new(207).set_body_string("<multistatus/>"))
        .mount(&server)
        .await;

    let response = client
        .send(
            &CalDavRequest::propfind("/me/")
                .with_depth("0")
                .with_body("<d:propfind/>"),
        )
        .await
        .unwrap();

    assert_eq!(response.status, 207);
}

#[tokio::test]
async fn unauthorized_maps_to_auth() {
    let (server, client) = start_server().await;
    Mock::given(method("PROPFIND"))
        .respond_with(ResponseTemplate::new(401).set_body_string(UNAUTHORIZED))
        .mount(&server)
        .await;

    let error = client
        .send(&CalDavRequest::propfind("/").with_depth("0"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "auth");
}

#[tokio::test]
async fn precondition_failed_maps_to_conflict() {
    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(412))
        .mount(&server)
        .await;

    let error = client
        .send(&CalDavRequest::put("/me/cal/a.ics", "BEGIN:VCALENDAR"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "conflict");
}

#[tokio::test]
async fn gone_maps_to_gone() {
    let (server, client) = start_server().await;
    Mock::given(method("REPORT"))
        .respond_with(ResponseTemplate::new(410))
        .mount(&server)
        .await;

    let error = client
        .send(&CalDavRequest::report("/me/cal/"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "gone");
}

#[tokio::test]
async fn a_forbidden_response_is_a_sync_error_with_status_and_snippet() {
    let (server, client) = start_server().await;
    Mock::given(method("REPORT"))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_string(r#"<d:error xmlns:d="DAV:"><d:valid-sync-token/></d:error>"#),
        )
        .mount(&server)
        .await;

    let error = client
        .send(&CalDavRequest::report("/me/cal/"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "remote");
    let message = error.to_string();
    assert!(message.contains("403"), "message was: {message}");
    assert!(
        message.contains("<d:valid-sync-token"),
        "the snippet carries the server body: {message}"
    );
}

#[tokio::test]
async fn a_redirect_is_available_to_discovery_through_send_raw() {
    let (server, client) = start_server().await;
    Mock::given(method("PROPFIND"))
        .and(path("/.well-known/caldav"))
        .respond_with(ResponseTemplate::new(301).insert_header("Location", "/"))
        .mount(&server)
        .await;

    let response = client
        .send_raw(&CalDavRequest::propfind("/.well-known/caldav").with_depth("0"))
        .await
        .unwrap();

    assert!(response.is_redirect());
    assert_eq!(response.location.as_deref(), Some("/"));
}

#[tokio::test]
async fn creating_a_resource_sends_if_none_match_and_returns_the_etag() {
    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path("/me/cal/a.ics"))
        .and(header("if-none-match", "*"))
        .and(header("content-type", "text/calendar; charset=utf-8"))
        .and(body_string_contains("BEGIN:VCALENDAR"))
        .respond_with(ResponseTemplate::new(201).insert_header("ETag", r#""new-etag""#))
        .mount(&server)
        .await;

    let etag = client
        .put_resource(
            "/me/cal/a.ics",
            "BEGIN:VCALENDAR\nEND:VCALENDAR",
            None,
            true,
        )
        .await
        .unwrap();

    assert_eq!(etag.as_deref(), Some(r#""new-etag""#));
}

#[tokio::test]
async fn updating_a_resource_requires_and_sends_its_etag() {
    let (server, client) = start_server().await;
    Mock::given(method("PUT"))
        .and(path("/me/cal/a.ics"))
        .and(header("if-match", r#""old-etag""#))
        .respond_with(ResponseTemplate::new(204).insert_header("ETag", r#""next-etag""#))
        .mount(&server)
        .await;

    let missing_etag = client
        .put_resource("/me/cal/a.ics", "BEGIN:VCALENDAR", None, false)
        .await
        .unwrap_err();
    assert_eq!(missing_etag.code(), "invalid-input");

    let etag = client
        .put_resource(
            "/me/cal/a.ics",
            "BEGIN:VCALENDAR",
            Some(r#""old-etag""#),
            false,
        )
        .await
        .unwrap();
    assert_eq!(etag.as_deref(), Some(r#""next-etag""#));
}

#[tokio::test]
async fn deleting_a_missing_resource_counts_as_success() {
    let (server, client) = start_server().await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    client
        .delete_resource("/me/cal/gone.ics", None)
        .await
        .unwrap();
}

#[tokio::test]
async fn deleting_with_a_stale_etag_maps_to_conflict() {
    let (server, client) = start_server().await;
    Mock::given(method("DELETE"))
        .and(header("if-match", r#""stale""#))
        .respond_with(ResponseTemplate::new(412))
        .mount(&server)
        .await;

    let error = client
        .delete_resource("/me/cal/a.ics", Some(r#""stale""#))
        .await
        .unwrap_err();

    assert_eq!(error.code(), "conflict");
}
