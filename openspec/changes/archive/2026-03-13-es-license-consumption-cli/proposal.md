## Why

Elasticsearch license consumption is calculated based on cluster RAM usage, but there is currently no automated tool to query multiple clusters and produce a consolidated license usage report. This CLI fills that gap by automating data collection and calculation across any number of clusters defined in a simple config file.

## What Changes

- Introduce a new Rust CLI binary (`es-license-consumption`) as the primary deliverable of this repository.
- Add support for loading a user-supplied YAML file listing Elasticsearch clusters with connection parameters (host/port, basic auth or API key, SSL settings).
- Implement Elasticsearch API queries against `GET /`, `GET _nodes/stats?filter_path=**.mem.total_in_bytes,**.roles`, and `GET _license` to retrieve cluster identity, node RAM, node roles, and license metadata per cluster.
- Implement license consumption calculation logic based on normalized node RAM in GB, node roles, cluster identity, and cluster license metadata for `enterprise`, `platinum`, and fallback reporting for other license types.
- Produce a YAML output report as a list of licenses including the license name, UID, licensed capacity (`max_resource_units` or `max_nodes` when applicable), and a per-cluster breakdown of the relevant consumption metrics.

## Capabilities

### New Capabilities

- `cluster-config`: Read and validate a user-supplied YAML file describing one or more Elasticsearch clusters, including connection details (host, port), authentication (username/password or API key), and SSL configuration (insecure skip-verify or custom PEM certificate).
- `es-api-client`: Connect to each configured Elasticsearch cluster and query `/`, `_nodes/stats`, and `_license`, extracting cluster identity, node roles, node RAM, and cluster license metadata.
- `license-calculation`: Apply the defined business logic to translate normalized node RAM in GB and eligible node roles into the correct quantity of licenses consumed for each cluster, including enterprise ERU and platinum precedence rules.
- `report-output`: Serialize the calculated results into a YAML report file containing a list of licenses with their name, UID, licensed capacity, and a breakdown of consumption metrics per cluster.

### Modified Capabilities

## Impact

- New Rust binary and `Cargo.toml` dependencies (HTTP client, YAML serialization/deserialization, TLS support).
- No existing code is modified (greenfield implementation).
- Users must supply a cluster YAML config file as CLI input; output YAML is written to a user-specified path or stdout and groups clusters that share the same Elasticsearch `license.uid` under one license list entry.
