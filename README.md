# Rusty CLI

A command-line tool for parallel computation in Rust: checked arithmetic, a
Rayon work pool, async pipelines with progress reporting, and configuration that
actually takes effect.

## Quick start

```bash
cargo build --release
./target/release/rusty-cli --help
```

```console
$ rusty-cli bench --limit 2000000
Counting primes below 2000000 on 8 thread(s)

sequential      0.242s  (148933 primes)
parallel        0.061s  (148933 primes)

Speedup: 3.97x on 8 thread(s)
```

## Commands

| Command | What it does |
| --- | --- |
| `compute` | Sums `i*i` below an iteration count, sequentially or across all cores |
| `primes` | Counts primes below a limit by trial division |
| `bench` | Times both kernels against each other and reports the speedup |
| `process` | Runs an async pipeline with a progress bar |
| `config` | Prints an example config file |

Global options: `--name`, `--config <FILE>`, `--quiet`.

```bash
rusty-cli compute --iterations 100000        # across all cores
rusty-cli compute --iterations 100000 --sequential
rusty-cli primes --limit 500000
rusty-cli process --count 20 --delay-ms 50
rusty-cli bench --limit 2000000
```

## Configuration

`--config` reads a JSON file whose values become the defaults. Generate a
starter file:

```bash
rusty-cli config > rusty.json
```

```json
{
  "name": "Dhruv",
  "count": 20,
  "iterations": 100000,
  "limit": 500000,
  "delay_ms": 50,
  "quiet": false
}
```

Precedence is command-line flag, then config file, then built-in default:

```console
$ rusty-cli --config rusty.json compute
Hello, Dhruv
Summing squares below 100000 in parallel      # from the file
Result: 333328333350000

$ rusty-cli --config rusty.json --name Ada compute --iterations 10
Hello, Ada                                    # flag beat the file
Summing squares below 10 in parallel
Result: 285
```

An unknown key is an error rather than being quietly ignored, so a typo gets
reported instead of doing nothing:

```console
$ rusty-cli --config typo.json compute
Error: could not load config from typo.json

Caused by:
    invalid config at line 1, column 12: unknown field `iteration`,
    expected one of `name`, `count`, `iterations`, `limit`, `delay_ms`, `quiet`
```

A leading UTF-8 BOM is tolerated, because Notepad and PowerShell redirection
both write one on Windows.

## Checked arithmetic

`sum_of_squares` grows as roughly `n^3/3`, so it leaves `u32` almost at once.
The accumulator is `u64` and every step is checked, so passing the limit is a
value the caller can act on rather than a panic:

```console
$ rusty-cli compute --iterations 4294967295
Cannot compute: running total overflowed a u64 at i = 0; try a smaller input
```

The sequential and parallel kernels use the same checked arithmetic, so they
agree on the answer and on where overflow begins. The test suite asserts that
against the closed form `(n-1)n(2n-1)/6`.

## Library use

The binary is a thin shell over a library:

```rust
use rusty_cli::compute::{count_primes_parallel, sum_of_squares};

assert_eq!(sum_of_squares(1_000)?, 332_833_500);
assert_eq!(count_primes_parallel(100), 25);
# Ok::<(), rusty_cli::ComputeError>(())
```

## Project layout

| File | Contains |
| --- | --- |
| `src/compute.rs` | The kernels, sequential and Rayon-parallel |
| `src/config.rs` | Config loading and precedence |
| `src/error.rs` | `ComputeError` and `ConfigError` |
| `src/main.rs` | Argument parsing and output |
| `tests/kernels.rs` | End-to-end checks through the public API |

## Testing

```bash
cargo test                 # 34 tests
cargo clippy --all-targets -- -D warnings
```

## Building for the browser

```bash
rustup target add wasm32-unknown-unknown
cargo build --lib --no-default-features --target wasm32-unknown-unknown
```

`--no-default-features` drops Rayon and the CLI dependencies, leaving the
sequential kernels that drive the compute demo on the project site.

## Design notes

- **Accumulation is checked, on 64 bits.** Sum-of-squares grows as roughly
  `n^3 / 3`, so it leaves `u32` at around 2,300 iterations and `u64` above
  roughly 3.1 million. Totals are accumulated with `checked_add` on a `u64`, and
  genuine overflow is returned as a typed error rather than panicking or
  silently wrapping. Results are verified against the closed form
  `n(n-1)(2n-1)/6` in the tests.
- **Configuration precedence is explicit.** An argument given on the command
  line beats the config file, which beats the built-in default. `--config` loads
  a real file, and every value it supplies is read by the command that uses it.
- **stdout carries only data.** The greeting and progress output go to stderr,
  so `rusty-cli config > cfg.json` produces a file that is valid JSON and
  nothing else.
- **The kernels are a library.** The binary is a thin shell over it, which is
  what makes the computation testable and reusable.
- **The core carries no I/O.** Rayon sits behind the `parallel` feature, so the
  kernels compile for `wasm32-unknown-unknown` and run in a browser.

34 tests cover the above, with CI across Linux, macOS and Windows.


## License

MIT. See [LICENSE](LICENSE).
