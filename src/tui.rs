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

    let res = loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => break Ok(()),
                KeyCode::Down => app.next(),
                KeyCode::Char('j') => app.next(),
                KeyCode::Up => app.previous(),
                KeyCode::Char('k') => app.previous(),
                KeyCode::Tab => app.cycle_focus(),
                _ => {}
            }
        }
    };

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

pub struct App {
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
    pub fn new(files: Vec<PathBuf>, format_options: FormatOptions) -> Self {
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

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::FileSelection => Focus::Years,
            Focus::Years => Focus::YearDetails,
            Focus::YearDetails => Focus::FileSelection,
        };
        // Reset selection when changing focus areas
        self.selected_entry = 0;
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

pub fn ui(frame: &mut Frame, app: &mut App) {
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
