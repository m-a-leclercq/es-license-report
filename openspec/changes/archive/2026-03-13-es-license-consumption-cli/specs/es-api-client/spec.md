## ADDED Requirements

### Requirement: Query Elasticsearch APIs per cluster
The CLI SHALL query exactly three Elasticsearch API endpoints for each configured cluster: `GET /`, `GET _nodes/stats?filter_path=**.mem.total_in_bytes,**.roles`, and `GET _license`.

#### Scenario: Successful API query
- **WHEN** a cluster is reachable and returns the expected API response
- **THEN** the CLI extracts cluster identity, node roles, node memory totals, and cluster license metadata from the responses and passes them to the license calculation module

#### Scenario: Root endpoint is queried first
- **WHEN** the CLI begins querying a cluster
- **THEN** it issues `GET /` before issuing `_nodes/stats` or `_license` for that cluster

#### Scenario: Cluster unreachable
- **WHEN** a cluster's host is unreachable (connection refused, DNS failure, timeout)
- **THEN** the CLI records an error for that cluster, does not abort the overall run, and continues querying remaining clusters

#### Scenario: Authentication failure
- **WHEN** an Elasticsearch cluster returns HTTP 401 or 403
- **THEN** the CLI records an authentication error for that cluster and continues with the remaining clusters

#### Scenario: Unexpected API response shape
- **WHEN** a required field is absent or has an unexpected type in the API response
- **THEN** the CLI records a parse error for that cluster and continues with the remaining clusters

### Requirement: Extract and normalize node RAM and roles
The CLI SHALL read `nodes.*.roles` and `nodes.*.os.mem.total_in_bytes` from the `_nodes/stats` response for every node returned by the cluster. It SHALL normalize memory values from bytes to GB before passing them to the calculation module.

#### Scenario: Node stats response includes multiple nodes
- **WHEN** the `_nodes/stats` response contains multiple node entries under `nodes`
- **THEN** the CLI collects each node's `roles` array and `os.mem.total_in_bytes` value independently

#### Scenario: Node memory is normalized to GB
- **WHEN** the `_nodes/stats` response contains `os.mem.total_in_bytes`
- **THEN** the CLI converts that value from bytes into GB before storing it in the normalized cluster data model

### Requirement: Extract cluster license metadata
The CLI SHALL read `license.uid`, `license.type`, `license.issued_to`, `license.max_resource_units`, and `license.max_nodes` from the `_license` response for each cluster and preserve those values through report generation.

#### Scenario: License metadata is available
- **WHEN** the `_license` response contains `uid`, `type`, `issued_to`, and the relevant licensed-capacity field for the license type
- **THEN** the CLI stores those values in the cluster result and uses `license.uid` as the report aggregation key

#### Scenario: License UID missing
- **WHEN** the `_license` response omits `license.uid`
- **THEN** the CLI records a parse error for that cluster because the report cannot aggregate consumption without a license identifier

### Requirement: Extract cluster identity
The CLI SHALL read `cluster_name` and `cluster_uuid` from the `GET /` response for each cluster and preserve those values through report generation.

#### Scenario: Cluster identity is available
- **WHEN** the `GET /` response contains `cluster_name` and `cluster_uuid`
- **THEN** the CLI stores those values in the cluster result and exposes `cluster_uuid` as the cluster UID in the report

#### Scenario: Cluster UUID missing
- **WHEN** the `GET /` response omits `cluster_uuid`
- **THEN** the CLI records a parse error for that cluster because the report cannot identify the contributing cluster

### Requirement: Concurrent cluster queries
The CLI SHALL query all configured clusters concurrently rather than sequentially to minimise total execution time.

#### Scenario: Multiple clusters queried concurrently
- **WHEN** the config file contains multiple cluster entries
- **THEN** all cluster API requests are initiated concurrently and the CLI waits for all to complete before generating the report

### Requirement: Configurable HTTP timeout
The CLI SHALL apply a 20-second timeout to each cluster query workflow.

#### Scenario: Request exceeds timeout
- **WHEN** a cluster does not respond within 20 seconds
- **THEN** the CLI records a timeout error for that cluster and continues with the remaining clusters
