//! `--key value` argument parsing for the headless CLI.

use std::collections::BTreeMap;

use crate::error::AppError;

/// `--key value` pairs plus bare positionals.
///
/// A `--key` with no following value (or a following `--other`) is stored as
/// `"true"`, which is how boolean flags are expressed.
pub struct Cli {
    positionals: Vec<String>,
    options: BTreeMap<String, String>,
}

impl Cli {
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(std::env::args().skip(1))
    }

    #[must_use]
    pub fn parse(tokens: impl IntoIterator<Item = String>) -> Self {
        let mut positionals = Vec::new();
        let mut options = BTreeMap::new();
        let mut tokens = tokens.into_iter().peekable();

        while let Some(token) = tokens.next() {
            let Some(key) = token.strip_prefix("--") else {
                positionals.push(token);
                continue;
            };
            let value = match tokens.peek() {
                Some(next) if !next.starts_with("--") => tokens.next().unwrap_or_default(),
                _ => "true".to_string(),
            };
            options.insert(key.to_string(), value);
        }

        Self {
            positionals,
            options,
        }
    }

    #[must_use]
    pub fn positional(&self, index: usize) -> Option<&str> {
        self.positionals.get(index).map(String::as_str)
    }

    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] when the positional is absent.
    pub fn require_positional(&self, index: usize, name: &str) -> Result<&str, AppError> {
        self.positional(index)
            .ok_or_else(|| AppError::InvalidInput(format!("missing argument {name}")))
    }

    #[must_use]
    pub fn option(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(String::as_str)
    }

    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] when the option is absent.
    pub fn require_option(&self, key: &str) -> Result<&str, AppError> {
        self.option(key)
            .ok_or_else(|| AppError::InvalidInput(format!("missing --{key} <value>")))
    }

    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }

    /// Every positional after the verb, so one invocation can act on a whole set.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::InvalidInput`] when there is no id.
    pub fn require_all_positionals(&self, after: usize, name: &str) -> Result<Vec<&str>, AppError> {
        let ids: Vec<&str> = self
            .positionals
            .iter()
            .skip(after)
            .map(String::as_str)
            .collect();
        if ids.is_empty() {
            return Err(AppError::InvalidInput(format!("missing argument {name}")));
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(tokens: &[&str]) -> Cli {
        Cli::parse(tokens.iter().map(|token| (*token).to_string()))
    }

    #[test]
    fn positionals_and_options_are_separated() {
        let parsed = cli(&["task-add", "--title", "Ship the CLI", "--due", "2026-10-28"]);

        assert_eq!(parsed.positional(0), Some("task-add"));
        assert_eq!(parsed.option("title"), Some("Ship the CLI"));
        assert_eq!(parsed.option("due"), Some("2026-10-28"));
    }

    #[test]
    fn a_boolean_flag_does_not_swallow_the_next_flag() {
        let parsed = cli(&["task-set", "a2a8", "--clear-due", "--priority", "high"]);

        assert!(parsed.has("clear-due"));
        assert_eq!(parsed.option("priority"), Some("high"));
        assert_eq!(parsed.positional(1), Some("a2a8"));
    }

    #[test]
    fn several_ids_are_read_from_one_invocation() {
        let parsed = cli(&["task-set", "a2a8", "b7c1", "c9d0", "--priority", "high"]);

        assert_eq!(
            parsed.require_all_positionals(1, "<id>").unwrap(),
            vec!["a2a8", "b7c1", "c9d0"]
        );
        assert!(cli(&["task-set", "--priority", "high"])
            .require_all_positionals(1, "<id>")
            .is_err());
    }
}
