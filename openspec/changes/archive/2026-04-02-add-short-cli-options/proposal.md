## Why

The CLI currently only exposes long-form flags (`--config`, `--output`, `--timeout`, `--update`), which are verbose for frequent interactive use. Adding single-letter aliases reduces typing for power users.

## What Changes

- `--config` gains alias `-c`
- `--output` gains alias `-o`
- `--timeout` gains alias `-t`
- `--update` gains alias `-u`

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `cli-args`: The four existing flags each gain a short single-letter alias.

## Impact

- `src/main.rs`: `Args` struct `#[arg]` annotations updated with `short` aliases.
- No changes to behavior, output format, or configuration files.
