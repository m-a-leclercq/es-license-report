## 1. Add Short Flag Aliases

- [x] 1.1 Add `short = 'c'` to the `--config` arg in `src/main.rs`
- [x] 1.2 Add `short = 'o'` to the `--output` arg in `src/main.rs`
- [x] 1.3 Add `short = 't'` to the `--timeout` arg in `src/main.rs`
- [x] 1.4 Add `short = 'u'` to the `--update` arg in `src/main.rs`

## 2. Verify

- [x] 2.1 Run `cargo build` to confirm no compilation errors
- [x] 2.2 Run `cargo run -- --help` and verify `-c`, `-o`, `-t`, `-u` appear in the output
