/// Qualifying roles for Platinum license calculation (and fallback platinum node count).
const PLATINUM_QUALIFYING_ROLES: &[&str] = &[
    "data",
    "data_hot",
    "data_warm",
    "data_cold",
    "data_content",
    "ml",
    "master",
];

/// Per-node data extracted and normalized from the `_nodes/stats` API response.
#[derive(Debug, Clone)]
pub struct NodeData {
    pub roles: Vec<String>,
    /// RAM in GB (`os.mem.total_in_bytes` / 1 GiB), or `None` if the field was absent.
    pub memory_gb: Option<f64>,
}

/// License metadata from the `_license` API response.
#[derive(Debug, Clone)]
pub struct LicenseMetadata {
    pub uid: String,
    pub license_type: String,
    pub issued_to: String,
    pub max_resource_units: Option<u32>,
    pub max_nodes: Option<u32>,
}

/// Normalized data collected from a single Elasticsearch cluster.
#[derive(Debug, Clone)]
pub struct ClusterData {
    pub cluster_name: String,
    pub cluster_uuid: String,
    pub nodes: Vec<NodeData>,
    pub license: LicenseMetadata,
}

/// Per-license-type consumption breakdown for a single cluster.
#[derive(Debug, Clone)]
pub enum ConsumptionDetail {
    Enterprise {
        /// ERUs consumed: sum of all node RAM in GB / 64, rounded up to 2 decimal places.
        consumed: f64,
    },
    Platinum {
        /// Max of qualifying node count vs. ceil(qualifying RAM GB / 64).
        consumed: u64,
        /// `"node count"` or `"Total RAM used"`.
        reason: String,
    },
    Fallback {
        number_of_platinum_nodes: u64,
        number_of_enterprise_resource_units: f64,
    },
}

/// Calculated license consumption for one cluster.
#[derive(Debug, Clone)]
pub struct ClusterConsumption {
    pub cluster_name: String,
    pub cluster_uuid: String,
    pub license: LicenseMetadata,
    pub detail: ConsumptionDetail,
    /// `true` when some nodes had no memory data in the API response.
    pub is_partial: bool,
}

/// Calculate license consumption from normalized cluster data.
pub fn calculate(data: ClusterData) -> ClusterConsumption {
    let is_partial = data.nodes.iter().any(|n| n.memory_gb.is_none());

    let detail = match data.license.license_type.as_str() {
        "enterprise" => {
            let total_gb: f64 = data.nodes.iter().filter_map(|n| n.memory_gb).sum();
            ConsumptionDetail::Enterprise {
                consumed: round_up_2_decimals(total_gb / 64.0),
            }
        }
        "platinum" => {
            let qualifying: Vec<&NodeData> = data
                .nodes
                .iter()
                .filter(|n| is_qualifying(n))
                .collect();

            let node_count = qualifying.len() as u64;
            let qualifying_ram_gb: f64 = qualifying.iter().filter_map(|n| n.memory_gb).sum();
            let ram_quantity = (qualifying_ram_gb / 64.0).ceil() as u64;

            let (consumed, reason) = if node_count >= ram_quantity {
                (node_count, "node count".to_string())
            } else {
                (ram_quantity, "Total RAM used".to_string())
            };

            ConsumptionDetail::Platinum { consumed, reason }
        }
        _ => {
            // basic + all other license types: report both metrics without choosing one
            let platinum_nodes = data.nodes.iter().filter(|n| is_qualifying(n)).count() as u64;
            let total_gb: f64 = data.nodes.iter().filter_map(|n| n.memory_gb).sum();
            ConsumptionDetail::Fallback {
                number_of_platinum_nodes: platinum_nodes,
                number_of_enterprise_resource_units: round_up_2_decimals(total_gb / 64.0),
            }
        }
    };

    ClusterConsumption {
        cluster_name: data.cluster_name,
        cluster_uuid: data.cluster_uuid,
        license: data.license,
        detail,
        is_partial,
    }
}

fn is_qualifying(node: &NodeData) -> bool {
    node.roles
        .iter()
        .any(|r| PLATINUM_QUALIFYING_ROLES.contains(&r.as_str()))
}

