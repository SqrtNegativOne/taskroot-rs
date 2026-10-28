//! Collection-level contract tests: `calendar-query`, multiget batching and
//! `MKCALENDAR`, backed by `wiremock`.

use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::caldav::model::CalendarComponent;

use super::{test_client, MULTIGET};

const SQRT_CAL: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";

#[tokio::test]
async fn fetch_resources_runs_one_calendar_query() {
    let server = MockServer::start().await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains("<c:calendar-query"))
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

#[tokio::test]
async fn renaming_a_collection_proppatches_its_display_name() {
    let server = MockServer::start().await;
    Mock::given(method("PROPPATCH"))
        .and(path("/me/imported/"))
        .and(body_string_contains(
            "<d:displayname>Errands</d:displayname>",
        ))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;

    test_client(&server)
        .set_display_name("/me/imported/", "Errands")
        .await
        .unwrap();
}

#[tokio::test]
async fn clearing_a_collection_colour_removes_the_property() {
    let server = MockServer::start().await;
    Mock::given(method("PROPPATCH"))
        .and(path("/me/imported/"))
        .and(body_string_contains("<d:remove>"))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;

    test_client(&server)
        .set_calendar_color("/me/imported/", None)
        .await
        .unwrap();
}

#[tokio::test]
async fn deleting_a_collection_sends_delete_and_accepts_404() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/me/imported/"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    test_client(&server)
        .delete_collection("/me/imported/")
        .await
        .unwrap();
}
