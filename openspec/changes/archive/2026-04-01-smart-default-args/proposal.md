## Why

Running `es-license-consumption` currently requires explicit `--config` and optionally `--output` flags every invocation, which is tedious when working repeatedly with the same files. Defaulting to conventional filenames and prompting before overwriting reduces friction and prevents accidental data loss.

## What Changes

- `--config` defaults to `cluster.yml` in the current directory if the flag is omitted; the CLI exits with a descriptive error if the flag is omitted and `cluster.yml` does not exist
- `--output` defaults to `report.yml` in the current directory if the flag is omitted (previously defaulted to stdout)
- When the resolved output file already exists, the CLI prompts the user: overwrite (default) or write to a different file; if the user chooses a different file, the CLI asks for a filename and appends `.yml` when the provided name has no extension (`.yml` and `.yaml` are left unchanged)

## Capabilities

### New Capabilities

*(none)*

### Modified Capabilities

- `cluster-config`: Default config path — `--config` is now optional with a fallback to `cluster.yml`; error message must reflect that the default was attempted
- `report-output`: Default output path and overwrite prompt — `--output` now defaults to `report.yml` instead of stdout; an overwrite confirmation prompt is added when the target file already exists

## Impact

- `src/main.rs`: CLI argument definitions and pre-run resolution logic
- `src/report.rs`: `write_report` or a new helper to handle the overwrite prompt and alternate filename logic
- `openspec/specs/cluster-config/spec.md`: Updated requirement for optional `--config` with default
- `openspec/specs/report-output/spec.md`: Updated requirements for default output path and overwrite prompt
