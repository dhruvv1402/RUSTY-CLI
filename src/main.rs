//! The `rusty-cli` binary.
//!
//! A thin shell over the library: parse arguments, layer them over a config
//! file, run a kernel, report the result.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rusty_cli::compute::{
    count_primes, count_primes_parallel, sum_of_squares, sum_of_squares_parallel, thread_count,
};
use rusty_cli::Config;

#[derive(Parser)]
#[command(
    name = "rusty-cli",
    author,
    version,
    about = "Parallel computation from the command line",
    long_about = None
)]
struct Cli {
    /// Who to greet. Overrides `name` from the config file.
    #[arg(short, long, global = true)]
    name: Option<String>,

    /// JSON config file supplying defaults for the options below.
    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,

    /// Suppress colour and progress bars.
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an async pipeline with a progress bar.
    Process {
        /// How many items to process.
        #[arg(short, long)]
        count: Option<u32>,

        /// Milliseconds of simulated work per item.
        #[arg(short, long)]
        delay_ms: Option<u64>,
    },

    /// Sum i*i for every i below the iteration count.
    Compute {
        #[arg(short, long)]
        iterations: Option<u32>,

        /// Run sequentially instead of across all cores.
        #[arg(long)]
        sequential: bool,
    },

    /// Count the primes below a limit.
    Primes {
        #[arg(short, long)]
        limit: Option<u64>,

        #[arg(long)]
        sequential: bool,
    },

    /// Time the sequential and parallel kernels against each other.
    Bench {
        #[arg(short, long, default_value_t = 2_000_000)]
        limit: u64,
    },

    /// Print an example config file.
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // A config file supplies defaults; explicit flags win over it.
    let from_file = match &cli.config {
        Some(path) => Config::from_path(path)
            .with_context(|| format!("could not load config from {}", path.display()))?,
        None => Config::default(),
    };

    let flags = Config {
        name: cli.name.clone(),
        quiet: cli.quiet.then_some(true),
        ..Default::default()
    };
    let settings = flags.or(from_file);

    let quiet = settings.quiet.unwrap_or(false);
    if quiet {
        // `colored` honours this for every subsequent call.
        colored::control::set_override(false);
    }

    greet(settings.name.as_deref(), quiet);

    match &cli.command {
        Some(Commands::Process { count, delay_ms }) => {
            let count = count.or(settings.count).unwrap_or(10);
            let delay = delay_ms.or(settings.delay_ms).unwrap_or(100);
            process(count, delay, quiet).await;
        }

        Some(Commands::Compute {
            iterations,
            sequential,
        }) => {
            let iterations = iterations.or(settings.iterations).unwrap_or(1_000);
            compute(iterations, *sequential)?;
        }

        Some(Commands::Primes { limit, sequential }) => {
            let limit = limit.or(settings.limit).unwrap_or(100_000);
            primes(limit, *sequential);
        }

        Some(Commands::Bench { limit }) => bench(*limit),

        Some(Commands::Config) => println!("{}", Config::example().to_json()),

        None => {
            println!(
                "{}",
                "No command given. Run with --help to see what is available.".yellow()
            );
        }
    }

    Ok(())
}

/// Uses the name that `--name` or the config file supplied.
///
/// Written to stderr, not stdout. The greeting is decoration, and mixing it
/// into stdout would corrupt anything being piped: `rusty-cli config > cfg.json`
/// has to produce a file that is only JSON.
fn greet(name: Option<&str>, quiet: bool) {
    if quiet {
        return;
    }
    match name {
        Some(name) => eprintln!(
            "{} {}",
            "Hello,".bright_green().bold(),
            name.bright_white().bold()
        ),
        None => eprintln!("{}", "rusty-cli".bright_green().bold()),
    }
}

async fn process(count: u32, delay_ms: u64, quiet: bool) {
    let bar = if quiet {
        ProgressBar::hidden()
    } else {
        let bar = ProgressBar::new(count as u64);
        // A bad template is a programming mistake, not a runtime condition, so
        // fall back rather than propagating it out of the whole program.
        match ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        {
            Ok(style) => bar.set_style(style.progress_chars("=>-")),
            Err(_) => bar.set_style(ProgressStyle::default_bar()),
        }
        bar
    };

    let start = Instant::now();
    for i in 0..count {
        bar.set_message(format!("item {}", i + 1));
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        bar.inc(1);
    }
    bar.finish_and_clear();

    println!(
        "{} {count} items in {:.2}s",
        "Processed".green().bold(),
        start.elapsed().as_secs_f64()
    );
}

fn compute(iterations: u32, sequential: bool) -> Result<()> {
    let mode = if sequential {
        "sequentially"
    } else {
        "in parallel"
    };
    println!("Summing squares below {iterations} {mode}");

    let start = Instant::now();
    let result = if sequential {
        sum_of_squares(iterations)
    } else {
        sum_of_squares_parallel(iterations)
    };
    let elapsed = start.elapsed();

    match result {
        Ok(total) => {
            println!("{} {total}", "Result:".green().bold());
            println!("took {:.3}ms", elapsed.as_secs_f64() * 1000.0);
        }
        Err(e) => {
            // The old version panicked here with "attempt to add with overflow".
            println!("{} {e}", "Cannot compute:".red().bold());
        }
    }
    Ok(())
}

fn primes(limit: u64, sequential: bool) {
    let mode = if sequential {
        "sequentially"
    } else {
        "in parallel"
    };
    println!("Counting primes below {limit} {mode}");

    let start = Instant::now();
    let count = if sequential {
        count_primes(limit)
    } else {
        count_primes_parallel(limit)
    };
    let elapsed = start.elapsed();

    println!("{} {count}", "Primes found:".green().bold());
    println!("took {:.3}s", elapsed.as_secs_f64());
}

fn bench(limit: u64) {
    println!(
        "Counting primes below {limit} on {} thread(s)\n",
        thread_count()
    );

    let start = Instant::now();
    let sequential_count = count_primes(limit);
    let sequential = start.elapsed();
    println!(
        "{:<12} {:>8.3}s  ({sequential_count} primes)",
        "sequential",
        sequential.as_secs_f64()
    );

    let start = Instant::now();
    let parallel_count = count_primes_parallel(limit);
    let parallel = start.elapsed();
    println!(
        "{:<12} {:>8.3}s  ({parallel_count} primes)",
        "parallel",
        parallel.as_secs_f64()
    );

    assert_eq!(
        sequential_count, parallel_count,
        "the two kernels disagreed, which would be a bug"
    );

    if parallel.as_secs_f64() > 0.0 {
        let speedup = sequential.as_secs_f64() / parallel.as_secs_f64();
        println!(
            "\n{} {speedup:.2}x on {} thread(s)",
            "Speedup:".green().bold(),
            thread_count()
        );
    }
}
