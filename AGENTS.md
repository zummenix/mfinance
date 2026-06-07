# mfinance — Agent Guide

`mfinance` is a fun, minimalist Rust project for tracking personal finances in CSV files, with both scripted and interactive workflows.

## Key components

- **CLI** (`src/main.rs`): command-based interface for adding entries, reporting, and sorting CSV data.
- **TUI** (`src/tui.rs`): interactive terminal UI for browsing files, years, entries, and editing data.

## Useful commands during development

- `cargo check --all` can be run for a quick check
- `cargo fmt --all` runs the formatter

## Snapshot tests

- The test suite uses the `insta` crate.
- If snapshots need updates, use `cargo insta accept` (requires `cargo install cargo-insta`).
- Prefer **inline insta snapshots** when adding or updating snapshot assertions.

## Baseline quality contract

Agents should ensure all of these pass:

- `cargo clippy -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo build`
- `cargo test`
