use insta::assert_snapshot;
use mfinance::{
    tui::{App, ui},
    number_formatter::FormatOptions,
};
use ratatui::{
    backend::TestBackend,
    Terminal,
};
use std::{
    fs,
    path::PathBuf,
};
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
            "date;amount\n2024-06-15;500.00\n2024-12-31;1000.00\n"
        ).expect("write savings.csv");
        files.push(file3_path);

        TuiTestFixture { tempdir, files }
    }

    fn create_app(&self) -> App {
        App::new(self.files.clone(), FormatOptions::default())
    }
}

fn render_app_to_string(app: &mut App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    
    // Render the app to the terminal
    terminal.draw(|f| ui(f, app)).unwrap();
    
    // Return debug representation of the buffer for now
    format!("{:?}", terminal.backend().buffer())
}

#[test]
fn test_initial_display() {
    let fixture = TuiTestFixture::new();
    let mut app = fixture.create_app();
    
    let output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("initial_display", output);
}

#[test] 
fn test_file_navigation() {
    let fixture = TuiTestFixture::new();
    let mut app = fixture.create_app();
    
    // Test navigation through files
    app.next(); // Move to second file
    let output_file2 = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("file_navigation_second", output_file2);
    
    app.next(); // Move to third file  
    let output_file3 = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("file_navigation_third", output_file3);
    
    app.next(); // Should wrap to first file
    let output_wrapped = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("file_navigation_wrapped", output_wrapped);
}

#[test]
fn test_focus_cycling() {
    let fixture = TuiTestFixture::new();
    let mut app = fixture.create_app();
    
    // Initial focus should be on file selection
    let initial_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("focus_files", initial_output);
    
    // Cycle to years focus
    app.cycle_focus();
    let years_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("focus_years", years_output);
    
    // Cycle to year details focus
    app.cycle_focus(); 
    let details_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("focus_year_details", details_output);
    
    // Cycle back to file focus
    app.cycle_focus();
    let back_to_files = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("focus_back_to_files", back_to_files);
}

#[test]
fn test_year_navigation() {
    let fixture = TuiTestFixture::new();
    let mut app = fixture.create_app();
    
    // Switch to years focus
    app.cycle_focus();
    
    // Navigate through years
    app.previous(); // Should move to previous year (2024)
    let year_2024_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("year_navigation_2024", year_2024_output);
    
    app.next(); // Move back to 2025
    let year_2025_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("year_navigation_2025", year_2025_output);
}

#[test]
fn test_entry_navigation() {
    let fixture = TuiTestFixture::new();
    let mut app = fixture.create_app();
    
    // Switch to entry details focus
    app.cycle_focus(); // Years
    app.cycle_focus(); // Year details
    
    // Navigate through entries in the selected year
    app.next(); // Move to next entry
    let entry_1_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("entry_navigation_1", entry_1_output);
    
    app.previous(); // Move back to first entry
    let entry_0_output = render_app_to_string(&mut app, 120, 30);
    assert_snapshot!("entry_navigation_0", entry_0_output);
}