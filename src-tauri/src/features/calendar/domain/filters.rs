use serde::{Deserialize, Serialize};

use ts_rs::TS;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[ts(
    export,
    export_to = "../../src/lib/bindings/LogicalOperator.generated.ts"
)]
pub enum LogicalOperator {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/FilterGroup.generated.ts")]
pub struct FilterGroup<F> {
    pub operator: LogicalOperator,
    pub children: Vec<FilterNode<F>>,
}

impl<F> Default for FilterGroup<F> {
    fn default() -> Self {
        Self {
            operator: LogicalOperator::And,
            children: Vec::new(),
        }
    }
}

impl<F> From<Vec<F>> for FilterGroup<F> {
    fn from(filters: Vec<F>) -> Self {
        Self {
            operator: LogicalOperator::And,
            children: filters
                .into_iter()
                .map(|filter| FilterNode::Condition { filter })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[ts(export, export_to = "../../src/lib/bindings/FilterNode.generated.ts")]
pub enum FilterNode<F> {
    Condition {
        filter: F,
    },
    Group {
        operator: LogicalOperator,
        children: Vec<Self>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/lib/bindings/FilterType.generated.ts")]
pub enum FilterType {
    Text,
    Number,
    Date,
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
    #[serde(rename = "is empty")]
    IsEmpty,
    #[serde(rename = "is not empty")]
    IsNotEmpty,
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "does not contain")]
    DoesNotContain,
    #[serde(rename = "before")]
    Before,
    #[serde(rename = "after")]
    After,
    #[serde(rename = "on or before")]
    OnOrBefore,
    #[serde(rename = "on or after")]
    OnOrAfter,
}
