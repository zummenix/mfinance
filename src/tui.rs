use crate::add_entry;
use crate::{
    DELIMITER, Entry, entries_from_file,
    number_formatter::{FormatOptions, NumberFormatter},
};
use chrono::Datelike;
use chrono::NaiveDate;
use csv::WriterBuilder;
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    layout::Position as CursorPosition,
    prelude::*,
    widgets::{block::*, *},
};
use rust_decimal::Decimal;
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    path::{Path, PathBuf},
    str::FromStr,
};
use tui_input::{Input, backend::crossterm::EventHandler};

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
    let files = files
        .into_iter()
        .map(|path| File::new(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut app = App::new(files, format_options);

    // Draw initial state
    terminal.draw(|f| ui(f, &mut app))?;

    // Process events
    for event in events {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match app.popup.mode {
                PopupMode::None => {
                    // Normal navigation mode
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('n') => {
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
                            app.handle_saving_popup_entry();
                        }
                        KeyCode::Backspace | KeyCode::Char(_) => {
                            app.handle_popup_input(key);
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
    Files,
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
    files: Vec<File>,
    format_options: FormatOptions,
    report: ReportViewModel,
    selection: Selection,
    focus: Focus,
    popup: Popup,
}

struct Popup {
    mode: PopupMode,
    focus: PopupFocus,
    date_input: Input,
    amount_input: Input,
    error_message: Option<String>,
}

impl Popup {
    fn new() -> Self {
        Popup {
            mode: PopupMode::None,
            focus: PopupFocus::Date,
            date_input: Input::default(),
            amount_input: Input::default(),
            error_message: None,
        }
    }
}

#[derive(Default)]
struct Selection {
    file: usize,
    year: usize,
    entry: usize,
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
        file: &File,
        format_options: &FormatOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let entries = entries_from_file(&file.path)?;
        let total: Decimal = entries.iter().map(|entry| entry.amount).sum();
        let mut years_map: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
        for entry in entries {
            let date: NaiveDate = entry.date.parse()?;
            let year = date.year().to_string();
            years_map.entry(year).or_default().push(entry);
        }
        Ok(ReportViewModel {
            title: file.name.clone(),
            total: total.format(format_options),
            year_reports: years_map
                .into_iter()
                .map(|(year, entries)| {
                    let subtotal_amount: Decimal = entries.iter().map(|entry| entry.amount).sum();
                    let lines: Vec<(String, String)> = entries
                        .iter()
                        .map(|entry| (entry.day_month_date(), entry.amount.format(format_options)))
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

struct File {
    path: PathBuf,
    name: String,
}

impl File {
    fn new(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or(format!(
                "Failed to get file name for path: {}",
                path.display()
            ))?;
        Ok(File { path, name })
    }
}

impl App {
    fn new(files: Vec<File>, format_options: FormatOptions) -> Self {
        let mut app = Self {
            files,
            format_options,
            focus: Focus::Files,
            report: ReportViewModel::default(),
            selection: Selection::default(),
            popup: Popup::new(),
        };
        app.reload_file();
        app.select_last_year();
        app.select_last_entry();
        app
    }

    fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Files => Focus::Years,
            Focus::Years => Focus::YearDetails,
            Focus::YearDetails => Focus::Files,
        };
    }

    fn next(&mut self) {
        match self.focus {
            Focus::Files => {
                self.selection.file = next_index_cycled(self.selection.file, self.files.len());
                self.reload_file();
                self.select_last_year();
                self.select_last_entry();
            }
            Focus::Years => {
                self.selection.year =
                    next_index_cycled(self.selection.year, self.report.year_reports.len());
                self.select_last_entry();
            }
            Focus::YearDetails => {
                self.selection.entry =
                    next_index_cycled(self.selection.entry, self.year_entries_count());
            }
        }
    }

    fn previous(&mut self) {
        match self.focus {
            Focus::Files => {
                self.selection.file = previous_index_cycled(self.selection.file, self.files.len());
                self.reload_file();
                self.select_last_year();
                self.select_last_entry();
            }
            Focus::Years => {
                self.selection.year =
                    previous_index_cycled(self.selection.year, self.report.year_reports.len());
                self.select_last_entry();
            }
            Focus::YearDetails => {
                self.selection.entry =
                    previous_index_cycled(self.selection.entry, self.year_entries_count());
            }
        }
    }

    fn reload_file(&mut self) {
        if let Some(path) = self.files.get(self.selection.file) {
            match ReportViewModel::new(path, &self.format_options) {
                Ok(report) => {
                    self.report = report;
                }
                Err(e) => eprintln!("Error loading file: {e}"),
            }
        }
    }

    fn select_last_year(&mut self) {
        self.selection.year = self.report.year_reports.len().saturating_sub(1);
    }

    fn select_last_entry(&mut self) {
        self.selection.entry = self
            .report
            .year_reports
            .get(self.selection.year)
            .map(|year| year.lines.len().saturating_sub(1))
            .unwrap_or(0);
    }

    fn year_entries_count(&self) -> usize {
        self.report
            .year_reports
            .get(self.selection.year)
            .map(|year| year.lines.len())
            .unwrap_or(0)
    }

    fn open_add_entry_popup(&mut self) {
        self.popup.mode = PopupMode::AddEntry;
        self.popup.focus = PopupFocus::Amount;
        // Set current date as default
        self.popup.date_input = Input::new(chrono::Local::now().date_naive().to_string());
        self.popup.amount_input = Input::default();
        self.popup.error_message = None;
    }

    fn open_edit_entry_popup(&mut self) {
        if let Some(selected_entry) = self.get_selected_entry() {
            let date_input = selected_entry.date.clone();
            let amount_input = selected_entry.amount.to_string();

            self.popup.mode = PopupMode::EditEntry;
            self.popup.focus = PopupFocus::Date;
            self.popup.date_input = Input::new(date_input);
            self.popup.amount_input = Input::new(amount_input);
            self.popup.error_message = None;
        }
    }

    fn close_popup(&mut self) {
        self.popup = Popup::new();
    }

    fn get_selected_entry(&self) -> Option<&Entry> {
        self.report
            .year_reports
            .get(self.selection.year)?
            .entries
            .get(self.selection.entry)
    }

    fn cycle_popup_focus(&mut self) {
        self.popup.focus = match self.popup.focus {
            PopupFocus::Date => PopupFocus::Amount,
            PopupFocus::Amount => PopupFocus::Date,
        };
    }

    fn handle_popup_input(&mut self, key_event: ratatui::crossterm::event::KeyEvent) {
        // Clear error message when user starts typing
        if matches!(key_event.code, KeyCode::Char(_) | KeyCode::Backspace) {
            self.popup.error_message = None;
        }

        match self.popup.focus {
            PopupFocus::Date => {
                self.popup.date_input.handle_event(&Event::Key(key_event));
                // Ensure date doesn't exceed 10 characters (YYYY-MM-DD format)
                if self.popup.date_input.value().len() > 10 {
                    let truncated = self.popup.date_input.value()[..10].to_string();
                    self.popup.date_input = Input::new(truncated).with_cursor(10);
                }
            }
            PopupFocus::Amount => {
                // For amount field, we need to validate input
                let key = key_event.code;
                match key {
                    KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                        // Only allow minus at the beginning
                        if c == '-' && !self.popup.amount_input.value().is_empty() {
                            return;
                        }
                        self.popup.amount_input.handle_event(&Event::Key(key_event));
                    }
                    KeyCode::Backspace => {
                        self.popup.amount_input.handle_event(&Event::Key(key_event));
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_saving_popup_entry(&mut self) {
        // Clear any previous error message
        self.popup.error_message = None;

        // Validate inputs
        let date = match NaiveDate::parse_from_str(self.popup.date_input.value(), "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                self.popup.error_message = Some("Invalid date format. Use YYYY-MM-DD".to_string());
                return;
            }
        };

        let amount = match Decimal::from_str(self.popup.amount_input.value()) {
            Ok(amount) => amount,
            Err(_) => {
                self.popup.error_message =
                    Some("Invalid amount format. Use decimal number".to_string());
                return;
            }
        };

        let file = &self.files[self.selection.file];

        let result = match self.popup.mode {
            PopupMode::AddEntry => add_entry(&file.path, date, amount)
                .map(|_| ())
                .map_err(|err| err.into()),
            PopupMode::EditEntry => self.edit_entry_in_file(&file.path, date, amount),
            PopupMode::None => Ok(()),
        };

        match result {
            Ok(()) => {
                // Success - refresh the report and close popup
                self.reload_file();
                self.close_popup();
            }
            Err(e) => {
                // Error - show error message and keep popup open
                self.popup.error_message = Some(format!("Failed to save: {}", e));
            }
        }
    }

    fn edit_entry_in_file(
        &self,
        file_path: &Path,
        date: NaiveDate,
        amount: Decimal,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut entries = entries_from_file(file_path)?;

        // Find and update the entry
        if let Some(selected_entry) = self.get_selected_entry() {
            // Find the entry by matching date and amount (original values)
            if let Some(entry_to_edit) = entries
                .iter_mut()
                .find(|e| e.date == selected_entry.date && e.amount == selected_entry.amount)
            {
                entry_to_edit.date = date.to_string();
                entry_to_edit.amount = amount;

                // Rewrite the entire file
                let mut writer = WriterBuilder::new().delimiter(DELIMITER).from_writer(
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
    let [main_rect, help_rect] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .areas(frame.area());

    let [files_rect, years_rect, entries_rect] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3); 3])
        .areas(main_rect);

    let files_width = files_rect.width.saturating_sub(2) as usize; // Account for block borders
    let files = app.files.iter().enumerate().map(|(i, file)| {
        ListItem::new(make_line(
            &file.name,
            if i == app.selection.file {
                &app.report.total
            } else {
                ""
            },
            i == app.selection.file,
            app.focus == Focus::Files && app.popup.mode == PopupMode::None,
            files_width,
        ))
    });

    let has_focus = |focus| app.focus == focus && app.popup.mode == PopupMode::None;

    let files_list = List::new(files).block(make_block("Files", has_focus(Focus::Files)));
    frame.render_stateful_widget(files_list, files_rect, &mut ListState::default());

    // Years list (middle column)
    let years_width = years_rect.width.saturating_sub(2) as usize; // Account for block borders
    let years_list = List::new(app.report.year_reports.iter().enumerate().map(|(i, year)| {
        ListItem::new(make_line(
            &year.title,
            &year.subtotal_amount,
            i == app.selection.year,
            app.focus == Focus::Years && app.popup.mode == PopupMode::None,
            years_width,
        ))
    }))
    .block(make_block(&app.report.title, has_focus(Focus::Years)));

    frame.render_stateful_widget(years_list, years_rect, &mut ListState::default());

    // Entries list (right column)
    let entries_width = entries_rect.width.saturating_sub(2) as usize; // Account for block borders
    let selected_year = &app.report.year_reports[app.selection.year];
    let entries_list = List::new(selected_year.lines.iter().enumerate().map(
        |(i, (date, amount))| {
            ListItem::new(make_line(
                date,
                amount,
                i == app.selection.entry,
                app.focus == Focus::YearDetails && app.popup.mode == PopupMode::None,
                entries_width,
            ))
        },
    ))
    .block(make_block(
        &selected_year.title,
        has_focus(Focus::YearDetails),
    ));

    frame.render_stateful_widget(entries_list, entries_rect, &mut ListState::default());

    let footer_text = if app.popup.mode == PopupMode::None {
        "↓(j)/↑(k): Navigate | Tab: Focus | n/e: New/Edit Entry | q: Quit"
    } else {
        "Tab: Switch Field | Enter: Save | q: Cancel"
    };
    let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, help_rect);

    // Render popup if active
    if app.popup.mode != PopupMode::None {
        render_popup(frame, app);
    }
}

fn render_popup(frame: &mut Frame, app: &App) {
    // Create a centered popup area
    let area = frame.area();
    let [_, popup_rect, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Min(8),
            Constraint::Percentage(30),
        ])
        .areas(area);

    let [_, popup_rect, _] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Min(40),
            Constraint::Percentage(20),
        ])
        .areas(popup_rect);

    // Clear the area
    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(Clear, popup_rect);
    frame.render_widget(clear_block, popup_rect);

    // Create the popup content
    let title = match app.popup.mode {
        PopupMode::AddEntry => " Add New Entry ",
        PopupMode::EditEntry => " Edit Entry ",
        PopupMode::None => "",
    };

    let popup_block = Block::default()
        .title(Line::from(title).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner_area = popup_block.inner(popup_rect);
    frame.render_widget(popup_block, popup_rect);
    let [file_name_rect, _, date_rect, amount_rect, error_rect, _] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // File name
            Constraint::Length(1), // Empty line
            Constraint::Length(1), // Date field
            Constraint::Length(1), // Amount field
            Constraint::Length(1), // Empty line or error message
            Constraint::Min(1),    // Remaining space
        ])
        .areas(inner_area);

    // File name
    let file = &app.files[app.selection.file];
    let file_name_input = Input::new(file.name.clone());
    render_input_field(frame, "File  ", &file_name_input, file_name_rect, false);

    // Date field
    render_input_field(
        frame,
        "Date  ",
        &app.popup.date_input,
        date_rect,
        app.popup.focus == PopupFocus::Date,
    );

    // Amount field
    render_input_field(
        frame,
        "Amount",
        &app.popup.amount_input,
        amount_rect,
        app.popup.focus == PopupFocus::Amount,
    );

    // Error message
    if let Some(error_msg) = &app.popup.error_message {
        let error_line = Line::from(vec![
            Span::raw(" "),
            Span::raw("Error: ").style(Style::default().fg(Color::Red)),
            Span::raw(error_msg).style(Style::default().fg(Color::Red)),
        ]);
        frame.render_widget(Paragraph::new(error_line), error_rect);
    }
}

fn render_input_field(
    frame: &mut Frame,
    name: &str,
    input: &Input,
    layout: Rect,
    is_focused: bool,
) {
    let style = if is_focused {
        Style::default()
            .bg(FOCUSED_SELECTION_BG_COLOR)
            .fg(Color::White)
    } else {
        Style::default().fg(Color::White)
    };
    let prefix = if is_focused {
        Span::raw("▌").style(SELECTION_INDICATOR_COLOR)
    } else {
        Span::raw(" ")
    };
    let value_span = Span::raw(input.value());
    let value_width = value_span.width() as u16;
    let line = Line::from(vec![prefix, Span::raw(name), Span::raw("  "), value_span]).style(style);
    let line_width = line.width() as u16;
    frame.render_widget(line, layout);

    if is_focused {
        let cursor_pos = input.visual_cursor() as u16;
        frame.set_cursor_position(CursorPosition {
            x: layout.x + line_width - value_width + cursor_pos,
            y: layout.y,
        });
    }
}

fn make_block(title: &str, is_focused: bool) -> Block<'_> {
    let line = Line::raw(format!(" {title} "));
    Block::default()
        .title(line.add_modifier(if is_focused {
            Modifier::BOLD
        } else {
            Modifier::empty()
        }))
        .borders(Borders::ALL)
        .border_type(if is_focused {
            BorderType::Double
        } else {
            BorderType::Plain
        })
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

fn next_index_cycled(current: usize, count: usize) -> usize {
    if current + 1 >= count {
        0
    } else {
        current.saturating_add(1)
    }
}

fn previous_index_cycled(current: usize, count: usize) -> usize {
    if current == 0 {
        count.saturating_sub(1)
    } else {
        current.saturating_sub(1)
    }
}
