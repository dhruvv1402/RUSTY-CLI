//! Configuration loaded from a JSON file.
//!
//! `--config <FILE>` loads this structure, and the values it supplies become
//! the defaults for the commands that use them. Precedence is explicit and
//! tested: an argument given on the command line beats the config file, which
//! beats the built-in default.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// Settings that a config file may override.
///
/// Every field is optional so a file can set only what it cares about.
/// Precedence is: an explicit command-line flag, then the config file, then the
/// built-in default.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// Who to greet on startup. Supplies the default for `--name`.
    pub name: Option<String>,

    /// Default item count for `process`.
    pub count: Option<u32>,

    /// Default iteration count for `compute`.
    pub iterations: Option<u32>,

    /// Default upper bound for `primes`.
    pub limit: Option<u64>,

    /// Milliseconds of simulated work per item in `process`.
    pub delay_ms: Option<u64>,

    /// Suppress colour and progress bars.
    pub quiet: Option<bool>,
}

impl Config {
    /// Reads and parses a config file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Read`] if the file cannot be opened and
    /// [`ConfigError::Parse`] if it is not valid config JSON. An unknown key is
    /// a parse error rather than being ignored, so a typo is reported instead
    /// of silently doing nothing.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_json(&raw)
    }

    /// Parses config from a JSON string.
    ///
    /// ```
    /// use rusty_cli::Config;
    ///
    /// let config = Config::from_json(r#"{"name": "Dhruv", "iterations": 5000}"#)?;
    /// assert_eq!(config.name.as_deref(), Some("Dhruv"));
    /// assert_eq!(config.iterations, Some(5000));
    /// # Ok::<(), rusty_cli::ConfigError>(())
    /// ```
    pub fn from_json(raw: &str) -> Result<Self, ConfigError> {
        // Notepad and PowerShell redirection both write a UTF-8 BOM by default
        // on Windows, and serde_json rejects it as an unexpected character.
        // Stripping it turns a baffling "expected value at line 1 column 1"
        // into a file that simply works.
        let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);

        serde_json::from_str(raw).map_err(|source| ConfigError::Parse {
            line: source.line(),
            column: source.column(),
            message: source.to_string(),
        })
    }

    /// Renders the config as pretty JSON, for `rusty-cli config --example`.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Config always serializes")
    }

    /// A fully populated example, used to generate a starter file.
    pub fn example() -> Self {
        Self {
            name: Some("Dhruv".into()),
            count: Some(20),
            iterations: Some(100_000),
            limit: Some(500_000),
            delay_ms: Some(50),
            quiet: Some(false),
        }
    }

    /// Layers `other` underneath `self`: values already set here win.
    ///
    /// This is what implements flag-over-file precedence.
    pub fn or(self, other: Config) -> Config {
        Config {
            name: self.name.or(other.name),
            count: self.count.or(other.count),
            iterations: self.iterations.or(other.iterations),
            limit: self.limit.or(other.limit),
            delay_ms: self.delay_ms.or(other.delay_ms),
            quiet: self.quiet.or(other.quiet),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_object_is_valid_and_sets_nothing() {
        let config = Config::from_json("{}").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn reads_a_partial_config() {
        let config = Config::from_json(r#"{"name": "Dhruv"}"#).unwrap();
        assert_eq!(config.name.as_deref(), Some("Dhruv"));
        assert_eq!(config.iterations, None);
    }

    #[test]
    fn reads_every_field() {
        let config = Config::from_json(
            r#"{
                "name": "Ada",
                "count": 5,
                "iterations": 100,
                "limit": 999,
                "delay_ms": 10,
                "quiet": true
            }"#,
        )
        .unwrap();

        assert_eq!(config.name.as_deref(), Some("Ada"));
        assert_eq!(config.count, Some(5));
        assert_eq!(config.iterations, Some(100));
        assert_eq!(config.limit, Some(999));
        assert_eq!(config.delay_ms, Some(10));
        assert_eq!(config.quiet, Some(true));
    }

    #[test]
    fn tolerates_a_utf8_bom() {
        // Notepad and `Out-File -Encoding utf8` both produce this.
        let with_bom = "\u{feff}{\"name\": \"Dhruv\"}";
        assert_eq!(
            Config::from_json(with_bom).unwrap().name.as_deref(),
            Some("Dhruv")
        );
    }

    #[test]
    fn a_typo_in_a_key_is_an_error_not_a_silent_no_op() {
        let result = Config::from_json(r#"{"iteration": 100}"#);
        assert!(
            matches!(result, Err(ConfigError::Parse { .. })),
            "unknown keys must be rejected, got {result:?}"
        );
    }

    #[test]
    fn malformed_json_reports_where() {
        match Config::from_json("{\n  \"name\": \n}") {
            Err(ConfigError::Parse { line, .. }) => assert!(line >= 1),
            other => panic!("expected a parse error, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_file_reports_the_path() {
        match Config::from_path("definitely/not/here.json") {
            Err(ConfigError::Read { path, .. }) => assert!(path.contains("here.json")),
            other => panic!("expected a read error, got {other:?}"),
        }
    }

    #[test]
    fn the_example_round_trips() {
        let example = Config::example();
        assert_eq!(Config::from_json(&example.to_json()).unwrap(), example);
    }

    #[test]
    fn explicit_values_win_over_the_layer_below() {
        let flags = Config {
            iterations: Some(1),
            ..Default::default()
        };
        let file = Config {
            name: Some("from file".into()),
            iterations: Some(999),
            ..Default::default()
        };

        let merged = flags.or(file);
        assert_eq!(merged.iterations, Some(1), "the flag should win");
        assert_eq!(
            merged.name.as_deref(),
            Some("from file"),
            "the file fills in what the flag left unset"
        );
    }
}
