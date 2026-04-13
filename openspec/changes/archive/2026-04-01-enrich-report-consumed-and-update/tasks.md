## 1. Data Model — add report_time to cluster entries

- [x] 1.1 Add `report_time: Option<DateTime<Utc>>` field to the cluster struct in `src/report.rs`; derive serde serialise/deserialise with ISO 8601 format
- [x] 1.2 Capture `report_time = Utc::now()` immediately after a successful cluster API response is deserialised and thread it into the cluster result
- [x] 1.3 Update YAML serialisation so `report_time` is emitted on every successful cluster entry and absent on error entries

## 2. Data Model — add total_consumed to license entries

- [x] 2.1 Add `total_consumed: Option<f64>` field to the license struct in `src/report.rs`
- [x] 2.2 Extend the calculation layer to return both the rounded `consumed` value and the raw pre-ceiling float (`consumed_raw`) per cluster result
- [x] 2.3 In the report-assembly step, compute `total_consumed` for Enterprise licenses: sum `consumed_raw` values across all clusters for that license UID, then apply `ceil` to the nearest integer
- [x] 2.4 Compute `total_consumed` for Platinum licenses: sum `consumed` values across all clusters for that license UID and apply `ceil` to the nearest integer (handles edge-case fractional RAM-driven values)
- [x] 2.5 Leave `total_consumed` absent (`None`) for Basic and unsupported license types
- [x] 2.6 Verify the YAML output matches the updated schema example in the report-output spec

## 3. Output — update-mode detection

- [x] 3.1 After resolving the output path, attempt to read and parse the existing file as the known YAML report format before showing any prompt
- [x] 3.2 If parse succeeds, set an `existing_report` variable and proceed to update-mode prompt (step 4); if parse fails, print a stderr warning and fall through to the existing overwrite/alternate-file prompt

## 4. Output — interactive update prompt

- [x] 4.1 When `existing_report` is set and stdin is a TTY and `--update` is absent, show the `(u) update / (a) another file` prompt instead of the old overwrite prompt
- [x] 4.2 If the user chooses `a`, ask for a new filename and write the full fresh report there (no merge); use the same filename extension logic as the existing alternate-file path
- [x] 4.3 If the user chooses `u`, proceed to the per-cluster merge loop (step 5)

## 5. Output — per-cluster merge logic

- [x] 5.1 Implement `merge_reports(existing, fresh, mode)` in `src/report.rs`: iterate fresh clusters; for each, look up by `cluster_uid` in existing; if found with older `report_time` (or missing), mark as candidate; append new clusters; retain orphaned existing clusters
- [x] 5.2 Implement the per-cluster interactive loop: for each candidate cluster, print the cluster name and prompt `(u) update / (s) skip / (a) update all / (k) skip all`; honour the `a`/`k` short-circuit flag for remaining iterations
- [x] 5.3 After merge, recompute `total_consumed` for each license entry based on the merged cluster set and write the final report

## 6. Output — --update flag and non-interactive path

- [x] 6.1 Add `--update` boolean flag to the CLI argument parser in `src/main.rs` with help text: "If the report already exists, updates all existing clusters to their newer values"
- [x] 6.2 When `--update` is set (or stdin is not a TTY and `existing_report` is set), call `merge_reports` with `mode = UpdateAll` — no prompts, silent merge
- [x] 6.3 When `--update` is set and no clusters are stale, print a notice to stderr: "No clusters required updating"
- [x] 6.4 When `--update` is set and the output file does not exist, write the fresh report normally

## 7. Tests

- [x] 7.1 Unit test `merge_reports`: new cluster appended, stale cluster replaced, newer cluster retained, missing `report_time` treated as epoch
- [x] 7.2 Unit test `total_consumed` for Enterprise: two clusters with fractional ERUs sum before ceiling to nearest integer (not ceil-then-sum)
- [x] 7.3 Unit test `total_consumed` for Platinum: sum of integer consumed values
- [x] 7.4 Unit test `total_consumed` absent for Basic license
- [x] 7.5 Integration test: run with `--update` against an existing report file; verify stale clusters updated and non-stale clusters unchanged
