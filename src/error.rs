//! Error types.

use thiserror::Error;

/// A computation that could not produce a correct answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ComputeError {
    /// The result left the range of the accumulator.
    ///
    /// Returned rather than panicking or wrapping, so a caller can lower the
    /// input or widen the type instead of getting a wrong number.
    #[error("{operation} overflowed a u64 at i = {at}; try a smaller input")]
    Overflow { operation: &'static str, at: u64 },
}

/// A config file that could not be loaded.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("invalid config at line {line}, column {column}: {message}")]
    Parse {
        line: usize,
        column: usize,
        message: String,
    },
}
