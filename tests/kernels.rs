//! End-to-end checks through the public library API.

use rusty_cli::compute::{
    count_primes, count_primes_parallel, is_prime, sum_of_squares, sum_of_squares_closed_form,
    sum_of_squares_parallel,
};
use rusty_cli::{ComputeError, Config};

#[test]
fn sum_of_squares_agrees_with_its_closed_form_everywhere_it_fits() {
    for n in (0..20_000).step_by(937) {
        assert_eq!(
            sum_of_squares(n).unwrap(),
            sum_of_squares_closed_form(n).unwrap(),
            "disagreement at n = {n}"
        );
    }
}

#[test]
fn the_parallel_kernel_never_disagrees_with_the_sequential_one() {
    for n in (0..30_000).step_by(1_111) {
        assert_eq!(
            sum_of_squares(n).unwrap(),
            sum_of_squares_parallel(n).unwrap(),
            "disagreement at n = {n}"
        );
    }
}

#[test]
fn results_past_u32_are_computed_rather_than_wrapped() {
    // Each of these totals is past u32::MAX and must still be exact.
    for n in [3_000u32, 10_000, 50_000, 100_000] {
        let total = sum_of_squares(n).expect("should compute");
        assert!(
            total > u32::MAX as u64,
            "n = {n} gave {total}, which still fits in u32"
        );
        assert_eq!(total, sum_of_squares_closed_form(n).unwrap());
    }
}

#[test]
fn overflow_is_an_error_in_both_kernels() {
    assert!(matches!(
        sum_of_squares(u32::MAX),
        Err(ComputeError::Overflow { .. })
    ));
    assert!(matches!(
        sum_of_squares_parallel(u32::MAX),
        Err(ComputeError::Overflow { .. })
    ));
}

#[test]
fn prime_counts_match_known_values() {
    // pi(x) for powers of ten.
    assert_eq!(count_primes(10), 4);
    assert_eq!(count_primes(100), 25);
    assert_eq!(count_primes(1_000), 168);
    assert_eq!(count_primes(10_000), 1_229);
    assert_eq!(count_primes(100_000), 9_592);
}

#[test]
fn the_parallel_prime_count_matches_the_sequential_one() {
    for limit in [0, 1, 2, 3, 100, 5_000, 100_000] {
        assert_eq!(
            count_primes(limit),
            count_primes_parallel(limit),
            "disagreement below {limit}"
        );
    }
}

#[test]
fn primality_holds_around_tricky_boundaries() {
    assert!(!is_prime(0));
    assert!(!is_prime(1));
    assert!(is_prime(2));
    assert!(is_prime(3));
    assert!(!is_prime(4));
    // Perfect squares of primes are the classic off-by-one in a sqrt bound.
    for p in [3u64, 5, 7, 11, 13, 101, 1009] {
        assert!(!is_prime(p * p), "{p} squared should not be prime");
    }
}

#[test]
fn a_config_file_survives_a_write_and_read() {
    let dir = std::env::temp_dir().join("rusty-cli-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");

    let original = Config::example();
    std::fs::write(&path, original.to_json()).unwrap();

    let loaded = Config::from_path(&path).unwrap();
    assert_eq!(loaded, original);

    std::fs::remove_file(&path).ok();
}

#[test]
fn config_precedence_puts_flags_above_the_file() {
    let from_flags = Config {
        iterations: Some(7),
        ..Default::default()
    };
    let from_file = Config::from_json(r#"{"iterations": 999, "name": "file"}"#).unwrap();

    let merged = from_flags.or(from_file);
    assert_eq!(merged.iterations, Some(7));
    assert_eq!(merged.name.as_deref(), Some("file"));
}
