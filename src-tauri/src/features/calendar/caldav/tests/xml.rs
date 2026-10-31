//! Parser tests driven by the captured Radicale responses.

use crate::features::calendar::caldav::model::CalendarComponent;
use crate::features::calendar::caldav::xml::{
    calendar_color_body, mkcalendar_body, multiget_body, parse_calendar_home_set,
    parse_calendar_resources, parse_collections, parse_current_user_principal,
    rename_calendar_body, resolve_href, CALENDAR_QUERY_BODY,
};

use super::{HOME_PROPFIND, MULTIGET, PRINCIPAL_PROPFIND, ROOT_PROPFIND};

const SQRT_CAL: &str = "/me/2f07f5a4-5fc2-bc5c-b6de-31053f2b5d33/";
const SQRT_TASKS: &str = "/me/3be68138-2e0e-b50b-63ee-7c07e35627e1/";

#[test]
fn root_propfind_reports_the_principal() {
    let principal = parse_current_user_principal(ROOT_PROPFIND, "/").unwrap();

    assert_eq!(principal, "/me/");
}

#[test]
fn principal_propfind_reports_the_calendar_home() {
    let home = parse_calendar_home_set(PRINCIPAL_PROPFIND, "/me/").unwrap();

    assert_eq!(home, "/me/");
}

#[test]
fn home_propfind_reports_both_typed_calendars() {
    let calendars = parse_collections(HOME_PROPFIND, "/me/").unwrap();

    assert_eq!(
        calendars.len(),
        2,
        "only the two calendar collections count"
    );

    let sqrt_cal = calendars
        .iter()
        .find(|calendar| calendar.href == SQRT_CAL)
        .expect("SqrtCal is discovered");
    assert_eq!(sqrt_cal.display_name, "SqrtCal");
    assert_eq!(sqrt_cal.color.as_deref(), Some("#9966a7ff"));
    assert_eq!(sqrt_cal.component, CalendarComponent::VEvent);
    assert!(sqrt_cal.writable, "Radicale grants write privileges");

    let sqrt_tasks = calendars
        .iter()
        .find(|calendar| calendar.href == SQRT_TASKS)
        .expect("SqrtTasks is discovered");
    assert_eq!(sqrt_tasks.display_name, "SqrtTasks");
    assert_eq!(sqrt_tasks.color.as_deref(), Some("#55fce0ff"));
    assert_eq!(sqrt_tasks.component, CalendarComponent::VTodo);
    assert!(sqrt_tasks.writable);
}

#[test]
fn a_404_propstat_property_is_absent() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"
             xmlns:ICAL="http://apple.com/ns/ical/">
  <response>
    <href>/me/cal/</href>
    <propstat>
      <prop><displayname>Missing</displayname></prop>
      <status>HTTP/1.1 404 Not Found</status>
    </propstat>
    <propstat>
      <prop>
        <resourcetype><C:calendar/><collection/></resourcetype>
        <C:supported-calendar-component-set><C:comp name="VEVENT"/></C:supported-calendar-component-set>
        <ICAL:calendar-color>#112233</ICAL:calendar-color>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#;

    let calendars = parse_collections(xml, "/me/").unwrap();

    assert_eq!(calendars.len(), 1);
    assert_eq!(calendars[0].display_name, "");
    assert_eq!(calendars[0].color.as_deref(), Some("#112233"));
}

#[test]
fn a_collection_advertising_both_components_is_vevent_only() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/me/both/</href>
    <propstat>
      <prop>
        <displayname>Both</displayname>
        <resourcetype><C:calendar/><collection/></resourcetype>
        <C:supported-calendar-component-set>
          <C:comp name="VEVENT"/>
          <C:comp name="VTODO"/>
        </C:supported-calendar-component-set>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#;

    let calendars = parse_collections(xml, "/me/").unwrap();

    assert_eq!(calendars.len(), 1);
    assert_eq!(calendars[0].component, CalendarComponent::VEvent);
}

