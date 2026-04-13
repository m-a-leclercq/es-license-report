## Context

The CLI currently defines `--config` as a required `PathBuf` argument and `--output` as an `Option<PathBuf>` that defaults to stdout. Users running repeated queries against the same cluster file must type the full flags every time. The change makes both flags optional by resolving them to conventional filenames at startup.

The binary is a single-threaded async Rust program using `clap` for argument parsing, `tokio` for async I/O, and `anyhow` for error propagation. The output prompt requires synchronous stdin interaction, which must not block the async runtime.

## Goals / Non-Goals

**Goals:**
- Make `--config` optional, resolving to `cluster.yml` in the current directory when omitted
- Make `--output` optional, resolving to `report.yml` in the current directory when omitted
- Prompt the user for overwrite confirmation when the resolved output path already exists
- Support choosing an alternate filename at the prompt; normalise the name by appending `.yml` when no extension is present

**Non-Goals:**
- Changing the YAML format or content of the report
- Introducing interactive prompts for any other flag
- Supporting shell completion or config-file driven defaults beyond the two conventional filenames

## Decisions

### Decision: Resolve defaults in `main` before calling `run`, not inside `clap`

`clap` `default_value` would apply unconditionally — we want `--config` to error when the default file is absent, which requires post-parse logic. Keeping clap fields as `Option<PathBuf>` and resolving in `main` keeps the parser simple and the error messaging clear.

**Alternatives considered:**
- `clap` `default_value_os_t`: sets the value unconditionally, so the missing-file check would need to be deferred anyway — no benefit.
- `clap` `default_value_if`: too limited for filesystem existence checks.

### Decision: Prompt implementation using `std::io` on the main thread before entering async

The overwrite prompt is a short synchronous stdin read. Spawning a blocking task (`tokio::task::spawn_blocking`) for a two-character read is unnecessary overhead. Since the prompt happens once during startup, performing it synchronously in `main` before `#[tokio::main]` takes effect — or equivalently inside the sync preamble before the first `.await` — keeps the code simple.

Concretely: move argument resolution and the overwrite prompt into a sync helper `resolve_args(raw: RawArgs) -> anyhow::Result<Args>` called at the top of `main`.

### Decision: Extension normalisation rule

Append `.yml` when the user-provided alternate filename has no extension. If the name ends with `.yml` or `.yaml` (case-insensitive) leave it as-is. Any other extension (e.g. `.txt`) is also left as-is — the user is being explicit.

**Rationale:** Appending `.yml` only when there is no extension is the least surprising behaviour. Overriding an explicit `.txt` would be unexpected.

## Risks / Trade-offs

- **Surprising default output path** → Users who relied on `--output` being absent meaning stdout will now get a file. Mitigation: print a one-line notice to stderr when the default `report.yml` is used (`Writing report to report.yml`), so the user always knows where output went.
- **Prompt breaks non-interactive use** → A CI pipeline running without a TTY and without `--output` would hang if `report.yml` already exists. Mitigation: detect `!std::io::stdin().is_terminal()` (using `std::io::IsTerminal` stabilised in Rust 1.70) and default to overwrite silently in non-interactive mode.
- **`cluster.yml` vs `clusters.yml`** → The repo ships `clusters.yml` (plural). The default name chosen (`cluster.yml`) must be documented clearly; users can always pass `--config clusters.yml` explicitly.
