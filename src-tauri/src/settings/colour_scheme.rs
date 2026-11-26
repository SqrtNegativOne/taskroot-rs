//! Bundled application colour schemes; wire IDs also select frontend CSS palettes.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../src/lib/bindings/ColourScheme.generated.ts")]
pub enum ColourScheme {
    #[default]
    Slate,
    Ember,
}
