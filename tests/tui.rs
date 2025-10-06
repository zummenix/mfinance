use insta::assert_snapshot;
use mfinance::{number_formatter::FormatOptions, tui::run_tui_loop};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{Terminal, backend::TestBackend};
use std::{fs, path::PathBuf};
use temp_dir::TempDir;

struct TuiTestFixture {
    #[allow(dead_code)] // Used to keep temp directory alive
    tempdir: TempDir,
    files: Vec<PathBuf>,
    is_with_styles: bool,
}

impl TuiTestFixture {
    fn new() -> Self {
        let tempdir = TempDir::with_prefix("mfinance-tui-test-").unwrap();
        let mut files = Vec::new();

        // Create test CSV files with different data patterns
        let file1_path = tempdir.child("expenses.csv");
        fs::write(
            &file1_path,
            "date;amount\n2024-01-15;-50.25\n2024-02-20;-100.00\n2024-03-10;-25.50\n2025-01-05;-75.75\n"
        ).expect("write expenses.csv");
        files.push(file1_path);

        let file2_path = tempdir.child("income.csv");
        fs::write(
            &file2_path,
            "date;amount\n2024-01-01;2000.00\n2024-02-01;2000.00\n2024-03-01;2000.00\n2025-01-01;2000.00\n"
        ).expect("write income.csv");
        files.push(file2_path);

        let file3_path = tempdir.child("savings.csv");
        fs::write(
            &file3_path,
            "date;amount\n2024-06-15;500.00\n2024-12-31;1000.00\n",
        )
        .expect("write savings.csv");
        files.push(file3_path);

        TuiTestFixture {
            tempdir,
            files,
            is_with_styles: false,
        }
    }

    fn format_options() -> FormatOptions {
        FormatOptions::default()
    }

    /// Run TUI with events and return final buffer content
    fn run_with_events(&self, events: impl IntoIterator<Item = Vec<Event>>) -> String {
        let files = self.files.clone();
        let format_options = Self::format_options();
        let backend = TestBackend::new(86, 20);
        let mut terminal = Terminal::new(backend).expect("terminal created");

        run_tui_loop(
            files,
            format_options,
            &mut terminal,
            events.into_iter().flatten(),
        )
        .expect("tui loop finished successfully");

        if self.is_with_styles {
            format!("{:?}", terminal.backend().buffer())
        } else {
            format!("{}", terminal.backend())
        }
    }
}

/// Helper to create key event
fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: ratatui::crossterm::event::KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: ratatui::crossterm::event::KeyEventState::empty(),
    })
}

fn press_down() -> Vec<Event> {
    vec![key_event(KeyCode::Down)]
}

fn press_up() -> Vec<Event> {
    vec![key_event(KeyCode::Up)]
}

fn press_tab() -> Vec<Event> {
    vec![key_event(KeyCode::Tab)]
}

fn press_backspace() -> Vec<Event> {
    vec![key_event(KeyCode::Backspace)]
}

fn press_enter() -> Vec<Event> {
    vec![key_event(KeyCode::Enter)]
}

fn press_add_entry() -> Vec<Event> {
    vec![key_event(KeyCode::Char('a'))]
}

fn press_edit_entry() -> Vec<Event> {
    vec![key_event(KeyCode::Char('e'))]
}

fn press_close_popup() -> Vec<Event> {
    vec![key_event(KeyCode::Char('q'))]
}

fn type_text(s: &str) -> Vec<Event> {
    s.chars().map(|ch| key_event(KeyCode::Char(ch))).collect()
}

fn repeat(events: Vec<Event>, n_times: usize) -> Vec<Event> {
    let mut result: Vec<Event> = Vec::with_capacity(events.len() * n_times);
    for _ in 0..n_times {
        result.extend(events.iter().cloned());
    }
    result
}

#[test]
fn test_initial_display() {
    let mut fixture = TuiTestFixture::new();
    fixture.is_with_styles = true;

    // Test initial state with no events
    let output = fixture.run_with_events(vec![]);
    assert_snapshot!("initial_display", output);
}

