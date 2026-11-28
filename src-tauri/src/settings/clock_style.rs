//! Stopwatch clock styles; wire IDs select flowtime vs plain counter behaviour.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ClockStyle {
    #[default]
    Flowtime,
    Counter,
}
