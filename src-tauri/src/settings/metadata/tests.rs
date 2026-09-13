#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The control kinds `SettingRow`/`schema.ts` know how to render. `custom` is
/// for the button-only pseudo-settings, `action` for frontend-only controls;
/// anything else produces no control.
const KNOWN_SETTING_KINDS: &[&str] = &[
    "select",
    "checkbox",
    "number",
    "time",
    "keybinding",
    "custom",
    "action",
];

fn schema_setting_ids() -> Vec<String> {
    let schema = settings_schema_value();
    let mut ids = Vec::new();

    for tab in schema["tabs"].as_array().expect("tabs") {
        for section in tab["sections"].as_array().expect("sections") {
            for setting in section["settings"].as_array().expect("settings") {
                ids.push(setting["id"].as_str().expect("id").to_string());
            }
        }
    }

    ids
}

fn is_select_of(meta: &SettingMeta, predicate: impl Fn(&Value) -> bool) -> bool {
    meta.kind == "select"
        && meta
            .options
            .as_ref()
            .is_some_and(|options| !options.is_empty() && options.iter().all(|o| predicate(&o.value)))
}

#[test]
fn setting_metadata_covers_every_appsettings_field_exactly_once() {
    let metadata = AppSettings::setting_metadata();
    let ids: Vec<String> = metadata.iter().map(|meta| meta.id.clone()).collect();
    let unique: BTreeSet<String> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), ids.len(), "duplicate metadata id");

    let defaults = serde_json::to_value(AppSettings::default()).expect("defaults");
    let expected: BTreeSet<String> = defaults
        .as_object()
        .expect("defaults object")
        .keys()
        .cloned()
        .collect();

    assert_eq!(unique, expected);

    for meta in &metadata {
        assert_eq!(
            meta.default_value.as_ref(),
            defaults.get(&meta.id),
            "{}",
            meta.id
        );
    }
}

#[test]
fn schema_places_every_setting_exactly_once() {
    let ids = schema_setting_ids();
    let unique: BTreeSet<String> = ids.iter().cloned().collect();
    assert_eq!(unique.len(), ids.len(), "duplicate setting id in schema");

    let mut expected: BTreeSet<String> = AppSettings::setting_metadata()
        .iter()
        .map(|meta| meta.id.clone())
        .collect();
    expected.insert("logout".to_string());
    expected.insert("clear_all_data".to_string());

    assert_eq!(unique, expected);
}

#[test]
fn every_setting_kind_is_known() {
    let kinds: Vec<String> = AppSettings::setting_metadata()
        .into_iter()
        .map(|meta| meta.kind)
        .chain(CUSTOM_SETTINGS.iter().map(|_| "custom".to_string()))
        .collect();

    for kind in kinds {
        assert!(
            KNOWN_SETTING_KINDS.contains(&kind.as_str()),
            "unknown setting kind '{kind}'; expected one of {KNOWN_SETTING_KINDS:?}"
        );
    }
}

#[test]
fn every_setting_section_is_registered() {
    let registered: BTreeSet<&str> = SETTING_SECTIONS.iter().map(|section| section.key).collect();

    for meta in AppSettings::setting_metadata() {
        assert!(
            registered.contains(meta.section),
            "setting '{}' references unknown section '{}'; add it to SETTING_SECTIONS",
            meta.id,
            meta.section
        );
    }
    for custom in CUSTOM_SETTINGS {
        assert!(
            registered.contains(custom.section),
            "custom setting '{}' references unknown section '{}'; add it to SETTING_SECTIONS",
            custom.id,
            custom.section
        );
    }
}

