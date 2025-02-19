use chrono::{Datelike, NaiveDate};
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, WriterBuilder};
use rust_decimal::Decimal;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "mfinance")]
#[command(version, about = "A simple financial tool for managing CSV entries", long_about = None)]
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
        #[arg(short, long)]
        period: Option<String>,
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
            if let Some(period) = period {
                let total = generate_report(&file, &period)?.human_readable();
                println!("Total amount for {period}: {total}");
            } else {
                let total = generate_report_for_all(&file)?.human_readable();
                println!("Total amount: {total}");
            }
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

    let total_before_line = total_before.human_readable();
    let diff_line = (total_after - total_before).human_readable();
    let total_after_line = format!("Total: {}", total_after.human_readable());

    let max_len = [&total_before_line, &diff_line, &total_after_line]
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap();

    println!("{total_before_line:>max_len$}");
    println!("{diff_line:>max_len$}");
    println!("{total_after_line:>max_len$}");

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

fn generate_report_for_all(file_path: &Path) -> Result<Decimal, Box<dyn std::error::Error>> {
    let records = records_from_file(file_path)?;
    Ok(records.into_iter().map(|r| r.amount).sum())
}

trait HumanReadable {
    fn human_readable(&self) -> String;
}

impl HumanReadable for Decimal {
    fn human_readable(&self) -> String {
        let precision: usize = 2;
        let decimal = self.round_dp(precision as u32);
        let decimal_string = format!("{decimal:.precision$}");
        let sign_offset = usize::from(decimal.is_sign_negative());
        let len_till_dot = decimal_string.len() - 1 - precision;
        let mut group_separator_index = (len_till_dot - sign_offset) % 3 + sign_offset;
        if group_separator_index == sign_offset {
            group_separator_index = 3 + sign_offset;
        }
        let mut result = String::new();
        for (i, ch) in decimal_string.char_indices() {
            if group_separator_index == i && group_separator_index < len_till_dot {
                result.push(' ');
                group_separator_index += 3;
            }
            result.push(ch);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::{prelude::FromPrimitive, Decimal};

    use super::HumanReadable;

    #[test]
    fn format_fractions() {
        assert_eq!(Decimal::from_f32(0.006).unwrap().human_readable(), "0.01");
    }

    #[test]
    fn format_fractions_negative() {
        assert_eq!(Decimal::from_f32(-0.006).unwrap().human_readable(), "-0.01");
    }

    #[test]
    fn format_singles() {
        assert_eq!(Decimal::from_i8(1).unwrap().human_readable(), "1.00");
    }

    #[test]
    fn format_singles_negative() {
        assert_eq!(Decimal::from_i8(-1).unwrap().human_readable(), "-1.00");
    }

    #[test]
    fn format_tens() {
        assert_eq!(Decimal::from_i8(10).unwrap().human_readable(), "10.00");
    }

    #[test]
    fn format_tens_negative() {
        assert_eq!(Decimal::from_i8(-10).unwrap().human_readable(), "-10.00");
    }

    #[test]
    fn format_hundreds() {
        assert_eq!(Decimal::from_i8(100).unwrap().human_readable(), "100.00");
    }

    #[test]
    fn format_hundreds_negative() {
        assert_eq!(Decimal::from_i8(-100).unwrap().human_readable(), "-100.00");
    }

    #[test]
    fn format_thousands() {
        assert_eq!(
            Decimal::from_f32(1999.99).unwrap().human_readable(),
            "1 999.99"
        );
    }

    #[test]
    fn format_thousands_negative() {
        assert_eq!(
            Decimal::from_f32(-1999.99).unwrap().human_readable(),
            "-1 999.99"
        );
    }

    #[test]
    fn format_ten_thousands() {
        assert_eq!(
            Decimal::from_f32(19999.99).unwrap().human_readable(),
            "19 999.99"
        );
    }

    #[test]
    fn format_ten_thousands_negative() {
        assert_eq!(
            Decimal::from_f32(-19999.99).unwrap().human_readable(),
            "-19 999.99"
        );
    }

    #[test]
    fn format_hundred_thousands() {
        assert_eq!(
            Decimal::from_f64(199999.99).unwrap().human_readable(),
            "199 999.99"
        );
    }

    #[test]
    fn format_hundred_thousands_negative() {
        assert_eq!(
            Decimal::from_f64(-199999.99).unwrap().human_readable(),
            "-199 999.99"
        );
    }

    #[test]
    fn format_million() {
        assert_eq!(
            Decimal::from_f64(1999999.99).unwrap().human_readable(),
            "1 999 999.99"
        );
    }

    #[test]
    fn format_million_negative() {
        assert_eq!(
            Decimal::from_f64(-1999999.99).unwrap().human_readable(),
            "-1 999 999.99"
        );
    }
}
