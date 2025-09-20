use crate::{
    Entry, entries_from_file,
    number_formatter::{FormatOptions, NumberFormatter},
    DELIMITER,
};
use chrono::Datelike;
use chrono::NaiveDate;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    prelude::*,
    widgets::{block::*, *},
};
use rust_decimal::Decimal;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
    fs::OpenOptions,
};
use csv::WriterBuilder;

const FOCUSED_SELECTION_BG_COLOR: Color = Color::from_u32(0x001a1e24);
const UNFOCUSED_SELECTION_BG_COLOR: Color = Color::from_u32(0x00232730);
const SELECTION_INDICATOR_COLOR: Color = Color::Green;

/// Core TUI loop that works with any backend and event source
///
/// Exposed mostly for integration tests.
pub fn run_tui_loop<B, E>(
    files: Vec<PathBuf>,
    format_options: FormatOptions,
    terminal: &mut Terminal<B>,
    events: E,
) -> Result<(), Box<dyn std::error::Error>>
where
    B: ratatui::backend::Backend,
    E: IntoIterator<Item = Event>,
{
    let mut app = App::new(files, format_options);

    // Draw initial state
    terminal.draw(|f| ui(f, &mut app))?;

    // Process events
    for event in events {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match app.popup_mode {
                PopupMode::None => {
                    // Normal navigation mode
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('a') => {
                            app.open_add_entry_popup();
                        }
                        KeyCode::Char('e') => {
                            app.open_edit_entry_popup();
                        }
                        KeyCode::Down => {
                            app.next();
                        }
                        KeyCode::Char('j') => {
                            app.next();
                        }
                        KeyCode::Up => {
                            app.previous();
                        }
                        KeyCode::Char('k') => {
                            app.previous();
                        }
                        KeyCode::Tab => {
                            app.cycle_focus();
                        }
                        _ => {}
                    }
                }
                PopupMode::AddEntry | PopupMode::EditEntry => {
                    // Popup input mode
                    match key.code {
                        KeyCode::Char('q') => {
                            app.close_popup();
                        }
                        KeyCode::Tab => {
                            app.cycle_popup_focus();
                        }
                        KeyCode::Enter => {
                            if let Err(_e) = app.save_popup_entry() {
                                // Handle error - could show error message in popup
                                // For now, just close the popup
                                app.close_popup();
                            }
                        }
                        KeyCode::Backspace => {
                            app.remove_char_from_popup_input();
                        }
                        KeyCode::Char(c) => {
                            app.add_char_to_popup_input(c);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Redraw after each event
        terminal.draw(|f| ui(f, &mut app))?;
    }

    Ok(())
}

pub fn run_tui(
    files: Vec<PathBuf>,
    format_options: FormatOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Event iterator that reads from stdin until quit
    let events = std::iter::from_fn(|| event::read().ok());

    let res = run_tui_loop(files, format_options, &mut terminal, events);

    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    res
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Focus {
    FileSelection,
    Years,
    YearDetails,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PopupMode {
    None,
    AddEntry,
    EditEntry,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum PopupFocus {
    Date,
    Amount,
}

struct App {
    files: Vec<PathBuf>,
    format_options: FormatOptions,
    selected_file: usize,
    report: ReportViewModel,
    focus: Focus,
    selected_year: usize,
    selected_entry: usize,
    // Popup state
    popup_mode: PopupMode,
    popup_focus: PopupFocus,
    popup_date_input: String,
    popup_amount_input: String,
}

#[derive(Default)]
struct ReportViewModel {
    title: String,
    total: String,
    year_reports: Vec<YearReportViewModel>,
}

struct YearReportViewModel {
    title: String,
    subtotal_amount: String,
    lines: Vec<(String, String)>,
    entries: Vec<Entry>, // Store raw entries for editing
}

impl ReportViewModel {
    fn new(
        file: &Path,
        format_options: &FormatOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let entries = entries_from_file(file)?;
        let total: Decimal = entries.iter().map(|entry| entry.amount).sum();
        let mut years_map: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
        for entry in entries {
            let date: NaiveDate = entry.date.parse()?;
            let year = date.year().to_string();
            years_map.entry(year).or_default().push(entry);
        }
        Ok(ReportViewModel {
            title: file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .ok_or("Failed to get file name".to_string())?,
            total: total.format(format_options),
            year_reports: years_map
                .into_iter()
                .map(|(year, entries)| {
                    let subtotal_amount: Decimal = entries.iter().map(|entry| entry.amount).sum();
                    let lines: Vec<(String, String)> = entries
                        .iter()
                        .map(|entry| (entry.date.clone(), entry.amount.format(format_options)))
                        .collect();
                    YearReportViewModel {
                        title: year,
                        subtotal_amount: subtotal_amount.format(format_options),
                        lines,
                        entries,
                    }
                })
                .collect(),
        })
    }
}

impl App {
    fn new(files: Vec<PathBuf>, format_options: FormatOptions) -> Self {
        let mut app = Self {
            files,
            format_options,
            selected_file: 0,
            focus: Focus::FileSelection,
            report: ReportViewModel::default(),
            selected_year: 0,
            selected_entry: 0,
            popup_mode: PopupMode::None,
            popup_focus: PopupFocus::Date,
            popup_date_input: String::new(),
            popup_amount_input: String::new(),
        };
        app.select_file();
        app
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::FileSelection => Focus::Years,
            Focus::Years => Focus::YearDetails,
            Focus::YearDetails => Focus::FileSelection,
        };
        // Reset selection when changing focus areas
        self.selected_entry = 0;
    }

    fn next(&mut self) {
        match self.focus {
            Focus::FileSelection => {
                if self.selected_file + 1 >= self.files.len() {
                    self.selected_file = 0;
                } else {
                    self.selected_file += 1;
                }
                self.select_file();
            }
            Focus::Years => {
                if self.selected_year + 1 >= self.report.year_reports.len() {
                    self.selected_year = 0;
                } else {
                    self.selected_year += 1;
                }
                self.selected_entry = 0;
            }
            Focus::YearDetails => {
                let entry_count = self
                    .report
                    .year_reports
                    .get(self.selected_year)
                    .map(|yr| yr.lines.len())
                    .unwrap_or(0);
                if self.selected_entry + 1 >= entry_count {
                    self.selected_entry = 0;
                } else {
                    self.selected_entry += 1;
                }
            }
        }
    }

    fn previous(&mut self) {
        match self.focus {
            Focus::FileSelection => {
                if self.selected_file == 0 {
                    self.selected_file = self.files.len() - 1;
                } else {
                    self.selected_file -= 1;
                }
                self.select_file();
            }
            Focus::Years => {
                if self.selected_year == 0 {
                    self.selected_year = self.report.year_reports.len().saturating_sub(1);
                } else {
                    self.selected_year -= 1;
                }
                self.selected_entry = 0;
            }
            Focus::YearDetails => {
                let entry_count = self
                    .report
                    .year_reports
                    .get(self.selected_year)
                    .map(|yr| yr.lines.len())
                    .unwrap_or(0);
                if self.selected_entry == 0 {
                    self.selected_entry = entry_count.saturating_sub(1);
                } else {
                    self.selected_entry -= 1;
                }
            }
        }
    }

    fn select_file(&mut self) {
        if let Some(path) = self.files.get(self.selected_file) {
            match ReportViewModel::new(path, &self.format_options) {
                Ok(report) => {
                    self.selected_year = (report.year_reports.len() - 1).max(0);
                    self.report = report;
                }
                Err(e) => eprintln!("Error loading file: {e}"),
            }
        }
    }

    fn create_block<'a>(&self, title: Line<'a>, focus_area: Focus) -> Block<'a> {
        Block::default()
            .title(title.add_modifier(if self.focus == focus_area {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }))
            .borders(Borders::ALL)
            .border_type(if self.focus == focus_area {
                BorderType::Double
            } else {
                BorderType::Plain
            })
    }

    fn open_add_entry_popup(&mut self) {
        self.popup_mode = PopupMode::AddEntry;
        self.popup_focus = PopupFocus::Date;
        // Set current date as default
        self.popup_date_input = chrono::Local::now().date_naive().to_string();
        self.popup_amount_input.clear();
    }

    fn open_edit_entry_popup(&mut self) {
        if let Some(selected_entry) = self.get_selected_entry() {
            let date_input = selected_entry.date.clone();
            let amount_input = selected_entry.amount.to_string();
            
            self.popup_mode = PopupMode::EditEntry;
            self.popup_focus = PopupFocus::Date;
            self.popup_date_input = date_input;
            self.popup_amount_input = amount_input;
        }
    }

    fn close_popup(&mut self) {
        self.popup_mode = PopupMode::None;
        self.popup_date_input.clear();
        self.popup_amount_input.clear();
    }

    fn get_selected_entry(&self) -> Option<&Entry> {
        self.report
            .year_reports
            .get(self.selected_year)?
            .entries
            .get(self.selected_entry)
    }

    fn cycle_popup_focus(&mut self) {
        self.popup_focus = match self.popup_focus {
            PopupFocus::Date => PopupFocus::Amount,
            PopupFocus::Amount => PopupFocus::Date,
        };
    }

    fn add_char_to_popup_input(&mut self, c: char) {
        match self.popup_focus {
            PopupFocus::Date => {
                if self.popup_date_input.len() < 10 { // YYYY-MM-DD format
                    self.popup_date_input.push(c);
                }
            }
            PopupFocus::Amount => {
                // Allow digits, decimal point, and minus sign
                if c.is_ascii_digit() || c == '.' || (c == '-' && self.popup_amount_input.is_empty()) {
                    self.popup_amount_input.push(c);
                }
            }
        }
    }

    fn remove_char_from_popup_input(&mut self) {
        match self.popup_focus {
            PopupFocus::Date => {
                self.popup_date_input.pop();
            }
            PopupFocus::Amount => {
                self.popup_amount_input.pop();
            }
        }
    }

    fn save_popup_entry(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate inputs
        let date = NaiveDate::parse_from_str(&self.popup_date_input, "%Y-%m-%d")
            .map_err(|_| format!("Invalid date format: {}", self.popup_date_input))?;
        let amount = Decimal::from_str(&self.popup_amount_input)
            .map_err(|_| format!("Invalid amount format: {}", self.popup_amount_input))?;
        
        let file_path = &self.files[self.selected_file];
        
        match self.popup_mode {
            PopupMode::AddEntry => {
                self.add_entry_to_file(file_path, date, amount)?;
            }
            PopupMode::EditEntry => {
                self.edit_entry_in_file(file_path, date, amount)?;
            }
            PopupMode::None => {}
        }
        
        // Refresh the report and close popup
        self.select_file();
        self.close_popup();
        
        Ok(())
    }

    fn add_entry_to_file(&self, file_path: &Path, date: NaiveDate, amount: Decimal) -> Result<(), Box<dyn std::error::Error>> {
        let entries = entries_from_file(file_path).unwrap_or_default();
        
        let new_entry = Entry {
            date: date.to_string(),
            amount,
        };

        // Write to the end of the file
        let mut writer = WriterBuilder::new()
            .delimiter(DELIMITER)
            .has_headers(entries.is_empty())
            .from_writer(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)?,
            );

        writer.serialize(new_entry)?;
        writer.flush()?;
        
        Ok(())
    }

    fn edit_entry_in_file(&self, file_path: &Path, date: NaiveDate, amount: Decimal) -> Result<(), Box<dyn std::error::Error>> {
        let mut entries = entries_from_file(file_path)?;
        
        // Find and update the entry
        if let Some(selected_entry) = self.get_selected_entry() {
            // Find the entry by matching date and amount (original values)
            if let Some(entry_to_edit) = entries.iter_mut().find(|e| 
                e.date == selected_entry.date && e.amount == selected_entry.amount
            ) {
                entry_to_edit.date = date.to_string();
                entry_to_edit.amount = amount;
                
                // Rewrite the entire file
                let mut writer = WriterBuilder::new()
                    .delimiter(DELIMITER)
                    .from_writer(
                        OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .open(file_path)?,
                    );

                for entry in entries {
                    writer.serialize(entry)?;
                }
                writer.flush()?;
            }
        }
        
        Ok(())
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(main_layout[0]);

    let files_width = content_layout[0].width.saturating_sub(2) as usize; // Account for block borders
    let files = app.files.iter().enumerate().map(|(i, path)| {
        ListItem::new(make_line(
            path.file_name().unwrap().to_string_lossy(),
            if i == app.selected_file {
                &app.report.total
            } else {
                ""
            },
            i == app.selected_file,
            app.focus == Focus::FileSelection,
            files_width,
        ))
    });

    let highlight_style = Style::default().bg(Color::Blue).fg(Color::Black);
    let files_list = List::new(files)
        .block(app.create_block(Line::from(" Files "), Focus::FileSelection))
        .highlight_style(highlight_style);
    frame.render_stateful_widget(files_list, content_layout[0], &mut ListState::default());

    // Years list (middle column)
    let years_width = content_layout[1].width.saturating_sub(2) as usize; // Account for block borders
    let years_list = List::new(app.report.year_reports.iter().enumerate().map(|(i, year)| {
        ListItem::new(make_line(
            &year.title,
            &year.subtotal_amount,
            i == app.selected_year,
            app.focus == Focus::Years,
            years_width,
        ))
    }))
    .block(app.create_block(Line::from(format!(" {} ", app.report.title)), Focus::Years))
    .highlight_style(highlight_style);

    frame.render_stateful_widget(years_list, content_layout[1], &mut ListState::default());

    // Entries list (right column)
    let entries_width = content_layout[2].width.saturating_sub(2) as usize; // Account for block borders
    let selected_year = &app.report.year_reports[app.selected_year];
    let entries_list = List::new(selected_year.lines.iter().enumerate().map(
        |(i, (date, amount))| {
            ListItem::new(make_line(
                date,
                amount,
                i == app.selected_entry,
                app.focus == Focus::YearDetails,
                entries_width,
            ))
        },
    ))
    .block(app.create_block(
        Line::from(format!(" {} ", selected_year.title)),
        Focus::YearDetails,
    ))
    .highlight_style(highlight_style);

    frame.render_stateful_widget(entries_list, content_layout[2], &mut ListState::default());

    let footer_text = if app.popup_mode == PopupMode::None {
        "↓(j)/↑(k): Navigate | Tab: Focus | a: Add Entry | e: Edit Entry | q: Quit"
    } else {
        "Tab: Switch Field | Enter: Save | q: Cancel"
    };
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, main_layout[1]);

    // Render popup if active
    if app.popup_mode != PopupMode::None {
        render_popup(frame, app);
    }
}

fn render_popup(frame: &mut Frame, app: &App) {
    // Create a centered popup area
    let area = frame.area();
    let popup_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(8),
            Constraint::Percentage(30),
        ])
        .split(area)[1];
    
    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Min(40),
            Constraint::Percentage(20),
        ])
        .split(popup_area)[1];

    // Clear the area
    let clear_block = Block::default()
        .style(Style::default().bg(Color::Black));
    frame.render_widget(Clear, popup_area);
    frame.render_widget(clear_block, popup_area);

    // Create the popup content
    let title = match app.popup_mode {
        PopupMode::AddEntry => " Add New Entry ",
        PopupMode::EditEntry => " Edit Entry ",
        PopupMode::None => "",
    };

    let popup_block = Block::default()
        .title(Line::from(title).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner_area = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);
    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner_area);

    // File name
    let file_name = app.files[app.selected_file]
        .file_name()
        .unwrap()
        .to_string_lossy();
    let file_line = Paragraph::new(format!("File: {}", file_name))
        .style(Style::default().fg(Color::White));
    frame.render_widget(file_line, content_layout[0]);

    // Date field
    let date_style = if app.popup_focus == PopupFocus::Date {
        Style::default().bg(Color::Blue).fg(Color::White)
    } else {
        Style::default().fg(Color::White)
    };
    let date_line = Paragraph::new(format!("Date: {}", app.popup_date_input))
        .style(date_style);
    frame.render_widget(date_line, content_layout[2]);

    // Amount field
    let amount_style = if app.popup_focus == PopupFocus::Amount {
        Style::default().bg(Color::Blue).fg(Color::White)
    } else {
        Style::default().fg(Color::White)
    };
    let amount_line = Paragraph::new(format!("Amount: {}", app.popup_amount_input))
        .style(amount_style);
    frame.render_widget(amount_line, content_layout[3]);
}

fn make_line<'a>(
    left: impl Into<std::borrow::Cow<'a, str>>,
    right: &'a str,
    is_selected: bool,
    is_focused: bool,
    width: usize,
) -> Line<'a> {
    let padding_span_left = if is_selected {
        if is_focused {
            Span::raw("▌").style(SELECTION_INDICATOR_COLOR)
        } else {
            Span::raw("▎")
        }
    } else {
        Span::raw(" ")
    };
    let padding_span_right = Span::raw(" ");
    let left_span = Span::raw(left);
    let right_span = Span::raw(right);
    let spacer = " ".repeat(width.saturating_sub(
        left_span.width()
            + right_span.width()
            + padding_span_left.width()
            + padding_span_right.width(),
    ));
    let line = Line::from(vec![
        padding_span_left,
        left_span,
        Span::raw(spacer),
        right_span,
        padding_span_right,
    ]);
    if is_selected {
        let bg_color = if is_focused {
            FOCUSED_SELECTION_BG_COLOR
        } else {
            UNFOCUSED_SELECTION_BG_COLOR
        };
        line.style(Style::default().bg(bg_color))
    } else {
        line
    }
}
