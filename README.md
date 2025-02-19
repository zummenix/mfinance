# mfinance

A command line tool I've written just for fun to manage finances.

## About

The tool works with csv files that look like the following:

```csv
date;amount
2024-09-11;700
2024-09-12;42.42
2024-10-01;-200
2024-10-02;3000.42
2025-01-01;10
```

Think of it as each file represents a separate expense (or income) item.
Currently the tool supports recording a new entry and reporting total amounts.

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
