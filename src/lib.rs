pub mod number_formatter;
pub mod tui;

use csv::ReaderBuilder;
use rust_decimal::Decimal;
use std::path::Path;
use thiserror::Error;

const DELIMITER: u8 = b';';

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub date: String,
    pub amount: Decimal,
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