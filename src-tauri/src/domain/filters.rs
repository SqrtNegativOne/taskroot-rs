use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(serde::Serialize, Default, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/AppTaskDefaults.generated.ts"
)]
pub struct AppTaskDefaults {
    pub status: Option<super::AppTaskStatus>,
    pub priority: Option<super::TaskPriority>,
    pub tags: Option<Vec<String>>,
}

pub fn compute_filter_defaults(filters: Vec<AppTaskFilter>) -> AppTaskDefaults {
    let mut defaults = AppTaskDefaults::default();

    for f in filters {
        if f.operator.as_deref().unwrap_or("is") == "is" {
            if let (Some(col), Some(val)) = (f.column, f.value) {
                let values = val
                    .as_array()
                    .map_or_else(|| vec![val.clone()], std::clone::Clone::clone);
                if values.len() == 1 {
                    apply_single_value(&mut defaults, &col, &values[0]);
                }
            }
        }
    }

    defaults
}

fn apply_single_value(defaults: &mut AppTaskDefaults, col: &TaskFilterColumn, v: &Value) {
    match col {
        TaskFilterColumn::Status => {
            if let Ok(status) = serde_json::from_value(v.clone()) {
                defaults.status = Some(status);
            }
        }
        TaskFilterColumn::Priority => {
            if let Ok(priority) = serde_json::from_value(v.clone()) {
                defaults.priority = Some(priority);
            } else if let Some(n) = v.as_i64() {
                if let Ok(priority) = serde_json::from_value(serde_json::json!(n)) {
                    defaults.priority = Some(priority);
                }
            }
        }
        TaskFilterColumn::Tag => {
            if let Some(s) = v.as_str() {
                defaults.tags = Some(vec![s.to_string()]);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/TaskFilterColumn.generated.ts"
)]
#[serde(rename_all = "camelCase")]
pub enum TaskFilterColumn {
    Status,
    Priority,
    Tag,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/AppTaskFilter.generated.ts"
)]
pub struct AppTaskFilter {
    pub column: Option<TaskFilterColumn>,
    pub operator: Option<String>,
    #[ts(type = "unknown")]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/EventFilterColumn.generated.ts"
)]
#[serde(rename_all = "camelCase")]
pub enum EventFilterColumn {
    Calendar,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../../src/lib/bindings/AppEventFilter.generated.ts"
)]
pub struct AppEventFilter {
    pub column: Option<EventFilterColumn>,
    pub operator: Option<String>,
    #[ts(type = "unknown")]
    pub value: Option<Value>,
}
