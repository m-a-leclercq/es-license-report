## MODIFIED Requirements

### Requirement: Write report to stdout or file
The CLI SHALL write the YAML report to a file by default. When `--output` is omitted, the CLI SHALL use `report.yml` in the current working directory as the output path and SHALL print a notice to stderr indicating the path used (e.g. `Writing report to report.yml`). An explicit `--output <path>` flag overrides the default. If the resolved output file already exists, the CLI SHALL prompt the user interactively to choose between overwriting the existing file or writing to a different file. In non-interactive environments (stdin is not a TTY), the existing file SHALL be overwritten silently.

#### Scenario: Default output path used
- **WHEN** `--output` is omitted
- **THEN** the YAML report is written to `report.yml` in the current directory and a notice is printed to stderr: `Writing report to report.yml`

#### Scenario: Explicit output path used
- **WHEN** `--output report.yaml` is specified
- **THEN** the YAML report is written to `report.yaml` and nothing is printed to stdout (except diagnostic messages on stderr)

#### Scenario: Output directory does not exist
- **WHEN** `--output /nonexistent/dir/report.yaml` is specified and the parent directory does not exist
- **THEN** the CLI exits with a non-zero code and a descriptive error

## ADDED Requirements

### Requirement: Overwrite confirmation prompt
When the resolved output file already exists and stdin is a TTY, the CLI SHALL prompt the user with a message indicating the file exists and offering two choices: overwrite (`o`, the default, selected by pressing Enter) or write to another file (`a`). The prompt SHALL loop until a valid choice is entered.

#### Scenario: User accepts overwrite (default)
- **WHEN** the output file exists, the prompt is shown, and the user presses Enter or types `o`
- **THEN** the existing file is overwritten with the new report

#### Scenario: User chooses alternate file
- **WHEN** the output file exists, the prompt is shown, and the user types `a`
- **THEN** the CLI asks for a new filename

#### Scenario: Alternate filename without extension
- **WHEN** the user provides a filename with no file extension (e.g. `my-report`)
- **THEN** the CLI appends `.yml` and writes to `my-report.yml`

#### Scenario: Alternate filename with .yml extension
- **WHEN** the user provides a filename ending in `.yml` (e.g. `my-report.yml`)
- **THEN** the CLI writes to `my-report.yml` without modification

#### Scenario: Alternate filename with .yaml extension
- **WHEN** the user provides a filename ending in `.yaml` (e.g. `my-report.yaml`)
- **THEN** the CLI writes to `my-report.yaml` without modification

#### Scenario: Alternate filename with other extension
- **WHEN** the user provides a filename with a non-YAML extension (e.g. `my-report.txt`)
- **THEN** the CLI writes to `my-report.txt` without modification

#### Scenario: Non-interactive environment
- **WHEN** the output file exists and stdin is not a TTY
- **THEN** the CLI overwrites the existing file silently without prompting
