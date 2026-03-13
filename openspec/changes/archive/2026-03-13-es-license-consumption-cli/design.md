## Context

This is a greenfield Rust CLI project. The repository already has a `Cargo.toml` stub and a `src/` directory. There are no existing services or APIs to integrate with beyond the Elasticsearch clusters that end users will provide. The tool is designed to be run ad-hoc or in automation pipelines (CI, cron) by Elasticsearch operators or license administrators.

## Goals / Non-Goals

**Goals:**
- Produce a single self-contained binary that reads a YAML cluster list and outputs a YAML license consumption report.
- Support basic auth (username/password) and API key authentication.
- Support TLS with insecure skip-verify or a user-supplied PEM certificate.
- Implement the agreed Elasticsearch API queries to collect RAM usage and license metrics.
- Implement the license calculation logic to derive license quantities from raw cluster data.
- Emit structured errors when a cluster is unreachable or returns unexpected data, without aborting the entire run.

**Non-Goals:**
- Persistent storage or database backends.
- Real-time monitoring or daemon mode.
- Modifying Elasticsearch cluster state or license assignments.
- Supporting Elasticsearch versions without the required API endpoints.

## Decisions

### D1 – Language: Rust
**Decision**: Implement as a Rust binary.
**Rationale**: Rust provides a single statically-linked binary with no runtime dependency, strong typing for config/API models, and excellent async HTTP support. This is also the language already established by the repository structure.
**Alternatives considered**: Go (similar binary model, less expressive type system for this use case); Python (easier prototyping but requires runtime).

### D2 – HTTP Client: `reqwest` (async, with `rustls` backend)
**Decision**: Use `reqwest` with `rustls` for all Elasticsearch HTTP calls.
**Rationale**: `reqwest` is the de-facto async HTTP client for Rust with first-class TLS support (native-tls or rustls). `rustls` is preferred to avoid OpenSSL linkage issues across platforms.
**Alternatives considered**: `ureq` (sync, simpler) – rejected because concurrent cluster queries benefit from async parallelism.

### D3 – Concurrency: `tokio` runtime, concurrent cluster queries
**Decision**: Query all clusters concurrently using `tokio::spawn` / `futures::join_all`.
**Rationale**: License reporting tools are expected to run against potentially many clusters; sequential queries would be slow. Errors from one cluster must not block others.

### D4 – Configuration format: YAML (serde + serde_yaml)
**Decision**: Both the input cluster list and the output report use YAML.
**Rationale**: YAML is human-readable, supports comments (useful in config files), and is widely used in operations tooling. `serde_yaml` integrates cleanly with `serde` derive macros already used for JSON API response deserialization.
**Implementation**: The input config is a top-level mapping where each key is a user-defined cluster alias (for example `example_cluster`) and each value contains flat connection settings: `host`, `port`, `username`, `password`, `api_key`, `verify_certs`, and `ca_certs`.

### D5 – TLS configuration
**Decision**: Support three TLS modes per cluster: system roots (default), insecure (skip verify), custom PEM file.
**Rationale**: Enterprise environments often use internal CAs; skip-verify is needed for dev/testing.
**Implementation**: Map `verify_certs: false` to `.danger_accept_invalid_certs(true)`. Map `ca_certs` to `.add_root_certificate()` when provided. When `verify_certs` is omitted or `true`, validate against system trust roots unless `ca_certs` is supplied.

### D6 – Authentication
**Decision**: Support flat auth fields per cluster entry: `username`, `password`, and `api_key`.
**Rationale**: Elasticsearch supports both; operators may manage clusters with different auth strategies.
**Implementation**: Normalize the flat config into an internal enum `Auth { Basic { username, password }, ApiKey(String) }`. If `api_key` is set, it takes priority over `username`/`password`. If `username` is provided without `password`, treat it as a validation error.

### D7 – Error handling strategy
**Decision**: Per-cluster errors (connection failures, auth errors, unexpected API responses) are collected and included in the output report as an `errors` field rather than causing a non-zero exit code, unless ALL clusters fail.
**Rationale**: Partial results are valuable; a single unreachable cluster should not suppress data from the others. If all clusters fail, exit non-zero so CI pipelines can detect total failure.

