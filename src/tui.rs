use crate::{
    Entry, entries_from_file,
    number_formatter::{FormatOptions, NumberFormatter},
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
};

/// Process a single key event and update the app state
fn handle_key_event(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('q') => true, // quit
        KeyCode::Down => {
            app.next();
            false
        }
        KeyCode::Char('j') => {
            app.next();
            false
        }
        KeyCode::Up => {
            app.previous();
            false
        }
        KeyCode::Char('k') => {
            app.previous();
            false
        }
        KeyCode::Tab => {
            app.cycle_focus();
            false
        }
        _ => false,
    }
}

/// Core TUI loop that works with any backend and event source
fn run_tui_loop<B, E>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: E,
) -> Result<(), Box<dyn std::error::Error>>
where
    B: ratatui::backend::Backend,
    E: IntoIterator<Item = Event>,
{
    // Draw initial state
    terminal.draw(|f| ui(f, app))?;

    // Process events
    for event in events {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            let should_quit = handle_key_event(app, key.code);
            if should_quit {
                break;
            }
        }

        // Redraw after each event
        terminal.draw(|f| ui(f, app))?;
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

    let mut app = App::new(files, format_options);

    // Event iterator that reads from stdin until quit
    let events = std::iter::from_fn(|| event::read().ok());

    let res = run_tui_loop(&mut terminal, &mut app, events);

    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    res
}

/// Testable version that accepts a backend and event iterator  
/// For testing with TestBackend - returns rendered buffer as string
pub fn run_tui_with_events_test(
    files: Vec<PathBuf>,
    format_options: FormatOptions,
    events: impl IntoIterator<Item = Event>,
    width: u16,
    height: u16,
) -> Result<String, Box<dyn std::error::Error>> {
    use ratatui::backend::TestBackend;

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(files, format_options);

    run_tui_loop(&mut terminal, &mut app, events)?;

    // Return buffer content
    Ok(format!("{:?}", terminal.backend().buffer()))
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Focus {
    FileSelection,
    Years,
    YearDetails,
}

struct App {
    files: Vec<PathBuf>,
    format_options: FormatOptions,
    selected_file: usize,
    report: ReportViewModel,
    focus: Focus,
    selected_year: usize,
    selected_entry: usize,
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
                    YearReportViewModel {
                        title: year,
                        subtotal_amount: subtotal_amount.format(format_options),
                        lines: entries
                            .into_iter()
                            .map(|entry| (entry.date, entry.amount.format(format_options)))
                            .collect(),
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
            files_width,
        ))
    });

    let highlight_style = Style::default().bg(Color::Blue).fg(Color::Black);
    let files_list = List::new(files)
        .block(app.create_block(Line::from(" Files "), Focus::FileSelection))
        .highlight_style(highlight_style);
    frame.render_stateful_widget(
        files_list,
        content_layout[0],
        &mut ListState::default().with_selected(match app.focus {
            Focus::FileSelection => Some(app.selected_file),
            _ => None,
        }),
    );

    // Years list (middle column)
    let years_width = content_layout[1].width.saturating_sub(2) as usize; // Account for block borders
    let years_list = List::new(app.report.year_reports.iter().enumerate().map(|(i, year)| {
        ListItem::new(make_line(
            &year.title,
            &year.subtotal_amount,
            i == app.selected_year,
            years_width,
        ))
    }))
    .block(app.create_block(Line::from(format!(" {} ", app.report.title)), Focus::Years))
    .highlight_style(highlight_style);

    frame.render_stateful_widget(
        years_list,
        content_layout[1],
        &mut ListState::default().with_selected(match app.focus {
            Focus::Years => Some(app.selected_year),
            _ => None,
        }),
    );

    // Entries list (right column)
    let entries_width = content_layout[2].width.saturating_sub(2) as usize; // Account for block borders
    let selected_year = &app.report.year_reports[app.selected_year];
    let entries_list = List::new(selected_year.lines.iter().enumerate().map(
        |(i, (date, amount))| {
            ListItem::new(make_line(
                date,
                amount,
                i == app.selected_entry,
                entries_width,
            ))
        },
    ))
    .block(app.create_block(
        Line::from(format!(" {} ", selected_year.title)),
        Focus::YearDetails,
    ))
    .highlight_style(highlight_style);

    frame.render_stateful_widget(
        entries_list,
        content_layout[2],
        &mut ListState::default().with_selected(match app.focus {
            Focus::YearDetails => Some(app.selected_entry),
            _ => None,
        }),
    );

    let footer = Paragraph::new("↓(j)/↑(k): Navigate | Tab: Focus | q: Quit")
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, main_layout[1]);
}

fn make_line<'a>(
    left: impl Into<std::borrow::Cow<'a, str>>,
    right: &'a str,
    is_selected: bool,
    width: usize,
) -> Line<'a> {
    let padding_span = Span::raw(" ");
    let left_span = Span::raw(left);
    let right_span = Span::raw(right);
    let spacer = " ".repeat(
        width.saturating_sub(left_span.width() + right_span.width() + padding_span.width() * 2),
    );
    let line = Line::from(vec![
        padding_span.clone(),
        left_span,
        Span::raw(spacer),
        right_span,
        padding_span,
    ]);
    if is_selected {
        line.style(Style::default().bg(Color::DarkGray))
    } else {
        line
    }
}
