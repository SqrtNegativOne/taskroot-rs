use serde::{Deserialize, Serialize};
use specta::Type;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOperator {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
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

// `specta::Type` expands field types inside a nested item where `Self` is out of scope.
#[allow(clippy::use_self)]
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FilterNode<F> {
    Condition {
        filter: F,
    },
    Group {
        operator: LogicalOperator,
        children: Vec<FilterNode<F>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub enum FilterType {
    Text,
    Number,
    Date,
    Enum(Vec<String>),
    Relation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
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
