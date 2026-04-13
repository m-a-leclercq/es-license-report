## Context

Cluster entries in `cluster.yml` have separate `host` and `port` fields. Users sometimes include the port in the URL (e.g. `https://example.com:9200`) and may also—or may not—fill in the `port` field. Additionally, trailing slashes on the host URL are common copy-paste artifacts. These inconsistencies can result in double-port URLs being sent to the HTTP client or silent misconfiguration.

Sanitization runs after YAML is parsed and before any HTTP client is constructed, so it is a focused pre-flight step with no schema changes.

## Goals / Non-Goals

**Goals:**
- Strip trailing `/` from `host`
- Extract an embedded port from `host` when `port` is absent
- Detect and resolve conflicts between an embedded port in `host` and the `port` field
- Keep the YAML schema unchanged

**Non-Goals:**
- Full URL validation (scheme, hostname resolution)
- Sanitizing fields other than `host` and `port`
- Persisting the sanitized values back to disk

## Decisions

### Sanitization runs at parse time, not call time

Apply sanitization immediately after the YAML is deserialized, before any cluster struct is used elsewhere. This keeps all other code free from defensive checks.

*Alternative considered*: sanitize inside the HTTP client constructor. Rejected because the port-conflict warning needs to be surfaced early and clearly, before any network activity begins.

### Port extraction uses URL parsing, not regex

Use the standard URL parser (Rust's `url` crate, already present as a transitive dependency, or `std` URI helpers) to split `host` into scheme+host and port. This avoids brittle regex patterns and handles edge cases (IPv6 addresses, no scheme, etc.) correctly.

*Alternative*: manual string split on `:`. Rejected as fragile for IPv6 and schemeless URLs.

### Port conflict prompts the user interactively; non-interactive mode errors out

When a conflict is detected (embedded port ≠ `port` field), the CLI prompts the user to choose which port to use. If stdin is not a TTY (piped / CI), the CLI exits with a non-zero code and a descriptive error instead of silently picking one.

*Alternative*: always prefer the `port` field silently. Rejected because silent resolution could mask a genuine misconfiguration.

## Risks / Trade-offs

- **Interactive prompt blocks automation** → In non-interactive environments the conflict is a hard error; users must fix the YAML before running.
- **URL parser behavior on schemeless hosts** → If `host` is `example.com:9200` (no scheme), some parsers may interpret the whole string as a path. Add a fallback: if parsing fails, attempt a colon-split on the last segment to extract a numeric port.

## Migration Plan

No migration needed. The sanitization is transparent when there is no conflict. Users with conflicting config will be prompted once; they should then fix their `cluster.yml` to avoid the prompt on future runs.
