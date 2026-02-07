# mfinance

A minimalist command-line tool for personal finance tracking based on CSV files.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- Simple date-based transaction tracking (YYYY-MM-DD)
- Add income/expenses with positive/negative amounts
- Generate filtered financial reports (year/month)
- Sort CSV file by date
- Interactive terminal user interface
- Human-readable currency formatting with thousands separators

## Installation

```bash
cargo install --git "https://github.com/zummenix/mfinance"
```

Verify installation:
```bash
mfinance --version
```

## Usage

### Basic workflow

```bash
# Add new entry (negative amounts are supported)
mfinance new-entry --amount -199.99 --date 2024-09-15 finances.csv

# Generate full report
mfinance report finances.csv

# Show September 2024 transactions
mfinance report --filter 2024-09 finances.csv

# Sort CSV file by date
mfinance sort finances.csv

# Open a simple terminal user interface with a list of files
mfinance tui path/to/dir
```

### CSV Format Example

```csv
date;amount
2024-09-11;700.00
2024-09-12;42.42
2024-10-01;-200.00
2025-01-01;10.00
```

## Configuration

mfinance supports two levels of configuration: global and local (data).

### Global Configuration

Global configuration applies to all mfinance instances and is stored in a file
named `config.toml` in the user's configuration directory. The location of this
directory varies by operating system:

- **Linux**: `~/.config/mfinance/config.toml`
- **macOS**: `~/Library/Application Support/mfinance/config.toml`
- **Windows**: `%APPDATA%\mfinance\config.toml`

### Local (Data) Configuration

Local configuration is stored in a file named `mfinance.toml` and is applied to
all CSV files located in the same directory (including when using `mfinance tui`
with that directory).

### Configuration Precedence

1. Local (data) configuration takes precedence over global configuration
2. If no configuration file is found, mfinance uses default settings

### Configuration Format

Configuration files use TOML format. Here's an example configuration:

```toml
[formatting]
currency_symbol = "€"          # The currency symbol to display (e.g., "$", "€", "£")
currency_position = "Prefix"   # Where to place the currency symbol ("Prefix" or "Suffix")
thousands_separator = "\u{a0}" # Character used to separate thousands (default: non-breaking space)
decimal_separator = ","        # Character used for decimal points (default: ".")
```

## Contributing

Contributions are welcome! Please open an issue first to discuss proposed changes.

## License

MIT - See [LICENSE](LICENSE) for details
