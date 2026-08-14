use crate::db;
use crate::domain::{AppEvent, AppTask};
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

#[derive(Serialize, Deserialize)]
pub struct AppFilter {
    pub column: Option<String>,
    pub operator: Option<String>,
    pub value: Option<serde_json::Value>,
}

fn parse_iso_to_mins(iso: &str, day_start_prefix: &str) -> Option<f64> {
    // iso looks like "2026-08-12T14:30:00"
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M:%S") {
        let date_str = dt.date().format("%Y-%m-%d").to_string();
        if date_str < day_start_prefix.to_string() {
            return Some(0.0);
        } else if date_str > day_start_prefix.to_string() {
            return Some(1440.0);
        }
        let time = dt.time();
        return Some(
            (time.format("%H").to_string().parse::<f64>().unwrap_or(0.0) * 60.0)
                + time.format("%M").to_string().parse::<f64>().unwrap_or(0.0),
        );
    }
    // Try without seconds
    if let Ok(dt) = NaiveDateTime::parse_from_str(iso, "%Y-%m-%dT%H:%M") {
        let date_str = dt.date().format("%Y-%m-%d").to_string();
        if date_str < day_start_prefix.to_string() {
            return Some(0.0);
        } else if date_str > day_start_prefix.to_string() {
            return Some(1440.0);
        }
        let time = dt.time();
        return Some(
            (time.format("%H").to_string().parse::<f64>().unwrap_or(0.0) * 60.0)
                + time.format("%M").to_string().parse::<f64>().unwrap_or(0.0),
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
            if end_iso < &format!("{}T00:00:00", date_str)
                || start_iso > &format!("{}T23:59:59", date_str)
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

    for ev in mapped.iter_mut() {
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

    for ev in mapped.iter_mut() {
        let mut max_lane = ev.lane;
        for p in &placed {
            if !(p.1 <= ev.start_mins || p.0 >= ev.end_mins) {
                if p.2 > max_lane {
                    max_lane = p.2;
                }
            }
        }
        ev.lanes = max_lane + 1;
    }

    mapped
}

#[tauri::command]
pub async fn get_plan_layout(
    app: tauri::AppHandle,
    dates: Vec<String>,
) -> Result<Vec<PlanDayLayout>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    let all_events = db::get_events(&pool).await.map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn get_filtered_tasks(
    app: tauri::AppHandle,
    filters: Vec<AppFilter>,
    sort: String,
    query: String,
) -> Result<Vec<AppTask>, String> {
    let pool = app
        .try_state::<sqlx::SqlitePool>()
        .ok_or("Database not initialized yet")?;
    let mut tasks = db::get_tasks(&pool).await.map_err(|e| e.to_string())?;

    // Filtering logic (in memory for now, fast enough for 10,000 items in rust)
    if !filters.is_empty() {
        tasks.retain(|t| {
            // we port checkTaskAgainstFilters to Rust
            for f in &filters {
                if let (Some(col), Some(val)) = (&f.column, &f.value) {
                    let op = f.operator.as_deref().unwrap_or("is");

                    let mut match_found = false;
                    let values = if let Some(arr) = val.as_array() {
                        arr.clone()
                    } else {
                        vec![val.clone()]
                    };
                    if values.is_empty() {
                        continue;
                    }

                    if col == "status" {
                        let status = match t.status {
                            Some(crate::domain::AppTaskStatus::Todo) => "todo",
                            Some(crate::domain::AppTaskStatus::NextUp) => "next-up",
                            Some(crate::domain::AppTaskStatus::Doing) => "doing",
                            Some(crate::domain::AppTaskStatus::Done) => "done",
                            None => "",
                        };
                        match_found = values.iter().any(|v| v.as_str().unwrap_or("") == status);
                    } else if col == "priority" {
                        let prio = t.priority.unwrap_or(0);
                        match_found = values.iter().any(|v| {
                            if let Some(n) = v.as_i64() {
                                n as i32 == prio
                            } else if let Some(s) = v.as_str() {
                                s.parse::<i32>().unwrap_or(-1) == prio
                            } else {
                                false
                            }
                        });
                    } else if col == "tag" {
                        let tags = t.tags.as_ref().unwrap_or(&vec![]).clone();
                        match_found = values.iter().any(|v| {
                            let vs = v.as_str().unwrap_or("");
                            tags.iter().any(|t| t == vs)
                        });
                    }

                    let is_keep = if op == "is not" {
                        !match_found
                    } else {
                        match_found
                    };
                    if !is_keep {
                        return false;
                    }
                }
            }
            true
        });
    }

    // Sort logic
    tasks.sort_by(|a, b| {
        if sort == "priority" {
            let pa = a.priority.unwrap_or(0);
            let pb = b.priority.unwrap_or(0);
            pb.cmp(&pa)
        } else if sort == "due" {
            let da = a.due.as_deref().unwrap_or("9999");
            let db = b.due.as_deref().unwrap_or("9999");
            da.cmp(db)
        } else if sort == "title" {
            a.title.cmp(&b.title)
        } else {
            std::cmp::Ordering::Equal
        }
    });

    // Query search logic
    if !query.trim().is_empty() {
        let q = query.to_lowercase();
        tasks.retain(|t| {
            t.title.to_lowercase().contains(&q)
                || t.tags.as_ref().map_or(false, |tags| {
                    tags.iter().any(|tag| tag.to_lowercase().contains(&q))
                })
        });
    }

    Ok(tasks)
}
