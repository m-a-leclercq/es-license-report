use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::calculation::{ClusterConsumption, ConsumptionDetail};
use crate::client::ClusterFailed;

// ── Output YAML structures ────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EnterpriseClusterEntry {
    cluster_name: String,
    cluster_uid: String,
    consumed: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    report_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct PlatinumClusterEntry {
    cluster_name: String,
    cluster_uid: String,
    consumed: u64,
    reason: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    report_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct FallbackClusterEntry {
    cluster_name: String,
    cluster_uid: String,
    number_of_platinum_nodes: u64,
    number_of_enterprise_resource_units: f64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    report_time: Option<DateTime<Utc>>,
}

/// Untagged so each variant serializes as a flat YAML mapping.
/// Variant order matters for untagged deserialization: most-specific first.
#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum ClusterEntry {
    Platinum(PlatinumClusterEntry),  // has unique `reason` field
    Fallback(FallbackClusterEntry),  // has unique `number_of_platinum_nodes` field
    Enterprise(EnterpriseClusterEntry),
}

#[derive(Serialize, Deserialize)]
struct LicenseEntry {
    name: String,
    uid: String,
    #[serde(rename = "type")]
    license_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_resource_units: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_nodes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_consumed: Option<u64>,
    clusters: Vec<ClusterEntry>,
}

#[derive(Serialize, Deserialize)]
struct ErrorEntry {
    cluster: String,
    message: String,
}

/// Top-level report structure serialized to YAML.
#[derive(Serialize, Deserialize)]
pub struct Report {
    licenses: Vec<LicenseEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    errors: Vec<ErrorEntry>,
}

impl Report {
    pub fn all_failed(&self) -> bool {
        self.licenses.is_empty() && !self.errors.is_empty()
    }
}

// ── Report building ───────────────────────────────────────────────────────────

/// Build the final report from a mixed vec of successes and failures.
/// Successful results are grouped by `license.uid`; failures go into `errors`.
pub fn build_report(results: Vec<Result<ClusterConsumption, ClusterFailed>>) -> Report {
    // Track per-license: partial LicenseEntry + raw ERU sum for enterprise total_consumed
    struct LicenseGroup {
        entry: LicenseEntry,
        enterprise_raw_sum: f64,
        platinum_consumed_sum: u64,
    }

    let mut license_map: HashMap<String, LicenseGroup> = HashMap::new();
    let mut errors: Vec<ErrorEntry> = Vec::new();

    for result in results {
        match result {
            Ok(consumption) => {
                // Accumulate raw sums for total_consumed
                let enterprise_raw = match &consumption.detail {
                    ConsumptionDetail::Enterprise { consumed_raw, .. } => *consumed_raw,
                    _ => 0.0,
                };
                let platinum_consumed = match &consumption.detail {
                    ConsumptionDetail::Platinum { consumed, .. } => *consumed,
                    _ => 0,
                };

                let uid = consumption.license.uid.clone();
                let group = license_map.entry(uid).or_insert_with(|| {
                    let (max_resource_units, max_nodes) =
                        capacity_fields(&consumption.license.license_type, &consumption.license);
                    LicenseGroup {
                        entry: LicenseEntry {
                            name: consumption.license.issued_to.clone(),
                            uid: consumption.license.uid.clone(),
                            license_type: consumption.license.license_type.clone(),
                            max_resource_units,
                            max_nodes,
                            total_consumed: None,
                            clusters: Vec::new(),
                        },
                        enterprise_raw_sum: 0.0,
                        platinum_consumed_sum: 0,
                    }
                });

                group.enterprise_raw_sum += enterprise_raw;
                group.platinum_consumed_sum += platinum_consumed;
                group.entry.clusters.push(to_cluster_entry(consumption));
            }
            Err(e) => {
                errors.push(ErrorEntry {
                    cluster: e.alias,
                    message: e.message,
                });
            }
        }
    }

    // Compute total_consumed for each license group
    let mut licenses: Vec<LicenseEntry> = license_map
        .into_values()
        .map(|mut g| {
            g.entry.total_consumed = match g.entry.license_type.as_str() {
                "enterprise" if !g.entry.clusters.is_empty() => {
                    Some(g.enterprise_raw_sum.ceil() as u64)
                }
                "platinum" if !g.entry.clusters.is_empty() => Some(g.platinum_consumed_sum),
                _ => None,
            };
            g.entry
        })
        .collect();

    licenses.sort_by(|a, b| a.uid.cmp(&b.uid));

    Report { licenses, errors }
}

