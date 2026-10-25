//! Collection-level contract tests: `sync-collection`, multiget batching and
//! `MKCALENDAR`, backed by `wiremock`.

use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::caldav::model::CalendarComponent;
use crate::caldav::xml::SYNC_COLLECTION_INITIAL_BODY;

use super::{test_client, MULTIGET, SYNC_DELTA, SYNC_INITIAL};

const SQRT_CAL: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";

#[tokio::test]
async fn fetch_resources_runs_an_initial_sync_then_one_multiget() {
    let server = MockServer::start().await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains("<d:sync-token/>"))
        .respond_with(ResponseTemplate::new(207).set_body_string(SYNC_INITIAL))
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains("calendar-multiget"))
        .respond_with(ResponseTemplate::new(207).set_body_string(MULTIGET))
        .mount(&server)
        .await;

    let resources = test_client(&server)
        .fetch_resources(SQRT_CAL)
        .await
        .unwrap();

    assert_eq!(resources.len(), 1);
    assert!(resources[0]
        .calendar_data
        .as_deref()
        .is_some_and(|raw| raw.contains("UID:dsh-20261017-first-day-of-durga@dsh-import")));
}

#[tokio::test]
async fn fetch_hrefs_asks_for_only_the_given_resources() {
    let server = MockServer::start().await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains(
            "<d:href>/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/dsh-routine-walk.ics</d:href>",
        ))
        .respond_with(ResponseTemplate::new(207).set_body_string(MULTIGET))
        .mount(&server)
        .await;

    let resources = test_client(&server)
        .fetch_hrefs(SQRT_CAL, &[format!("{SQRT_CAL}dsh-routine-walk.ics")])
        .await
        .unwrap();

    assert_eq!(resources.len(), 1);
}

#[tokio::test]
async fn sync_collection_sends_the_stored_token() {
    let server = MockServer::start().await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains(
            "<d:sync-token>http://radicale.org/ns/sync/572734e7</d:sync-token>",
        ))
        .respond_with(ResponseTemplate::new(207).set_body_string(SYNC_DELTA))
        .mount(&server)
        .await;

    let sync = test_client(&server)
        .sync_collection(SQRT_CAL, Some("http://radicale.org/ns/sync/572734e7"))
        .await
        .unwrap();

    assert_eq!(
        sync.token.as_deref(),
        Some("http://radicale.org/ns/sync/572734e7eac248e21146e7a7ff811067b02e20bb6f870c6826d4a1e33e559435")
    );
    assert!(sync.changes.is_empty());
}

#[tokio::test]
async fn an_initial_sync_body_is_used_without_a_token() {
    assert!(SYNC_COLLECTION_INITIAL_BODY.contains("<d:sync-token/>"));
}

#[tokio::test]
async fn mkcalendar_creates_a_typed_collection() {
    let server = MockServer::start().await;
    Mock::given(method("MKCALENDAR"))
        .and(path("/me/imported/"))
        .and(body_string_contains(
            "<d:displayname>Google Tasks (imported)</d:displayname>",
        ))
        .and(body_string_contains(r#"<c:comp name="VTODO"/>"#))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    test_client(&server)
        .mkcalendar(
            "/me/imported/",
            "Google Tasks (imported)",
            CalendarComponent::VTodo,
        )
        .await
        .unwrap();
}
