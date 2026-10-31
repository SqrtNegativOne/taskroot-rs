//! `CalDAV` resource operations on top of [`super::transport::CalDavClient`].
//!
//! Hrefs are server-rooted paths (see [`super::xml::resolve_href`]); `ETags` are
//! kept exactly as the server returned them, quotes included.

use crate::error::AppError;

use super::account::Account;
use super::model::CalendarComponent;
pub use super::transport::CalDavClient;
use super::transport::{check_status, CalDavRequest, HttpClient};
use super::xml::{
    calendar_color_body, mkcalendar_body, multiget_body, parse_calendar_resources,
    rename_calendar_body, MultiGetResource, CALENDAR_QUERY_BODY,
};

/// `calendar-multiget` batch size; Radicale is happy with far more, but 50 keeps
/// one slow response from stalling the whole collection.
pub const MULTIGET_BATCH_SIZE: usize = 50;

impl CalDavClient {
    /// Build a client from a stored account and its keychain password.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] when the account's server URL is
    /// invalid.
    pub fn from_account(
        http: &HttpClient,
        account: &Account,
        password: &str,
    ) -> Result<Self, AppError> {
        Self::new(http, &account.server_url, &account.username, password)
    }

    /// Fetch every resource in a collection with one `calendar-query` REPORT.
    ///
    /// # Errors
    ///
    /// Propagates transport errors and returns [`AppError::Sync`] for malformed
    /// responses.
    pub async fn fetch_resources(
        &self,
        collection_href: &str,
    ) -> Result<Vec<MultiGetResource>, AppError> {
        let response = self
            .send(
                &CalDavRequest::report(collection_href)
                    .with_depth("1")
                    .with_body(CALENDAR_QUERY_BODY),
            )
            .await?;
        parse_calendar_resources(&response.body, collection_href)
    }

    /// Fetch a specific set of hrefs from one collection, in `multiget` batches.
    ///
    /// # Errors
    ///
    /// Propagates transport errors and returns [`AppError::Sync`] for malformed
    /// responses.
    pub async fn fetch_hrefs(
        &self,
        collection_href: &str,
        hrefs: &[String],
    ) -> Result<Vec<MultiGetResource>, AppError> {
        let mut resources = Vec::new();
        for batch in hrefs.chunks(MULTIGET_BATCH_SIZE) {
            resources.extend(self.multiget(collection_href, batch).await?);
        }
        Ok(resources)
    }

    /// Create or update one resource.
    ///
    /// `create` sends `If-None-Match: *`; an update requires `etag` and sends
    /// `If-Match`. Returns the `ETag` from the response, if the server sent one.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] for an update without an `ETag`,
    /// [`AppError::Conflict`] for a stale `ETag` and [`AppError::Auth`]/
    /// [`AppError::Sync`] for the usual transport failures.
    pub async fn put_resource(
        &self,
        href: &str,
        raw: &str,
        etag: Option<&str>,
        create: bool,
    ) -> Result<Option<String>, AppError> {
        let request = CalDavRequest::put(href, raw);
        let request = if create {
            request.create_only()
        } else {
            let etag = etag.ok_or_else(|| {
                AppError::InvalidInput("Updating a CalDAV resource requires its ETag".to_string())
            })?;
            request.with_if_match(etag)
        };

        let response = self.send(&request).await?;
        Ok(response.etag)
    }

    /// Delete one resource. A `404` or `410` counts as success.
    ///
    /// # Errors
    ///
    /// Propagates transport errors, including [`AppError::Conflict`] for a
    /// stale `If-Match`.
    pub async fn delete_resource(&self, href: &str, etag: Option<&str>) -> Result<(), AppError> {
        let request = CalDavRequest::delete(href);
        let request = match etag {
            Some(etag) => request.with_if_match(etag),
            None => request,
        };
        let response = self.send_raw(&request).await?;

        if matches!(response.status, 404 | 410) {
            return Ok(());
        }
        check_status(response).map(|_| ())
    }

    /// Create a calendar collection.
    ///
    /// # Errors
    ///
    /// Propagates transport errors; a collection that already exists surfaces
    /// as [`AppError::Sync`].
    pub async fn mkcalendar(
        &self,
        href: &str,
        display_name: &str,
        component: CalendarComponent,
    ) -> Result<(), AppError> {
        let body = mkcalendar_body(display_name, component);
        self.send(&CalDavRequest::mkcalendar(href, &body)).await?;
        Ok(())
    }

    /// Set a collection's display name.
    ///
    /// # Errors
    ///
    /// Propagates transport errors.
    pub async fn set_display_name(&self, href: &str, display_name: &str) -> Result<(), AppError> {
        let body = rename_calendar_body(display_name);
        self.send(&CalDavRequest::proppatch(href, &body)).await?;
        Ok(())
    }

    /// Set or clear a collection's `calendar-color`.
    ///
    /// # Errors
    ///
    /// Propagates transport errors.
    pub async fn set_calendar_color(
        &self,
        href: &str,
        color: Option<&str>,
    ) -> Result<(), AppError> {
        let body = calendar_color_body(color);
        self.send(&CalDavRequest::proppatch(href, &body)).await?;
        Ok(())
    }

    /// Delete a whole collection and everything in it. A `404`/`410` counts as
    /// success.
    ///
    /// # Errors
    ///
    /// Propagates transport errors.
    pub async fn delete_collection(&self, href: &str) -> Result<(), AppError> {
        self.delete_resource(href, None).await
    }

    async fn multiget(
        &self,
        collection_href: &str,
        hrefs: &[String],
    ) -> Result<Vec<MultiGetResource>, AppError> {
        let body = multiget_body(hrefs);
        let response = self
            .send(
                &CalDavRequest::report(collection_href)
                    .with_depth("1")
                    .with_body(&body),
            )
            .await?;
        parse_calendar_resources(&response.body, collection_href)
    }
}
