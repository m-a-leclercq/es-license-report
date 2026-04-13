## 1. CLI Argument Changes

- [x] 1.1 Change `config` field in `Args` from `PathBuf` to `Option<PathBuf>` so `--config` is optional
- [x] 1.2 Change `output` field in `Args` to `Option<PathBuf>` (already optional, keep as-is — verify no change needed)

## 2. Default Config Resolution

- [x] 2.1 Add `resolve_config(raw: Option<PathBuf>) -> anyhow::Result<PathBuf>` that returns the supplied path when given, or falls back to `cluster.yml` in the current directory
- [x] 2.2 Return a clear error when the fallback `cluster.yml` does not exist, distinguishing it from an explicitly supplied missing path

## 3. Default Output Resolution and Overwrite Prompt

- [x] 3.1 Add `resolve_output(raw: Option<PathBuf>) -> anyhow::Result<PathBuf>` that returns the supplied path or `report.yml`; print `Writing report to report.yml` to stderr when the default is used
- [x] 3.2 Add `confirm_overwrite(path: &Path) -> anyhow::Result<PathBuf>` that checks if the path exists and, when stdin is a TTY, prompts the user with `[o]verwrite / [a]lternate file (o)`
- [x] 3.3 Implement the overwrite branch: accept empty input or `o` and return the original path
- [x] 3.4 Implement the alternate-file branch: prompt for a filename, append `.yml` when the name has no extension, leave `.yml`/`.yaml` (case-insensitive) and any other extension unchanged
- [x] 3.5 Skip the prompt and return the original path when stdin is not a TTY (`std::io::stdin().is_terminal()`)

## 4. Wire into main

- [x] 4.1 Call `resolve_config` and `resolve_output` at the top of `main` (or in a sync `resolve_args` helper) before the async runtime starts
- [x] 4.2 Pass the resolved `config` path to `load_config` and the resolved `output` path to `write_report`
- [x] 4.3 Update `write_report` (or its call site) to accept a `&Path` (non-optional) now that a path is always provided

## 5. Tests

- [x] 5.1 Unit test `resolve_config`: explicit path returned as-is, default used when `None` and file present, error when `None` and file absent
- [x] 5.2 Unit test `resolve_output`: explicit path returned, default `report.yml` used with stderr notice when `None`
- [x] 5.3 Unit test extension normalisation in `confirm_overwrite`: no extension → `.yml` appended; `.yml` unchanged; `.yaml` unchanged; other extension unchanged
