//! Parallel computation kernels and configuration for the `rusty-cli` tool.
//!
//! The binary is a thin shell over this library, which keeps the interesting
//! parts testable and lets the same code compile to `wasm32-unknown-unknown`
//! for the browser demo on the project site.
//!
//! # Example
//!
//! ```
//! use rusty_cli::compute::{count_primes, sum_of_squares};
//!
//! assert_eq!(sum_of_squares(1_000)?, 332_833_500);
//! assert_eq!(count_primes(100), 25);
//! # Ok::<(), rusty_cli::ComputeError>(())
//! ```
//!
//! # Checked arithmetic
//!
//! Every accumulation is checked. Overflow comes back as a
//! [`ComputeError::Overflow`] the caller can act on, instead of a panic in
//! debug builds and a silently wrong answer in release builds:
//!
//! ```
//! use rusty_cli::compute::sum_of_squares;
//!
//! // Comfortably past u32, so this is a real answer rather than a wrap.
//! assert_eq!(sum_of_squares(3_000)?, 8_995_500_500);
//!
//! // And past u64, which is reported rather than wrapped.
//! assert!(sum_of_squares(u32::MAX).is_err());
//! # Ok::<(), rusty_cli::ComputeError>(())
//! ```
//!
//! # Building for the browser
//!
//! ```text
//! cargo build --lib --no-default-features --target wasm32-unknown-unknown
//! ```
//!
//! `--no-default-features` drops Rayon and the CLI dependencies, leaving the
//! sequential kernels and config parsing.

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod compute;
pub mod config;
pub mod error;

pub use compute::{count_primes, is_prime, sum_of_squares, thread_count};
pub use config::Config;
pub use error::{ComputeError, ConfigError};

#[cfg(feature = "parallel")]
pub use compute::{count_primes_parallel, sum_of_squares_parallel};
