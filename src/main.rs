use chrono::{Datelike, NaiveDate};
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, WriterBuilder};
use rust_decimal::Decimal;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "mfinance")]
#[command(about = "A simple financial tool for managing CSV entries", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new entry to the CSV file
    NewEntry {
        /// Amount to add
        #[arg(short, long)]
        amount: Decimal,
        /// Date of the entry (e.g. 2024-12-12, defaults to today)
        #[arg(short, long)]
        date: Option<String>,
        /// Path to the CSV file
        file: PathBuf,
    },
    /// Generate a report for a specific period
    Report {
        /// Period to report on (e.g., "2024" or "2024-02")
        period: String,
        /// Path to the CSV file
        file: PathBuf,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    date: String,
    amount: Decimal,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::NewEntry { amount, date, file } => {
            let date = date.unwrap_or_else(|| chrono::Local::now().date_naive().to_string());
            add_entry(&file, &date, amount)?;
        }
        Commands::Report { period, file } => {
            let total = generate_report(&file, &period)?;
            println!("Total amount for {}: {}", period, total);
        }
    }

    Ok(())
}

fn add_entry(
    file_path: &Path,
    date: &str,
    amount: Decimal,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut records = records_from_file(file_path)?;
    let total_before: Decimal = records.iter().map(|r| r.amount).sum();

    let new_record = Record {
        date: date.to_string(),
        amount,
    };

    // Insert the new record in the correct position to keep the list sorted by date
    let pos = records
        .iter()
        .position(|r| r.date > new_record.date)
        .unwrap_or(records.len());
    records.insert(pos, new_record);

    // Write back to the file
    let mut writer = WriterBuilder::new().delimiter(b';').from_writer(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_path)?,
    );

    for record in records {
        writer.serialize(record)?;
    }

    writer.flush()?;

    let total_after: Decimal = records_from_file(file_path)?.iter().map(|r| r.amount).sum();

    let total_before_line = format!("{total_before:.2}");
    let diff_line = format!("{:.2}", (total_after - total_before));
    let total_after_line = format!("Total: {total_after:.2}");

    let max_len = [&total_before_line, &diff_line, &total_after_line]
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap();

    println!("{:>max_len$}", total_before_line);
    println!("{:>max_len$}", diff_line);
    println!("{:>max_len$}", total_after_line);

    Ok(())
}

fn records_from_file(path: &Path) -> Result<Vec<Record>, Box<dyn std::error::Error>> {
    let records = if path.exists() {
        let mut rdr = ReaderBuilder::new().delimiter(b';').from_path(path)?;
        rdr.deserialize::<Record>().collect::<Result<Vec<_>, _>>()?
    } else {
        vec![]
    };

    Ok(records)
}

fn generate_report(file_path: &Path, period: &str) -> Result<Decimal, Box<dyn std::error::Error>> {
    let records = records_from_file(file_path)?;

    let total = records
        .into_iter()
        .filter(|r| {
            let record_date = NaiveDate::parse_from_str(&r.date, "%Y-%m-%d").unwrap();
            match period.len() {
                4 => record_date.year().to_string() == period, // Year
                7 => record_date.format("%Y-%m").to_string() == period, // Month
                _ => false,
            }
        })
        .map(|r| r.amount)
        .sum();

    Ok(total)
}
