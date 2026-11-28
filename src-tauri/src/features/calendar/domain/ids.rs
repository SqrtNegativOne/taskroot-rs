use serde::{Deserialize, Serialize};
use specta::Type;

macro_rules! define_string_id {
    ($($name:ident),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, Type)]
            #[serde(transparent)]
            pub struct $name(pub String);

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str(&self.0)
                }
            }

            impl From<String> for $name {
                fn from(s: String) -> Self {
                    Self(s)
                }
            }

            impl From<&str> for $name {
                fn from(s: &str) -> Self {
                    Self(s.to_string())
                }
            }

            impl std::ops::Deref for $name {
                type Target = String;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+
    };
}

define_string_id!(TaskId, EventId, RemoteId, CollectionId);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, Type)]
#[serde(try_from = "String", into = "String")]
#[specta(transparent)]
pub struct Color(pub String);

impl TryFrom<String> for Color {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.starts_with('#') && (s.len() == 7 || s.len() == 9) {
            Ok(Self(s))
        } else {
            Err(format!("Invalid color format: '{s}'"))
        }
    }
}

impl From<Color> for String {
    fn from(c: Color) -> Self {
        c.0
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for Color {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
