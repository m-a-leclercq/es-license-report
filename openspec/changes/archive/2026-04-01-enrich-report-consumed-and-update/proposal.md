## Why

The report currently lacks aggregate consumption totals and per-cluster timestamps, making it hard to track utilisation at the license level or detect stale data. When re-running the tool against an existing report file, users also need finer control over which cluster entries to refresh rather than a blunt overwrite.

## What Changes

- Add `total_consumed` field at the license level: sum of consumed license units across all clusters sharing that license. For Enterprise licenses, the sum is computed from unrounded per-cluster values and rounded up once on the final result.
- Add `report_time` field at the cluster level: ISO 8601 timestamp of when that cluster's API was queried.
- **BREAKING**: When `--output` resolves to an existing file and stdin is a TTY, replace the `(o) overwrite` prompt option with `(u) update`. Choosing update triggers per-cluster merge logic driven by `cluster_uid` and `report_time`.
- Per-cluster update prompt: when a matching `cluster_uid` exists in the existing report with an older `report_time`, ask the user whether to update that cluster, skip it, update all, or skip all.
- Add global `--update` flag: non-interactive equivalent of "update all clusters" — silently merges all clusters to their newer values without prompting.

## Capabilities

### New Capabilities
- `report-update-mode`: Interactive and non-interactive update flow for merging new cluster data into an existing report file, driven by `cluster_uid` matching and `report_time` comparison.

### Modified Capabilities
- `report-output`: Add `total_consumed` to each license entry, add `report_time` to each cluster entry, replace the overwrite prompt with an update prompt when the output file already exists.
- `license-calculation`: Add `total_consumed` aggregation rule: for Enterprise licenses, sum unrounded per-cluster ERU values then apply ceiling; for other types, sum the already-reported `consumed` values directly.

## Impact

- `src/report.rs`: report struct gains `total_consumed` on license entries and `report_time` on cluster entries; serialisation changes.
- `src/main.rs`: `--update` flag added; output-file resolution logic updated to detect existing file and branch into update vs overwrite flow.
- Existing `report.yml` files written before this change lack `report_time`; the update prompt treats missing `report_time` as "in the past" so those clusters are eligible for update.
