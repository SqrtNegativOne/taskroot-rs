//! Explicit opt-in contract against the disposable fixture, never port 5232.

use super::super::account::{AccountService, JsonAccountRepository};
use super::account_support::MemoryCredentials;
use crate::features::calendar::{HttpClient, Store, StoreState};
use std::sync::Arc;

#[tokio::test]
#[ignore = "requires scripts/dummy-caldav.py --check-backend on loopback port 15232"]
async fn disposable_dummy_accounts_switch_real_resources_without_keychain_access() {
    let dir = tempfile::tempdir().unwrap();
    let service = AccountService::new(
        Arc::new(JsonAccountRepository::new(dir.path().to_path_buf())),
        Arc::new(MemoryCredentials::default()),
        HttpClient::new().unwrap(),
    );
    let store = StoreState::new(Store::load(dir.path().to_path_buf()));
    store
        .write(|store| {
            store.settings.default_task_duration = 42;
            store
                .ui_state
                .insert("test.window".to_string(), serde_json::json!({"width": 800}));
        })
        .await;
    let server = "http://127.0.0.1:15232";
    service
        .connect(&store, server, "taskroot-dummy-a", "dummy-a-only")
        .await
        .unwrap();
    let first = service.registry().await.unwrap().active_id.unwrap();
    assert_eq!(
        store
            .read(|store| store.task("dummy-tasks").unwrap().title.clone())
            .await,
        "taskroot-dummy-a sample tasks"
    );
    service
        .connect(&store, server, "taskroot-dummy-b", "dummy-b-only")
        .await
        .unwrap();
    assert_eq!(
        store
            .read(|store| store.task("dummy-tasks").unwrap().title.clone())
            .await,
        "taskroot-dummy-b sample tasks"
    );
    service.activate(&store, &first).await.unwrap();
    assert_eq!(
        store
            .read(|store| store.task("dummy-tasks").unwrap().title.clone())
            .await,
        "taskroot-dummy-a sample tasks"
    );
    assert!(
        store
            .read(|store| store
                .resources
                .values()
                .all(|resource| resource.href.starts_with("/taskroot-dummy-a/")))
            .await
    );
    assert_eq!(
        store
            .read(|store| store.settings.default_task_duration)
            .await,
        42
    );
    assert_eq!(
        store
            .read(|store| store.ui_state.get("test.window").cloned())
            .await,
        Some(serde_json::json!({"width": 800}))
    );
}

#[tokio::test]
#[ignore = "requires scripts/dummy-caldav.py --check-backend on loopback port 15232"]
async fn radicale_discovery_and_collection_lifecycle() {
    use crate::features::calendar::caldav::discovery::{discover, fetch_calendars};
    use crate::features::calendar::caldav::model::CalendarComponent;
    use crate::features::calendar::CalDavClient;

    let server = "http://127.0.0.1:15232";
    let http = HttpClient::new().unwrap();

    let bad = discover(&http, server, "taskroot-dummy-a", "wrong-password")
        .await
        .expect_err("wrong password must fail");
    assert_eq!(bad.code(), "auth");

    let discovered = discover(&http, server, "taskroot-dummy-a", "dummy-a-only")
        .await
        .unwrap();
    assert_eq!(discovered.principal_href, "/taskroot-dummy-a/");
    assert_eq!(discovered.home_href, "/taskroot-dummy-a/");
    assert!(discovered.calendars.len() >= 2);

    let client = CalDavClient::new(&http, server, "taskroot-dummy-a", "dummy-a-only").unwrap();
    let col_href = format!("/taskroot-dummy-a/contract-col-{}/", uuid::Uuid::new_v4());

    client
        .mkcalendar(
            &col_href,
            "Tasks & <Notes> — 日本語",
            CalendarComponent::VTodo,
        )
        .await
        .unwrap();

    let created = fetch_calendars(&client, &discovered.home_href)
        .await
        .unwrap()
        .into_iter()
        .find(|cal| cal.href == col_href)
        .expect("newly created collection is discovered");
    assert_eq!(created.display_name, "Tasks & <Notes> — 日本語");
    assert_eq!(created.component, CalendarComponent::VTodo);
    assert!(created.writable);

    client
        .set_display_name(&col_href, "Renamed & <Updated>")
        .await
        .unwrap();
    client
        .set_calendar_color(&col_href, Some("#ff8800ff"))
        .await
        .unwrap();
    let updated = fetch_calendars(&client, &discovered.home_href)
        .await
        .unwrap()
        .into_iter()
        .find(|cal| cal.href == col_href)
        .unwrap();
    assert_eq!(updated.display_name, "Renamed & <Updated>");
    assert_eq!(updated.color.as_deref(), Some("#ff8800ff"));

    client.set_calendar_color(&col_href, None).await.unwrap();
    let cleared = fetch_calendars(&client, &discovered.home_href)
        .await
        .unwrap()
        .into_iter()
        .find(|cal| cal.href == col_href)
        .unwrap();
    assert_eq!(cleared.color, None);

    client.delete_collection(&col_href).await.unwrap();
    client.delete_collection(&col_href).await.unwrap();
    assert!(
        !fetch_calendars(&client, &discovered.home_href)
            .await
            .unwrap()
            .iter()
            .any(|cal| cal.href == col_href)
    );
}

