## Context

The CLI collects Elasticsearch license consumption data from one or more clusters and writes a YAML report. Currently the report has no aggregate totals and no per-cluster timestamps. The output-file handling supports overwrite or alternate-file, but no partial-update path. The codebase is a single Rust binary with `src/main.rs` (CLI wiring, output handling) and `src/report.rs` (data structures, serialisation, calculation).

## Goals / Non-Goals

**Goals:**
- Add `total_consumed` to each license entry in the report (aggregated and correctly rounded for Enterprise).
- Add `report_time` (ISO 8601) to each cluster entry at query time.
- Replace the "overwrite" prompt with an "update" prompt when the output file already exists and is a valid YAML report.
- Support a per-cluster interactive update flow guided by `cluster_uid` + `report_time` comparison.
- Add `--update` flag for silent, non-interactive "update all" behaviour.

**Non-Goals:**
- Merging error entries from an existing report.
- Supporting YAML report formats other than the one produced by this tool.
- Changing how per-cluster `consumed` values are displayed (they remain rounded as today).

## Decisions

### 1. Where `report_time` is captured

`report_time` is recorded per cluster at the moment the API call to that cluster completes successfully (immediately after deserialising the response). This gives the most accurate timestamp for that cluster's data.

Alternatives considered:
- Single top-level `report_time` for the whole run: rejected because clusters are queried sequentially/concurrently and may span significant wall time, making a single timestamp misleading.
- Timestamp at report-write time: rejected because it does not reflect when the data was actually collected.

### 2. `total_consumed` calculation for Enterprise

`total_consumed` is computed from the **unrounded** intermediate ERU values (total_ram_gb / 64, before `ceil`) for each cluster, summed together, and then `ceil`'d once to the nearest integer.  This requires the calculation layer to return both the rounded `consumed` value (for cluster-level display) and the raw float (for aggregation).  The report struct carries `consumed_raw: f64` internally; it is not serialised to YAML.

Alternatives considered:
- Sum already-rounded per-cluster values: rejected per explicit requirement — double-rounding inflates the total for multi-cluster Enterprise licenses.
- Store only raw values and round at serialisation: rejected because it would change the existing per-cluster `consumed` display format.

### 3. Update-mode detection

When `--output` resolves to an existing file, the tool attempts to parse it as the known YAML report format. If parsing succeeds, it enters update mode; if parsing fails (unrecognised format, corrupt file), it falls back to the existing overwrite/alternate-file prompt so no data is silently lost.

### 4. Interactive update prompt granularity

The per-cluster prompt mirrors the git-add patch hunk model: update this one (`u`), skip this one (`s`), update all remaining (`a`), skip all remaining (`k`). State is held in a simple enum flag that short-circuits the per-cluster loop once `a` or `k` is chosen.

### 5. `--update` flag semantics

`--update` implies "update all existing clusters silently" — equivalent to answering `a` at every per-cluster prompt and `u` at the file-exists prompt. It does **not** suppress the per-cluster prompt when stdin is a TTY and `--update` is absent. This keeps the flag's meaning narrow and predictable.

### 6. Clusters with no `report_time` in an existing report

Existing reports written before this change have no `report_time` on cluster entries. The update logic treats a missing `report_time` as `epoch` (i.e., always in the past), so those clusters are always eligible for update without special-casing.

## Risks / Trade-offs

- **Unrounded intermediate values not in the public model**: `consumed_raw` must be threaded through the calculation → report pipeline. If calculation is ever refactored this field must not be dropped silently. → Mitigation: name it clearly (`consumed_raw`) and document the invariant in code comments.
- **YAML parse failure on existing file**: If the file exists but is not a valid report, the tool falls back gracefully — but the user may be surprised. → Mitigation: print a clear stderr warning explaining the fallback.
- **`report_time` timezone**: Using UTC (`chrono::Utc::now()`) avoids ambiguity; the ISO 8601 string will carry the `Z` suffix.

## Migration Plan

No migration needed for existing report files. The absence of `report_time` is handled by treating it as epoch (see Decision 6). `total_consumed` is a purely additive field; any tooling that reads existing reports will ignore the new key.
