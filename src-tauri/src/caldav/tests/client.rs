//! Collection-level contract tests: `calendar-query`, multiget batching and
//! `MKCALENDAR`, backed by `wiremock`.

use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::caldav::model::CalendarComponent;
use crate::test_support::caldav::start_server;

use super::MULTIGET;

const SQRT_CAL: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";

#[tokio::test]
async fn fetch_resources_runs_one_calendar_query() {
    let (server, client) = start_server().await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains("<c:calendar-query"))
        .respond_with(ResponseTemplate::new(207).set_body_string(MULTIGET))
        .mount(&server)
        .await;

    let resources = client.fetch_resources(SQRT_CAL).await.unwrap();

    assert_eq!(resources.len(), 1);
    assert!(resources[0]
        .calendar_data
        .as_deref()
        .is_some_and(|raw| raw.contains("UID:dsh-20261017-first-day-of-durga@dsh-import")));
}

#[tokio::test]
async fn fetch_hrefs_asks_for_only_the_given_resources() {
    let (server, client) = start_server().await;
    Mock::given(method("REPORT"))
        .and(path(SQRT_CAL))
        .and(body_string_contains(
            "<d:href>/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/dsh-routine-walk.ics</d:href>",
        ))
        .respond_with(ResponseTemplate::new(207).set_body_string(MULTIGET))
        .mount(&server)
        .await;

    let resources = client
        .fetch_hrefs(SQRT_CAL, &[format!("{SQRT_CAL}dsh-routine-walk.ics")])
        .await
        .unwrap();

    assert_eq!(resources.len(), 1);
}

#[tokio::test]
async fn mkcalendar_creates_a_typed_collection() {
    let (server, client) = start_server().await;
    Mock::given(method("MKCALENDAR"))
        .and(path("/me/imported/"))
        .and(body_string_contains(
            "<d:displayname>Google Tasks (imported)</d:displayname>",
        ))
        .and(body_string_contains(r#"<c:comp name="VTODO"/>"#))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    client
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
    let (server, client) = start_server().await;
    Mock::given(method("PROPPATCH"))
        .and(path("/me/imported/"))
        .and(body_string_contains(
            "<d:displayname>Errands</d:displayname>",
        ))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;

    client
        .set_display_name("/me/imported/", "Errands")
        .await
        .unwrap();
}

#[tokio::test]
async fn clearing_a_collection_colour_removes_the_property() {
    let (server, client) = start_server().await;
    Mock::given(method("PROPPATCH"))
        .and(path("/me/imported/"))
        .and(body_string_contains("<d:remove>"))
        .respond_with(ResponseTemplate::new(207))
        .mount(&server)
        .await;

    client
        .set_calendar_color("/me/imported/", None)
        .await
        .unwrap();
}

#[tokio::test]
async fn deleting_a_collection_sends_delete_and_accepts_404() {
    let (server, client) = start_server().await;
    Mock::given(method("DELETE"))
        .and(path("/me/imported/"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    client
        .delete_collection("/me/imported/")
        .await
        .unwrap();
}
