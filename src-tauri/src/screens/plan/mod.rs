use crate::db;
use crate::domain::{AppEvent, AppEventFilter, AppTask, AppTaskFilter};
use crate::error::AppError;
use chrono::{NaiveDateTime, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tauri::Manager;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Clone, TS)]
#[ts(export, export_to = "../../src/lib/bindings/LaidEvent.generated.ts")]
pub struct LaidEvent {
    pub event: AppEvent,
    #[serde(rename = "startMins")]
    pub start_mins: f64,
    #[serde(rename = "endMins")]
    pub end_mins: f64,
    pub lane: u16,
    pub lanes: u16,
}

#[derive(Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/PlanDayLayout.generated.ts"
)]
pub struct PlanDayLayout {
    pub date: String,
    pub events: Vec<LaidEvent>,
}

const MINUTES_PER_DAY: f64 = 1440.0;

fn minutes_of(time: NaiveTime) -> f64 {
    f64::from(time.hour()).mul_add(60.0, f64::from(time.minute()))
}

/// Positions an event's date relative to the laid-out day:
/// before -> 0.0, after -> full day, same day -> None (caller computes time).
fn clamp_to_day(date_str: &str, day_start_prefix: &str) -> Option<f64> {
    match date_str.cmp(day_start_prefix) {
        Ordering::Less => Some(0.0),
        Ordering::Greater => Some(MINUTES_PER_DAY),
        Ordering::Equal => None,
    }
}

fn parse_iso_parts(iso: &str) -> Option<(String, Option<NaiveTime>)> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) {
        let local = dt.with_timezone(&chrono::Local);
        return Some((
            local.date_naive().format("%Y-%m-%d").to_string(),
            Some(local.time()),
        ));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S") {
        return Some((dt.date().format("%Y-%m-%d").to_string(), Some(dt.time())));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M") {
        return Some((dt.date().format("%Y-%m-%d").to_string(), Some(dt.time())));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%d %H:%M:%S") {
        return Some((dt.date().format("%Y-%m-%d").to_string(), Some(dt.time())));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%d %H:%M") {
        return Some((dt.date().format("%Y-%m-%d").to_string(), Some(dt.time())));
    }
    if iso.len() >= 10 {
        return iso.get(0..10).map(|s| (s.to_string(), None));
    }
    None
}

fn parse_iso_to_mins(iso: &str, day_start_prefix: &str) -> Option<f64> {
    let (date_str, time) = parse_iso_parts(iso)?;
    clamp_to_day(&date_str, day_start_prefix)
        .map_or_else(|| Some(time.map_or(0.0, minutes_of)), Some)
}