#[test]
fn test_down_or_j() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![type_text("j"), press_down()]);

    assert_snapshot!(output, @r#"
    "╔ Files ════════════════════╗┌ savings.csv ─────────────┐┌ 2024 ─────────────────────┐"
    "║ expenses.csv              ║│▎2024            1 500.00 ││▎June 15            500.00 │"
    "║ income.csv                ║│                          ││ December 31      1 000.00 │"
    "║▌savings.csv      1 500.00 ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "╚═══════════════════════════╝└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_up_or_k() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![type_text("k"), press_up()]);

    assert_snapshot!(output, @r#"
    "╔ Files ════════════════════╗┌ income.csv ──────────────┐┌ 2025 ─────────────────────┐"
    "║ expenses.csv              ║│ 2024            6 000.00 ││▎January 1        2 000.00 │"
    "║▌income.csv       8 000.00 ║│▎2025            2 000.00 ││                           │"
    "║ savings.csv               ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "╚═══════════════════════════╝└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_focus_on_years() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![press_tab()]);
    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐╔ expenses.csv ════════════╗┌ 2025 ─────────────────────┐"
    "│▎expenses.csv      -251.50 │║ 2024             -175.75 ║│▎January 5          -75.75 │"
    "│ income.csv                │║▌2025              -75.75 ║│                           │"
    "│ savings.csv               │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "└───────────────────────────┘╚══════════════════════════╝└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_focus_on_entries() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![repeat(press_tab(), 2)]);
    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐┌ expenses.csv ────────────┐╔ 2025 ═════════════════════╗"
    "│▎expenses.csv      -251.50 ││ 2024             -175.75 │║▌January 5          -75.75 ║"
    "│ income.csv                ││▎2025              -75.75 │║                           ║"
    "│ savings.csv               ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "└───────────────────────────┘└──────────────────────────┘╚═══════════════════════════╝"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_cycle_back_focus_on_files() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![repeat(press_tab(), 3)]);
    assert_snapshot!(output, @r#"
    "╔ Files ════════════════════╗┌ expenses.csv ────────────┐┌ 2025 ─────────────────────┐"
    "║▌expenses.csv      -251.50 ║│ 2024             -175.75 ││▎January 5          -75.75 │"
    "║ income.csv                ║│▎2025              -75.75 ││                           │"
    "║ savings.csv               ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "╚═══════════════════════════╝└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_years_navigation() {
    let fixture = TuiTestFixture::new();

    let to_years = press_tab();
    let to_first_year = press_up();
    let output = fixture.run_with_events(vec![to_years, to_first_year]);
    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐╔ expenses.csv ════════════╗┌ 2024 ─────────────────────┐"
    "│▎expenses.csv      -251.50 │║▌2024             -175.75 ║│▎January 15         -50.25 │"
    "│ income.csv                │║ 2025              -75.75 ║│ February 20       -100.00 │"
    "│ savings.csv               │║                          ║│ March 10           -25.50 │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "│                           │║                          ║│                           │"
    "└───────────────────────────┘╚══════════════════════════╝└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_entries_navigation() {
    let fixture = TuiTestFixture::new();

    let to_years = press_tab();
    let to_entries = press_tab();
    let cycle_to_first_year = press_down();
    let to_last_line = repeat(press_down(), 2);
    let output = fixture.run_with_events(vec![
        to_years,
        cycle_to_first_year,
        to_entries,
        to_last_line,
    ]);
    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐┌ expenses.csv ────────────┐╔ 2024 ═════════════════════╗"
    "│▎expenses.csv      -251.50 ││▎2024             -175.75 │║ January 15         -50.25 ║"
    "│ income.csv                ││ 2025              -75.75 │║ February 20       -100.00 ║"
    "│ savings.csv               ││                          │║▌March 10           -25.50 ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "│                           ││                          │║                           ║"
    "└───────────────────────────┘└──────────────────────────┘╚═══════════════════════════╝"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_add_entry_popup_open() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![press_add_entry()]);

    let mut settings = insta::Settings::clone_current();
    let current_date = chrono::Local::now().date_naive().to_string();
    settings.add_filter(&current_date, "0000-00-00");
    settings.bind(|| {
        assert_snapshot!(output, @r#"
        "┌ Files ────────────────────┐┌ expenses.csv ────────────┐┌ 2025 ─────────────────────┐"
        "│▎expenses.csv      -251.50 ││ 2024             -175.75 ││▎January 5          -75.75 │"
        "│ income.csv                ││▎2025              -75.75 ││                           │"
        "│ savings.csv               ││                          ││                           │"
        "│                           ││                          ││                           │"
        "│                           ││                          ││                           │"
        "│                ╔ Add New Entry ═══════════════════════════════════╗                │"
        "│                ║ File    expenses.csv                             ║                │"
        "│                ║                                                  ║                │"
        "│                ║▌Date    0000-00-00                               ║                │"
        "│                ║ Amount                                           ║                │"
        "│                ║                                                  ║                │"
        "│                ║                                                  ║                │"
        "│                ╚══════════════════════════════════════════════════╝                │"
        "│                           ││                          ││                           │"
        "│                           ││                          ││                           │"
        "└───────────────────────────┘└──────────────────────────┘└───────────────────────────┘"
        "┌────────────────────────────────────────────────────────────────────────────────────┐"
        "│Tab: Switch Field | Enter: Save | q: Cancel                                         │"
        "└────────────────────────────────────────────────────────────────────────────────────┘"
        "#);
    });
}

#[test]
fn test_edit_entry_popup_open() {
    let fixture = TuiTestFixture::new();

    let to_second_file = press_down();
    let to_entries = repeat(press_tab(), 2);
    let output = fixture.run_with_events(vec![to_second_file, to_entries, press_edit_entry()]);

    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐┌ income.csv ──────────────┐┌ 2025 ─────────────────────┐"
    "│ expenses.csv              ││ 2024            6 000.00 ││▎January 1        2 000.00 │"
    "│▎income.csv       8 000.00 ││▎2025            2 000.00 ││                           │"
    "│ savings.csv               ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                ╔ Edit Entry ══════════════════════════════════════╗                │"
    "│                ║ File    income.csv                               ║                │"
    "│                ║                                                  ║                │"
    "│                ║▌Date    2025-01-01                               ║                │"
    "│                ║ Amount  2000                                     ║                │"
    "│                ║                                                  ║                │"
    "│                ║                                                  ║                │"
    "│                ╚══════════════════════════════════════════════════╝                │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "└───────────────────────────┘└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│Tab: Switch Field | Enter: Save | q: Cancel                                         │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_popup_input_and_focus() {
    let fixture = TuiTestFixture::new();

    let delete_day_date = repeat(press_backspace(), 2);
    let change_day_date = type_text("10");
    let switch_to_amount_field = press_tab();
    let delete_old_amount = repeat(press_backspace(), 10);
    let enter_new_amount = type_text("100");
    let output = fixture.run_with_events(vec![
        press_edit_entry(),
        delete_day_date,
        change_day_date,
        switch_to_amount_field,
        delete_old_amount,
        enter_new_amount,
    ]);

    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐┌ expenses.csv ────────────┐┌ 2025 ─────────────────────┐"
    "│▎expenses.csv      -251.50 ││ 2024             -175.75 ││▎January 5          -75.75 │"
    "│ income.csv                ││▎2025              -75.75 ││                           │"
    "│ savings.csv               ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                ╔ Edit Entry ══════════════════════════════════════╗                │"
    "│                ║ File    expenses.csv                             ║                │"
    "│                ║                                                  ║                │"
    "│                ║ Date    2025-01-10                               ║                │"
    "│                ║▌Amount  100                                      ║                │"
    "│                ║                                                  ║                │"
    "│                ║                                                  ║                │"
    "│                ╚══════════════════════════════════════════════════╝                │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "└───────────────────────────┘└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│Tab: Switch Field | Enter: Save | q: Cancel                                         │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_popup_close() {
    let fixture = TuiTestFixture::new();

    let output = fixture.run_with_events(vec![press_add_entry(), press_close_popup()]);

    assert_snapshot!(output, @r#"
    "╔ Files ════════════════════╗┌ expenses.csv ────────────┐┌ 2025 ─────────────────────┐"
    "║▌expenses.csv      -251.50 ║│ 2024             -175.75 ││▎January 5          -75.75 │"
    "║ income.csv                ║│▎2025              -75.75 ││                           │"
    "║ savings.csv               ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "║                           ║│                          ││                           │"
    "╚═══════════════════════════╝└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│↓(j)/↑(k): Navigate | Tab: Focus | a/e: Add/Edit Entry | q: Quit                    │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_add_entry_save_functionality() {
    let fixture = TuiTestFixture::new();

    // Get the CSV content before adding entry
    let file_path = &fixture.files[0];
    let initial_content = std::fs::read_to_string(file_path).unwrap();

    let delete_old_date = repeat(press_backspace(), 10);
    let enter_new_date = type_text("2024-12-15");
    let switch_to_amount_field = press_tab();
    let enter_amount = type_text("500");
    let save_and_close_popup = press_enter();
    let _output = fixture.run_with_events(vec![
        press_add_entry(),
        delete_old_date,
        enter_new_date,
        switch_to_amount_field,
        enter_amount,
        save_and_close_popup,
    ]);

    // Check that the CSV file was updated
    let final_content = std::fs::read_to_string(file_path).unwrap();
    assert_ne!(
        initial_content, final_content,
        "CSV file should have been modified"
    );
    assert!(
        final_content.contains("2024-12-15"),
        "Should contain new date"
    );
    assert!(final_content.contains("500"), "Should contain new amount");
}

#[test]
fn test_popup_error_handling() {
    let fixture = TuiTestFixture::new();

    let delete_old_date = repeat(press_backspace(), 10);
    let enter_invalid_date = type_text("invalid");
    let switch_to_amount_field = press_tab();
    let enter_valid_amount = type_text("500");
    let try_to_save = press_enter();
    let output = fixture.run_with_events(vec![
        press_add_entry(),
        delete_old_date,
        enter_invalid_date,
        switch_to_amount_field,
        enter_valid_amount,
        try_to_save,
    ]);

    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐┌ expenses.csv ────────────┐┌ 2025 ─────────────────────┐"
    "│▎expenses.csv      -251.50 ││ 2024             -175.75 ││▎January 5          -75.75 │"
    "│ income.csv                ││▎2025              -75.75 ││                           │"
    "│ savings.csv               ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                ╔ Add New Entry ═══════════════════════════════════╗                │"
    "│                ║ File    expenses.csv                             ║                │"
    "│                ║                                                  ║                │"
    "│                ║ Date    invalid                                  ║                │"
    "│                ║▌Amount  500                                      ║                │"
    "│                ║ Error: Invalid date format. Use YYYY-MM-DD       ║                │"
    "│                ║                                                  ║                │"
    "│                ╚══════════════════════════════════════════════════╝                │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "└───────────────────────────┘└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│Tab: Switch Field | Enter: Save | q: Cancel                                         │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}

#[test]
fn test_popup_error_clearing() {
    let fixture = TuiTestFixture::new();

    let delete_old_date = repeat(press_backspace(), 10);
    let enter_invalid_date = type_text("bad");
    let switch_to_amount_field = press_tab();
    let enter_valid_amount = type_text("10");
    let try_to_save = press_enter();
    let start_typing_to_clear_error = type_text("2");
    let output = fixture.run_with_events(vec![
        press_add_entry(),
        delete_old_date,
        enter_invalid_date,
        switch_to_amount_field,
        enter_valid_amount,
        try_to_save,
        start_typing_to_clear_error,
    ]);

    assert_snapshot!(output, @r#"
    "┌ Files ────────────────────┐┌ expenses.csv ────────────┐┌ 2025 ─────────────────────┐"
    "│▎expenses.csv      -251.50 ││ 2024             -175.75 ││▎January 5          -75.75 │"
    "│ income.csv                ││▎2025              -75.75 ││                           │"
    "│ savings.csv               ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "│                ╔ Add New Entry ═══════════════════════════════════╗                │"
    "│                ║ File    expenses.csv                             ║                │"
    "│                ║                                                  ║                │"
    "│                ║ Date    bad                                      ║                │"
    "│                ║▌Amount  102                                      ║                │"
    "│                ║                                                  ║                │"
    "│                ║                                                  ║                │"
    "│                ╚══════════════════════════════════════════════════╝                │"
    "│                           ││                          ││                           │"
    "│                           ││                          ││                           │"
    "└───────────────────────────┘└──────────────────────────┘└───────────────────────────┘"
    "┌────────────────────────────────────────────────────────────────────────────────────┐"
    "│Tab: Switch Field | Enter: Save | q: Cancel                                         │"
    "└────────────────────────────────────────────────────────────────────────────────────┘"
    "#);
}
