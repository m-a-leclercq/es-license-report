## ADDED Requirements

### Requirement: Strip trailing slash from host URL
After loading a cluster entry, the CLI SHALL remove any trailing `/` character from the `host` value before constructing the HTTP client URL.

#### Scenario: Host with trailing slash is normalized
- **WHEN** a cluster entry has `host: https://example.com/`
- **THEN** the CLI uses `https://example.com` as the effective host, with no trailing slash

#### Scenario: Host without trailing slash is unchanged
- **WHEN** a cluster entry has `host: https://example.com`
- **THEN** the CLI uses `https://example.com` unchanged

### Requirement: Extract embedded port from host URL when port field is absent
After loading a cluster entry, if the `host` value contains an embedded port and the `port` field is not set, the CLI SHALL extract the port from `host` and use it as the effective port.

#### Scenario: Host contains port, port field is absent
- **WHEN** a cluster entry has `host: https://example.com:9200` and `port` is not specified
- **THEN** the CLI uses `9200` as the effective port and removes the port from the host string

#### Scenario: Host contains no port and port field is absent
- **WHEN** a cluster entry has `host: https://example.com` and `port` is not specified
- **THEN** the CLI proceeds with no port extracted (behavior unchanged)

### Requirement: Accept matching embedded port and port field without warning
If both the `host` URL and the `port` field specify the same port number, the CLI SHALL accept the configuration silently and use that port.

#### Scenario: Host port and port field agree
- **WHEN** a cluster entry has `host: https://example.com:9200` and `port: 9200`
- **THEN** the CLI uses port `9200` without emitting any warning or prompt

### Requirement: Warn and prompt on port conflict between host URL and port field
If the `host` URL contains an embedded port that differs from the `port` field value, the CLI SHALL emit a warning and, when running interactively, prompt the user to choose which port to use. In non-interactive mode the CLI SHALL exit with a non-zero code and a descriptive error.

#### Scenario: Conflicting ports in interactive mode
- **WHEN** a cluster entry has `host: https://example.com:9300` and `port: 9200` and stdin is a TTY
- **THEN** the CLI prints a warning identifying the cluster alias and the two conflicting port values, then prompts the user to select either the URL port (`9300`) or the port field (`9200`), and proceeds with the chosen value

#### Scenario: Conflicting ports in non-interactive mode
- **WHEN** a cluster entry has `host: https://example.com:9300` and `port: 9200` and stdin is not a TTY
- **THEN** the CLI exits with a non-zero code and prints an error naming the cluster alias and both conflicting port values, instructing the user to resolve the conflict in `cluster.yml`
