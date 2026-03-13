use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::calculation::{ClusterConsumption, ConsumptionDetail};
use crate::client::ClusterFailed;

// ── Output YAML structures ────────────────────────────────────────────────────

#[derive(Serialize)]
struct EnterpriseClusterEntry {
    cluster_name: String,
    cluster_uid: String,
    consumed: f64,
}

#[derive(Serialize)]
struct PlatinumClusterEntry {
    cluster_name: String,
    cluster_uid: String,
    consumed: u64,
    reason: String,
}

#[derive(Serialize)]
struct FallbackClusterEntry {
    cluster_name: String,
    cluster_uid: String,
    number_of_platinum_nodes: u64,
    number_of_enterprise_resource_units: f64,
}

/// Untagged so each variant serializes as a flat YAML mapping.
#[derive(Serialize)]
#[serde(untagged)]
enum ClusterEntry {
    Enterprise(EnterpriseClusterEntry),
    Platinum(PlatinumClusterEntry),
    Fallback(FallbackClusterEntry),
}

#[derive(Serialize)]
struct LicenseEntry {
    name: String,
    uid: String,
    #[serde(rename = "type")]
    license_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_resource_units: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_nodes: Option<u32>,
    clusters: Vec<ClusterEntry>,
}

#[derive(Serialize)]
struct ErrorEntry {
    cluster: String,
    message: String,
}

/// Top-level report structure serialized to YAML.
#[derive(Serialize)]
pub struct Report {
    licenses: Vec<LicenseEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    let mut license_map: HashMap<String, LicenseEntry> = HashMap::new();
    let mut errors: Vec<ErrorEntry> = Vec::new();

    for result in results {
        match result {
            Ok(consumption) => {
                let uid = consumption.license.uid.clone();
                let entry = license_map.entry(uid.clone()).or_insert_with(|| {
                    let (max_resource_units, max_nodes) =
                        capacity_fields(&consumption.license.license_type, &consumption.license);
                    LicenseEntry {
                        name: consumption.license.issued_to.clone(),
                        uid,
                        license_type: consumption.license.license_type.clone(),
                        max_resource_units,
                        max_nodes,
                        clusters: Vec::new(),
                    }
                });
                entry.clusters.push(to_cluster_entry(&consumption));
            }
            Err(e) => {
                errors.push(ErrorEntry {
                    cluster: e.alias,
                    message: e.message,
                });
            }
        }
    }

    let mut licenses: Vec<LicenseEntry> = license_map.into_values().collect();
    licenses.sort_by(|a, b| a.uid.cmp(&b.uid));

    Report { licenses, errors }
}

/// Serialize the report to YAML and write it to stdout or a file.
pub fn write_report(report: &Report, output: Option<&Path>) -> Result<()> {
    let yaml = serde_yaml::to_string(report)?;

    match output {
        Some(path) => {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty() && !parent.exists()
            {
                return Err(anyhow::anyhow!(
                    "Output directory does not exist: {}",
                    parent.display()
                ));
            }
            std::fs::write(path, yaml)?;
        }
        None => print!("{yaml}"),
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn to_cluster_entry(c: &ClusterConsumption) -> ClusterEntry {
    if c.is_partial {
        eprintln!(
            "warning: cluster '{}' has partial RAM data — calculation may be incomplete",
            c.cluster_name
        );
    }
    match &c.detail {
        ConsumptionDetail::Enterprise { consumed } => {
            ClusterEntry::Enterprise(EnterpriseClusterEntry {
                cluster_name: c.cluster_name.clone(),
                cluster_uid: c.cluster_uuid.clone(),
                consumed: *consumed,
            })
        }
        ConsumptionDetail::Platinum { consumed, reason } => {
            ClusterEntry::Platinum(PlatinumClusterEntry {
                cluster_name: c.cluster_name.clone(),
                cluster_uid: c.cluster_uuid.clone(),
                consumed: *consumed,
                reason: reason.clone(),
            })
        }
        ConsumptionDetail::Fallback {
            number_of_platinum_nodes,
            number_of_enterprise_resource_units,
        } => ClusterEntry::Fallback(FallbackClusterEntry {
            cluster_name: c.cluster_name.clone(),
            cluster_uid: c.cluster_uuid.clone(),
            number_of_platinum_nodes: *number_of_platinum_nodes,
            number_of_enterprise_resource_units: *number_of_enterprise_resource_units,
        }),
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculation::{ClusterConsumption, ConsumptionDetail, LicenseMetadata};

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
                ConsumptionDetail::Enterprise { consumed: 8.0 },
            )),
            Ok(make_consumption(
                "b",
                "cluster-b",
                "uuid-b",
                "shared-uid",
                "enterprise",
                Some(24),
                None,
                ConsumptionDetail::Enterprise { consumed: 4.0 },
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
            ConsumptionDetail::Enterprise { consumed: 8.0 },
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
                ConsumptionDetail::Enterprise { consumed: 2.0 },
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
}