#[tokio::test]
#[ignore = "requires scripts/dummy-caldav.py --check-backend on loopback port 15232"]
async fn radicale_resource_crud_conflicts_and_encoding() {
    use crate::features::calendar::caldav::model::CalendarComponent;
    use crate::features::calendar::ics::read_task_resource;
    use crate::features::calendar::CalDavClient;

    let server = "http://127.0.0.1:15232";
    let http = HttpClient::new().unwrap();
    let client = CalDavClient::new(&http, server, "taskroot-dummy-a", "dummy-a-only").unwrap();
    let col_href = format!("/taskroot-dummy-a/contract-res-{}/", uuid::Uuid::new_v4());
    client
        .mkcalendar(&col_href, "Resource Contract", CalendarComponent::VTodo)
        .await
        .unwrap();

    let item_href = format!("{col_href}utf8-item.ics");
    let v1 = concat!(
        "BEGIN:VCALENDAR\r\n",
        "VERSION:2.0\r\n",
        "PRODID:-//Taskroot//Contract Test//EN\r\n",
        "BEGIN:VTODO\r\n",
        "UID:utf8-item\r\n",
        "DTSTAMP:20261127T120000Z\r\n",
        "SUMMARY:Comidas: ñoquis\\, 西红柿 & <test>\r\n",
        "STATUS:NEEDS-ACTION\r\n",
        "END:VTODO\r\n",
        "END:VCALENDAR\r\n",
    );
    let etag1 = client
        .put_resource(&item_href, v1, None, true)
        .await
        .unwrap()
        .expect("Radicale returns an ETag on PUT create");

    let dup = client
        .put_resource(&item_href, v1, None, true)
        .await
        .expect_err("If-None-Match: * must reject duplicate create");
    assert_eq!(dup.code(), "conflict");

    let all = client.fetch_resources(&col_href).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].href, item_href);
    assert_eq!(all[0].etag.as_deref(), Some(etag1.as_str()));
    let fetched_raw = all[0].calendar_data.as_deref().unwrap();
    assert!(
        fetched_raw.contains("\r\n"),
        "calendar-data must preserve CRLF line endings: {fetched_raw:?}"
    );
    let parsed = read_task_resource(fetched_raw, &item_href, &col_href).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].title, "Comidas: ñoquis, 西红柿 & <test>");

    let multiget = client
        .fetch_hrefs(
            &col_href,
            &[item_href.clone(), format!("{col_href}does-not-exist.ics")],
        )
        .await
        .unwrap();
    assert_eq!(multiget.len(), 1);
    assert_eq!(multiget[0].href, item_href);

    let v2 = v1.replace("Comidas:", "Cena:");
    let etag2 = client
        .put_resource(&item_href, &v2, Some(&etag1), false)
        .await
        .unwrap()
        .expect("Radicale returns a new ETag on PUT update");
    assert_ne!(etag1, etag2);

    assert_eq!(
        client
            .put_resource(&item_href, &v2, Some(&etag1), false)
            .await
            .unwrap_err()
            .code(),
        "conflict"
    );
    assert_eq!(
        client
            .delete_resource(&item_href, Some(&etag1))
            .await
            .unwrap_err()
            .code(),
        "conflict"
    );

    client
        .delete_resource(&item_href, Some(&etag2))
        .await
        .unwrap();
    client.delete_resource(&item_href, None).await.unwrap();

    let encoded_href = format!("{col_href}my%20task%40caf%C3%A9.ics");
    let encoded_ics = v1.replace("UID:utf8-item", "UID:my task@café");
    let encoded_etag = client
        .put_resource(&encoded_href, &encoded_ics, None, true)
        .await
        .unwrap()
        .unwrap();
    let encoded_list = client.fetch_resources(&col_href).await.unwrap();
    assert_eq!(encoded_list.len(), 1);
    assert_eq!(encoded_list[0].href, encoded_href);
    client
        .delete_resource(&encoded_list[0].href, Some(&encoded_etag))
        .await
        .unwrap();

    client.delete_collection(&col_href).await.unwrap();
}
