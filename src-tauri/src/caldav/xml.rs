//! CalDAV request bodies and `roxmltree` response parsers.
//!
//! Responses are matched by local element name, so a server may use any XML
//! namespace prefix (Radicale emits `C:` / `ICAL:` in some responses and the
//! default `DAV:` namespace in others). Only `2xx` `propstat` blocks count: a
//! property advertised in a `404` block is treated as absent.
//!
//! Hrefs are normalised to server-rooted paths (see [`resolve_href`]) because
//! Radicale reports absolute paths while other servers may report absolute URLs
//! or hrefs relative to the requested collection.

use reqwest::Url;
use roxmltree::{Document, Node};

use crate::error::AppError;

use super::model::{CalendarComponent, CalendarInfo};

pub const XML_CONTENT_TYPE: &str = "application/xml; charset=utf-8";
pub const CALENDAR_CONTENT_TYPE: &str = "text/calendar; charset=utf-8";

/// `PROPFIND` `.well-known/caldav` or `/`: who is the current user?
pub const ROOT_PROPFIND_BODY: &str = concat!(
    r#"<?xml version="1.0" encoding="utf-8"?>"#,
    r#"<d:propfind xmlns:d="DAV:"><d:prop><d:current-user-principal/></d:prop></d:propfind>"#,
);

/// `PROPFIND` the principal: where is its calendar home set?
pub const PRINCIPAL_PROPFIND_BODY: &str = concat!(
    r#"<?xml version="1.0" encoding="utf-8"?>"#,
    r#"<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">"#,
    r#"<d:prop><c:calendar-home-set/></d:prop></d:propfind>"#,
);

/// `PROPFIND` the calendar home with `Depth: 1`: which calendars does it hold?
pub const HOME_PROPFIND_BODY: &str = concat!(
    r#"<?xml version="1.0" encoding="utf-8"?>"#,
    r#"<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" "#,
    r#"xmlns:ical="http://apple.com/ns/ical/">"#,
    r#"<d:prop><d:displayname/><d:resourcetype/><c:supported-calendar-component-set/>"#,
    r#"<ical:calendar-color/><d:current-user-privilege-set/></d:prop></d:propfind>"#,
);

/// `PROPFIND` one collection.
pub const COLLECTION_PROPFIND_BODY: &str = concat!(
    r#"<?xml version="1.0" encoding="utf-8"?>"#,
    r#"<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" "#,
    r#"xmlns:ical="http://apple.com/ns/ical/">"#,
    r#"<d:prop><d:displayname/><d:getetag/><d:resourcetype/>"#,
    r#"<c:supported-calendar-component-set/><ical:calendar-color/></d:prop></d:propfind>"#,
);

/// The first `calendar-multiget` request: no known hrefs, hence no filter.
pub const SYNC_COLLECTION_INITIAL_BODY: &str = concat!(
    r#"<?xml version="1.0" encoding="utf-8"?>"#,
    r#"<d:sync-collection xmlns:d="DAV:"><d:sync-token/>"#,
    r#"<d:sync-level>1</d:sync-level><d:prop><d:getetag/></d:prop></d:sync-collection>"#,
);

/// A `sync-collection` body for an empty (initial) or stored token.
#[must_use]
pub fn sync_collection_body(token: Option<&str>) -> String {
    let Some(token) = token else {
        return SYNC_COLLECTION_INITIAL_BODY.to_string();
    };
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="utf-8"?>"#,
            r#"<d:sync-collection xmlns:d="DAV:"><d:sync-token>{}</d:sync-token>"#,
            r#"<d:sync-level>1</d:sync-level><d:prop><d:getetag/></d:prop></d:sync-collection>"#,
        ),
        escape_xml(token)
    )
}

/// A `calendar-multiget` body asking for the ETag and body of every href.
#[must_use]
pub fn multiget_body(hrefs: &[String]) -> String {
    let mut body = String::from(concat!(
        r#"<?xml version="1.0" encoding="utf-8"?>"#,
        r#"<c:calendar-multiget xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">"#,
        r#"<d:prop><d:getetag/><c:calendar-data/></d:prop>"#,
    ));
    for href in hrefs {
        body.push_str("<d:href>");
        body.push_str(&escape_xml(href));
        body.push_str("</d:href>");
    }
    body.push_str("</c:calendar-multiget>");
    body
}

/// A `MKCALENDAR` body creating a collection with one component kind.
#[must_use]
pub fn mkcalendar_body(display_name: &str, component: CalendarComponent) -> String {
    format!(
        concat!(
            r#"<?xml version="1.0" encoding="utf-8"?>"#,
            r#"<c:mkcalendar xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">"#,
            r#"<d:set><d:prop><d:displayname>{}</d:displayname>"#,
            r#"<c:supported-calendar-component-set><c:comp name="{}"/>"#,
            r#"</c:supported-calendar-component-set></d:prop></d:set></c:mkcalendar>"#,
        ),
        escape_xml(display_name),
        component.as_str()
    )
}

