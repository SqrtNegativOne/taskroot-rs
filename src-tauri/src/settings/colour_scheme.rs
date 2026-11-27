//! Bundled application colour schemes; wire IDs also select frontend CSS palettes.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum ColourScheme {
    #[default]
    Slate,
    Ember,
}
