## ADDED Requirements

### Requirement: Load cluster list from YAML file
The CLI SHALL accept a path to a YAML configuration file that lists one or more Elasticsearch clusters to query. The file SHALL be validated on startup, and the CLI SHALL exit with a descriptive error if the file is missing, unreadable, or fails schema validation.

#### Scenario: Valid config file is loaded
- **WHEN** the user passes `--config clusters.yaml` and the file is valid YAML matching the schema
- **THEN** the CLI parses all cluster entries without error and proceeds to query each cluster

#### Scenario: Config file not found
- **WHEN** the user passes `--config clusters.yaml` and the file does not exist
- **THEN** the CLI exits with a non-zero code and prints an error indicating the file path was not found

#### Scenario: Config file has invalid YAML
- **WHEN** the config file contains malformed YAML
- **THEN** the CLI exits with a non-zero code and prints the YAML parse error with line/column information

### Requirement: Cluster entry schema
The YAML config SHALL be a top-level mapping where each key is a user-defined cluster alias and each value is a cluster entry. Each cluster entry SHALL contain at minimum `host` and `port`. Authentication and TLS fields SHALL be flat fields on the cluster entry rather than nested blocks.

```yaml
example_cluster:
  host: https://example.com
  port: 9200
  username: user
  password: pass
  api_key: null
  verify_certs: true
  ca_certs: /path/to/ca.crt
```

#### Scenario: Basic auth cluster entry
- **WHEN** a cluster entry specifies `username` and `password` and does not specify `api_key`
- **THEN** all HTTP requests to that cluster use HTTP Basic authentication with those credentials

#### Scenario: API key cluster entry
- **WHEN** a cluster entry specifies `api_key` with a key string
- **THEN** all HTTP requests to that cluster include the `Authorization: ApiKey <value>` header

#### Scenario: Cluster entry missing authentication
- **WHEN** a cluster entry contains neither a valid `api_key` nor a valid `username`/`password` pair
- **THEN** the CLI exits with a validation error identifying the offending cluster entry

#### Scenario: API key takes priority
- **WHEN** a cluster entry specifies both `api_key` and `username`/`password`
- **THEN** the CLI uses `api_key` authentication for that cluster and ignores the basic auth fields

#### Scenario: Username without password
- **WHEN** a cluster entry specifies `username` but omits `password`
- **THEN** the CLI exits with a validation error identifying the offending cluster entry

### Requirement: TLS configuration per cluster
Each cluster entry SHALL support three TLS modes using flat fields: system roots (default), insecure skip-verify through `verify_certs: false`, or a custom PEM certificate authority through `ca_certs`.

#### Scenario: Default TLS (system roots)
- **WHEN** `verify_certs` is omitted or `true` and `ca_certs` is not set
- **THEN** the HTTP client validates the server certificate against the system trust store

#### Scenario: Insecure TLS
- **WHEN** `verify_certs: false` is set in a cluster entry
- **THEN** the HTTP client skips TLS certificate verification for that cluster

#### Scenario: Custom CA PEM
- **WHEN** `ca_certs` is set to a valid PEM file path
- **THEN** the HTTP client uses the provided certificate as the trusted CA for that cluster

#### Scenario: Custom CA PEM file not found
- **WHEN** `ca_certs` points to a file that does not exist
- **THEN** the CLI exits with a non-zero code and a descriptive error indicating the missing file