fn layout_day_events(date_str: &str, events: &[AppEvent]) -> Vec<LaidEvent> {
    let mut mapped: Vec<LaidEvent> = Vec::new();

    let mut exceptions = std::collections::HashSet::new();
    for e in events {
        if let Some(recurring_id) = &e.recurring_event_id {
            if let Some(orig_start) = &e.original_start_time {
                exceptions.insert((recurring_id.clone(), orig_start.clone()));
            }
        }
    }

    // Helper to process a concrete event instance
    let mut process_instance = |e: AppEvent, start_iso: String, end_iso: String| {
        if e.is_all_day == Some(true) {
            return;
        }
        if e.status == Some(crate::domain::EventStatus::Cancelled) {
            return;
        }

        let Some(start_mins) = parse_iso_to_mins(&start_iso, date_str) else { return; };
        let Some(end_mins) = parse_iso_to_mins(&end_iso, date_str) else { return; };

        // Filter out if not in this day (in local time string comparison)
        // Wait, start_iso and end_iso are UTC RFC3339 strings, but they get parsed to local strings inside parse_iso_to_mins
        // So we can just check if start_mins == 1440 and end_mins == 0
        if start_mins >= MINUTES_PER_DAY && end_mins >= MINUTES_PER_DAY {
            return;
        }
        if start_mins <= 0.0 && end_mins <= 0.0 {
            return;
        }

        mapped.push(LaidEvent {
            event: e,
            start_mins,
            end_mins,
            lane: 0,
            lanes: 1,
        });
    };

    for e in events {
        if let Some(rrule) = &e.rrule {
            // Expand rrule
            if let Ok(dt_start) = chrono::DateTime::parse_from_rfc3339(&e.start_time) {
                let dt_start_utc = dt_start.with_timezone(&chrono::Utc);
                
                // Get end of the day in UTC to bound the rrule search
                if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    if let Some(day_end) = naive_date.and_hms_opt(23, 59, 59) {
                        if let Some(day_end_local) = day_end.and_local_timezone(chrono::Local).single() {
                            let day_end_utc = day_end_local.with_timezone(&chrono::Utc);
                            // Add some buffer for long events
                            let search_end = day_end_utc.checked_add_signed(chrono::Duration::days(1)).unwrap_or(day_end_utc);

                            if let Ok(occurrences) = crate::time_utils::rrule_utils::get_occurrences(rrule, &dt_start_utc, &search_end) {
                                let duration = chrono::DateTime::parse_from_rfc3339(&e.end_time).map_or_else(
                                    |_| chrono::Duration::zero(),
                                    |dt_end| dt_end.with_timezone(&chrono::Utc).signed_duration_since(dt_start_utc)
                                );

                                for occ in occurrences {
                                    let occ_utc = occ.with_timezone(&chrono::Utc);
                                    let occ_iso = occ_utc.to_rfc3339();
                                    
                                    // Check if this occurrence is an exception
                                    if exceptions.contains(&(e.id.clone(), occ_iso.clone())) {
                                        continue;
                                    }

                                    let occ_end_utc = occ_utc.checked_add_signed(duration).unwrap_or(occ_utc);
                                    
                                    process_instance(e.clone(), occ_iso, occ_end_utc.to_rfc3339());
                                }
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        // Non-recurring or failed to parse recurring
        if e.recurring_event_id.is_none() || e.status != Some(crate::domain::EventStatus::Cancelled) {
            process_instance(e.clone(), e.start_time.clone(), e.end_time.clone());
        }
    }

    let mut placed: Vec<(f64, f64, u16)> = Vec::new();

    for ev in &mut mapped {
        let mut lane: u16 = 0;
        loop {
            let conflict = placed
                .iter()
                .any(|p| p.2 == lane && !(p.1 <= ev.start_mins || p.0 >= ev.end_mins));
            if !conflict {
                break;
            }
            lane = lane.saturating_add(1);
        }
        ev.lane = lane;
        placed.push((ev.start_mins, ev.end_mins, lane));
    }

    for ev in &mut mapped {
        let mut max_lane = ev.lane;
        for p in &placed {
            if !(p.1 <= ev.start_mins || p.0 >= ev.end_mins) && p.2 > max_lane {
                max_lane = p.2;
            }
        }
        ev.lanes = max_lane.saturating_add(1);
    }

    mapped
}

fn db_pool(app: &tauri::AppHandle) -> Result<tauri::State<'_, sqlx::SqlitePool>, AppError> {
    app.try_state::<sqlx::SqlitePool>()
        .ok_or_else(|| AppError::NotReady("Database not initialized yet".to_string()))
}

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn query_plan_layout(
    app: tauri::AppHandle,
    dates: Vec<String>,
    filters: Option<Vec<AppEventFilter>>,
    query: Option<String>,
) -> Result<Vec<PlanDayLayout>, AppError> {
    let pool = db_pool(&app)?;
    let all_events = db::query_events(
        &pool,
        filters.unwrap_or_default(),
        query.unwrap_or_default(),
    )
    .await?;

    let mut result = Vec::new();
    for date in dates {
        let laid_events = layout_day_events(&date, &all_events);
        result.push(PlanDayLayout {
            date,
            events: laid_events,
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso_to_mins_debug() {
        let mins = parse_iso_to_mins("2026-08-26T10:00:00.000Z", "2026-08-26");
        println!("parse_iso_to_mins returned: {mins:?}");
        assert_eq!(mins, Some(930.0)); // Adjust based on local timezone, wait let's just print
    }
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

    let Ok(range_start_dt) = chrono::DateTime::parse_from_rfc3339(range_start_iso).map(|d| d.with_timezone(&chrono::Utc)) else {
        return events.to_vec();
    };
    let Ok(range_end_dt) = chrono::DateTime::parse_from_rfc3339(range_end_iso).map(|d| d.with_timezone(&chrono::Utc)) else {
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
