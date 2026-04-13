mod calculation;
mod client;
mod config;
mod report;

use std::collections::HashSet;
use std::io::{self, BufRead, IsTerminal, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

use calculation::calculate;
use client::query_all_clusters;
use config::load_config;
use report::{
    build_report, find_update_candidates, merge_reports, try_parse_existing, write_report,
    UpdateCandidate,
};

/// Query Elasticsearch clusters and produce a YAML license consumption report.
#[derive(Parser)]
#[command(name = "es-license-consumption", version)]
struct Args {
    /// Path to the cluster YAML configuration file (default: cluster.yml).
    #[arg(long, short = 'c')]
    config: Option<PathBuf>,

    /// Write the YAML report to this file (default: report.yml).
    #[arg(long, short = 'o')]
    output: Option<PathBuf>,

    /// Per-cluster HTTP request timeout in seconds (default: 20).
    #[arg(long, short = 't', default_value_t = 20)]
    timeout: u64,

    /// If the report already exists, updates all existing clusters to their newer values.
    #[arg(long, short = 'u')]
    update: bool,
}

fn main() {
    let raw = Args::parse();

    let config_path = match resolve_config(raw.config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(2);
        }
    };

    let output_path = match resolve_output(raw.output) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(2);
        }
    };

    // Detect whether the output file already exists and whether it is a valid report.
    // We do this before querying so that "write to another file" can redirect early.
    let pre_decision = match pre_query_output_decision(&output_path, raw.update) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(2);
        }
    };

    // Extract the resolved output path (may have changed to an alternate file).
    let final_output = match &pre_decision {
        PreQueryDecision::Write(p) | PreQueryDecision::Overwrite(p) => p.clone(),
        PreQueryDecision::Update { path, .. } | PreQueryDecision::SilentUpdate { path, .. } => {
            path.clone()
        }
    };

    let fresh_report = match run_sync(config_path, raw.timeout) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e:#}");
            process::exit(2);
        }
    };

    let report_to_write = match pre_decision {
        PreQueryDecision::Write(_) | PreQueryDecision::Overwrite(_) => fresh_report,
        PreQueryDecision::SilentUpdate { existing, .. } => {
            let candidates = find_update_candidates(&existing, &fresh_report);
            // Silent update: update all stale clusters
            let keep_set: HashSet<(String, String)> = HashSet::new(); // keep nothing → update all
            let any_updates = !candidates.is_empty();
            let merged = merge_reports(existing, fresh_report, &keep_set);
            if !any_updates {
                eprintln!("No clusters required updating");
            }
            merged
        }
        PreQueryDecision::Update { existing, .. } => {
            let candidates = find_update_candidates(&existing, &fresh_report);
            if candidates.is_empty() {
                eprintln!("No clusters required updating");
            }
            let keep_set = if candidates.is_empty() {
                HashSet::new()
            } else {
                interactive_update_prompt(&candidates)
            };
            merge_reports(existing, fresh_report, &keep_set)
        }
    };

    if let Err(e) = write_report(&report_to_write, &final_output) {
        eprintln!("error: {e:#}");
        process::exit(2);
    }

    if report_to_write.all_failed() {
        process::exit(1);
    }
}

// ── Pre-query output decision ─────────────────────────────────────────────────

enum PreQueryDecision {
    /// Output path does not exist — write fresh report.
    Write(PathBuf),
    /// Output path exists but is not a valid report — user chose overwrite.
    Overwrite(PathBuf),
    /// Output path is a valid report; update all clusters silently.
    SilentUpdate { path: PathBuf, existing: report::Report },
    /// Output path is a valid report; user will be prompted per cluster.
    Update { path: PathBuf, existing: report::Report },
}

