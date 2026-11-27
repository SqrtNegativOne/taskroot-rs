use super::*;
use crate::features::calendar::domain::EventId;
use chrono::Datelike;
use chrono::NaiveDate;

fn event(id: &str, start: &str, end: &str) -> AppEvent {
    AppEvent {
        id: EventId::from(id),
        remote_id: None,
        remote_collection_id: None,
        task_id: None,
        title: "Event".into(),
        description: None,
        priority: None,
        tags: None,
        start_time: start.parse().unwrap(),
        end_time: end.parse().unwrap(),
        rrule: None,
        exdates: None,
        recurring_event_id: None,
        original_start_time: None,
        status: Some(EventStatus::Confirmed),
        updated_at: None,
        added: None,
        color: None,
        etag: None,
        is_all_day: None,
        timezone: None,
    }
}

fn timed_start(instance: &EventInstance) -> String {
    match &instance.timing {
        EventTiming::Timed { start, .. } => start.to_rfc3339(),
        EventTiming::AllDay { .. } => panic!("expected timed instance"),
    }
}

fn all_day_start(instance: &EventInstance) -> String {
    match &instance.timing {
        EventTiming::AllDay { start_date, .. } => start_date.format("%Y-%m-%d").to_string(),
        EventTiming::Timed { .. } => panic!("expected all-day instance"),
    }
}

fn occurrence_starts(instances: &[EventInstance]) -> Vec<String> {
    instances
        .iter()
        .map(|instance| instance.occurrence_start.to_string())
        .collect()
}

fn weekly_kolkata_master() -> AppEvent {
    let mut master = event("evt-1", "2026-10-19T11:00:00", "2026-10-19T12:00:00");
    master.rrule = Some("RRULE:FREQ=WEEKLY".into());
    master.timezone = Some("Asia/Kolkata".into());
    master
}

const KOLKATA_RANGE: (&str, &str) = ("2026-10-19T00:00:00Z", "2026-10-26T23:59:59Z");

mod recurrence;
mod timing;
