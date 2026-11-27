use serde::{Deserialize, Serialize};
use specta::Type;

macro_rules! impl_string_newtype_deref {
    ($($name:ident),+ $(,)?) => {
        $(
            impl std::ops::Deref for $name {
                type Target = String;
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, Type)]
#[serde(transparent)]
pub struct TaskId(pub String);
impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<String> for TaskId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for TaskId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, Type)]
#[serde(transparent)]
pub struct EventId(pub String);
impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<String> for EventId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for EventId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, Type)]
#[serde(transparent)]
pub struct RemoteId(pub String);
impl std::fmt::Display for RemoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<String> for RemoteId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for RemoteId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, std::hash::Hash, Type)]
#[serde(transparent)]
pub struct CollectionId(pub String);
impl std::fmt::Display for CollectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl From<String> for CollectionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl From<&str> for CollectionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

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
        write!(f, "{}", self.0)
    }
}

impl_string_newtype_deref!(TaskId, EventId, RemoteId, CollectionId, Color);
