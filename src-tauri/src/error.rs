use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Db(#[from] sqlx::Error),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Auth(String),
    #[error("{0}")]
    Sync(String),
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    NotReady(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Db(_) => "db",
            Self::NotFound(_) => "not-found",
            Self::Auth(_) => "auth",
            Self::Sync(_) => "sync",
            Self::InvalidInput(_) => "invalid-input",
            Self::NotReady(_) => "not-ready",
            Self::Internal(_) => "internal",
        }
    }
}

impl From<color_eyre::eyre::Error> for AppError {
    fn from(err: color_eyre::eyre::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn test_error_serializes_as_code_and_message() {
        let json = serde_json::to_string(&AppError::NotFound("task missing".into())).unwrap();
        assert_eq!(json, r#"{"code":"not-found","message":"task missing"}"#);
    }

    #[test]
    fn test_codes_map_to_kebab_case() {
        assert_eq!(AppError::Db(sqlx::Error::RowNotFound).code(), "db");
        assert_eq!(AppError::Auth("x".into()).code(), "auth");
        assert_eq!(AppError::Sync("x".into()).code(), "sync");
        assert_eq!(AppError::InvalidInput("x".into()).code(), "invalid-input");
        assert_eq!(AppError::NotReady("x".into()).code(), "not-ready");
        assert_eq!(AppError::from(color_eyre::eyre::eyre!("boom")).code(), "internal");
    }

    #[test]
    fn test_db_error_message_is_preserved() {
        let err = AppError::Db(sqlx::Error::RowNotFound);
        assert!(err.to_string().contains("no rows returned"));
    }
}