/// One entry of a `sync-collection` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncChange {
    pub href: String,
    /// `200` for a created/updated resource, `404`/`410` for a deleted one.
    pub status: u16,
    pub etag: Option<String>,
}

impl SyncChange {
    /// Whether this change reports a resource that no longer exists.
    #[must_use]
    pub const fn is_removed(&self) -> bool {
        matches!(self.status, 404 | 410)
    }
}

/// A parsed `sync-collection` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCollection {
    /// The token to send next time. `None` when the server omitted it.
    pub token: Option<String>,
    pub changes: Vec<SyncChange>,
}

/// One resource returned by a `calendar-multiget` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiGetResource {
    pub href: String,
    pub etag: Option<String>,
    /// The raw `.ics` body, or `None` when the server skipped the resource.
    pub calendar_data: Option<String>,
}

/// Parse the `current-user-principal` href from a `PROPFIND` response.
///
/// # Errors
///
/// Returns [`AppError::Sync`] for malformed XML or a response without a
/// principal.
pub fn parse_current_user_principal(xml: &str, base_href: &str) -> Result<String, AppError> {
    let document = parse_document(xml)?;
    for response in response_nodes(&document) {
        let props = successful_props(response);
        let Some(principal) = prop(&props, "current-user-principal") else {
            continue;
        };
        let Some(href) = href_child(principal) else {
            continue;
        };
        return Ok(resolve_href(base_href, &href));
    }
    Err(AppError::Sync(
        "The server did not advertise a current-user-principal".to_string(),
    ))
}

/// Parse the `calendar-home-set` href from a principal `PROPFIND` response.
///
/// # Errors
///
/// Returns [`AppError::Sync`] for malformed XML or a response without a home
/// set.
pub fn parse_calendar_home_set(xml: &str, base_href: &str) -> Result<String, AppError> {
    let document = parse_document(xml)?;
    for response in response_nodes(&document) {
        let props = successful_props(response);
        let Some(home_set) = prop(&props, "calendar-home-set") else {
            continue;
        };
        let Some(href) = href_child(home_set) else {
            continue;
        };
        return Ok(resolve_href(base_href, &href));
    }
    Err(AppError::Sync(
        "The server did not advertise a calendar-home-set".to_string(),
    ))
}

/// Parse the calendars in a `Depth: 1` calendar-home `PROPFIND` response.
///
/// Collections whose `resourcetype` is not a calendar, or which advertise
/// neither `VEVENT` nor `VTODO`, are skipped. A collection advertising both is
/// logged and treated as `VEVENT`-only.
///
/// # Errors
///
/// Returns [`AppError::Sync`] for malformed XML.
pub fn parse_collections(xml: &str, home_href: &str) -> Result<Vec<CalendarInfo>, AppError> {
    let document = parse_document(xml)?;
    let mut calendars = Vec::new();

    for response in response_nodes(&document) {
        let Some(href) = element(response, "href").map(|node| resolve_href(home_href, &text(node)))
        else {
            continue;
        };
        let props = successful_props(response);
        if !is_calendar(&props) {
            continue;
        }
        let (has_event, has_task) = supported_components(&props);
        let component = match (has_event, has_task) {
            (true, true) => {
                eprintln!(
                    "[caldav] collection {href} advertises both VEVENT and VTODO; \
                     treating it as VEVENT-only"
                );
                CalendarComponent::VEvent
            }
            (true, false) => CalendarComponent::VEvent,
            (false, true) => CalendarComponent::VTodo,
            (false, false) => continue,
        };
        calendars.push(CalendarInfo {
            href,
            display_name: prop(&props, "displayname").map(text).unwrap_or_default(),
            color: prop(&props, "calendar-color")
                .map(text)
                .filter(|color| color.starts_with('#')),
            component,
            writable: privileges_allow_write(&props),
        });
    }

    Ok(calendars)
}

/// Parse a `sync-collection` response into its token and changes.
///
/// # Errors
///
/// Returns [`AppError::Sync`] for malformed XML.
pub fn parse_sync_collection(xml: &str, collection_href: &str) -> Result<SyncCollection, AppError> {
    let document = parse_document(xml)?;
    let token = element(document.root_element(), "sync-token")
        .map(text)
        .filter(|token| !token.is_empty());

    let mut changes = Vec::new();
    for response in response_nodes(&document) {
        let Some(href) = element(response, "href") else {
            continue;
        };
        let href = resolve_href(collection_href, &text(href));
        let props = successful_props(response);
        let etag = prop(&props, "getetag")
            .map(text)
            .filter(|etag| !etag.is_empty());
        let status = response_status(response).unwrap_or(200);
        changes.push(SyncChange { href, status, etag });
    }

    Ok(SyncCollection { token, changes })
}

