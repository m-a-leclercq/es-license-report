# es-license-consumption

A command-line tool that queries one or more Elasticsearch clusters, calculates license consumption based on node RAM and roles, and emits a consolidated YAML report.

## Installation

### From source

Requires Rust 1.85+ (edition 2024).

```bash
cargo build --release
# Binary is at target/release/es-license-consumption
```

Copy the binary to a location on your `$PATH`, for example:

```bash
cp target/release/es-license-consumption /usr/local/bin/
```

## Usage

```
es-license-consumption --config <path> [--output <path>] [--timeout <secs>]
```

### CLI flags

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--config <path>` | yes | — | Path to the cluster YAML configuration file |
| `--output <path>` | no | stdout | Write the YAML report to this file |
| `--timeout <secs>` | no | 20 | Per-cluster HTTP request timeout in seconds |

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | At least one cluster was queried successfully |
| `1` | All clusters failed |
| `2` | Fatal configuration or I/O error |

## Cluster configuration file

The config file is a YAML mapping where each key is a user-defined cluster alias. Each cluster entry supports the following fields:

| Field | Required | Description |
|-------|----------|-------------|
| `host` | yes | Scheme + hostname (e.g. `https://my-es.example.com`) |
| `port` | yes | Port number (e.g. `9200`) |
| `username` | one of* | Username for HTTP Basic authentication |
| `password` | one of* | Password for HTTP Basic authentication (required when `username` is set) |
| `api_key` | one of* | Elasticsearch API key (`api_key` takes priority over `username`/`password`) |
| `verify_certs` | no | `true` (default) validates the server cert against system trust store; `false` disables TLS verification |
| `ca_certs` | no | Path to a PEM certificate to use as the trusted CA |

\* Each cluster entry must have either `api_key` **or** both `username` and `password`.

### Example `clusters.yaml`

```yaml
production_us:
  host: https://prod-es-us.example.com
  port: 9200
  api_key: YOUR_API_KEY_HERE

staging:
  host: https://staging-es.example.com
  port: 9200
  username: elastic
  password: CHANGE_ME

dev_local:
  host: http://localhost
  port: 9200
  api_key: dev-key
  verify_certs: false

corp_internal:
  host: https://es.corp.internal
  port: 9200
  username: admin
  password: CHANGE_ME
  ca_certs: /etc/ssl/certs/corp-ca.pem
```

> **Security note:** Keep your config file secret. Store credentials in a secrets manager (Vault, AWS Secrets Manager, etc.) and mount the file at runtime rather than committing it to version control.

## Example output

```yaml
licenses:
  - name: "Example Production"
    uid: "123e4567-e89b-12d3-a456-426614174000"
    type: enterprise
    max_resource_units: 24
    clusters:
      - cluster_name: prod-cluster-1
        cluster_uid: 8d4f4efb-57de-4b4b-a8be-f1cbe2f7af63
        consumed: 8.0
      - cluster_name: prod-cluster-2
        cluster_uid: f5bc692a-7a10-43af-9162-bf7ac28d2ce1
        consumed: 4.0
  - name: "Example Platinum"
    uid: "523e4567-e89b-12d3-a456-426614174999"
    type: platinum
    max_nodes: 12
    clusters:
      - cluster_name: prod-cluster-3
        cluster_uid: a1132481-7f64-44ea-a84a-d83f2f4c3341
        consumed: 7
        reason: Total RAM used
  - name: "Example Basic"
    uid: "623e4567-e89b-12d3-a456-426614174888"
    type: basic
    clusters:
      - cluster_name: dev-cluster
        cluster_uid: e463dc1b-4396-42e6-b2dc-3d29a0dd2f93
        number_of_platinum_nodes: 3
        number_of_enterprise_resource_units: 1.57
errors:
  - cluster: broken-cluster
    message: "connection refused"
```

### License calculation rules

| License type | Calculation |
|---|---|
| `enterprise` | `ceil(total_node_ram_gb / 64, 2)` → Enterprise Resource Units (ERU) |
| `platinum` | `max(qualifying_node_count, ceil(qualifying_ram_gb / 64))` where qualifying roles are: `data`, `data_hot`, `data_warm`, `data_cold`, `data_content`, `ml`, `master` |
| `basic` / other | Reports both `number_of_platinum_nodes` and `number_of_enterprise_resource_units` |

Clusters that share the same `license.uid` are grouped under one license entry in the report.
