## Why

Cluster host URLs entered by users often contain inconsistencies—trailing slashes, embedded ports—that cause silent misconfigurations or unexpected behavior when the `port` field is also set. Sanitizing these inputs at load time removes a class of user errors before they reach the HTTP client.

## What Changes

- Strip a trailing `/` from the cluster `host` value before use
- If `host` contains an embedded port (e.g. `https://example.com:9200`) and `port` is not set, extract and apply the embedded port
- If `host` contains an embedded port and `port` is set to the same value, silently accept (no conflict)
- If `host` contains an embedded port and `port` is set to a **different** value, emit a warning and prompt the user to confirm which port to use

## Capabilities

### New Capabilities

*(none)*

### Modified Capabilities

- `cluster-config`: Add URL sanitization requirements—trailing-slash stripping and port conflict detection/resolution between the `host` URL and the `port` field.

## Impact

- `src/main.rs` or config-loading code: sanitization logic runs after YAML is parsed, before cluster entries are used
- No changes to the YAML schema or CLI flags
- Interactive prompt only triggered on port conflict; non-interactive runs (e.g. CI) need a defined fallback behavior