/// Parse a `calendar-multiget` response into href/etag/`calendar-data` tuples.
///
/// Responses without a `2xx` `calendar-data` property (e.g. a `404` for a
/// deleted href) are skipped.
///
/// # Errors
///
/// Returns [`AppError::Sync`] for malformed XML.
pub fn parse_multiget(xml: &str, collection_href: &str) -> Result<Vec<MultiGetResource>, AppError> {
    let document = parse_document(xml)?;
    let mut resources = Vec::new();

    for response in response_nodes(&document) {
        let Some(href) = element(response, "href") else {
            continue;
        };
        let href = resolve_href(collection_href, &text(href));
        let props = successful_props(response);
        let Some(calendar_data) = prop(&props, "calendar-data").map(text) else {
            continue;
        };
        let etag = prop(&props, "getetag")
            .map(text)
            .filter(|etag| !etag.is_empty());
        resources.push(MultiGetResource {
            href,
            etag,
            calendar_data: Some(calendar_data),
        });
    }

    Ok(resources)
}

/// Resolve an href reported by the server to an absolute path on the server.
///
/// Absolute URLs are reduced to their path, server-rooted paths pass through,
/// and a bare relative href is joined onto `base_href` (the requested
/// collection, which is treated as a directory when it ends with `/`).
#[must_use]
pub fn resolve_href(base_href: &str, href: &str) -> String {
    let href = href.trim();
    if let Some(path) = absolute_url_path(href) {
        return path;
    }
    if href.starts_with('/') {
        return href.to_string();
    }
    let base_path = absolute_url_path(base_href).unwrap_or_else(|| base_href.trim().to_string());
    if base_path.ends_with('/') {
        return format!("{base_path}{href}");
    }
    format!("{base_path}/{href}")
}

fn parse_document(xml: &str) -> Result<Document<'_>, AppError> {
    Document::parse(xml).map_err(|error| AppError::Sync(format!("Malformed CalDAV XML: {error}")))
}

fn response_nodes<'a, 'input>(
    document: &'a Document<'input>,
) -> impl Iterator<Item = Node<'a, 'input>> + 'a {
    document
        .root_element()
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "response")
}

fn successful_props<'a, 'input>(response: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    response
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "propstat")
        .filter(|propstat| propstat_is_success(*propstat))
        .filter_map(|propstat| element(propstat, "prop"))
        .flat_map(|prop| prop.children().filter(Node::is_element))
        .collect()
}

fn propstat_is_success(propstat: Node<'_, '_>) -> bool {
    element(propstat, "status")
        .map(text)
        .is_some_and(|status| status_code(&status).is_some_and(|code| (200..300).contains(&code)))
}

fn response_status(response: Node<'_, '_>) -> Option<u16> {
    element(response, "status").and_then(|status| status_code(&text(status)))
}

fn status_code(status: &str) -> Option<u16> {
    status.split_whitespace().nth(1)?.parse().ok()
}

fn element<'a, 'input>(parent: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    parent
        .children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

fn prop<'a, 'input>(props: &[Node<'a, 'input>], name: &str) -> Option<Node<'a, 'input>> {
    props
        .iter()
        .copied()
        .find(|node| node.tag_name().name() == name)
}

fn text(node: Node<'_, '_>) -> String {
    node.text().unwrap_or_default().trim().to_string()
}

fn href_child(node: Node<'_, '_>) -> Option<String> {
    let href = text(element(node, "href")?);
    (!href.is_empty()).then_some(href)
}

fn is_calendar(props: &[Node<'_, '_>]) -> bool {
    prop(props, "resourcetype")
        .is_some_and(|resource_type| element(resource_type, "calendar").is_some())
}

fn supported_components(props: &[Node<'_, '_>]) -> (bool, bool) {
    let Some(component_set) = prop(props, "supported-calendar-component-set") else {
        return (false, false);
    };
    let mut has_event = false;
    let mut has_task = false;
    for component in component_set
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == "comp")
    {
        match component.attribute("name") {
            Some("VEVENT") => has_event = true,
            Some("VTODO") => has_task = true,
            _ => {}
        }
    }
    (has_event, has_task)
}

fn privileges_allow_write(props: &[Node<'_, '_>]) -> bool {
    let Some(privilege_set) = prop(props, "current-user-privilege-set") else {
        // Servers that do not advertise privileges are assumed writable; a
        // rejected write surfaces as an error, which is less surprising than
        // hiding every calendar.
        return true;
    };
    privilege_set
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "privilege")
        .flat_map(|privilege| privilege.children())
        .filter(Node::is_element)
        .any(|grant| matches!(grant.tag_name().name(), "write" | "write-content" | "all"))
}

fn absolute_url_path(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.path().to_string())
}

fn escape_xml(value: &str) -> String {
    value
        .chars()
        .fold(String::with_capacity(value.len()), |mut out, ch| {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&apos;"),
                _ => out.push(ch),
            }
            out
        })
}
