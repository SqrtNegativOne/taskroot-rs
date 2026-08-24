use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;





#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/AppTaskFilter.generated.ts"
)]
// Rebuild me please
pub struct AppTaskFilter {
    pub column: Option<super::AppTaskFilterColumn>,
    pub operator: Option<String>,
    #[ts(type = "unknown")]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/AppEventFilter.generated.ts"
)]
pub struct AppEventFilter {
    pub column: Option<super::AppEventFilterColumn>,
    pub operator: Option<String>,
    #[ts(type = "unknown")]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/FilterType.generated.ts"
)]
pub enum FilterType {
    Text,
    Number,
    Enum(Vec<String>),
    Relation(String),
}

