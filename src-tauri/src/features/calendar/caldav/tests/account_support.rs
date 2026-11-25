//! Shared fake credential backend and authenticated `CalDAV` fixtures.

use super::super::account::CredentialStore;
use super::{HOME_PROPFIND, PRINCIPAL_PROPFIND, ROOT_PROPFIND};
use crate::error::AppError;
use std::collections::BTreeMap;
use std::sync::Mutex;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Default)]
pub(super) struct MemoryCredentials(pub(super) Mutex<BTreeMap<String, String>>);

impl CredentialStore for MemoryCredentials {
    fn load(&self, id: &str) -> Result<Option<String>, AppError> {
        Ok(self.0.lock().unwrap().get(id).cloned())
    }
    fn save(&self, id: &str, password: &str) -> Result<(), AppError> {
        self.0
            .lock()
            .unwrap()
            .insert(id.to_string(), password.to_string());
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), AppError> {
        self.0.lock().unwrap().remove(id);
        Ok(())
    }
}

pub(super) async fn server(password: &str) -> MockServer {
    let server = MockServer::start().await;
    let auth = if password == "first" {
        "Basic bWU6Zmlyc3Q="
    } else {
        "Basic bWU6c2Vjb25k"
    };
    for (href, property, body) in [
        (
            "/.well-known/caldav",
            "current-user-principal",
            ROOT_PROPFIND,
        ),
        ("/me/", "calendar-home-set", PRINCIPAL_PROPFIND),
        ("/me/", "resourcetype", HOME_PROPFIND),
    ] {
        Mock::given(method("PROPFIND"))
            .and(path(href))
            .and(body_string_contains(property))
            .and(header("authorization", auth))
            .respond_with(ResponseTemplate::new(207).set_body_string(body))
            .mount(&server)
            .await;
    }
    // Both servers intentionally advertise identical hrefs and UIDs.
    let raw = crate::test_support::ics::all_day();
    let raw = raw
        .lines()
        .map(|line| {
            if line.starts_with("SUMMARY:") {
                format!("SUMMARY:{password} sample")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n");
    let xml = format!(
        "<d:multistatus xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\"><d:response>\
        <d:href>/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/sample.ics</d:href><d:propstat><d:prop>\
        <d:getetag>\"fixture\"</d:getetag><c:calendar-data>{raw}</c:calendar-data></d:prop>\
        <d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"
    );
    Mock::given(method("REPORT"))
        .and(path("/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/"))
        .and(header("authorization", auth))
        .respond_with(ResponseTemplate::new(207).set_body_string(xml))
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(method("REPORT"))
        .and(header("authorization", auth))
        .respond_with(
            ResponseTemplate::new(207).set_body_string("<d:multistatus xmlns:d=\"DAV:\"/>"),
        )
        .mount(&server)
        .await;
    server
}
