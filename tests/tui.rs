use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use insta::assert_snapshot;
use mfinance::{number_formatter::FormatOptions, tui::run_tui_with_events_test};
use std::{fs, path::PathBuf};
use temp_dir::TempDir;

struct TuiTestFixture {
    #[allow(dead_code)] // Used to keep temp directory alive
    tempdir: TempDir,
    files: Vec<PathBuf>,
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

        TuiTestFixture { tempdir, files }
    }

    fn format_options() -> FormatOptions {
        FormatOptions::default()
    }

    /// Helper to create key event
    fn key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: crossterm::event::KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        })
    }

    /// Run TUI with events and return final buffer content
    fn run_with_events(&self, events: Vec<Event>) -> String {
        run_tui_with_events_test(
            self.files.clone(),
            Self::format_options(),
            events,
            120, // width
            30,  // height
        )
        .expect("TUI should run without error")
    }
}

#[test]
fn test_initial_display() {
    let fixture = TuiTestFixture::new();

    // Test initial state with no events
    let output = fixture.run_with_events(vec![]);
    assert_snapshot!("initial_display", output);
}

#[test]
fn test_file_navigation_with_keys() {
    let fixture = TuiTestFixture::new();

    // Test navigation through files with j/k keys
    let output_file2 = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('j')), // Move to second file
    ]);
    assert_snapshot!("file_navigation_second", output_file2);

    let output_file3 = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('j')), // Move to second file
        TuiTestFixture::key_event(KeyCode::Down),      // Move to third file
    ]);
    assert_snapshot!("file_navigation_third", output_file3);

    let output_wrapped = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('j')), // Move to second file
        TuiTestFixture::key_event(KeyCode::Down),      // Move to third file
        TuiTestFixture::key_event(KeyCode::Char('j')), // Should wrap to first file
    ]);
    assert_snapshot!("file_navigation_wrapped", output_wrapped);
}

#[test]
fn test_focus_cycling_with_tab() {
    let fixture = TuiTestFixture::new();

    // Initial focus should be on file selection
    let initial_output = fixture.run_with_events(vec![]);
    assert_snapshot!("focus_files", initial_output);

    // Cycle to years focus with Tab
    let years_output = fixture.run_with_events(vec![TuiTestFixture::key_event(KeyCode::Tab)]);
    assert_snapshot!("focus_years", years_output);

    // Cycle to year details focus with Tab
    let details_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab),
        TuiTestFixture::key_event(KeyCode::Tab),
    ]);
    assert_snapshot!("focus_year_details", details_output);

    // Cycle back to file focus with Tab
    let back_to_files = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab),
        TuiTestFixture::key_event(KeyCode::Tab),
        TuiTestFixture::key_event(KeyCode::Tab),
    ]);
    assert_snapshot!("focus_back_to_files", back_to_files);
}

#[test]
fn test_year_navigation_with_keys() {
    let fixture = TuiTestFixture::new();

    // Switch to years focus and navigate through years
    let year_2024_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab), // Switch to years focus
        TuiTestFixture::key_event(KeyCode::Char('k')), // Move to previous year (2024)
    ]);
    assert_snapshot!("year_navigation_2024", year_2024_output);

    let year_2025_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab), // Switch to years focus
        TuiTestFixture::key_event(KeyCode::Char('k')), // Move to 2024
        TuiTestFixture::key_event(KeyCode::Up),  // Move back to 2025
    ]);
    assert_snapshot!("year_navigation_2025", year_2025_output);
}

#[test]
fn test_entry_navigation_with_keys() {
    let fixture = TuiTestFixture::new();

    // Switch to entry details focus and navigate through entries
    let entry_1_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab),       // Years focus
        TuiTestFixture::key_event(KeyCode::Tab),       // Year details focus
        TuiTestFixture::key_event(KeyCode::Char('j')), // Move to next entry
    ]);
    assert_snapshot!("entry_navigation_1", entry_1_output);

    let entry_0_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab),       // Years focus
        TuiTestFixture::key_event(KeyCode::Tab),       // Year details focus
        TuiTestFixture::key_event(KeyCode::Down),      // Move to next entry
        TuiTestFixture::key_event(KeyCode::Char('k')), // Move back to first entry
    ]);
    assert_snapshot!("entry_navigation_0", entry_0_output);
}
