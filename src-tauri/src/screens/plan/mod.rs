use crate::db;
use crate::domain::{AppEvent, AppEventFilter, AppTask, AppTaskFilter};
use crate::error::AppError;
use tauri::Manager;

fn db_pool(app: &tauri::AppHandle) -> Result<tauri::State<'_, sqlx::SqlitePool>, AppError> {
    app.try_state::<sqlx::SqlitePool>()
        .ok_or_else(|| AppError::NotReady("Database not initialized yet".to_string()))
}



/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn query_tasks(
    app: tauri::AppHandle,
    filters: Vec<AppTaskFilter>,
    sort: Vec<crate::domain::AppTaskSort>,
    query: String,
) -> Result<Vec<AppTask>, AppError> {
    let pool = db_pool(&app)?;
    Ok(db::query_tasks(&pool, filters, sort, query).await?)
}



#[must_use]
pub fn expand_events_for_range(
    events: &[AppEvent],
    range_start_iso: &str,
    range_end_iso: &str,
) -> Vec<AppEvent> {
    let mut expanded = Vec::new();
    let mut exceptions = std::collections::HashSet::new();
    
    for e in events {
        if let Some(recurring_id) = &e.recurring_event_id {
            if let Some(orig_start) = &e.original_start_time {
                exceptions.insert((recurring_id.clone(), orig_start.clone()));
            }
        }
    }

    let parse_date = |iso: &str, end_of_day: bool| -> Option<chrono::DateTime<chrono::Utc>> {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
            return Some(dt.with_timezone(&chrono::Utc));
        }
        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
            let time = if end_of_day {
                chrono::NaiveTime::from_hms_opt(23, 59, 59)?
            } else {
                chrono::NaiveTime::from_hms_opt(0, 0, 0)?
            };
            if let Some(local_dt) = naive_date.and_time(time).and_local_timezone(chrono::Local).single() {
                return Some(local_dt.with_timezone(&chrono::Utc));
            }
        }
        None
    };

    let Some(range_start_dt) = parse_date(range_start_iso, false) else {
        return events.to_vec();
    };
    let Some(range_end_dt) = parse_date(range_end_iso, true) else {
        return events.to_vec();
    };

    for e in events {
        if let Some(rrule) = &e.rrule {
            if let Ok(dt_start) = chrono::DateTime::parse_from_rfc3339(&e.start_time) {
                let dt_start_utc = dt_start.with_timezone(&chrono::Utc);
                if let Ok(occurrences) = crate::time_utils::rrule_utils::get_occurrences(rrule, &dt_start_utc, &range_end_dt) {
                    let duration = chrono::DateTime::parse_from_rfc3339(&e.end_time).map_or_else(
                        |_| chrono::Duration::zero(),
                        |dt_end| dt_end.with_timezone(&chrono::Utc).signed_duration_since(dt_start_utc)
                    );
                    
                    for occ in occurrences {
                        let occ_utc = occ.with_timezone(&chrono::Utc);
                        let occ_iso = occ_utc.to_rfc3339();
                        
                        if exceptions.contains(&(e.id.clone(), occ_iso.clone())) {
                            continue;
                        }
                        
                        let occ_end_utc = occ_utc.checked_add_signed(duration).unwrap_or(occ_utc);
                        
                        if occ_end_utc > range_start_dt && occ_utc < range_end_dt {
                            let mut instance = e.clone();
                            instance.start_time = occ_iso;
                            instance.end_time = occ_end_utc.to_rfc3339();
                            expanded.push(instance);
                        }
                    }
                    continue;
                }
            }
        }
        
        if e.recurring_event_id.is_none() || e.status != Some(crate::domain::EventStatus::Cancelled) {
            // Check if non-recurring event overlaps with range
            if let (Ok(s), Ok(e_dt)) = (chrono::DateTime::parse_from_rfc3339(&e.start_time), chrono::DateTime::parse_from_rfc3339(&e.end_time)) {
                let s_utc = s.with_timezone(&chrono::Utc);
                let e_utc = e_dt.with_timezone(&chrono::Utc);
                if e_utc > range_start_dt && s_utc < range_end_dt {
                    expanded.push(e.clone());
                }
            } else {
                // If it can't parse, just include it to be safe
                expanded.push(e.clone());
            }
        }
    }
    
    expanded
}

#[tauri::command]
pub async fn query_events(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<AppEvent>, AppError> {
    let pool = db_pool(&app)?;
    let mut events = db::query_events(&pool, filters, query).await?;

    if let (Some(start), Some(end)) = (start_date, end_date) {
        events = expand_events_for_range(&events, &start, &end);
    }

    Ok(events)
}

#[tauri::command]
#[must_use]
pub fn get_task_schema() -> Vec<crate::domain::AppTaskColumnDef> {
    crate::domain::AppTask::get_schema()
}

#[tauri::command]
#[must_use]
pub fn get_event_schema() -> Vec<crate::domain::AppEventColumnDef> {
    crate::domain::AppEvent::get_schema()
}