fn pre_query_output_decision(
    path: &Path,
    update_flag: bool,
) -> anyhow::Result<PreQueryDecision> {
    if !path.exists() {
        return Ok(PreQueryDecision::Write(path.to_path_buf()));
    }

    let stdin = io::stdin();
    let is_tty = stdin.is_terminal();

    // Try to parse the existing file as a valid report.
    match try_parse_existing(path) {
        Some(existing) => {
            if update_flag || !is_tty {
                Ok(PreQueryDecision::SilentUpdate { path: path.to_path_buf(), existing })
            } else {
                // Interactive update prompt
                let resolved = prompt_update_or_alternate(path)?;
                match resolved {
                    UpdateChoice::Update => Ok(PreQueryDecision::Update { path: path.to_path_buf(), existing }),
                    UpdateChoice::Alternate(new_path) => Ok(PreQueryDecision::Write(new_path)),
                }
            }
        }
        None => {
            // File exists but is not a valid report — fall back to overwrite prompt
            eprintln!(
                "warning: `{}` exists but could not be parsed as a report; treating as overwrite",
                path.display()
            );
            if !is_tty {
                return Ok(PreQueryDecision::Overwrite(path.to_path_buf()));
            }
            let resolved = confirm_overwrite(path.to_path_buf())?;
            Ok(PreQueryDecision::Overwrite(resolved))
        }
    }
}

// ── Interactive prompts ───────────────────────────────────────────────────────

enum UpdateChoice {
    Update,
    Alternate(PathBuf),
}

fn prompt_update_or_alternate(path: &Path) -> anyhow::Result<UpdateChoice> {
    let stdin = io::stdin();
    loop {
        eprint!(
            "`{}` already exists. [u]pdate / [a]lternate file (u): ",
            path.display()
        );
        io::stderr().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let choice = line.trim().to_lowercase();

        match choice.as_str() {
            "" | "u" => return Ok(UpdateChoice::Update),
            "a" => {
                eprint!("Enter filename: ");
                io::stderr().flush()?;
                let mut name = String::new();
                stdin.lock().read_line(&mut name)?;
                let name = name.trim().to_string();
                if name.is_empty() {
                    eprintln!("Filename cannot be empty, please try again.");
                    continue;
                }
                return Ok(UpdateChoice::Alternate(normalize_extension(&name)));
            }
            _ => eprintln!("Please enter 'u' to update or 'a' to choose another file."),
        }
    }
}

/// Per-cluster interactive prompt. Returns the set of `(license_uid, cluster_uid)` to KEEP
/// from the existing report (i.e. NOT update).
fn interactive_update_prompt(candidates: &[UpdateCandidate]) -> HashSet<(String, String)> {
    let stdin = io::stdin();
    let mut keep_set: HashSet<(String, String)> = HashSet::new();
    let mut update_all = false;
    let mut skip_all = false;

    for candidate in candidates {
        if update_all {
            // update → do not add to keep_set
            continue;
        }
        if skip_all {
            keep_set.insert((candidate.license_uid.clone(), candidate.cluster_uid.clone()));
            continue;
        }

        loop {
            eprint!(
                "License \"{}\" cluster \"{}\" has newer data. [u]pdate / [s]kip / [a]update all / [k]skip all (u): ",
                candidate.license_name, candidate.cluster_name
            );
            io::stderr().flush().unwrap_or(());

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).is_err() {
                break;
            }
            let choice = line.trim().to_lowercase();

            match choice.as_str() {
                "" | "u" => break, // update this cluster (not in keep_set)
                "s" => {
                    keep_set.insert((candidate.license_uid.clone(), candidate.cluster_uid.clone()));
                    break;
                }
                "a" => {
                    update_all = true;
                    break;
                }
                "k" => {
                    skip_all = true;
                    keep_set.insert((candidate.license_uid.clone(), candidate.cluster_uid.clone()));
                    break;
                }
                _ => eprintln!("Please enter 'u', 's', 'a', or 'k'."),
            }
        }
    }

    keep_set
}

/// If the output path already exists AND is not a valid report, prompt to overwrite or choose
/// an alternate filename. In non-interactive environments the file is overwritten silently.
fn confirm_overwrite(path: PathBuf) -> anyhow::Result<PathBuf> {
    let stdin = io::stdin();

    loop {
        eprint!(
            "`{}` already exists. [o]verwrite / [a]lternate file (o): ",
            path.display()
        );
        io::stderr().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let choice = line.trim().to_lowercase();

        match choice.as_str() {
            "" | "o" => return Ok(path),
            "a" => {
                eprint!("Enter filename: ");
                io::stderr().flush()?;
                let mut name = String::new();
                stdin.lock().read_line(&mut name)?;
                let name = name.trim().to_string();
                if name.is_empty() {
                    eprintln!("Filename cannot be empty, please try again.");
                    continue;
                }
                return Ok(normalize_extension(&name));
            }
            _ => eprintln!("Please enter 'o' to overwrite or 'a' to choose another file."),
        }
    }
}