/// Attempt to read and parse an existing file as a `Report`.
/// Returns `None` if the file cannot be read or is not a valid report.
pub fn try_parse_existing(path: &Path) -> Option<Report> {
    let yaml = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str::<Report>(&yaml).ok()
}

/// Merge `fresh` into `existing`, keeping the cluster entry from `existing`
/// for any `(license_uid, cluster_uid)` pair present in `keep_set`.
/// Clusters in `fresh` not in `existing` are appended; clusters in `existing`
/// not in `fresh` are retained unchanged. `total_consumed` is recomputed.
pub fn merge_reports(
    mut existing: Report,
    fresh: Report,
    keep_set: &HashSet<(String, String)>,
) -> Report {
    for fresh_lic in fresh.licenses {
        if let Some(existing_lic) = existing.licenses.iter_mut().find(|l| l.uid == fresh_lic.uid) {
            // Index existing clusters by uid for O(1) lookup, consuming the vec
            let mut existing_by_uid: HashMap<String, ClusterEntry> = existing_lic
                .clusters
                .drain(..)
                .map(|c| (cluster_uid_of(&c).to_string(), c))
                .collect();

            let mut merged: Vec<ClusterEntry> =
                Vec::with_capacity(fresh_lic.clusters.len() + existing_by_uid.len());

            // For each cluster in the fresh license, decide keep vs. update
            for fresh_cluster in fresh_lic.clusters {
                let fuid = cluster_uid_of(&fresh_cluster).to_string();
                let key = (existing_lic.uid.clone(), fuid.clone());
                if keep_set.contains(&key) {
                    if let Some(existing_cluster) = existing_by_uid.remove(&fuid) {
                        merged.push(existing_cluster);
                    } else {
                        merged.push(fresh_cluster);
                    }
                } else {
                    existing_by_uid.remove(&fuid);
                    merged.push(fresh_cluster);
                }
            }

            // Retain clusters from existing that had no match in fresh
            merged.extend(existing_by_uid.into_values());

            existing_lic.clusters = merged;
            existing_lic.total_consumed =
                recompute_total_consumed(&existing_lic.license_type, &existing_lic.clusters);
        } else {
            // License not in existing → add it wholesale
            existing.licenses.push(fresh_lic);
        }
    }

    existing
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn to_cluster_entry(c: ClusterConsumption) -> ClusterEntry {
    if c.is_partial {
        eprintln!(
            "warning: cluster '{}' has partial RAM data — calculation may be incomplete",
            c.cluster_name
        );
    }
    let rt = Some(c.report_time);
    match c.detail {
        ConsumptionDetail::Enterprise { consumed, .. } => {
            ClusterEntry::Enterprise(EnterpriseClusterEntry {
                cluster_name: c.cluster_name,
                cluster_uid: c.cluster_uuid,
                consumed,
                report_time: rt,
            })
        }
        ConsumptionDetail::Platinum { consumed, reason } => {
            ClusterEntry::Platinum(PlatinumClusterEntry {
                cluster_name: c.cluster_name,
                cluster_uid: c.cluster_uuid,
                consumed,
                reason,
                report_time: rt,
            })
        }
        ConsumptionDetail::Fallback {
            number_of_platinum_nodes,
            number_of_enterprise_resource_units,
        } => ClusterEntry::Fallback(FallbackClusterEntry {
            cluster_name: c.cluster_name,
            cluster_uid: c.cluster_uuid,
            number_of_platinum_nodes,
            number_of_enterprise_resource_units,
            report_time: rt,
        }),
    }
}

fn cluster_uid_of(entry: &ClusterEntry) -> &str {
    match entry {
        ClusterEntry::Enterprise(e) => &e.cluster_uid,
        ClusterEntry::Platinum(p) => &p.cluster_uid,
        ClusterEntry::Fallback(f) => &f.cluster_uid,
    }
}

fn report_time_of(entry: &ClusterEntry) -> Option<DateTime<Utc>> {
    match entry {
        ClusterEntry::Enterprise(e) => e.report_time,
        ClusterEntry::Platinum(p) => p.report_time,
        ClusterEntry::Fallback(f) => f.report_time,
    }
}

fn cluster_name_of(entry: &ClusterEntry) -> &str {
    match entry {
        ClusterEntry::Enterprise(e) => &e.cluster_name,
        ClusterEntry::Platinum(p) => &p.cluster_name,
        ClusterEntry::Fallback(f) => &f.cluster_name,
    }
}

/// A cluster in the existing report that has a newer counterpart in the fresh report.
pub struct UpdateCandidate {
    pub license_uid: String,
    pub license_name: String,
    pub cluster_uid: String,
    pub cluster_name: String,
}

/// Return all clusters from `existing` that have a matching `cluster_uid` in `fresh`
/// with a newer `report_time` (or the existing entry has no `report_time`).
pub fn find_update_candidates(existing: &Report, fresh: &Report) -> Vec<UpdateCandidate> {
    let mut candidates = Vec::new();
    for fresh_lic in &fresh.licenses {
        let Some(existing_lic) = existing.licenses.iter().find(|l| l.uid == fresh_lic.uid) else {
            continue;
        };
        // Index existing clusters by uid for O(1) lookup
        let existing_by_uid: HashMap<&str, &ClusterEntry> = existing_lic
            .clusters
            .iter()
            .map(|c| (cluster_uid_of(c), c))
            .collect();

        for fresh_cluster in &fresh_lic.clusters {
            let fuid = cluster_uid_of(fresh_cluster);
            let Some(existing_cluster) = existing_by_uid.get(fuid) else {
                continue;
            };
            let fresh_rt = report_time_of(fresh_cluster);
            let existing_rt = report_time_of(existing_cluster);
            // Candidate when existing has no report_time (old format) or existing is older
            let is_stale = match (existing_rt, fresh_rt) {
                (None, _) => true,
                (Some(old), Some(new)) => old < new,
                _ => false,
            };
            if is_stale {
                candidates.push(UpdateCandidate {
                    license_uid: existing_lic.uid.clone(),
                    license_name: existing_lic.name.clone(),
                    cluster_uid: fuid.to_string(),
                    cluster_name: cluster_name_of(fresh_cluster).to_string(),
                });
            }
        }
    }
    candidates
}

/// Recompute `total_consumed` from already-serialised cluster entries (used after merge).
/// For Enterprise: sum `consumed` values (per-cluster rounded) then ceil to nearest integer.
/// For Platinum: sum `consumed` values then ceil to nearest integer.
fn recompute_total_consumed(license_type: &str, clusters: &[ClusterEntry]) -> Option<u64> {
    match license_type {
        "enterprise" => {
            let sum: f64 = clusters
                .iter()
                .filter_map(|c| match c {
                    ClusterEntry::Enterprise(e) => Some(e.consumed),
                    _ => None,
                })
                .sum();
            if clusters.iter().any(|c| matches!(c, ClusterEntry::Enterprise(_))) {
                Some(sum.ceil() as u64)
            } else {
                None
            }
        }
        "platinum" => {
            let sum: f64 = clusters
                .iter()
                .filter_map(|c| match c {
                    ClusterEntry::Platinum(p) => Some(p.consumed as f64),
                    _ => None,
                })
                .sum();
            if clusters.iter().any(|c| matches!(c, ClusterEntry::Platinum(_))) {
                Some(sum.ceil() as u64)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn capacity_fields(
    license_type: &str,
    license: &crate::calculation::LicenseMetadata,
) -> (Option<u32>, Option<u32>) {
    match license_type {
        "enterprise" => (license.max_resource_units, None),
        "platinum" => (None, license.max_nodes),
        _ => (None, None),
    }
}

/// Serialize the report to YAML and write it to a file.
pub fn write_report(report: &Report, output: &Path) -> Result<()> {
    let yaml = serde_yaml::to_string(report)?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty() && !parent.exists()
    {
        return Err(anyhow::anyhow!(
            "Output directory does not exist: {}",
            parent.display()
        ));
    }
    std::fs::write(output, yaml)?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculation::{round_up_2_decimals_pub, ClusterConsumption, ConsumptionDetail, LicenseMetadata};

    fn make_consumption(
        _alias: &str,
        cluster_name: &str,
        cluster_uuid: &str,
        license_uid: &str,
        license_type: &str,
        max_resource_units: Option<u32>,
        max_nodes: Option<u32>,
        detail: ConsumptionDetail,
    ) -> ClusterConsumption {
        ClusterConsumption {
            cluster_name: cluster_name.to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            license: LicenseMetadata {
                uid: license_uid.to_string(),
                license_type: license_type.to_string(),
                issued_to: "Test Corp".to_string(),
                max_resource_units,
                max_nodes,
            },
            detail,
            is_partial: false,
            report_time: DateTime::<Utc>::UNIX_EPOCH,
        }
    }

    fn enterprise_detail(consumed_raw: f64) -> ConsumptionDetail {
        ConsumptionDetail::Enterprise {
            consumed: crate::calculation::round_up_2_decimals_pub(consumed_raw),
            consumed_raw,
        }
    }

    #[test]
    fn shared_license_across_multiple_clusters() {
        let results = vec![
            Ok(make_consumption(
                "a",
                "cluster-a",
                "uuid-a",
                "shared-uid",
                "enterprise",
                Some(24),
                None,
                enterprise_detail(8.0),
            )),
            Ok(make_consumption(
                "b",
                "cluster-b",
                "uuid-b",
                "shared-uid",
                "enterprise",
                Some(24),
                None,
                enterprise_detail(4.0),
            )),
        ];

        let report = build_report(results);
        assert_eq!(report.licenses.len(), 1);
        assert_eq!(report.licenses[0].clusters.len(), 2);
    }

    #[test]
    fn enterprise_license_includes_max_resource_units() {
        let results = vec![Ok(make_consumption(
            "a",
            "cluster-a",
            "uuid-a",
            "lic-1",
            "enterprise",
            Some(24),
            None,
            enterprise_detail(8.0),
        ))];

        let report = build_report(results);
        assert_eq!(report.licenses[0].max_resource_units, Some(24));
        assert_eq!(report.licenses[0].max_nodes, None);
    }

    #[test]
    fn platinum_license_includes_max_nodes_and_reason() {
        let results = vec![Ok(make_consumption(
            "a",
            "cluster-a",
            "uuid-a",
            "lic-1",
            "platinum",
            None,
            Some(12),
            ConsumptionDetail::Platinum {
                consumed: 7,
                reason: "Total RAM used".to_string(),
            },
        ))];

        let report = build_report(results);
        assert_eq!(report.licenses[0].max_nodes, Some(12));
        assert_eq!(report.licenses[0].max_resource_units, None);

        let yaml = serde_yaml::to_string(&report).unwrap();
        assert!(yaml.contains("Total RAM used"));
    }

    #[test]
    fn basic_license_includes_fallback_metrics() {
        let results = vec![Ok(make_consumption(
            "a",
            "cluster-a",
            "uuid-a",
            "lic-1",
            "basic",
            None,
            None,
            ConsumptionDetail::Fallback {
                number_of_platinum_nodes: 3,
                number_of_enterprise_resource_units: 1.57,
            },
        ))];

        let report = build_report(results);
        let yaml = serde_yaml::to_string(&report).unwrap();
        assert!(yaml.contains("number_of_platinum_nodes"));
        assert!(yaml.contains("number_of_enterprise_resource_units"));
        assert!(!yaml.contains("consumed:"));
    }

    #[test]
    fn partial_failure_includes_both_licenses_and_errors() {
        let results: Vec<Result<ClusterConsumption, ClusterFailed>> = vec![
            Ok(make_consumption(
                "good",
                "cluster-good",
                "uuid-good",
                "lic-1",
                "enterprise",
                Some(24),
                None,
                enterprise_detail(2.0),
            )),
            Err(ClusterFailed {
                alias: "bad-cluster".to_string(),
                message: "connection refused".to_string(),
            }),
        ];

        let report = build_report(results);
        assert_eq!(report.licenses.len(), 1);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].cluster, "bad-cluster");
        assert!(!report.all_failed());
    }

    #[test]
    fn all_failures_reports_all_failed() {
        let results: Vec<Result<ClusterConsumption, ClusterFailed>> = vec![
            Err(ClusterFailed {
                alias: "a".to_string(),
                message: "timeout".to_string(),
            }),
            Err(ClusterFailed {
                alias: "b".to_string(),
                message: "connection refused".to_string(),
            }),
        ];

        let report = build_report(results);
        assert_eq!(report.licenses.len(), 0);
        assert_eq!(report.errors.len(), 2);
        assert!(report.all_failed());
    }

    // ── total_consumed tests ──────────────────────────────────────────────────

    #[test]
    fn enterprise_total_consumed_sums_raw_then_ceils_to_integer() {
        // Two clusters: raw ERUs 1.234 and 2.567
        // sum = 3.801 → ceil → 4, NOT ceil(1.234)+ceil(2.567) = 2+3 = 5
        let results = vec![
            Ok(make_consumption(
                "a", "cluster-a", "uuid-a", "lic-1", "enterprise", Some(24), None,
                ConsumptionDetail::Enterprise {
                    consumed: round_up_2_decimals_pub(1.234),
                    consumed_raw: 1.234,
                },
            )),
            Ok(make_consumption(
                "b", "cluster-b", "uuid-b", "lic-1", "enterprise", Some(24), None,
                ConsumptionDetail::Enterprise {
                    consumed: round_up_2_decimals_pub(2.567),
                    consumed_raw: 2.567,
                },
            )),
        ];
        let report = build_report(results);
        assert_eq!(report.licenses[0].total_consumed, Some(4));
    }

    #[test]
    fn enterprise_total_consumed_exact_integer_sum() {
        // raw sum = 4.0 → ceil → 4
        let results = vec![
            Ok(make_consumption(
                "a", "cluster-a", "uuid-a", "lic-1", "enterprise", Some(24), None,
                ConsumptionDetail::Enterprise { consumed: 2.0, consumed_raw: 2.0 },
            )),
            Ok(make_consumption(
                "b", "cluster-b", "uuid-b", "lic-1", "enterprise", Some(24), None,
                ConsumptionDetail::Enterprise { consumed: 2.0, consumed_raw: 2.0 },
            )),
        ];
        let report = build_report(results);
        assert_eq!(report.licenses[0].total_consumed, Some(4));
    }

    #[test]
    fn platinum_total_consumed_sums_cluster_values() {
        let results = vec![
            Ok(make_consumption(
                "a", "cluster-a", "uuid-a", "lic-1", "platinum", None, Some(12),
                ConsumptionDetail::Platinum { consumed: 5, reason: "node count".to_string() },
            )),
            Ok(make_consumption(
                "b", "cluster-b", "uuid-b", "lic-1", "platinum", None, Some(12),
                ConsumptionDetail::Platinum { consumed: 7, reason: "Total RAM used".to_string() },
            )),
        ];
        let report = build_report(results);
        assert_eq!(report.licenses[0].total_consumed, Some(12));
    }

    #[test]
    fn basic_license_has_no_total_consumed() {
        let results = vec![Ok(make_consumption(
            "a", "cluster-a", "uuid-a", "lic-1", "basic", None, None,
            ConsumptionDetail::Fallback {
                number_of_platinum_nodes: 3,
                number_of_enterprise_resource_units: 1.57,
            },
        ))];
        let report = build_report(results);
        assert_eq!(report.licenses[0].total_consumed, None);
    }

    // ── merge_reports tests ───────────────────────────────────────────────────

    #[test]
    fn merge_stale_cluster_is_replaced() {
        let old_time = "2026-01-01T00:00:00Z";
        let new_time = "2026-04-01T00:00:00Z";

        let existing_yaml = format!(
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.0\n        report_time: {old_time}\n"
        );
        let existing: Report = serde_yaml::from_str(&existing_yaml).unwrap();

        let fresh_yaml = format!(
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 2.0\n        report_time: {new_time}\n"
        );
        let fresh: Report = serde_yaml::from_str(&fresh_yaml).unwrap();

        let merged = merge_reports(existing, fresh, &HashSet::new());
        let yaml = serde_yaml::to_string(&merged).unwrap();
        assert!(yaml.contains("consumed: 2.0"), "stale cluster should be updated");
        assert!(!yaml.contains("consumed: 1.0"), "old value should be gone");
    }

    #[test]
    fn merge_kept_cluster_is_retained() {
        let old_time = "2026-01-01T00:00:00Z";
        let new_time = "2026-04-01T00:00:00Z";

        let existing_yaml = format!(
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.0\n        report_time: {old_time}\n"
        );
        let existing: Report = serde_yaml::from_str(&existing_yaml).unwrap();

        let fresh_yaml = format!(
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 2.0\n        report_time: {new_time}\n"
        );
        let fresh: Report = serde_yaml::from_str(&fresh_yaml).unwrap();

        let mut keep_set = HashSet::new();
        keep_set.insert(("lic-1".to_string(), "uuid-1".to_string()));

        let merged = merge_reports(existing, fresh, &keep_set);
        let yaml = serde_yaml::to_string(&merged).unwrap();
        assert!(yaml.contains("consumed: 1.0"), "kept cluster should be retained");
    }

    #[test]
    fn merge_new_cluster_is_appended() {
        let existing_yaml =
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.0\n";
        let existing: Report = serde_yaml::from_str(existing_yaml).unwrap();

        let fresh_yaml =
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c2\n        cluster_uid: uuid-2\n        consumed: 3.0\n";
        let fresh: Report = serde_yaml::from_str(fresh_yaml).unwrap();

        let merged = merge_reports(existing, fresh, &HashSet::new());
        assert_eq!(merged.licenses[0].clusters.len(), 2);
    }

    #[test]
    fn yaml_output_contains_report_time_and_total_consumed() {
        // Task 2.6: verify YAML shape matches spec example
        let ts = "2026-04-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let results = vec![Ok(ClusterConsumption {
            cluster_name: "prod-cluster-1".to_string(),
            cluster_uuid: "uuid-1".to_string(),
            license: LicenseMetadata {
                uid: "lic-1".to_string(),
                license_type: "enterprise".to_string(),
                issued_to: "Example Production".to_string(),
                max_resource_units: Some(24),
                max_nodes: None,
            },
            detail: ConsumptionDetail::Enterprise { consumed: 8.0, consumed_raw: 8.0 },
            is_partial: false,
            report_time: ts,
        })];
        let report = build_report(results);
        let yaml = serde_yaml::to_string(&report).unwrap();
        assert!(yaml.contains("report_time:"), "cluster entry should have report_time");
        assert!(yaml.contains("total_consumed:"), "license entry should have total_consumed");
    }

    #[test]
    fn merge_missing_report_time_treated_as_epoch() {
        // Task 7.1: existing cluster with no report_time is always a candidate
        let existing_yaml =
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.0\n";
        let existing: Report = serde_yaml::from_str(existing_yaml).unwrap();

        let fresh_yaml =
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 5.0\n        report_time: 2026-04-01T00:00:00Z\n";
        let fresh: Report = serde_yaml::from_str(fresh_yaml).unwrap();

        let candidates = find_update_candidates(&existing, &fresh);
        assert_eq!(candidates.len(), 1, "cluster with no report_time should be a candidate");
        assert_eq!(candidates[0].cluster_uid, "uuid-1");
    }

    #[test]
    fn update_all_silently_replaces_stale_clusters() {
        // Task 7.5: simulate --update behavior (empty keep_set = update all)
        let old_time = "2026-01-01T00:00:00Z";
        let new_time = "2026-04-01T00:00:00Z";

        let existing_yaml = format!(
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.0\n        report_time: {old_time}\n      - cluster_name: c2\n        cluster_uid: uuid-2\n        consumed: 2.0\n        report_time: {new_time}\n"
        );
        let existing: Report = serde_yaml::from_str(&existing_yaml).unwrap();

        let fresh_yaml = format!(
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 9.0\n        report_time: {new_time}\n      - cluster_name: c2\n        cluster_uid: uuid-2\n        consumed: 2.0\n        report_time: {new_time}\n"
        );
        let fresh: Report = serde_yaml::from_str(&fresh_yaml).unwrap();

        // --update: empty keep_set means update everything stale
        let candidates = find_update_candidates(&existing, &fresh);
        assert_eq!(candidates.len(), 1, "only uuid-1 is stale");

        let merged = merge_reports(existing, fresh, &HashSet::new());
        let yaml = serde_yaml::to_string(&merged).unwrap();
        assert!(yaml.contains("consumed: 9.0"), "stale cluster should be updated to fresh value");
        assert!(yaml.contains("consumed: 2.0"), "non-stale cluster should be retained");
    }

    #[test]
    fn merge_cluster_absent_from_fresh_is_retained() {
        let existing_yaml =
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.0\n      - cluster_name: c2\n        cluster_uid: uuid-2\n        consumed: 2.0\n";
        let existing: Report = serde_yaml::from_str(existing_yaml).unwrap();

        // Fresh only has uuid-1
        let fresh_yaml =
            "licenses:\n  - name: Test\n    uid: lic-1\n    type: enterprise\n    clusters:\n      - cluster_name: c1\n        cluster_uid: uuid-1\n        consumed: 1.5\n";
        let fresh: Report = serde_yaml::from_str(fresh_yaml).unwrap();

        let merged = merge_reports(existing, fresh, &HashSet::new());
        // uuid-2 should still be there
        assert_eq!(merged.licenses[0].clusters.len(), 2);
    }
}

