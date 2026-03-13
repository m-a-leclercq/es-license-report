## 1. Project Setup

- [x] 1.1 Update `Cargo.toml` with required dependencies: `tokio` (async runtime), `reqwest` (HTTP client with `rustls-tls` feature), `serde` + `serde_yaml` + `serde_json` (serialization), `clap` (CLI argument parsing), `anyhow` (error handling)
- [x] 1.2 Create module skeleton: `src/main.rs`, `src/config.rs`, `src/client.rs`, `src/calculation.rs`, `src/report.rs`

## 2. Cluster Configuration (`cluster-config`)

- [x] 2.1 Define `ClusterConfig` to match the provided YAML shape: a top-level mapping of cluster aliases to entries with `host`, `port`, `username`, `password`, `api_key`, `verify_certs`, and `ca_certs`
- [x] 2.2 Implement YAML deserialization for the top-level cluster-alias mapping using `serde` derive macros
- [x] 2.3 Normalize flat auth/TLS fields into internal types, using `api_key` when present and validating that `password` exists when `username` is set
- [x] 2.4 Implement config validation: require either `api_key` or `username`/`password`, and verify `ca_certs` exists if set
- [x] 2.5 Implement `load_config(path: &Path) -> Result<Vec<ClusterConfig>>` that reads, parses, validates, and preserves the user-defined cluster alias
- [x] 2.6 Write unit tests for config validation (missing auth, valid basic, valid api key, api key priority, missing password, missing PEM file)

## 3. Elasticsearch HTTP Client (`es-api-client`)

- [x] 3.1 Implement `build_client(cluster: &ClusterConfig) -> Result<reqwest::Client>` in `src/client.rs` that configures TLS mode (system roots, `verify_certs: false`, custom PEM) and sets a 20-second timeout
- [x] 3.2 Implement authentication injection: apply `Authorization: Basic <b64>` header for basic auth, `Authorization: ApiKey <key>` header for API key auth
- [x] 3.3 Implement query functions and response structs for `GET /`, `GET _nodes/stats?filter_path=**.mem.total_in_bytes,**.roles`, and `GET _license`
- [x] 3.4 Implement `query_cluster(cluster: &ClusterConfig) -> Result<ClusterData, ClusterError>` that starts with `GET /`, then queries the remaining endpoints, extracts `cluster_name` and `cluster_uuid`, and returns collected data or a structured error
- [x] 3.5 Implement `query_all_clusters(clusters: &[ClusterConfig]) -> Vec<ClusterResult>` using `tokio::join_all` for concurrent execution
- [x] 3.6 Normalize `nodes.*.os.mem.total_in_bytes` to GB and preserve each node's `roles` array in the cluster data model
- [x] 3.7 Preserve `license.uid`, `license.type`, `license.issued_to`, `license.max_resource_units`, and `license.max_nodes` in the cluster data model
- [x] 3.8 Write integration tests (or mock-based tests) for: successful query, auth failure (401), connection refused, timeout, malformed `GET /` response, malformed `_nodes/stats` response, malformed `_license` response

## 4. License Calculation (`license-calculation`)

- [x] 4.1 Define `ClusterData` to hold cluster identity (`cluster_name`, `cluster_uuid`), normalized per-node memory in GB, per-node roles, and the cluster license metadata (`uid`, `type`, `issued_to`, `max_resource_units`, `max_nodes`)
- [x] 4.2 Define `LicenseConsumption` struct with `license_id: String`, `quantity: Decimal or f64`, and the cluster identity fields needed for reporting
- [x] 4.3 Implement enterprise calculation in `src/calculation.rs`: sum all node memory in GB, divide by 64, and round up to two decimals
- [x] 4.4 Implement graceful handling for partial data: return a `PartialConsumption` variant when required fields are missing
- [x] 4.5 Implement platinum calculation using the qualifying roles set (`data`, `data_hot`, `data_warm`, `data_cold`, `data_content`, `ml`, `master`) and select the higher of qualifying node count vs rounded qualifying RAM/64
- [x] 4.6 Record platinum `reason` as `node count` or `Total RAM used` based on which value prevails
- [x] 4.7 Implement fallback reporting for `basic` and other license types: `number_of_platinum_nodes` plus `number_of_enterprise_resource_units`
- [x] 4.8 Add calculation dispatch by `license.type` for enterprise, platinum, and fallback behavior for all other license types
- [x] 4.9 Write unit tests for calculation logic covering: whole-number enterprise ERU, fractional enterprise ERU, round-up-at-two-decimals behavior, platinum node-count precedence, platinum RAM precedence, ignored non-qualifying roles, partial data, and zero RAM

## 5. Report Output (`report-output`)

- [x] 5.1 Define output structs in `src/report.rs`: `Report { licenses, errors }`, `LicenseEntry { name, uid, type, max_resource_units, max_nodes, clusters }`, and cluster output variants/fields for enterprise (`consumed`), platinum (`consumed`, `reason`), and fallback licenses (`number_of_platinum_nodes`, `number_of_enterprise_resource_units`)
- [x] 5.2 Implement `build_report(results: Vec<ClusterResult>) -> Report` that groups results by `license.uid`, carries forward license metadata and capacity fields, and appends one cluster item per contributing cluster
- [x] 5.3 Implement `write_report(report: &Report, output: Option<&Path>) -> Result<()>` that serializes to YAML and writes to stdout or the specified file
- [x] 5.4 Implement exit code logic: exit 0 if at least one cluster succeeded, non-zero if all clusters failed
- [x] 5.5 Write unit tests for report building: shared license across multiple clusters, enterprise `max_resource_units`, platinum `max_nodes` plus `reason`, fallback metrics for basic/other licenses, partial failure, and all failure

## 6. CLI Entrypoint

- [x] 6.1 Define CLI arguments in `src/main.rs` using `clap`: `--config <path>` (required), `--output <path>` (optional), `--timeout <secs>` (optional global default)
- [x] 6.2 Wire together: load config → build clients → query clusters → calculate consumption → build report → write report → exit with appropriate code
- [x] 6.3 Ensure secrets (passwords, API keys) are never logged or printed to stderr

## 7. Documentation & Validation

- [x] 7.1 Write a `README.md` with: installation instructions, example `clusters.yaml`, example output YAML, CLI flag reference
- [x] 7.2 Add an example `clusters.yaml` file in the repository root (with placeholder values, no real credentials)
- [x] 7.3 Run `cargo clippy` and resolve all warnings
- [x] 7.4 Run `cargo test` and confirm all tests pass
- [x] 7.5 Build a release binary with `cargo build --release` and verify it runs end-to-end against a test cluster
