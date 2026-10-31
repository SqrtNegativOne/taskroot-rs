use serde::{Deserialize, Serialize};

use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/FilterType.generated.ts")]
pub enum FilterType {
    Text,
    Number,
    Enum(Vec<String>),
    Relation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/SortDirection.generated.ts"
)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(
    export,
    export_to = "../../src/lib/bindings/FilterOperator.generated.ts"
)]
pub enum FilterOperator {
    #[serde(rename = "is")]
    Is,
    #[serde(rename = "is not")]
    IsNot,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "does not contain")]
    DoesNotContain,
}
