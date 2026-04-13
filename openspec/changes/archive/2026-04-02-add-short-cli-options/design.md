## Context

The tool uses `clap` for CLI argument parsing. Each flag is defined as a struct field annotated with `#[arg(long)]`. Adding a short alias requires adding `short = 'x'` to the same annotation.

## Goals / Non-Goals

**Goals:**
- Add `-c`, `-o`, `-t`, `-u` as short aliases for the four existing flags.

**Non-Goals:**
- No new flags or behavior changes.
- No changes to config file format or report output.

## Decisions

**Use `clap`'s built-in `short` attribute** — the `#[arg(long, short = 'x')]` form is the idiomatic clap way to add a short alias alongside a long flag. No alternative needed; there is only one way to do this in clap.

## Risks / Trade-offs

- No known risks. Short flags are purely additive; long flags continue to work unchanged.
- `-u` for `--update` is a boolean flag (no value), consistent with clap's short flag behavior for booleans.
