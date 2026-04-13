## Requirements

### Requirement: Detect existing report file for update mode
When the resolved output path points to an existing file that can be parsed as a valid YAML license consumption report, the CLI SHALL enter update mode instead of the standard overwrite/alternate-file prompt.  If the file exists but cannot be parsed as a valid report, the CLI SHALL fall back to the standard overwrite confirmation prompt and print a warning to stderr.

#### Scenario: Existing valid report detected
- **WHEN** the resolved output path exists and the file is a valid YAML report produced by this tool
- **THEN** the CLI enters update mode and does not offer the overwrite/alternate-file prompt

#### Scenario: Existing file is not a valid report
- **WHEN** the resolved output path exists but the file cannot be parsed as a known report format
- **THEN** the CLI prints a warning to stderr and falls back to the standard overwrite/alternate-file prompt

### Requirement: Update prompt replaces overwrite prompt
When the CLI enters update mode and stdin is a TTY, it SHALL present the user with a prompt offering `(u) update` as the primary action and `(a) write to another file` as the alternative, instead of the previous `(o) overwrite` / `(a) another file` prompt.

#### Scenario: User chooses update
- **WHEN** the output file exists, the update prompt is shown, and the user presses Enter or types `u`
- **THEN** the CLI proceeds with the per-cluster merge flow

#### Scenario: User chooses another file from update prompt
- **WHEN** the output file exists, the update prompt is shown, and the user types `a`
- **THEN** the CLI asks for a new filename and writes the full new report there (no merge)

### Requirement: Per-cluster update interaction
During update mode, for each cluster in the freshly collected report that matches a `cluster_uid` found in the existing report with an older `report_time`, the CLI SHALL prompt the user with four choices: `(u)` update this cluster, `(s)` skip this cluster, `(a)` update all remaining clusters, `(k)` skip all remaining clusters.  The prompt SHALL loop until a valid choice is entered.  A missing `report_time` in the existing cluster entry SHALL be treated as epoch (always in the past).

#### Scenario: User updates a single cluster
- **WHEN** a matching cluster with an older `report_time` is found and the user types `u`
- **THEN** that cluster's entry in the merged report is replaced with the newly collected data

#### Scenario: User skips a single cluster
- **WHEN** a matching cluster with an older `report_time` is found and the user types `s`
- **THEN** that cluster's entry in the merged report retains the existing data

#### Scenario: User updates all remaining clusters
- **WHEN** a matching cluster with an older `report_time` is found and the user types `a`
- **THEN** that cluster and all subsequent matching clusters are updated without further prompting

#### Scenario: User skips all remaining clusters
- **WHEN** a matching cluster with an older `report_time` is found and the user types `k`
- **THEN** that cluster and all subsequent matching clusters retain their existing data without further prompting

#### Scenario: Cluster exists in existing report with newer or equal report_time
- **WHEN** a matching `cluster_uid` is found in the existing report and its `report_time` is equal to or newer than the freshly collected value
- **THEN** the cluster entry is left unchanged and no prompt is shown for it

#### Scenario: Cluster is new (not present in existing report)
- **WHEN** a `cluster_uid` from the freshly collected report has no matching entry in the existing report
- **THEN** the cluster entry is appended to the merged report without prompting

#### Scenario: Existing cluster not present in fresh report
- **WHEN** a `cluster_uid` from the existing report has no matching entry in the freshly collected report
- **THEN** the existing cluster entry is retained in the merged report unchanged

#### Scenario: Existing cluster has no report_time
- **WHEN** a matching `cluster_uid` is found in the existing report and that entry has no `report_time` field
- **THEN** the cluster is treated as having `report_time` of epoch and is eligible for the update prompt

### Requirement: --update flag for non-interactive update
The CLI SHALL accept a global `--update` flag. When present and the resolved output file exists and is a valid report, the CLI SHALL silently update all existing cluster entries whose `report_time` is older than the freshly collected value, append new clusters, and retain clusters absent from the fresh report — without any interactive prompt.

#### Scenario: --update flag updates all stale clusters silently
- **WHEN** `--update` is passed, the output file is a valid existing report, and multiple clusters have older `report_time` values
- **THEN** all those clusters are replaced with fresh data and no prompts are shown

#### Scenario: --update flag with no stale clusters
- **WHEN** `--update` is passed and no cluster in the existing report has a `report_time` older than the freshly collected data
- **THEN** the report is written unchanged (existing data retained) and a notice is printed to stderr

#### Scenario: --update flag with non-existent output file
- **WHEN** `--update` is passed but the output file does not yet exist
- **THEN** the CLI writes the fresh report as if no existing file was present (normal write path)

#### Scenario: Non-interactive environment with existing valid report
- **WHEN** the output file is a valid existing report and stdin is not a TTY and `--update` is not passed
- **THEN** the CLI behaves as if `--update` were passed (silent update all)
