use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use csv::WriterBuilder;
use rust_decimal::Decimal;
use std::fs::OpenOptions;
use std::path::PathBuf;

use mfinance::number_formatter::FormatOptions;
use mfinance::tui;
use mfinance::{AppError, add_entry, entries_from_file, generate_report, generate_report_for_all};

#[derive(Parser)]
#[command(name = "mfinance")]
#[command(version, about = "A simple financial tool for managing CSV entries", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive terminal UI
    Tui {
        /// Directory containing CSV files
        path: PathBuf,
    },
    /// Add a new entry with amount to the CSV file
    NewEntry {
        /// Amount to add (e.g. -999.99)
        #[arg(short, long, allow_negative_numbers = true)]
        amount: Decimal,
        /// Date of the entry (e.g. 2024-12-12, defaults to today)
        #[arg(short, long)]
        date: Option<String>,
        /// Path to the CSV file
        file: PathBuf,
    },
    /// Generate a report possibly filtered by date
    Report {
        /// Filters entries by date
        ///
        /// Currently, only the `starts_with` filter is supported.
        ///
        /// # Examples
        /// - To filter entries for a specific year, use `2024`.
        /// - To filter entries for a specific month, use `2024-02`.
        #[arg(short, long)]
        filter: Option<String>,
        /// Path to the CSV file
        file: PathBuf,
    },
    /// Sort the entries in the CSV file by date
    Sort {
        /// Path to the CSV file
        file: PathBuf,
    },
}

fn main() -> Result<(), main_error::MainError> {
    let cli = Cli::parse();
    let format_options = FormatOptions::default();

    match cli.command {
        Commands::NewEntry { amount, date, file } => {
            let date: NaiveDate = if let Some(date) = date {
                date.parse().map_err(|source| AppError::DateParse {
                    source,
                    input: date.clone(),
                })?
            } else {
                chrono::Local::now().date_naive()
            };
            let info = add_entry(&file, date, amount)?;
            print!("{}", info.display(format_options));
        }
        Commands::Report { filter, file } => {
            let report = if let Some(filter) = filter {
                generate_report(&file, &filter)?
            } else {
                generate_report_for_all(&file)?
            };
            print!("{}", report.display(format_options));
        }
        Commands::Tui { path } => {
            let files = mfinance::get_csv_files(&path)?;
            if files.is_empty() {
                return Err(main_error::MainError::from(AppError::Io {
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "No CSV files found"),
                    context: format!("No CSV files found in directory: {}", path.display()),
                }));
            }
            tui::run_tui(files, format_options)?;
        }
        Commands::Sort { file } => {
            let mut entries = entries_from_file(&file)?;
            entries.sort_by(|a, b| a.date.cmp(&b.date));
            let mut writer = WriterBuilder::new()
                .delimiter(mfinance::DELIMITER)
                .from_writer(
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&file)
                        .map_err(|source| AppError::Io {
                            source,
                            context: String::from("Failed to open file when saving sorted csv"),
                        })?,
                );

            for entry in entries {
                writer.serialize(entry)?;
            }
            writer.flush().map_err(|source| AppError::Io {
                source,
                context: String::from("Failed to flush the sorted csv writer buffer"),
            })?;
        }
    }

    Ok(())
}
