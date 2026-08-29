//! The computation kernels, sequential and parallel.
//!
//! Every function that accumulates a total does so with checked arithmetic on
//! a `u64`. Sum-of-squares grows as roughly `n^3 / 3`, so it leaves `u32` at
//! around 2,300 iterations and `u64` above roughly 3.1 million. Overflow is
//! returned as a value the caller can handle rather than a panic or a silently
//! wrapped answer.

use crate::error::ComputeError;

/// Sums `i * i` for every `i` below `n`.
///
/// The result grows as roughly `n^3 / 3`, so it leaves `u32` almost immediately
/// and can leave `u64` too. Both steps are checked.
///
/// # Example
///
/// ```
/// use rusty_cli::compute::sum_of_squares;
///
/// assert_eq!(sum_of_squares(4).unwrap(), 0 + 1 + 4 + 9);
///
/// // Well past u32::MAX, and still exact.
/// assert_eq!(sum_of_squares(3_000).unwrap(), 8_995_500_500);
/// ```
///
/// # Errors
///
/// Returns [`ComputeError::Overflow`] rather than wrapping or panicking.
pub fn sum_of_squares(n: u32) -> Result<u64, ComputeError> {
    let mut total: u64 = 0;
    for i in 0..n as u64 {
        let square = i.checked_mul(i).ok_or(ComputeError::Overflow {
            operation: "i * i",
            at: i,
        })?;
        total = total.checked_add(square).ok_or(ComputeError::Overflow {
            operation: "running total",
            at: i,
        })?;
    }
    Ok(total)
}

/// The closed form of [`sum_of_squares`], used to verify the loops.
///
/// `sum(i^2) for i in 0..n` is `(n-1)n(2n-1)/6`.
pub fn sum_of_squares_closed_form(n: u32) -> Result<u64, ComputeError> {
    if n == 0 {
        return Ok(0);
    }
    let n = n as u128;
    let total = ((n - 1) * n * (2 * n - 1)) / 6;
    u64::try_from(total).map_err(|_| ComputeError::Overflow {
        operation: "closed form",
        at: n as u64,
    })
}

/// Whether `candidate` is prime, by trial division up to its square root.
pub fn is_prime(candidate: u64) -> bool {
    if candidate < 2 {
        return false;
    }
    if candidate < 4 {
        return true;
    }
    if candidate % 2 == 0 {
        return false;
    }
    // Only odd divisors remain, and only up to sqrt(candidate).
    let mut divisor: u64 = 3;
    while divisor.saturating_mul(divisor) <= candidate {
        if candidate % divisor == 0 {
            return false;
        }
        divisor += 2;
    }
    true
}

/// Counts the primes below `limit`, one at a time.
pub fn count_primes(limit: u64) -> u64 {
    (2..limit).filter(|&n| is_prime(n)).count() as u64
}

/// Counts the primes below `limit` across every available core.
///
/// This is real work rather than a sleep, so the speedup it reports is the
/// machine actually being used.
#[cfg(feature = "parallel")]
pub fn count_primes_parallel(limit: u64) -> u64 {
    use rayon::prelude::*;

    (2..limit).into_par_iter().filter(|&n| is_prime(n)).count() as u64
}

/// Sums `i * i` below `n` across every available core.
///
/// Rayon reduces with the same checked arithmetic as the sequential version, so
/// the two agree on both the answer and on overflow.
#[cfg(feature = "parallel")]
pub fn sum_of_squares_parallel(n: u32) -> Result<u64, ComputeError> {
    use rayon::prelude::*;

    (0..n as u64)
        .into_par_iter()
        .map(|i| {
            i.checked_mul(i)
                .ok_or(ComputeError::Overflow {
                    operation: "i * i",
                    at: i,
                })
                .map(Some)
        })
        // Fold and reduce with checked addition so a partial sum cannot wrap.
        .try_reduce(
            || None::<u64>,
            |a, b| match (a, b) {
                (Some(a), Some(b)) => a
                    .checked_add(b)
                    .ok_or(ComputeError::Overflow {
                        operation: "running total",
                        at: 0,
                    })
                    .map(Some),
                (Some(v), None) | (None, Some(v)) => Ok(Some(v)),
                (None, None) => Ok(None),
            },
        )
        .map(|total| total.unwrap_or(0))
}

/// How many threads Rayon will use.
#[cfg(feature = "parallel")]
pub fn thread_count() -> usize {
    rayon::current_num_threads()
}

#[cfg(not(feature = "parallel"))]
pub fn thread_count() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_small_inputs_by_hand() {
        assert_eq!(sum_of_squares(0).unwrap(), 0);
        assert_eq!(sum_of_squares(1).unwrap(), 0);
        assert_eq!(sum_of_squares(2).unwrap(), 1);
        assert_eq!(sum_of_squares(4).unwrap(), 14);
        assert_eq!(sum_of_squares(11).unwrap(), 385);
    }

    #[test]
    fn matches_the_closed_form() {
        for n in [0, 1, 2, 10, 100, 1_000, 10_000, 100_000] {
            assert_eq!(
                sum_of_squares(n).unwrap(),
                sum_of_squares_closed_form(n).unwrap(),
                "disagreement at n = {n}"
            );
        }
    }

    #[test]
    fn totals_past_u32_are_exact() {
        assert_eq!(sum_of_squares(3_000).unwrap(), 8_995_500_500);
        assert!(sum_of_squares(3_000).unwrap() > u32::MAX as u64);
    }

    #[test]
    fn the_default_iteration_count_gives_the_known_answer() {
        // The documented default must not move.
        assert_eq!(sum_of_squares(1_000).unwrap(), 332_833_500);
    }

    #[test]
    fn u64_overflow_is_reported_not_wrapped() {
        // n^3/3 passes u64 somewhere above 3.1 million.
        let result = sum_of_squares(u32::MAX);
        assert!(
            matches!(result, Err(ComputeError::Overflow { .. })),
            "expected an overflow error, got {result:?}"
        );
    }

    #[test]
    fn identifies_primes() {
        let primes: Vec<u64> = (0..30).filter(|&n| is_prime(n)).collect();
        assert_eq!(primes, [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
    }

    #[test]
    fn rejects_non_primes() {
        for n in [0, 1, 4, 9, 25, 49, 91, 1_000_000] {
            assert!(!is_prime(n), "{n} should not be prime");
        }
    }

    #[test]
    fn handles_large_primes() {
        assert!(is_prime(1_000_003));
        assert!(!is_prime(1_000_005));
    }

    #[test]
    fn counts_primes_below_a_limit() {
        assert_eq!(count_primes(2), 0);
        assert_eq!(count_primes(10), 4);
        assert_eq!(count_primes(100), 25);
        assert_eq!(count_primes(10_000), 1_229);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_and_sequential_agree() {
        for n in [0, 1, 100, 5_000, 50_000] {
            assert_eq!(
                sum_of_squares(n).unwrap(),
                sum_of_squares_parallel(n).unwrap(),
                "disagreement at n = {n}"
            );
        }
        for limit in [10, 1_000, 20_000] {
            assert_eq!(count_primes(limit), count_primes_parallel(limit));
        }
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn parallel_overflow_is_also_reported() {
        assert!(matches!(
            sum_of_squares_parallel(u32::MAX),
            Err(ComputeError::Overflow { .. })
        ));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn reports_at_least_one_thread() {
        assert!(thread_count() >= 1);
    }
}
