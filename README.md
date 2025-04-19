# mfinance

A command line tool I've written just for fun to manage finances.

## About

`mfinance` is a command-line tool designed to help you manage your finances
using simple CSV files. Each CSV file represents a separate financial record,
such as expenses or income.

The CSV files should follow this format:

```csv
date;amount
2024-09-11;700
2024-09-12;42.42
2024-10-01;-200
2024-10-02;3000.42
2025-01-01;10
```

`date`: The date of the transaction in `YYYY-MM-DD` format.
`amount`: The transaction amount, can be positive or negative.

With `mfinance`, you can:

- Add new entries to a CSV file.
- Generate reports to calculate total amounts, optionally filtered by date.

## Installation

You need to make sure that you have the Rust compiler installed. If you don't
have it, head to https://rustup.rs and follow instructions.

After that run:

```bash
cargo install --git "https://github.com/zummenix/mfinance"
```

## Usage

See `mfinance --help` for details.

## LICENSE

MIT
