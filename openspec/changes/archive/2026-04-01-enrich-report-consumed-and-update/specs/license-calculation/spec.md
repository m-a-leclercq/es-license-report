## ADDED Requirements

### Requirement: Enterprise total_consumed aggregation
For a license entry whose `type` is `enterprise`, the CLI SHALL compute `total_consumed` by summing the **unrounded** intermediate ERU values (total node RAM in GB ÷ 64, before ceiling) across all clusters that share that license UID, and then applying a single ceiling to the nearest integer. Individual per-cluster `consumed` values in the report retain their existing per-cluster rounding and are not affected by this rule.

#### Scenario: Enterprise total_consumed sums unrounded values then ceils to integer
- **WHEN** an enterprise license has two clusters with intermediate ERU values of 1.234 and 2.567 respectively
- **THEN** `total_consumed` is `ceil(1.234 + 2.567)` = `4` (sum 3.801, ceiled to nearest integer), not `ceil(1.234) + ceil(2.567)` = `2 + 3` = `5`

#### Scenario: Enterprise single cluster total_consumed
- **WHEN** an enterprise license has exactly one cluster with an intermediate ERU value of 1.57
- **THEN** `total_consumed` is `ceil(1.57)` = `2`

#### Scenario: Enterprise total_consumed with already-integer sum
- **WHEN** the sum of unrounded ERU values across all enterprise clusters is exactly a whole number (e.g. 4.0)
- **THEN** `total_consumed` is that integer (e.g. `4`)

### Requirement: Platinum total_consumed aggregation
For a license entry whose `type` is `platinum`, the CLI SHALL compute `total_consumed` by applying a ceiling to the nearest integer on the sum of all per-cluster `consumed` values. In practice, node-count-driven values are already integers, but if a RAM-driven cluster produces a fractional `consumed` value the ceiling ensures the total is always an integer.

#### Scenario: Platinum total_consumed with all-integer consumed values
- **WHEN** a platinum license has two clusters with `consumed` values of 5 and 7
- **THEN** `total_consumed` is `12`

#### Scenario: Platinum total_consumed with fractional consumed values
- **WHEN** a platinum license has two clusters where RAM-driven `consumed` values sum to a non-integer (e.g. 5 and 6.5)
- **THEN** `total_consumed` is `ceil(5 + 6.5)` = `12`
