## MODIFIED Requirements

### Requirement: Output YAML license consumption report
The CLI SHALL produce a YAML report summarising license consumption. The report SHALL contain a top-level `licenses` list. Each list item SHALL represent one Elasticsearch `license.uid`, include the license metadata, include the applicable licensed-capacity field when present, include a `total_consumed` field, and contain a `clusters` list describing the relevant consumption metrics for each contributing cluster. Each cluster entry SHALL include a `report_time` field containing the ISO 8601 UTC timestamp of when that cluster's API was queried.

```yaml
licenses:
  - name: "Example Production"
    uid: "123e4567-e89b-12d3-a456-426614174000"
    type: "enterprise"
    max_resource_units: 24
    total_consumed: 12
    clusters:
      - cluster_name: "prod-cluster-1"
        cluster_uid: "8d4f4efb-57de-4b4b-a8be-f1cbe2f7af63"
        consumed: 8.00
        report_time: "2026-04-01T10:00:00Z"
      - cluster_name: "prod-cluster-2"
        cluster_uid: "f5bc692a-7a10-43af-9162-bf7ac28d2ce1"
        consumed: 4.00
        report_time: "2026-04-01T10:00:05Z"
  - name: "Example Platinum"
    uid: "523e4567-e89b-12d3-a456-426614174999"
    type: "platinum"
    max_nodes: 12
    total_consumed: 7
    clusters:
      - cluster_name: "prod-cluster-3"
        cluster_uid: "a1132481-7f64-44ea-a84a-d83f2f4c3341"
        consumed: 7
        reason: "Total RAM used"
        report_time: "2026-04-01T10:00:02Z"
  - name: "Example Basic"
    uid: "623e4567-e89b-12d3-a456-426614174888"
    type: "basic"
    clusters:
      - cluster_name: "dev-cluster"
        cluster_uid: "e463dc1b-4396-42e6-b2dc-3d29a0dd2f93"
        number_of_platinum_nodes: 3
        number_of_enterprise_resource_units: 1.57
        report_time: "2026-04-01T10:00:08Z"
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
- **THEN** the cluster entry includes `consumed`, `reason`, and `report_time`

#### Scenario: Basic cluster includes fallback metrics
- **WHEN** a cluster entry belongs to a `basic` or unsupported license type
- **THEN** the cluster entry includes `number_of_platinum_nodes`, `number_of_enterprise_resource_units`, and `report_time` instead of a single `consumed` field

#### Scenario: report_time is present on every successful cluster entry
- **WHEN** a cluster is queried successfully
- **THEN** its entry in the report includes a `report_time` field with an ISO 8601 UTC timestamp

#### Scenario: total_consumed is present on every license entry
- **WHEN** a license entry is written to the report
- **THEN** it includes a `total_consumed` field representing the aggregated consumption across its clusters

### Requirement: Write report to stdout or file
The CLI SHALL write the YAML report to a file by default. When `--output` is omitted, the CLI SHALL use `report.yml` in the current working directory as the output path and SHALL print a notice to stderr indicating the path used (e.g. `Writing report to report.yml`). An explicit `--output <path>` flag overrides the default. If the resolved output file already exists and is a valid report, the CLI SHALL enter update mode (see report-update-mode spec). If the resolved output file exists but is not a valid report, the CLI SHALL prompt the user to overwrite or write to a different file. In non-interactive environments (stdin is not a TTY), the existing file SHALL be handled silently (update if valid report, overwrite otherwise).

#### Scenario: Default output path used
- **WHEN** `--output` is omitted
- **THEN** the YAML report is written to `report.yml` in the current directory and a notice is printed to stderr: `Writing report to report.yml`

#### Scenario: Explicit output path used
- **WHEN** `--output report.yaml` is specified
- **THEN** the YAML report is written to `report.yaml` and nothing is printed to stdout (except diagnostic messages on stderr)

#### Scenario: Output directory does not exist
- **WHEN** `--output /nonexistent/dir/report.yaml` is specified and the parent directory does not exist
- **THEN** the CLI exits with a non-zero code and a descriptive error

### Requirement: Overwrite confirmation prompt
When the resolved output file already exists, is NOT a valid report, and stdin is a TTY, the CLI SHALL prompt the user with a message indicating the file exists and offering two choices: overwrite (`o`, the default, selected by pressing Enter) or write to another file (`a`). The prompt SHALL loop until a valid choice is entered.

#### Scenario: User accepts overwrite (default)
- **WHEN** the output file exists and is not a valid report, the prompt is shown, and the user presses Enter or types `o`
- **THEN** the existing file is overwritten with the new report

#### Scenario: User chooses alternate file
- **WHEN** the output file exists and is not a valid report, the prompt is shown, and the user types `a`
- **THEN** the CLI asks for a new filename

#### Scenario: Alternate filename without extension
- **WHEN** the user provides a filename with no file extension (e.g. `my-report`)
- **THEN** the CLI appends `.yml` and writes to `my-report.yml`

#### Scenario: Alternate filename with .yml extension
- **WHEN** the user provides a filename ending in `.yml` (e.g. `my-report.yml`)
- **THEN** the CLI writes to `my-report.yml` without modification

#### Scenario: Alternate filename with .yaml extension
- **WHEN** the user provides a filename ending in `.yaml` (e.g. `my-report.yaml`)
- **THEN** the CLI writes to `my-report.yaml` without modification

#### Scenario: Alternate filename with other extension
- **WHEN** the user provides a filename with a non-YAML extension (e.g. `my-report.txt`)
- **THEN** the CLI writes to `my-report.txt` without modification

#### Scenario: Non-interactive environment with non-report file
- **WHEN** the output file exists, is not a valid report, and stdin is not a TTY
- **THEN** the CLI overwrites the existing file silently without prompting

### Requirement: Exit code reflects overall success
The CLI SHALL exit with code 0 when at least one cluster was queried successfully. It SHALL exit with a non-zero code when all clusters failed or a fatal configuration error occurred.

#### Scenario: All clusters failed
- **WHEN** every configured cluster produces an error
- **THEN** the CLI exits with a non-zero exit code

#### Scenario: Partial success
- **WHEN** at least one cluster was queried successfully and at least one failed
- **THEN** the CLI exits with code 0 and includes errors in the report

## ADDED Requirements

### Requirement: basic license entry omits total_consumed
For clusters whose `license.type` is `basic` or any other unsupported type, no single `consumed` value is defined, so the license entry SHALL omit the `total_consumed` field.

#### Scenario: Basic license entry has no total_consumed
- **WHEN** a license entry has `type: basic`
- **THEN** the `total_consumed` field is absent from that license entry