/// The Settings screen reads the command's JSON directly, so the serialized
/// keys are part of the contract: `kind` must be `type`, `default_value` must be
/// `defaultValue`, and a select must carry renderable options.
#[test]
fn schema_rows_expose_the_frontend_wire_contract() {
    let schema = settings_schema_value();

    for tab in schema["tabs"].as_array().expect("tabs") {
        for section in tab["sections"].as_array().expect("sections") {
            for setting in section["settings"].as_array().expect("settings") {
                let id = setting["id"].as_str().expect("id");
                assert!(setting["type"].is_string(), "{id}: missing `type`");
                assert!(setting["label"].is_string(), "{id}: missing `label`");
                assert!(setting["keywords"].is_array(), "{id}: missing `keywords`");
                assert!(
                    setting.get("kind").is_none(),
                    "{id}: `kind` leaked instead of `type`"
                );

                let kind = setting["type"].as_str().expect("type");
                if kind == "custom" {
                    assert!(
                        setting.get("defaultValue").is_none(),
                        "{id}: custom setting must not carry a default"
                    );
                    continue;
                }

                assert!(
                    setting.get("defaultValue").is_some(),
                    "{id}: missing `defaultValue`"
                );

                if kind == "select" {
                    let options = setting["options"].as_array().expect("select options");
                    assert!(!options.is_empty(), "{id}: select without options");
                    for option in options {
                        assert!(option.get("value").is_some(), "{id}: option missing value");
                        assert!(option["label"].is_string(), "{id}: option missing label");
                    }
                }
            }
        }
    }
}

/// A setting whose control kind disagrees with its default's type would render
/// a control the backend rejects on save, so the two must stay consistent.
#[test]
fn setting_kind_matches_the_default_value_type() {
    for meta in AppSettings::setting_metadata() {
        let default = meta.default_value.as_ref().expect("default value");
        let kind = meta.kind.as_str();

        match default {
            Value::Bool(_) => assert_eq!(kind, "checkbox", "{}", meta.id),
            Value::Number(_) => assert!(
                matches!(kind, "number" | "time") || is_select_of(&meta, Value::is_number),
                "{}: number setting declared as `{kind}`",
                meta.id
            ),
            Value::String(_) => assert!(
                kind == "keybinding" || is_select_of(&meta, Value::is_string),
                "{}: string setting declared as `{kind}`",
                meta.id
            ),
            other => panic!("{}: unexpected default {other}", meta.id),
        }
    }
}

#[test]
fn numeric_select_options_keep_numeric_values() {
    let metadata = AppSettings::setting_metadata();
    let duration = metadata
        .iter()
        .find(|meta| meta.id == "default_task_duration")
        .expect("default_task_duration metadata");
    let options = duration.options.as_ref().expect("options");

    assert_eq!(
        options.first().map(|option| &option.value),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        options.get(1).map(|option| &option.value),
        Some(&serde_json::json!(15))
    );
}

/// One line per setting: `id | label | keyword,keyword | value=Label | ...`.
fn metadata_strings() -> String {
    let mut out = String::new();
    for meta in AppSettings::setting_metadata() {
        let _ = write!(out, "{} | {} | {}", meta.id, meta.label, meta.keywords.join(","));
        for option in meta.options.as_deref().unwrap_or_default() {
            let _ = write!(out, " | {}={}", option.value, option.label);
        }
        out.push('\n');
    }
    out
}

/// P2 replaced a hand-written `json!` schema with derived metadata; these strings
/// are exactly what the settings screen renders. Pin them so a changed label,
/// keyword or select option is a conscious edit, not silent drift.
#[test]
fn user_visible_setting_strings_are_pinned() {
    let expected = "\
default_calendar_view | Default View | calendar,view,month,week | \"month\"=Month | \"week\"=Week
day_timeline_start_view | Timeline View Start Time | timeline,day,start,time,scroll,view
default_task_duration | Default Duration | task,duration,estimate,time | 0=Not set | 15=15m | 30=30m | 45=45m
clock_style | Clock Style | stopwatch,timer,guzey,counter,flowtime | \"counter\"=Counter | \"flowtime\"=Flowtime | \"guzey\"=Guzey
allow_stopwatch_without_task | Allow stopwatch use without selecting task | stopwatch,task,requirement,allow
flowtime_break_divisor | Flowtime Break Divisor | flowtime,break,divisor,rest
enable_calendar_sync | Enable Bidirectional Google Calendar Sync | google,calendar,sync,events
enable_tasks_sync | Enable Bidirectional Google Tasks Sync | google,tasks,sync,todos
sync_interval | Sync Interval (minutes) | sync,interval,poll,time
keybinding_launcher | Open Launcher | keyboard,shortcut,launcher,open
keybinding_open_settings | Open Settings | keyboard,shortcut,settings,open
keybinding_restore_app | Restore App | keyboard,shortcut,restore,maximize,mini tracker,minitracker
tracker_show_border | Show Window Border | tracker,border,show,outline
tracker_opacity | Base Opacity (%) | tracker,opacity,transparent,window
";

    assert_eq!(metadata_strings(), expected);
}
