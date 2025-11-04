# ChronoSheet - Time Tracking Tool

A Rust-based command-line tool for calculating monthly work hours from text-based timesheets.

## Features

- Parse multiple `.txt` timesheet files automatically
- Calculate work hours with rest period deductions
- Support for special daily deductions
- Date validation (including leap years)
- Comprehensive error handling and warnings
- TOML output format

## Installation

### Prerequisites

- Rust toolchain (1.70 or later)
- Cargo

### Build from source

```bash
cargo build --release
```

The executable will be in `target/release/chronosheet`

## Usage

1. Place your `.txt` timesheet file(s) in the same directory as the executable
2. Run the program:
   ```bash
   ./chronosheet
   # or if using cargo:
   cargo run
   ```
3. The program will automatically process all `.txt` files and generate corresponding TOML output files

## Input File Format

### Basic Structure

```txt
[settings]
month=202511
rest=1h

[PersonName]
1
08:15
19:30
#-1h

2
08:15
17:30
```

### Settings Section

The `[settings]` section must be the first section in the file:

- `month`: Month in YYYYMM format (e.g., 202511 for November 2025)
- `rest`: Rest period in format like `1h`, `30m`, or `1h30m` (hours: 0-2, minutes: 0-59)

### Person Section

Each person section `[PersonName]` contains daily work records:

1. **Day number** (1-31, must be valid for the specified month)
2. **Start time** in one of these formats:
   - `HHMM` (4 digits without colon, e.g., `0815`)
   - `HH:MM` (standard format, e.g., `08:15`)
   - `H:MM` (simplified format, e.g., `8:15`)
3. **End time** (same formats as start time)
4. **Special deduction** (optional) in format `#-1h`, `#30m`, or `#-1h30m`

Format requirements:

- Each field must be on a separate line
- Time formats: minutes must always be 2 digits (e.g., `08`, not `8`)
- Hour can be 1 or 2 digits in colon format (e.g., `8:15` or `08:15`)
- Empty lines are allowed between day records
- No empty lines within a single day record

## Calculation Logic

For each day:

1. Calculate raw work hours: `end time - start time`
2. If raw hours > 3 hours: subtract rest period
3. Subtract special deduction (if any)
4. If result < 0: set to 0

Monthly total:

1. Sum of all daily hours (in minutes)
2. Convert to hours:minutes format for output

## Output Format

### Single file

When processing one `.txt` file, output is `chronosheet_result.toml`

### Multiple files

When processing multiple files, output is `chronosheet_result_{filename}.toml`

Example output:

```toml
month = "202511"

[[sheets]]
name = "Tom"
work_days = 20
work_times = 42:00

[[sheets]]
name = "John"
work_days = 15
work_times = 19:45

[[sheets]]
name = "Alice"
work_days = 5
work_times = 6:30
```

Fields:

- `name`: Person's name
- `work_days`: Number of valid work days (invalid dates are excluded)
- `work_times`: Total work hours in HH:MM format

## Error Handling

### Fatal Errors (program exits)

- Missing or invalid `.txt` files
- Missing `[settings]` section
- Missing required fields (`month` or `rest`)
- Invalid time format (not HHMM, HH:MM, or H:MM)
- Invalid time range (HH not 00-23, MM not 00-59)
- Unexpected content in person sections

### Warnings (continues processing)

- Invalid dates (e.g., February 30)
- Start time after end time
- Duplicate day entries (uses first occurrence)

## Examples

### Example 1: Basic Usage

Input file `time.txt`:

```txt
[settings]
month=202511
rest=1h

[Tom]
1
08:15
19:30
#-1h
```

Calculation:

- Raw hours: 19:30 - 08:15 = 11h15m = 675 minutes
- Subtract rest: 675 - 60 = 615 minutes (because > 3 hours)
- Subtract deduction: 615 - 60 = 555 minutes = 9h15m
- Result: 9:15

Output `chronosheet_result.toml`:

```toml
month = "202511"

[[sheets]]
name = "Tom"
work_days = 1
work_times = 9:15
```

### Example 2: Different Time Formats

```txt
[settings]
month=202511
rest=1h

[Alice]
1
0815
1730

2
08:15
17:30

3
8:15
17:30
```

All three formats produce the same result (8:15 per day).

### Example 3: Multiple Days

```txt
[settings]
month=202511
rest=1h

[Alice]
1
08:00
09:05

2
08:00
09:23
```

Calculation:

- Day 1: 1h5m = 65 minutes = 1:05
- Day 2: 1h23m = 83 minutes = 1:23
- Total: 65 + 83 = 148 minutes = 2:28

### Example 4: Short Work Day

```txt
[settings]
month=202511
rest=1h

[Bob]
1
08:00
10:00
```

Calculation:

- Raw hours: 10:00 - 08:00 = 2 hours = 120 minutes
- No rest deduction (because <= 3 hours)
- Result: 2:00

### Example 5: Multiple People

```txt
[settings]
month=202511
rest=1h

[Tom]
1
08:15
19:30

[John]
1
09:00
18:00
#30m
```

Output:

```toml
month = "202511"

[[sheets]]
name = "Tom"
work_days = 1
work_times = 10:15

[[sheets]]
name = "John"
work_days = 1
work_times = 7:30
```

## Testing

Run unit tests:

```bash
cargo test
```

Run with test file:

```bash
# Create a test file
cat > test.txt << 'EOF'
[settings]
month=202511
rest=1h

[TestPerson]
1
08:00
17:00
EOF

# Run the program
cargo run
```

## Project Structure

```
src/
  main.rs          - Entry point and file processing
  types.rs         - Core data structures
  error.rs         - Error types and messages
  parser.rs        - Text file parsing logic
  calculator.rs    - Work hours calculation
  output.rs        - TOML output generation
```

## License

This project is dual-licensed under either:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
