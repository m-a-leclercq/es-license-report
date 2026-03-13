## ADDED Requirements

### Requirement: Output YAML license consumption report
The CLI SHALL produce a YAML report summarising license consumption. The report SHALL contain a top-level `licenses` list. Each list item SHALL represent one Elasticsearch `license.uid`, include the license metadata, include the applicable licensed-capacity field when present, and contain a `clusters` list describing the relevant consumption metrics for each contributing cluster.

```yaml
licenses:
  - name: "Example Production"
    uid: "123e4567-e89b-12d3-a456-426614174000"
    type: "enterprise"
    max_resource_units: 24
    clusters:
      - cluster_name: "prod-cluster-1"
        cluster_uid: "8d4f4efb-57de-4b4b-a8be-f1cbe2f7af63"
        consumed: 8.00
      - cluster_name: "prod-cluster-2"
        cluster_uid: "f5bc692a-7a10-43af-9162-bf7ac28d2ce1"
        consumed: 4.00
  - name: "Example Platinum"
    uid: "523e4567-e89b-12d3-a456-426614174999"
    type: "platinum"
    max_nodes: 12
    clusters:
      - cluster_name: "prod-cluster-3"
        cluster_uid: "a1132481-7f64-44ea-a84a-d83f2f4c3341"
        consumed: 7
        reason: "Total RAM used"
  - name: "Example Basic"
    uid: "623e4567-e89b-12d3-a456-426614174888"
    type: "basic"
    clusters:
      - cluster_name: "dev-cluster"
        cluster_uid: "e463dc1b-4396-42e6-b2dc-3d29a0dd2f93"
        number_of_platinum_nodes: 3
        number_of_enterprise_resource_units: 1.57
errors:
  - cluster: "broken-cluster"
    message: "connection refused"
```

#### Scenario: All clusters successful
- **WHEN** all clusters are queried successfully and calculations complete
- **THEN** the report contains a `licenses` list with no `errors` key (or an empty `errors` list)

#### Scenario: Some clusters failed
- **WHEN** one or more clusters produced errors during query or calculation
- **THEN** the report contains both a `licenses` list for successful clusters and an `errors` list for the failed clusters

#### Scenario: Multiple clusters share the same license
- **WHEN** two or more clusters return the same `license.uid`
- **THEN** the report contains a single license list entry for that UID and one cluster breakdown item per contributing cluster

#### Scenario: Enterprise license capacity is reported
- **WHEN** a license entry has `type: enterprise`
- **THEN** the report includes `max_resource_units` and omits `max_nodes`

#### Scenario: Platinum license capacity is reported
- **WHEN** a license entry has `type: platinum`
- **THEN** the report includes `max_nodes` and omits `max_resource_units`

#### Scenario: Platinum cluster includes selection reason
- **WHEN** a cluster entry belongs to a `platinum` license
- **THEN** the cluster entry includes `consumed` and `reason`

#### Scenario: Basic cluster includes fallback metrics
- **WHEN** a cluster entry belongs to a `basic` or unsupported license type
- **THEN** the cluster entry includes `number_of_platinum_nodes` and `number_of_enterprise_resource_units` instead of a single `consumed` field

### Requirement: Write report to stdout or file
The CLI SHALL write the YAML report to stdout by default. An optional `--output <path>` flag SHALL redirect the report to a file. If the output file already exists it SHALL be overwritten.

#### Scenario: Default stdout output
- **WHEN** `--output` is not specified
- **THEN** the YAML report is printed to stdout

#### Scenario: Output to file
- **WHEN** `--output report.yaml` is specified
- **THEN** the YAML report is written to `report.yaml` and nothing is printed to stdout (except diagnostic messages on stderr)

#### Scenario: Output directory does not exist
- **WHEN** `--output /nonexistent/dir/report.yaml` is specified and the parent directory does not exist
- **THEN** the CLI exits with a non-zero code and a descriptive error

### Requirement: Exit code reflects overall success
The CLI SHALL exit with code 0 when at least one cluster was queried successfully. It SHALL exit with a non-zero code when all clusters failed or a fatal configuration error occurred.

#### Scenario: All clusters failed
- **WHEN** every configured cluster produces an error
- **THEN** the CLI exits with a non-zero exit code

#### Scenario: Partial success
- **WHEN** at least one cluster was queried successfully and at least one failed
- **THEN** the CLI exits with code 0 and includes errors in the report