### D8 – Elasticsearch endpoint contract
**Decision**: Query exactly three Elasticsearch endpoints per cluster: `GET /`, `GET _nodes/stats?filter_path=**.mem.total_in_bytes,**.roles`, and `GET _license`.
**Rationale**: The root endpoint provides cluster identity for the report, the node stats endpoint provides the minimum data needed for license calculations while limiting payload size with `filter_path`, and the license endpoint supplies the aggregation key and licensed-capacity metadata required for the final report.
**Implementation**: For each cluster, call `GET /` first to establish cluster identity, then call `_nodes/stats` and `_license`. Deserialize `GET /` into a structure carrying `cluster_name` and `cluster_uuid`. Deserialize `_nodes/stats` into a per-node structure containing `roles` and `os.mem.total_in_bytes`, then normalize memory to GB before handing data to the calculation module. Deserialize `_license` into a structure carrying `license.uid`, `license.type`, `license.issued_to`, `license.max_resource_units`, and `license.max_nodes`.

### D8a – Timeout and concurrency
**Decision**: Query all configured clusters concurrently with a default per-cluster timeout of 20 seconds.
**Rationale**: Users may supply many clusters, so parallel execution is required for acceptable runtime. A 20-second timeout matches the requested operational behavior and prevents hung requests from blocking the report.
**Implementation**: Build one async task per configured cluster. Each cluster task uses a `reqwest::Client` configured with a 20-second timeout unless a future change explicitly makes it configurable.

### D9 – License calculation isolation
**Decision**: Keep the license consumption formula in a dedicated Rust module (`src/calculation.rs`) fed by normalized cluster data.
**Rationale**: Different license types can have different business rules. Isolating the formula keeps the API client and reporting layers stable while allowing license-type-specific calculations to evolve independently.
**Implementation**: Define a `ClusterData` model that includes cluster identity, normalized node memory in GB, node roles, and the cluster's license metadata. The calculation module dispatches by `license.type`; for `enterprise`, it sums all node memory in GB, divides by 64, and rounds up to two decimal places to produce ERU consumption. For `platinum`, it filters nodes by the eligible role set (`data`, `data_hot`, `data_warm`, `data_cold`, `data_content`, `ml`, `master`), computes both the qualifying node count and `ceil(total qualifying RAM in GB / 64)`, and uses whichever value is higher while recording the reason. For `basic` and any other license type, it reports both the platinum-style node count and the enterprise-style ERU total without choosing one as the prevailing metric.

### D10 – Report structure
**Decision**: Emit the final YAML report as a top-level list of license entries instead of a mapping keyed by UID.
**Rationale**: A list keeps related metadata together in one portable record and directly matches the requested output shape of "a list of licenses" each containing a list of clusters.
**Implementation**: Each license entry contains `name`, `uid`, `type`, the relevant capacity field when present, and a `clusters` list. Cluster entries vary by license type: enterprise clusters report ERU consumption; platinum clusters report the prevailing consumption value plus a `reason` field (`node count` or `Total RAM used`); basic or other license types report both `number_of_platinum_nodes` and `number_of_enterprise_resource_units`.

## Risks / Trade-offs

- **Risk**: Elasticsearch API shape differs between major versions → Mitigation: use lenient deserialization (`serde` with `#[serde(default)]` and `Option<>` fields); emit a warning when expected fields are absent.
- **Risk**: Large clusters with many nodes could produce slow or large API responses → Mitigation: use `filter_path` on `_nodes/stats`, set a 20-second per-cluster timeout, and only retain the fields needed for calculation.
- **Risk**: Secrets (passwords, API keys) in the YAML config file → Mitigation: document that the config file should be secrets-managed (e.g., mounted from a vault); the tool itself never logs credential values.
- **Risk**: `rustls` may not support some enterprise TLS configurations (e.g., older cipher suites) → Mitigation: document the constraint; consider a feature flag for `native-tls` if needed.
- **Risk**: Rounding behavior for enterprise ERU could be implemented inconsistently → Mitigation: centralize rounding in one helper and cover it with unit tests using representative decimal edge cases.
- **Risk**: Platinum eligibility could be miscomputed if role matching differs across node role combinations → Mitigation: implement role matching against an explicit constant set and cover mixed-role cases with unit tests.

## Migration Plan

This is a greenfield project; no migration is required. The binary is distributed as a compiled artifact. Users adopt it by pointing it at their cluster YAML config.

## Open Questions

- Should the output YAML be written to a file (with `--output` flag) and/or to stdout? Recommend: default to stdout with optional `--output <file>` flag.
- Should per-cluster query errors be surfaced inline in the report YAML or printed to stderr only? Recommend: inline in report under an `errors` key.
