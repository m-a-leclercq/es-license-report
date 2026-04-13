## 1. Make `port` optional in the raw config struct

- [x] 1.1 Change `port: u16` to `port: Option<u16>` in `RawClusterEntry` in `src/config.rs`

## 2. Implement URL sanitization logic

- [x] 2.1 Add a `sanitize_host_port` function in `src/config.rs` that accepts `alias`, `host: String`, and `port: Option<u16>` and returns `Result<(String, u16)>` — handling trailing-slash stripping, embedded port extraction, conflict detection, and the interactive prompt
- [x] 2.2 Strip trailing `/` from `host` inside `sanitize_host_port`
- [x] 2.3 Attempt to parse an embedded port from `host` using the `url` crate (already in `Cargo.toml`) inside `sanitize_host_port`; remove the port from the host string when extracted
- [x] 2.4 Implement the four resolution cases in `sanitize_host_port`: no embedded port (use `port` field as-is or error if absent), embedded port only (use it), both agree (use either), both conflict (prompt or error if non-interactive)
- [x] 2.5 Detect non-interactive mode via `std::io::stdin().is_terminal()` (from `std::io::IsTerminal`, stable since Rust 1.70) and exit with an error when a port conflict is found non-interactively
- [x] 2.6 Implement the interactive port-conflict prompt (print warning with alias and both port values, offer numbered choice, read from stdin)

## 3. Wire sanitization into `validate_entry`

- [x] 3.1 Call `sanitize_host_port` at the top of `validate_entry` and use the returned `(host, port)` when constructing `ClusterConfig`

## 4. Tests

- [x] 4.1 Add unit test: trailing slash is stripped
- [x] 4.2 Add unit test: embedded port extracted when `port` field absent
- [x] 4.3 Add unit test: embedded port and matching `port` field — no error, correct port used
- [x] 4.4 Add unit test: embedded port conflicts with `port` field in non-interactive mode — returns error
