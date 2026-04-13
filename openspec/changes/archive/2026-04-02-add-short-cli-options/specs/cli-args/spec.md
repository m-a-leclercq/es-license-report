## ADDED Requirements

### Requirement: Short aliases for CLI flags
The CLI SHALL accept single-letter short aliases alongside their existing long-form flags:
- `-c` as an alias for `--config`
- `-o` as an alias for `--output`
- `-t` as an alias for `--timeout`
- `-u` as an alias for `--update`

#### Scenario: Short config flag accepted
- **WHEN** the user passes `-c cluster.yml`
- **THEN** the tool SHALL behave identically to `--config cluster.yml`

#### Scenario: Short output flag accepted
- **WHEN** the user passes `-o report.yml`
- **THEN** the tool SHALL behave identically to `--output report.yml`

#### Scenario: Short timeout flag accepted
- **WHEN** the user passes `-t 30`
- **THEN** the tool SHALL behave identically to `--timeout 30`

#### Scenario: Short update flag accepted
- **WHEN** the user passes `-u`
- **THEN** the tool SHALL behave identically to `--update`

#### Scenario: Long flags still work
- **WHEN** the user uses any of the original long-form flags
- **THEN** the tool SHALL continue to work exactly as before