#[test]
fn calendar_resources_report_the_href_etag_and_calendar_data() {
    let resources = parse_calendar_resources(MULTIGET, SQRT_CAL).unwrap();

    assert_eq!(resources.len(), 1);
    let resource = &resources[0];
    assert_eq!(
        resource.href,
        format!("{SQRT_CAL}dsh-20261017-first-day-of-durga.ics")
    );
    assert_eq!(
        resource.etag.as_deref(),
        Some(r#""5d6e68d703bce2dbaa2a7fa1e555e5d8eaccac14b4784c94dd8cdf829c811926""#)
    );
    let calendar_data = resource
        .calendar_data
        .as_deref()
        .expect("the multiget carries the resource body");
    assert!(calendar_data.starts_with("BEGIN:VCALENDAR"));
    assert!(calendar_data.ends_with("END:VCALENDAR"));
    assert!(calendar_data.contains("UID:dsh-20261017-first-day-of-durga@dsh-import"));
}

#[test]
fn resolve_href_handles_paths_urls_and_relative_hrefs() {
    assert_eq!(resolve_href("/me/cal/", "/me/cal/a.ics"), "/me/cal/a.ics");
    assert_eq!(
        resolve_href("/me/cal/", "https://dav.example/me/cal/a.ics"),
        "/me/cal/a.ics"
    );
    assert_eq!(resolve_href("/me/cal/", "a.ics"), "/me/cal/a.ics");
    assert_eq!(
        resolve_href("https://dav.example/me/cal/", "a.ics"),
        "/me/cal/a.ics"
    );
    assert_eq!(resolve_href("/me/cal", "a.ics"), "/me/cal/a.ics");
}

#[test]
fn calendar_query_asks_for_etag_and_data_of_every_calendar() {
    assert!(CALENDAR_QUERY_BODY.contains("<c:calendar-query"));
    assert!(CALENDAR_QUERY_BODY.contains("<d:getetag/>"));
    assert!(CALENDAR_QUERY_BODY.contains("<c:calendar-data/>"));
    assert!(CALENDAR_QUERY_BODY.contains(r#"<c:comp-filter name="VCALENDAR"/>"#));
}

#[test]
fn multiget_body_lists_every_href_and_escapes_them() {
    let body = multiget_body(&["/me/cal/a.ics".to_string(), "/me/cal/b&c.ics".to_string()]);

    assert!(body.contains("<d:href>/me/cal/a.ics</d:href>"));
    assert!(body.contains("<d:href>/me/cal/b&amp;c.ics</d:href>"));
    assert!(body.contains("<c:calendar-data/>"));
}

#[test]
fn mkcalendar_body_sets_the_display_name_and_component() {
    let body = mkcalendar_body("Tasks & Notes", CalendarComponent::VTodo);

    assert!(body.contains("<d:displayname>Tasks &amp; Notes</d:displayname>"));
    assert!(body.contains(r#"<c:comp name="VTODO"/>"#));
}

#[test]
fn rename_calendar_body_is_a_propertyupdate_with_an_escaped_name() {
    let body = rename_calendar_body("Work & <Home>");

    assert!(body.contains(r#"<d:propertyupdate xmlns:d="DAV:">"#));
    assert!(body.contains("<d:displayname>Work &amp; &lt;Home&gt;</d:displayname>"));
}

#[test]
fn calendar_color_body_sets_or_removes_the_apple_property() {
    let set = calendar_color_body(Some("#ff8800"));
    assert!(set.contains("<ical:calendar-color>#ff8800</ical:calendar-color>"));
    assert!(set.contains(r#"xmlns:ical="http://apple.com/ns/ical/""#));

    let removed = calendar_color_body(None);
    assert!(removed.contains("<d:remove><d:prop><ical:calendar-color/></d:prop></d:remove>"));
    assert!(!removed.contains("<ical:calendar-color>"));
}
