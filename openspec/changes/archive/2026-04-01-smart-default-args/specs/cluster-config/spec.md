## MODIFIED Requirements

### Requirement: Load cluster list from YAML file
The CLI SHALL accept an optional `--config` flag pointing to a YAML configuration file. When `--config` is omitted, the CLI SHALL look for `cluster.yml` in the current working directory. The resolved file SHALL be validated on startup, and the CLI SHALL exit with a descriptive error if the file is missing, unreadable, or fails schema validation. The error message SHALL indicate whether the path was supplied explicitly or resolved from the default.

#### Scenario: Valid config file is loaded via explicit flag
- **WHEN** the user passes `--config clusters.yaml` and the file is valid YAML matching the schema
- **THEN** the CLI parses all cluster entries without error and proceeds to query each cluster

#### Scenario: Valid default config file is loaded
- **WHEN** `--config` is omitted and `cluster.yml` exists in the current directory and is valid YAML
- **THEN** the CLI parses all cluster entries without error and proceeds to query each cluster

#### Scenario: Config file not found via explicit flag
- **WHEN** the user passes `--config clusters.yaml` and the file does not exist
- **THEN** the CLI exits with a non-zero code and prints an error indicating the supplied file path was not found

#### Scenario: Default config file not found
- **WHEN** `--config` is omitted and `cluster.yml` does not exist in the current directory
- **THEN** the CLI exits with a non-zero code and prints an error stating that no `--config` flag was provided and the default `cluster.yml` was not found

#### Scenario: Config file has invalid YAML
- **WHEN** the resolved config file contains malformed YAML
- **THEN** the CLI exits with a non-zero code and prints the YAML parse error with line/column information
