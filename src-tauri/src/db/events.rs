use crate::domain::AppEvent;
use sqlx::SqlitePool;

macro_rules! event_select_sql {
    ($suffix:literal) => {
        concat!(
            "SELECT id, remote_id, remote_collection_id, task_id, title, description, start_time, end_time, ",
            "rrule, COALESCE(exdates, 'null') as exdates, recurring_event_id, original_start_time, status, ",
            "updated_at, color, etag, dirty, is_all_day, timezone FROM events",
            $suffix
        )
    };
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_event(pool: &SqlitePool, id: &str) -> Result<Option<AppEvent>, sqlx::Error> {
    let event = sqlx::query_as::<_, AppEvent>(event_select_sql!(" WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(event)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_event_by_remote_id(
    pool: &SqlitePool,
    remote_id: &str,
) -> Result<Option<AppEvent>, sqlx::Error> {
    let event = sqlx::query_as::<_, AppEvent>(event_select_sql!(" WHERE remote_id = ?"))
        .bind(remote_id)
        .fetch_optional(pool)
        .await?;
    Ok(event)
}

/// Dirty-event feed for the offline-enqueue roadmap (see TODO.md).
#[allow(dead_code)]
/// # Errors
///
/// Returns an error if the operation fails.
pub async fn get_dirty_events(pool: &SqlitePool) -> Result<Vec<AppEvent>, sqlx::Error> {
    let events = sqlx::query_as::<_, AppEvent>(event_select_sql!(" WHERE dirty = 1"))
        .fetch_all(pool)
        .await?;
    Ok(events)
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn create_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO events (
            id, remote_id, remote_collection_id, task_id, title, description, 
            start_time, end_time, rrule, exdates, recurring_event_id, 
            original_start_time, status, updated_at, color, etag, dirty, is_all_day, timezone
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(event.id)
    .bind(event.remote_id)
    .bind(event.remote_collection_id)
    .bind(event.task_id)
    .bind(event.title)
    .bind(event.description)
    .bind(event.start_time)
    .bind(event.end_time)
    .bind(event.rrule)
    .bind(event.exdates.map(sqlx::types::Json))
    .bind(event.recurring_event_id)
    .bind(event.original_start_time)
    .bind(event.status)
    .bind(event.updated_at)
    .bind(event.color)
    .bind(event.etag)
    .bind(event.dirty)
    .bind(event.is_all_day)
    .bind(event.timezone)
    .execute(pool)
    .await?;

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn update_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE events SET 
            remote_id = ?, remote_collection_id = ?, task_id = ?, title = ?, 
            description = ?, start_time = ?, end_time = ?, 
            rrule = ?, exdates = ?, recurring_event_id = ?, original_start_time = ?, 
            status = ?, updated_at = ?, color = ?, etag = ?, dirty = ?, is_all_day = ?, timezone = ?
        WHERE id = ?",
    )
    .bind(event.remote_id)
    .bind(event.remote_collection_id)
    .bind(event.task_id)
    .bind(event.title)
    .bind(event.description)
    .bind(event.start_time)
    .bind(event.end_time)
    .bind(event.rrule)
    .bind(event.exdates.map(sqlx::types::Json))
    .bind(event.recurring_event_id)
    .bind(event.original_start_time)
    .bind(event.status)
    .bind(event.updated_at)
    .bind(event.color)
    .bind(event.etag)
    .bind(event.dirty)
    .bind(event.is_all_day)
    .bind(event.timezone)
    .bind(event.id)
    .execute(pool)
    .await?;

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn upsert_event(pool: &SqlitePool, event: AppEvent) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO events (
            id, remote_id, remote_collection_id, task_id, title, description, 
            start_time, end_time, rrule, exdates, recurring_event_id, 
            original_start_time, status, updated_at, color, etag, dirty, is_all_day, timezone
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            remote_id = excluded.remote_id, 
            remote_collection_id = excluded.remote_collection_id, 
            task_id = excluded.task_id, 
            title = excluded.title, 
            description = excluded.description, 
            start_time = excluded.start_time, 
            end_time = excluded.end_time, 
            rrule = excluded.rrule, 
            exdates = excluded.exdates, 
            recurring_event_id = excluded.recurring_event_id, 
            original_start_time = excluded.original_start_time, 
            status = excluded.status, 
            updated_at = excluded.updated_at, 
            color = excluded.color,
            etag = excluded.etag,
            dirty = excluded.dirty,
            is_all_day = excluded.is_all_day,
            timezone = excluded.timezone",
    )
    .bind(event.id)
    .bind(event.remote_id)
    .bind(event.remote_collection_id)
    .bind(event.task_id)
    .bind(event.title)
    .bind(event.description)
    .bind(event.start_time)
    .bind(event.end_time)
    .bind(event.rrule)
    .bind(event.exdates.map(sqlx::types::Json))
    .bind(event.recurring_event_id)
    .bind(event.original_start_time)
    .bind(event.status)
    .bind(event.updated_at)
    .bind(event.color)
    .bind(event.etag)
    .bind(event.dirty)
    .bind(event.is_all_day)
    .bind(event.timezone)
    .execute(pool)
    .await?;

    Ok(())
}

/// # Errors
///
/// Returns an error if the operation fails.
pub async fn delete_event(pool: &SqlitePool, id: String) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM events WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

fn apply_sql(
    builder: &mut sqlx::QueryBuilder<sqlx::Sqlite>,
    col_id: &crate::domain::AppEventFilterColumn,
    op: &crate::domain::FilterOperator,
    val: &serde_json::Value,
    schema: &[crate::domain::AppEventColumnDef],
) {
    let Some(def) = schema.iter().find(|c| &c.id == col_id) else {
        return;
    };

    let is_not = matches!(
        op,
        crate::domain::FilterOperator::IsNot | crate::domain::FilterOperator::DoesNotContain
    );
    let values = val
        .as_array()
        .map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
    if values.is_empty() {
        return;
    }

    builder.push(if is_not {
        format!(" AND {} NOT IN (", def.db_col)
    } else {
        format!(" AND {} IN (", def.db_col)
    });
    let mut separated = builder.separated(", ");
    for v in &values {
        if let Some(s) = v.as_str() {
            separated.push_bind(s);
        } else {
            separated.push_bind(v.to_string());
        }
    }
    builder.push(")");
}

pub async fn query_events(
    pool: &SqlitePool,
    filters: Vec<crate::domain::AppEventFilter>,
    query_text: String,
) -> Result<Vec<AppEvent>, sqlx::Error> {
    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
        sqlx::QueryBuilder::new(event_select_sql!(" WHERE 1=1"));

    let schema = AppEvent::get_schema();

    for f in &filters {
        if let (Some(col_id), Some(val)) = (&f.column, &f.value) {
            let op = f
                .operator
                .as_ref()
                .unwrap_or(&crate::domain::FilterOperator::Is);
            apply_sql(&mut query_builder, col_id, op, val, &schema);
        }
    }

    if !query_text.trim().is_empty() {
        let q = format!("%{}%", query_text.trim().to_lowercase());
        query_builder.push(" AND (LOWER(title) LIKE ");
        query_builder.push_bind(q.clone());
        query_builder.push(" OR LOWER(description) LIKE ");
        query_builder.push_bind(q);
        query_builder.push(")");
    }

    let events = query_builder
        .build_query_as::<AppEvent>()
        .fetch_all(pool)
        .await?;

    Ok(events)
}

/// # Errors
/// Returns an error if the operation fails.
pub async fn upsert_calendar(
    pool: &SqlitePool,
    calendar: crate::domain::AppCalendar,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO calendars (id, summary, color, is_primary, access_role) VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            summary = excluded.summary, 
            color = excluded.color,
            is_primary = excluded.is_primary,
            access_role = excluded.access_role",
    )
    .bind(calendar.id)
    .bind(calendar.summary)
    .bind(calendar.color)
    .bind(calendar.is_primary)
    .bind(calendar.access_role)
    .execute(pool)
    .await?;
    Ok(())
}

/// # Errors
/// Returns an error if the operation fails.
pub async fn delete_calendar(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM calendars WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// # Errors
/// Returns an error if the operation fails.
pub async fn get_calendar_sync_token(
    pool: &SqlitePool,
    calendar_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT sync_token FROM calendars WHERE id = ?")
            .bind(calendar_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|(token,)| token))
}

/// # Errors
/// Returns an error if the operation fails.
pub async fn set_calendar_sync_token(
    pool: &SqlitePool,
    calendar_id: &str,
    token: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE calendars SET sync_token = ? WHERE id = ?")
        .bind(token)
        .bind(calendar_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete a calendar's events without clobbering rows edited locally.
///
/// # Errors
/// Returns an error if the operation fails.
pub async fn delete_synced_events_for_calendar(
    pool: &SqlitePool,
    calendar_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM events WHERE remote_collection_id = ? AND (dirty IS NULL OR dirty = 0)",
    )
    .bind(calendar_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// # Errors
/// Returns an error if the operation fails.
pub async fn get_calendars(
    pool: &SqlitePool,
) -> Result<Vec<crate::domain::AppCalendar>, sqlx::Error> {
    let calendars = sqlx::query_as::<_, crate::domain::AppCalendar>(
        "SELECT id, summary, color, is_primary, access_role FROM calendars",
    )
    .fetch_all(pool)
    .await?;
    Ok(calendars)
}

/// Resolve the background color a locally authored event should inherit.
///
/// The Google sync path bakes the calendar's background color into
/// `events.color` (with a per-event `colorId` taking precedence). Rows created
/// outside that path — e.g. a new event in the UI — skip the denormalization and
/// would render uncolored until their first round-trip. This mirrors the sync
/// fallback chain: explicit calendar, then primary, then the first calendar.
///
/// # Errors
///
/// Returns an error if the calendar lookup fails.
pub async fn resolve_calendar_color(
    pool: &SqlitePool,
    calendar_id: Option<&crate::domain::CollectionId>,
) -> Result<Option<crate::domain::Color>, sqlx::Error> {
    let calendars = get_calendars(pool).await?;
    let selected = calendar_id
        .and_then(|id| calendars.iter().find(|cal| &cal.id == id))
        .or_else(|| calendars.iter().find(|cal| cal.is_primary == Some(true)))
        .or_else(|| calendars.first());
    Ok(selected.and_then(|cal| cal.color.clone()))
}
