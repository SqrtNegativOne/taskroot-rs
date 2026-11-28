use super::*;
use std::collections::BTreeSet;

fn schema_setting_ids() -> Vec<String> {
    settings_schema()
        .tabs
        .into_iter()
        .flat_map(|tab| tab.sections)
        .flat_map(|section| section.settings)
        .map(|setting| setting.id)
        .collect()
}

fn is_select_of(meta: &SettingMeta, predicate: impl Fn(&Value) -> bool) -> bool {
    meta.kind == SettingKind::Select
        && meta.options.as_ref().is_some_and(|options| {
            !options.is_empty() && options.iter().all(|o| predicate(&o.value))
        })
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

    let expected: BTreeSet<String> = AppSettings::setting_metadata()
        .iter()
        .map(|meta| meta.id.clone())
        .collect();

    assert_eq!(unique, expected);
}

#[test]
fn every_setting_section_is_registered() {
    let registered: BTreeSet<&str> = SETTING_SECTIONS.iter().map(|section| section.key).collect();
    let used: BTreeSet<&str> = AppSettings::setting_metadata()
        .into_iter()
        .map(|meta| meta.section)
        .collect();

    for section in &used {
        assert!(
            registered.contains(section),
            "setting references unknown section '{section}'; add it to SETTING_SECTIONS"
        );
    }
    assert_eq!(
        registered, used,
        "every registered section must contain at least one setting"
    );
}

/// The Settings screen consumes `SettingsSchema` over IPC, so the serialized
/// wire shape is part of the contract: `kind` must serialize as `type`,
/// `default_value` as `defaultValue`, `section` must be omitted, and every
/// `select` row must carry renderable options.
#[test]
fn schema_rows_expose_the_frontend_wire_contract() {
    let schema = serde_json::to_value(settings_schema()).expect("serialize settings schema");

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
                assert!(
                    setting.get("section").is_none(),
                    "{id}: internal `section` key leaked onto wire"
                );
                assert!(
                    setting.get("defaultValue").is_some(),
                    "{id}: missing `defaultValue`"
                );

                let kind = setting["type"].as_str().expect("type");
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
        let kind = meta.kind;

        match default {
            Value::Bool(_) => assert_eq!(kind, SettingKind::Checkbox, "{}", meta.id),
            Value::Number(_) => assert!(
                matches!(kind, SettingKind::Number | SettingKind::Time)
                    || is_select_of(&meta, Value::is_number),
                "{}: number setting declared as `{kind:?}`",
                meta.id
            ),
            Value::String(_) => assert!(
                kind == SettingKind::Calendar || is_select_of(&meta, Value::is_string),
                "{}: string setting declared as `{kind:?}`",
                meta.id
            ),
            other => panic!("{}: unexpected default {other}", meta.id),
        }
    }
}
