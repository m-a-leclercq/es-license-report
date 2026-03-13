mod calculation;
mod client;
mod config;
mod report;

use std::path::PathBuf;
use std::process;

use clap::Parser;

use calculation::calculate;
use client::query_all_clusters;
use config::load_config;
use report::{build_report, write_report};

/// Query Elasticsearch clusters and produce a YAML license consumption report.
#[derive(Parser)]
#[command(name = "es-license-consumption", version)]
struct Args {
    /// Path to the cluster YAML configuration file.
    #[arg(long)]
    config: PathBuf,

    /// Write the YAML report to this file instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Per-cluster HTTP request timeout in seconds (default: 20).
    #[arg(long, default_value_t = 20)]
    timeout: u64,
}

#[tokio::main]
async fn main() {
    // Secrets must not appear in log output — never log Args fields directly.
    let args = Args::parse();

    if let Err(e) = run(args).await {
        eprintln!("error: {e:#}");
        process::exit(2);
    }
}

async fn run(args: Args) -> anyhow::Result<()> {
    let configs = load_config(&args.config)?;

    let query_results = query_all_clusters(&configs, args.timeout).await;

    let calc_results = query_results
        .into_iter()
        .map(|r| r.map(calculate))
        .collect();

    let report = build_report(calc_results);
    write_report(&report, args.output.as_deref())?;

    if report.all_failed() {
        process::exit(1);
    }

    Ok(())
}
