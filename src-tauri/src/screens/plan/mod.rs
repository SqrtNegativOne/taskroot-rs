use crate::db;
use crate::domain::{AppEvent, AppTaskFilter, AppEventFilter, AppTask};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone)]
pub struct LaidEvent {
    pub event: AppEvent,
    #[serde(rename = "startMins")]
    pub start_mins: f64,
    #[serde(rename = "endMins")]
    pub end_mins: f64,
    pub lane: i32,
    pub lanes: i32,
}

#[derive(Serialize, Deserialize)]
pub struct PlanDayLayout {
    pub date: String,
    pub events: Vec<LaidEvent>,
}

fn parse_iso_to_mins(iso: &str, day_start_prefix: &str) -> Option<f64> {
    // iso looks like "2026-08-12T14:30:00"
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S") {
        let date_str = dt.date().format("%Y-%m-%d").to_string();
        if date_str.as_str() < day_start_prefix {
            return Some(0.0);
        } else if date_str.as_str() > day_start_prefix {
            return Some(1440.0);
        }
        let time = dt.time();
        return Some(
            time.format("%H").to_string().parse::<f64>().unwrap_or(0.0).mul_add(60.0, time.format("%M").to_string().parse::<f64>().unwrap_or(0.0)),
        );
    }
    // Try without seconds
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M") {
        let date_str = dt.date().format("%Y-%m-%d").to_string();
        if date_str.as_str() < day_start_prefix {
            return Some(0.0);
        } else if date_str.as_str() > day_start_prefix {
            return Some(1440.0);
        }
        let time = dt.time();
        return Some(
            time.format("%H").to_string().parse::<f64>().unwrap_or(0.0).mul_add(60.0, time.format("%M").to_string().parse::<f64>().unwrap_or(0.0)),
        );
    }
    None
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

/// # Errors
///
/// Returns an error if the operation fails.
#[tauri::command]
pub async fn get_plan_layout(
    app: tauri::AppHandle,
    dates: Vec<String>,
    filters: Option<Vec<AppEventFilter>>,
    query: Option<String>,
) -> Result<Vec<PlanDayLayout>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    let all_events = if filters.is_some() || query.is_some() {
        db::get_filtered_events(&pool, filters.unwrap_or_default(), query.unwrap_or_default()).await.map_err(|e| e.to_string())?
    } else {
        db::get_events(&pool).await.map_err(|e| e.to_string())?
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
) -> Result<Vec<AppTask>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    
    db::get_filtered_tasks(&pool, filters, sort, query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compute_filter_defaults(filters: Vec<AppTaskFilter>) -> Result<crate::domain::AppTaskDefaults, String> {
    Ok(crate::domain::compute_filter_defaults(filters))
}

#[tauri::command]
pub async fn get_filtered_events(
    app: tauri::AppHandle,
    filters: Vec<AppEventFilter>,
    query: String,
) -> Result<Vec<AppEvent>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    
    db::get_filtered_events(&pool, filters, query)
        .await
        .map_err(|e| e.to_string())
}
