## ADDED Requirements

### Requirement: Calculate license consumption from normalized cluster data
The CLI SHALL apply the defined license calculation logic to the normalized API data collected from each cluster to produce the reportable consumption metrics for that cluster's license. The normalized input SHALL include per-node roles, per-node RAM in GB, and the cluster's `license.uid`, `license.type`, and `license.issued_to`.

#### Scenario: RAM data available for all nodes
- **WHEN** the API response contains RAM usage data for all cluster nodes
- **THEN** the calculation module returns the correct license quantity for each applicable license ID

#### Scenario: Partial node data
- **WHEN** the API response contains RAM data for only a subset of nodes (e.g., some nodes unreachable)
- **THEN** the calculation module computes the license quantity based on available data and marks the result as partial in the output

#### Scenario: Node roles are available for all nodes
- **WHEN** the normalized cluster data includes a roles array for each node
- **THEN** the calculation module can apply role-specific license rules to those nodes

### Requirement: Enterprise ERU calculation
For clusters whose `license.type` is `enterprise`, the CLI SHALL calculate consumed license quantity as the sum of all node memory values in GB divided by 64, rounded up to two decimal places.

#### Scenario: Enterprise cluster with whole-number ERU
- **WHEN** an enterprise cluster has a total normalized memory of 128 GB
- **THEN** the calculated consumption is `2.00`

#### Scenario: Enterprise cluster with fractional ERU
- **WHEN** an enterprise cluster has a total normalized memory of 100 GB
- **THEN** the calculated consumption is `1.57`

#### Scenario: Enterprise cluster rounds up at two decimals
- **WHEN** an enterprise cluster has a total normalized memory that yields more than two decimal places after division by 64
- **THEN** the CLI rounds the result upward to exactly two decimal places

### Requirement: Platinum calculation
For clusters whose `license.type` is `platinum`, the CLI SHALL consider only nodes that have at least one of these roles: `data`, `data_hot`, `data_warm`, `data_cold`, `data_content`, `ml`, or `master`. It SHALL compute both the qualifying node count and the qualifying RAM total in GB divided by 64 and rounded up to the nearest integer. The higher of those two values SHALL be reported as the cluster's consumed quantity.

#### Scenario: Platinum node count prevails
- **WHEN** a platinum cluster has 5 qualifying nodes and the qualifying RAM total produces `4` after dividing by 64 and rounding up to the nearest integer
- **THEN** the reported consumed quantity is `5`

#### Scenario: Platinum RAM total prevails
- **WHEN** a platinum cluster has 5 qualifying nodes and the qualifying RAM total produces `7` after dividing by 64 and rounding up to the nearest integer
- **THEN** the reported consumed quantity is `7`

#### Scenario: Non-qualifying roles are ignored for platinum
- **WHEN** a node has none of the qualifying platinum roles
- **THEN** that node is excluded from the platinum node count and platinum RAM total

### Requirement: Platinum reason reporting
For clusters whose `license.type` is `platinum`, the CLI SHALL record why the reported quantity was selected in a `reason` field. The value SHALL be `node count` when qualifying node count prevails and `Total RAM used` when the qualifying RAM total prevails.

#### Scenario: Platinum reason is node count
- **WHEN** the qualifying node count is greater than or equal to the rounded qualifying RAM quantity
- **THEN** the report includes `reason: "node count"`

#### Scenario: Platinum reason is total RAM used
- **WHEN** the rounded qualifying RAM quantity is greater than the qualifying node count
- **THEN** the report includes `reason: "Total RAM used"`

### Requirement: Basic and other license fallback metrics
For clusters whose `license.type` is `basic` or any other unsupported type, the CLI SHALL compute and report both of the following metrics without choosing one to prevail: `number of platinum nodes` using the platinum qualifying-role logic, and `number of Enterprise Resource Units` using all Elasticsearch nodes regardless of role.

#### Scenario: Basic license reports both fallback metrics
- **WHEN** a cluster has `license.type: basic`
- **THEN** the result includes both `number_of_platinum_nodes` and `number_of_enterprise_resource_units`

#### Scenario: Other license types report both fallback metrics
- **WHEN** a cluster has a license type other than `enterprise` or `platinum`
- **THEN** the result includes both `number_of_platinum_nodes` and `number_of_enterprise_resource_units`

### Requirement: License ID mapping
Each cluster's calculated consumption SHALL be associated with the `license.uid` returned by that cluster's `_license` endpoint. The associated `license.type`, `license.issued_to`, `license.max_resource_units`, and `license.max_nodes` values SHALL travel with the result so reports can display them alongside the aggregated usage. A single cluster MAY contribute to multiple license IDs only if a future calculation rule explicitly defines that behavior.

#### Scenario: Single license ID per cluster
- **WHEN** a cluster returns one `license.uid`
- **THEN** the full calculated consumption is attributed to that license ID

#### Scenario: Multiple license IDs per cluster
- **WHEN** a cluster's RAM usage spans multiple license IDs (e.g., different node roles map to different licenses)
- **THEN** each license ID receives the portion of consumption attributed to it independently

### Requirement: Graceful handling of missing calculation inputs
If required inputs for the calculation are missing due to API errors, the CLI SHALL produce a zero or marked-absent consumption value for the affected cluster rather than crashing.

#### Scenario: API error prevents calculation
- **WHEN** a cluster query fails and no data is available
- **THEN** the license consumption for that cluster is recorded as unavailable with an error reference, not as zero
