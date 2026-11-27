use super::*;

#[test]
fn parses_and_formats_all_three_wire_shapes() {
    let date: CalendarDateTime = "2026-09-07".parse().unwrap();
    assert!(date.is_date());
    assert!(!date.is_timed());
    assert_eq!(date.to_string(), "2026-09-07");

    let wall: CalendarDateTime = "2026-10-19T11:00:00".parse().unwrap();
    assert!(wall.is_timed());
    assert_eq!(wall.to_string(), "2026-10-19T11:00:00");

    let spaced: CalendarDateTime = "2026-10-19 11:00:00".parse().unwrap();
    assert_eq!(spaced, wall);

    let offset: CalendarDateTime = "2026-09-07T09:00:00-07:00".parse().unwrap();
    assert!(offset.is_timed());
    assert_eq!(offset.to_string(), "2026-09-07T16:00:00Z");

    let millis: CalendarDateTime = "2026-09-07T09:00:00.000Z".parse().unwrap();
    assert_eq!(millis.to_string(), "2026-09-07T09:00:00Z");
}

#[test]
fn rejects_empty_and_malformed_values() {
    for raw in ["", "not-a-date", "2026-13-01", "2026-09-07T25:00:00"] {
        assert!(
            raw.parse::<CalendarDateTime>().is_err(),
            "expected `{raw}` to be rejected"
        );
    }
}

#[test]
fn json_round_trips_as_a_plain_string() {
    let value: CalendarDateTime = "2026-10-19T11:00:00".parse().unwrap();
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, r#""2026-10-19T11:00:00""#);
    assert_eq!(
        serde_json::from_str::<CalendarDateTime>(&json).unwrap(),
        value
    );
}

#[test]
fn wall_clock_resolution_respects_timezone_dst_and_gaps() {
    let kolkata: CalendarDateTime = "2026-10-19T11:00:00".parse().unwrap();
    assert_eq!(
        kolkata.to_utc(Some("Asia/Kolkata")).unwrap().to_rfc3339(),
        "2026-10-19T05:30:00+00:00"
    );
    assert_eq!(
        kolkata.to_utc(None).unwrap().to_rfc3339(),
        "2026-10-19T11:00:00+00:00"
    );

    // Europe/London switches to BST on 2026-03-29: 09:00 local moves from 09:00Z to 08:00Z.
    let before: CalendarDateTime = "2026-03-23T09:00:00".parse().unwrap();
    let after: CalendarDateTime = "2026-03-30T09:00:00".parse().unwrap();
    assert_eq!(
        before.to_utc(Some("Europe/London")).unwrap().to_rfc3339(),
        "2026-03-23T09:00:00+00:00"
    );
    assert_eq!(
        after.to_utc(Some("Europe/London")).unwrap().to_rfc3339(),
        "2026-03-30T08:00:00+00:00"
    );

    // 01:30 does not exist in Europe/London on 2026-03-29; degrades to UTC.
    let gap: CalendarDateTime = "2026-03-29T01:30:00".parse().unwrap();
    assert_eq!(
        gap.to_utc(Some("Europe/London")).unwrap().to_rfc3339(),
        "2026-03-29T01:30:00+00:00"
    );
}

#[test]
fn utc_projects_into_target_zone_wall_clock() {
    let instant: CalendarDateTime = "2026-10-26T08:30:00Z".parse().unwrap();
    assert_eq!(
        instant.to_wall_clock(Some("Asia/Kolkata")).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 26)
            .unwrap()
            .and_hms_opt(14, 0, 0)
            .unwrap()
    );
    assert_eq!(
        instant.to_wall_clock(None).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 26)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap()
    );
}

#[test]
fn floating_dates_resolve_with_a_default_time_and_order_before_same_day_times() {
    let date: CalendarDateTime = "2026-10-27".parse().unwrap();
    let timed: CalendarDateTime = "2026-10-27T09:00:00".parse().unwrap();
    let next: CalendarDateTime = "2026-10-28".parse().unwrap();

    assert!(date < timed && timed < next);
    assert_eq!(date.to_utc(Some("Asia/Kolkata")), None);

    let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
    assert_eq!(
        date.to_utc_with_default_time(nine, Some("Asia/Kolkata"))
            .to_rfc3339(),
        "2026-10-27T03:30:00+00:00"
    );
}
