# mfinance — Agent Guide

`mfinance` is a fun, minimalist Rust project for tracking personal finances in CSV files, with both scripted and interactive workflows.

## Key components

- **CLI** (`src/main.rs`): command-based interface for adding entries, reporting, and sorting CSV data.
- **TUI** (`src/tui.rs`): interactive terminal UI for browsing files, years, entries, and editing data.

## Useful commands during development

- `cargo check --all` can be run for a quick check
- `cargo fmt --all` runs the formatter

## Snapshot tests

The project uses the [`insta`](https://insta.rs/) crate for snapshot testing,
with most snapshots stored **inline** in the test file. It also has plain
`assert_eq!`-style unit tests in `src/` — for example, see the test modules
in `src/config.rs` and `src/number_formatter.rs`.

Prerequisite: `cargo install cargo-insta`.

### Workflow

When `cargo test` reports failures:

1. **Fix non-snapshot failures first.** They usually point to real bugs or
   test-logic problems; address them before touching snapshots.
2. **Review every snapshot failure.** Open the diff and confirm the new
   output is what you intended. **If you didn't intend to change the output,
   the failure is a real bug — don't accept it, find the cause.**
3. **Accept snapshots.** Use `cargo insta accept` (works in non-interactive
   environments) or `cargo insta review` if your shell supports an
   interactive TUI (safer — confirms each snapshot one at a time).
4. **Sanity-check the accepted diff.** Confirm the changes match exactly
   what you expected and nothing else moved.

### Preferences

- Prefer **inline insta snapshots** (`@r"…"`) when adding or updating
  snapshot assertions. They live next to the test and are easier to keep
  in sync.

## Baseline quality contract

Agents should ensure all of these pass:

- `cargo clippy -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo build`
- `cargo test`