/// Round `x` up to exactly two decimal places.
/// e.g. 1.5625 → 1.57, 2.0 → 2.00, 0.001 → 0.01
fn round_up_2_decimals(x: f64) -> f64 {
    (x * 100.0).ceil() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_data(license_type: &str, nodes: Vec<NodeData>) -> ClusterData {
        ClusterData {
            cluster_name: "test-cluster".to_string(),
            cluster_uuid: "uuid-1".to_string(),
            nodes,
            license: LicenseMetadata {
                uid: "uid-1".to_string(),
                license_type: license_type.to_string(),
                issued_to: "Test Corp".to_string(),
                max_resource_units: Some(24),
                max_nodes: Some(12),
            },
        }
    }

    fn node(roles: &[&str], memory_gb: Option<f64>) -> NodeData {
        NodeData {
            roles: roles.iter().map(|r| r.to_string()).collect(),
            memory_gb,
        }
    }

    // Enterprise tests

    #[test]
    fn enterprise_whole_number_eru() {
        // 128 GB / 64 = 2.00
        let data = make_data(
            "enterprise",
            vec![node(&["master", "data"], Some(64.0)), node(&["data"], Some(64.0))],
        );
        let result = calculate(data);
        assert!(matches!(result.detail, ConsumptionDetail::Enterprise { consumed } if consumed == 2.0));
    }

    #[test]
    fn enterprise_fractional_eru() {
        // 100 GB / 64 = 1.5625 → round up to 2 decimals → 1.57
        let data = make_data("enterprise", vec![node(&["data"], Some(100.0))]);
        let result = calculate(data);
        assert!(matches!(result.detail, ConsumptionDetail::Enterprise { consumed } if (consumed - 1.57).abs() < 1e-10));
    }

    #[test]
    fn enterprise_round_up_at_two_decimals() {
        // 64.001 GB / 64 = 1.0000156... → ceil at 2 dec → 1.01
        let data = make_data("enterprise", vec![node(&["data"], Some(64.001))]);
        let result = calculate(data);
        assert!(matches!(result.detail, ConsumptionDetail::Enterprise { consumed } if (consumed - 1.01).abs() < 1e-10));
    }

    #[test]
    fn enterprise_zero_ram() {
        let data = make_data("enterprise", vec![node(&["data"], Some(0.0))]);
        let result = calculate(data);
        assert!(matches!(result.detail, ConsumptionDetail::Enterprise { consumed } if consumed == 0.0));
    }

    // Platinum tests

    #[test]
    fn platinum_node_count_prevails() {
        // 5 qualifying nodes, qualifying RAM = 5 * 50 = 250 GB → ceil(250/64) = ceil(3.906) = 4
        // node_count=5 >= ram_quantity=4 → consumed=5, reason="node count"
        let nodes: Vec<NodeData> = (0..5).map(|_| node(&["data"], Some(50.0))).collect();
        let data = make_data("platinum", nodes);
        let result = calculate(data);
        match result.detail {
            ConsumptionDetail::Platinum { consumed, reason } => {
                assert_eq!(consumed, 5);
                assert_eq!(reason, "node count");
            }
            _ => panic!("expected Platinum"),
        }
    }

    #[test]
    fn platinum_ram_prevails() {
        // 5 qualifying nodes, qualifying RAM = 5 * 90 = 450 GB → ceil(450/64) = ceil(7.03) = 8
        // node_count=5 < ram_quantity=8 → consumed=8, reason="Total RAM used"
        let nodes: Vec<NodeData> = (0..5).map(|_| node(&["data"], Some(90.0))).collect();
        let data = make_data("platinum", nodes);
        let result = calculate(data);
        match result.detail {
            ConsumptionDetail::Platinum { consumed, reason } => {
                assert_eq!(consumed, 8);
                assert_eq!(reason, "Total RAM used");
            }
            _ => panic!("expected Platinum"),
        }
    }

    #[test]
    fn platinum_non_qualifying_roles_ignored() {
        // 3 qualifying (data) + 2 non-qualifying (ingest, coordinating)
        // RAM: 3 * 64 = 192 GB → ceil(192/64) = 3
        // node_count=3, ram_quantity=3 → node_count >= ram_quantity → reason="node count"
        let data = make_data(
            "platinum",
            vec![
                node(&["data"], Some(64.0)),
                node(&["data"], Some(64.0)),
                node(&["data"], Some(64.0)),
                node(&["ingest"], Some(64.0)),
                node(&["coordinating_only"], Some(64.0)),
            ],
        );
        let result = calculate(data);
        match result.detail {
            ConsumptionDetail::Platinum { consumed, reason } => {
                assert_eq!(consumed, 3);
                assert_eq!(reason, "node count");
            }
            _ => panic!("expected Platinum"),
        }
    }

    #[test]
    fn partial_data_handled_gracefully() {
        // One node has no memory data — should compute on available nodes, mark is_partial=true
        let data = make_data(
            "enterprise",
            vec![node(&["data"], Some(64.0)), node(&["data"], None)],
        );
        let result = calculate(data);
        assert!(result.is_partial);
        // Only 64 GB counted → 64/64 = 1.00
        assert!(matches!(result.detail, ConsumptionDetail::Enterprise { consumed } if consumed == 1.0));
    }

    // Fallback / basic tests

    #[test]
    fn basic_license_reports_fallback_metrics() {
        // 2 qualifying (data), 1 non-qualifying (ingest), total RAM = 3 * 64 = 192 GB → ERU 3.00
        let data = make_data(
            "basic",
            vec![
                node(&["data"], Some(64.0)),
                node(&["data"], Some(64.0)),
                node(&["ingest"], Some(64.0)),
            ],
        );
        let result = calculate(data);
        match result.detail {
            ConsumptionDetail::Fallback {
                number_of_platinum_nodes,
                number_of_enterprise_resource_units,
            } => {
                assert_eq!(number_of_platinum_nodes, 2);
                assert!((number_of_enterprise_resource_units - 3.0).abs() < 1e-10);
            }
            _ => panic!("expected Fallback"),
        }
    }

    #[test]
    fn unknown_license_type_uses_fallback() {
        let data = make_data("trial", vec![node(&["master"], Some(32.0))]);
        let result = calculate(data);
        assert!(matches!(result.detail, ConsumptionDetail::Fallback { .. }));
    }
}
