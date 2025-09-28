use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use insta::assert_snapshot;
use mfinance::{number_formatter::FormatOptions, tui::run_tui_loop};
use ratatui::{Terminal, backend::TestBackend};
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
        let files = self.files.clone();
        let format_options = Self::format_options();
        let backend = TestBackend::new(86, 20);
        let mut terminal = Terminal::new(backend).expect("terminal created");

        run_tui_loop(files, format_options, &mut terminal, events)
            .expect("tui loop finished successfully");

        format!("{:?}", terminal.backend().buffer())
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
        TuiTestFixture::key_event(KeyCode::Char('k')), // Move to first file
        TuiTestFixture::key_event(KeyCode::Up),        // Should wrap to last file
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

#[test]
fn test_add_entry_popup_open() {
    let fixture = TuiTestFixture::new();

    // Test opening add entry popup with 'a' key
    let add_popup_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('a')), // Open add entry popup
    ]);

    let mut settings = insta::Settings::clone_current();

    let current_date = chrono::Local::now().date_naive().to_string();
    settings.add_filter(&current_date, "0000-00-00");
    settings.bind(|| {
        assert_snapshot!("add_entry_popup", add_popup_output);
    });
}

#[test]
fn test_edit_entry_popup_open() {
    let fixture = TuiTestFixture::new();

    // Test opening edit entry popup with 'e' key
    let edit_popup_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Tab),       // Years focus
        TuiTestFixture::key_event(KeyCode::Tab),       // Year details focus
        TuiTestFixture::key_event(KeyCode::Char('e')), // Open edit entry popup
    ]);
    assert_snapshot!("edit_entry_popup", edit_popup_output);
}

#[test]
fn test_popup_input_and_focus() {
    let fixture = TuiTestFixture::new();

    // Test popup input and focus switching
    let popup_input_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('a')), // Open add entry popup
        TuiTestFixture::key_event(KeyCode::Backspace), // Delete some of default date
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Char('1')), // Change date
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Tab), // Switch to amount field
        TuiTestFixture::key_event(KeyCode::Char('1')), // Enter amount
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Char('0')),
    ]);
    assert_snapshot!("popup_input_focus", popup_input_output);
}

#[test]
fn test_popup_close_with_q() {
    let fixture = TuiTestFixture::new();

    // Test closing popup with 'q' key
    let popup_close_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('a')), // Open add entry popup
        TuiTestFixture::key_event(KeyCode::Char('q')), // Close popup
    ]);
    assert_snapshot!("popup_close", popup_close_output);
}

#[test]
fn test_add_entry_save_functionality() {
    let fixture = TuiTestFixture::new();

    // Get the CSV content before adding entry
    let file_path = &fixture.files[0];
    let initial_content = std::fs::read_to_string(file_path).unwrap();

    // Test adding an entry with valid input (this should save and close popup)
    let _output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('a')), // Open add entry popup
        TuiTestFixture::key_event(KeyCode::Backspace), // Clear current date
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Char('2')), // Enter new date
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Char('2')),
        TuiTestFixture::key_event(KeyCode::Char('4')),
        TuiTestFixture::key_event(KeyCode::Char('-')),
        TuiTestFixture::key_event(KeyCode::Char('1')),
        TuiTestFixture::key_event(KeyCode::Char('2')),
        TuiTestFixture::key_event(KeyCode::Char('-')),
        TuiTestFixture::key_event(KeyCode::Char('1')),
        TuiTestFixture::key_event(KeyCode::Char('5')),
        TuiTestFixture::key_event(KeyCode::Tab), // Switch to amount field
        TuiTestFixture::key_event(KeyCode::Char('5')), // Enter amount
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Enter), // Save entry
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

    // Test error message display with invalid date
    let invalid_date_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('a')), // Open add entry popup
        TuiTestFixture::key_event(KeyCode::Backspace), // Clear current date
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Char('i')), // Enter invalid date
        TuiTestFixture::key_event(KeyCode::Char('n')),
        TuiTestFixture::key_event(KeyCode::Char('v')),
        TuiTestFixture::key_event(KeyCode::Char('a')),
        TuiTestFixture::key_event(KeyCode::Char('l')),
        TuiTestFixture::key_event(KeyCode::Char('i')),
        TuiTestFixture::key_event(KeyCode::Char('d')),
        TuiTestFixture::key_event(KeyCode::Tab), // Switch to amount field
        TuiTestFixture::key_event(KeyCode::Char('1')), // Enter valid amount
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Enter), // Try to save (should show error)
    ]);

    assert_snapshot!("popup_error_invalid_date", invalid_date_output);
}

#[test]
fn test_popup_error_clearing() {
    let fixture = TuiTestFixture::new();

    // Test that error message is cleared when user starts typing
    let error_cleared_output = fixture.run_with_events(vec![
        TuiTestFixture::key_event(KeyCode::Char('a')), // Open add entry popup
        TuiTestFixture::key_event(KeyCode::Backspace), // Clear current date
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Backspace),
        TuiTestFixture::key_event(KeyCode::Char('b')), // Enter invalid date
        TuiTestFixture::key_event(KeyCode::Char('a')),
        TuiTestFixture::key_event(KeyCode::Char('d')),
        TuiTestFixture::key_event(KeyCode::Tab), // Switch to amount field
        TuiTestFixture::key_event(KeyCode::Char('1')), // Enter valid amount
        TuiTestFixture::key_event(KeyCode::Char('0')),
        TuiTestFixture::key_event(KeyCode::Enter), // Try to save (should show error)
        TuiTestFixture::key_event(KeyCode::Char('2')), // Start typing to clear error
    ]);

    assert_snapshot!("popup_error_cleared", error_cleared_output);
}
