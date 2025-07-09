# mfinance

A minimalist command-line tool for personal finance tracking using CSV files.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- Simple date-based transaction tracking (YYYY-MM-DD)
- Add income/expenses with positive/negative amounts
- Generate filtered financial reports (year/month)
- Automatic CSV file sorting by date
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
# Add new entry (negative amount = expense)
mfinance new-entry --amount -199.99 --date 2024-09-15 finances.csv

# Generate full report
mfinance report finances.csv

# Show September 2024 transactions
mfinance report --filter 2024-09 finances.csv

# Sort CSV file by date
mfinance sort finances.csv
```

### CSV Format Example

```csv
date;amount
2024-09-11;700.00
2024-09-12;42.42
2024-10-01;-200.00
2025-01-01;10.00
```

## Contributing

Contributions are welcome! Please open an issue first to discuss proposed changes.

## License

MIT - See [LICENSE](LICENSE) for details