// ── Async query + calculate ───────────────────────────────────────────────────

#[tokio::main]
async fn run_sync(config: PathBuf, timeout: u64) -> anyhow::Result<report::Report> {
    let configs = load_config(&config)?;
    let query_results = query_all_clusters(&configs, timeout).await;
    let calc_results = query_results.into_iter().map(|r| r.map(calculate)).collect();
    Ok(build_report(calc_results))
}

// ── Argument resolution ────────────────────────────────────────────────────────

/// Resolve the config path: use the supplied value or fall back to `cluster.yml`.
fn resolve_config(raw: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match raw {
        Some(p) => Ok(p),
        None => {
            let default = PathBuf::from("cluster.yml");
            if default.exists() {
                Ok(default)
            } else {
                Err(anyhow::anyhow!(
                    "no --config flag provided and the default `cluster.yml` was not found in the current directory"
                ))
            }
        }
    }
}

/// Resolve the output path: use the supplied value or fall back to `report.yml`.
/// Prints a notice to stderr when the default is used.
fn resolve_output(raw: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match raw {
        Some(p) => Ok(p),
        None => {
            let default = PathBuf::from("report.yml");
            eprintln!("Writing report to report.yml");
            Ok(default)
        }
    }
}

/// Append `.yml` to `name` when it has no extension.
/// Leave `.yml` / `.yaml` (case-insensitive) and any other extension unchanged.
fn normalize_extension(name: &str) -> PathBuf {
    let p = Path::new(name);
    match p.extension() {
        None => {
            let mut s = name.to_string();
            s.push_str(".yml");
            PathBuf::from(s)
        }
        Some(_) => PathBuf::from(name),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── resolve_config ──────────────────────────────────────────────────────

    #[test]
    fn resolve_config_explicit_path_returned_as_is() {
        let path = PathBuf::from("/some/explicit/path.yml");
        assert_eq!(resolve_config(Some(path.clone())).unwrap(), path);
    }

    #[test]
    fn resolve_config_default_used_when_file_present() {
        let dir = TempDir::new().unwrap();
        let default = dir.path().join("cluster.yml");
        fs::write(&default, "{}").unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = resolve_config(None);
        std::env::set_current_dir(original).unwrap();

        assert_eq!(result.unwrap(), PathBuf::from("cluster.yml"));
    }

    #[test]
    fn resolve_config_error_when_default_absent() {
        let dir = TempDir::new().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = resolve_config(None);
        std::env::set_current_dir(original).unwrap();

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("cluster.yml"));
    }

    // ── resolve_output ──────────────────────────────────────────────────────

    #[test]
    fn resolve_output_explicit_path_returned() {
        let path = PathBuf::from("custom.yml");
        assert_eq!(resolve_output(Some(path.clone())).unwrap(), path);
    }

    #[test]
    fn resolve_output_defaults_to_report_yml() {
        let result = resolve_output(None).unwrap();
        assert_eq!(result, PathBuf::from("report.yml"));
    }

    // ── normalize_extension ─────────────────────────────────────────────────

    #[test]
    fn normalize_no_extension_appends_yml() {
        assert_eq!(normalize_extension("my-report"), PathBuf::from("my-report.yml"));
    }

    #[test]
    fn normalize_yml_extension_unchanged() {
        assert_eq!(normalize_extension("my-report.yml"), PathBuf::from("my-report.yml"));
    }

    #[test]
    fn normalize_yaml_extension_unchanged() {
        assert_eq!(normalize_extension("my-report.yaml"), PathBuf::from("my-report.yaml"));
    }

    #[test]
    fn normalize_other_extension_unchanged() {
        assert_eq!(normalize_extension("my-report.txt"), PathBuf::from("my-report.txt"));
    }
}
