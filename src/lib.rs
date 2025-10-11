pub mod number_formatter;
pub mod tui;

use chrono::NaiveDate;
use csv::{ReaderBuilder, WriterBuilder};
use number_formatter::{FormatOptions, NumberFormatter};
use rust_decimal::Decimal;
use std::fmt::Display;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DELIMITER: u8 = b';';

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub date: String,
    pub amount: Decimal,
}

impl Entry {
    pub fn day_month_date(&self) -> String {
        if let Ok(date) = self.date.parse::<NaiveDate>() {
            date.format("%B %-d").to_string()
        } else {
            self.date.clone()
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {context}")]
    Io {
        #[source]
        source: std::io::Error,
        context: String,
    },

    #[error("CSV error: {source}")]
    Csv {
        #[from]
        source: csv::Error,
    },

    #[error("Invalid date format: {input} ({source})")]
    DateParse {
        source: chrono::format::ParseError,
        input: String,
    },

    #[error("No entries found")]
    NoEntries,

    #[error("No entries matching filter: {0}")]
    FilteredNoEntries(String),
}

pub fn add_entry(
    file_path: &Path,
    date: NaiveDate,
    amount: Decimal,
) -> Result<NewEntryInfo, AppError> {
    let entries = entries_from_file(file_path).unwrap_or_default();
    let total_before: Decimal = entries.iter().map(|entry| entry.amount).sum();

    let new_entry = Entry {
        date: date.to_string(),
        amount,
    };

    // Write to the end of the file.
    let mut writer = WriterBuilder::new()
        .delimiter(DELIMITER)
        .has_headers(entries.is_empty())
        .from_writer(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)
                .map_err(|source| AppError::Io {
                    source,
                    context: String::from("Failed to open file to add a new entry"),
                })?,
        );

    writer.serialize(new_entry)?;
    writer.flush().map_err(|source| AppError::Io {
        source,
        context: String::from("Failed to flush the writer buffer when saving new entry"),
    })?;

    Ok(NewEntryInfo {
        total_before,
        total_after: entries_from_file(file_path)?
            .iter()
            .map(|entry| entry.amount)
            .sum(),
    })
}

pub struct NewEntryInfo {
    pub total_before: Decimal,
    pub total_after: Decimal,
}

impl NewEntryInfo {
    pub fn display(&self, options: FormatOptions) -> NewEntryInfoDisplay<'_> {
        NewEntryInfoDisplay {
            info: self,
            options,
        }
    }
}

pub struct NewEntryInfoDisplay<'a> {
    info: &'a NewEntryInfo,
    options: FormatOptions,
}

impl<'a> Display for NewEntryInfoDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_before_line = self.info.total_before.format(&self.options);
        let diff_line = (self.info.total_after - self.info.total_before).format(&self.options);
        let total_after_line = format!("Total: {}", self.info.total_after.format(&self.options));

        let max_len = [&total_before_line, &diff_line, &total_after_line]
            .iter()
            .map(|s| s.chars().count())
            .max()
            .unwrap();

        writeln!(f, "{total_before_line:>max_len$}")?;
        writeln!(f, "{diff_line:>max_len$}")?;
        writeln!(f, "{total_after_line:>max_len$}")?;
        Ok(())
    }
}

pub fn generate_report(file_path: &Path, date_filter: &str) -> Result<Report, AppError> {
    let mut entries: Vec<Entry> = entries_from_file(file_path)?
        .into_iter()
        .filter(|entry| entry.date.starts_with(date_filter))
        .collect();

    if entries.is_empty() {
        return Err(AppError::FilteredNoEntries(date_filter.to_string()));
    }

    entries.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(Report {
        filter: Some(String::from(date_filter)),
        entries,
    })
}

pub fn generate_report_for_all(file_path: &Path) -> Result<Report, AppError> {
    let mut entries = entries_from_file(file_path)?;
    if entries.is_empty() {
        return Err(AppError::NoEntries);
    }

    entries.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(Report {
        filter: None,
        entries,
    })
}

pub struct Report {
    filter: Option<String>,
    entries: Vec<Entry>,
}

impl Report {
    pub fn display(&self, options: FormatOptions) -> ReportDisplay<'_> {
        ReportDisplay {
            report: self,
            options,
        }
    }
}

pub struct ReportDisplay<'a> {
    report: &'a Report,
    options: FormatOptions,
}

impl<'a> Display for ReportDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rows: Vec<(String, String)> = self
            .report
            .entries
            .iter()
            .map(|entry| {
                (
                    format!("{}:", entry.date),
                    entry.amount.format(&self.options),
                )
            })
            .collect();

        let final_line_prefix: String = if let Some(filter) = self.report.filter.as_ref() {
            format!("Total amount for filter '{filter}':")
        } else {
            "Total amount:".to_string()
        };
        let total: Decimal = self.report.entries.iter().map(|entry| entry.amount).sum();
        let final_line_suffix: String = total.format(&self.options);
        let mut max_prefix_len = rows.iter().map(|row| row.0.chars().count()).max().unwrap();
        let mut max_suffix_len = rows.iter().map(|row| row.1.chars().count()).max().unwrap();
        max_prefix_len = max_prefix_len.max(final_line_prefix.chars().count());
        max_suffix_len = max_suffix_len.max(final_line_suffix.chars().count()) + 1;

        for (prefix, suffix) in rows {
            write!(f, "{prefix:>max_prefix_len$}")?;
            writeln!(f, "{suffix:>max_suffix_len$}")?;
        }

        write!(f, "{final_line_prefix:>max_prefix_len$}")?;
        writeln!(f, "{final_line_suffix:>max_suffix_len$}")?;

        Ok(())
    }
}

pub fn entries_from_file(path: &Path) -> Result<Vec<Entry>, AppError> {
    std::fs::metadata(path).map_err(|e| AppError::Io {
        source: e,
        context: format!("Failed to access file: {}", path.display()),
    })?;

    let mut reader = ReaderBuilder::new()
        .delimiter(DELIMITER)
        .from_path(path)
        .map_err(|source| AppError::Csv { source })?;
    let entries = reader
        .deserialize::<Entry>()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn get_csv_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? == "csv" {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}
