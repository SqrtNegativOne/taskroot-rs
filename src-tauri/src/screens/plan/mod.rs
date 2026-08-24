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
    pub lane: i32,
    pub lanes: i32,
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
        let local = dt.naive_local();
        return Some((
            local.date().format("%Y-%m-%d").to_string(),
            Some(local.time()),
        ));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S") {
        return Some((dt.date().format("%Y-%m-%d").to_string(), Some(dt.time())));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M") {
        return Some((dt.date().format("%Y-%m-%d").to_string(), Some(dt.time())));
    }
    if iso.len() >= 10 {
        return Some((iso[0..10].to_string(), None));
    }
    None
}

fn parse_iso_to_mins(iso: &str, day_start_prefix: &str) -> Option<f64> {
    let (date_str, time) = parse_iso_parts(iso)?;
    clamp_to_day(&date_str, day_start_prefix)
        .map_or_else(|| Some(time.map_or(0.0, minutes_of)), Some)
}

fn layout_day_events(date_str: &str, events: &[AppEvent]) -> Vec<LaidEvent> {
    let mut mapped: Vec<LaidEvent> = events
        .iter()
        .filter_map(|e| {
            let start_iso = &e.start_time;
            let end_iso = &e.end_time;

            let start_mins = parse_iso_to_mins(start_iso, date_str)?;
            let end_mins = parse_iso_to_mins(end_iso, date_str)?;

            // Filter out if not in this day
            if end_iso < &format!("{date_str}T00:00:00")
                || start_iso > &format!("{date_str}T23:59:59")
            {
                return None;
            }

            Some(LaidEvent {
                event: e.clone(),
                start_mins,
                end_mins,
                lane: 0,
                lanes: 1,
            })
        })
        .collect();

    let mut placed: Vec<(f64, f64, i32)> = Vec::new();

    for ev in &mut mapped {
        let mut lane = 0;
        loop {
            let conflict = placed
                .iter()
                .any(|p| p.2 == lane && !(p.1 <= ev.start_mins || p.0 >= ev.end_mins));
            if !conflict {
                break;
            }
            lane += 1;
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
        ev.lanes = max_lane + 1;
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
pub async fn get_plan_layout(
    app: tauri::AppHandle,
    dates: Vec<String>,
    filters: Option<Vec<AppEventFilter>>,
    query: Option<String>,
) -> Result<Vec<PlanDayLayout>, AppError> {
    let pool = db_pool(&app)?;
    let all_events = if filters.is_some() || query.is_some() {
        db::get_filtered_events(
            &pool,
            filters.unwrap_or_default(),
            query.unwrap_or_default(),
        )
        .await?
    } else {
        db::get_events(&pool).await?
    };

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

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn get_filtered_tasks(
    app: tauri::AppHandle,
    filters: Vec<AppTaskFilter>,
    sort: String,
    query: String,
) -> Result<Vec<AppTask>, AppError> {
    let pool = db_pool(&app)?;
    Ok(db::get_filtered_tasks(&pool, filters, sort, query).await?)
}

#[tauri::command]
pub fn compute_filter_defaults(
    filters: Vec<AppTaskFilter>,
) -> Result<crate::domain::AppTaskDefaults, AppError> {
    Ok(crate::domain::compute_filter_defaults(filters))
}

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn get_filtered_events(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
) -> Result<Vec<AppEvent>, AppError> {
    let pool = db_pool(&app)?;
    Ok(db::get_filtered_events(&pool, filters, query).await?)
}
