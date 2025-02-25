use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, WriterBuilder};
use rust_decimal::Decimal;
use std::fmt::Display;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

const DELIMITER: u8 = b';';

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
        #[arg(short, long, allow_negative_numbers = true)]
        amount: Decimal,
        /// Date of the entry (e.g. 2024-12-12, defaults to today)
        #[arg(short, long)]
        date: Option<String>,
        /// Path to the CSV file
        file: PathBuf,
    },
    /// Generate a report for a specific period
    Report {
        /// Filter records by date. Currently, only `starts_with` filter is supported.
        /// For example, you can use "2024" to filter out a year or "2024-02" for a month.
        #[arg(short, long)]
        filter: Option<String>,
        /// Path to the CSV file
        file: PathBuf,
    },
    /// Sort the entries in the file by date
    Sort {
        /// Path to the CSV file
        file: PathBuf,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Record {
    date: String,
    amount: Decimal,
}

fn main() -> Result<(), main_error::MainError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::NewEntry { amount, date, file } => {
            let date: NaiveDate = match date {
                Some(date) => date
                    .parse()
                    .map_err(|err| format!("failed to parse input date: {err}"))?,
                None => chrono::Local::now().date_naive(),
            };
            let info = add_entry(&file, date, amount)?;
            print!("{info}");
        }
        Commands::Report { filter, file } => {
            if let Some(filter) = filter {
                let report = generate_report(&file, &filter)?;
                print!("{report}");
            } else {
                let report = generate_report_for_all(&file)?;
                print!("{report}");
            }
        }
        Commands::Sort { file } => {
            let mut records = records_from_file(&file)?;
            records.sort_by(|a, b| a.date.cmp(&b.date));
            let mut writer = WriterBuilder::new()
                .delimiter(DELIMITER)
                .from_writer(OpenOptions::new().write(true).truncate(true).open(&file)?);

            for record in records {
                writer.serialize(record)?;
            }
            writer.flush()?;
        }
    }

    Ok(())
}

fn add_entry(
    file_path: &Path,
    date: NaiveDate,
    amount: Decimal,
) -> Result<NewEntryInfo, main_error::MainError> {
    let records = records_from_file(file_path).unwrap_or_default();
    let total_before: Decimal = records.iter().map(|r| r.amount).sum();

    let new_record = Record {
        date: date.to_string(),
        amount,
    };

    // Write to the end of the file.
    let mut writer = WriterBuilder::new()
        .delimiter(DELIMITER)
        .has_headers(records.is_empty())
        .from_writer(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?,
        );

    writer.serialize(new_record)?;
    writer.flush()?;

    Ok(NewEntryInfo {
        total_before,
        total_after: records_from_file(file_path)?.iter().map(|r| r.amount).sum(),
    })
}

struct NewEntryInfo {
    total_before: Decimal,
    total_after: Decimal,
}

impl Display for NewEntryInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_before_line = self.total_before.human_readable();
        let diff_line = (self.total_after - self.total_before).human_readable();
        let total_after_line = format!("Total: {}", self.total_after.human_readable());

        let max_len = [&total_before_line, &diff_line, &total_after_line]
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap();

        writeln!(f, "{total_before_line:>max_len$}")?;
        writeln!(f, "{diff_line:>max_len$}")?;
        writeln!(f, "{total_after_line:>max_len$}")?;
        Ok(())
    }
}

fn records_from_file(path: &Path) -> Result<Vec<Record>, main_error::MainError> {
    if !path.exists() {
        return Err(format!("File '{}' does not exist", path.to_string_lossy()).into());
    }

    let mut reader = ReaderBuilder::new().delimiter(DELIMITER).from_path(path)?;
    let records = reader
        .deserialize::<Record>()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn generate_report(file_path: &Path, date_filter: &str) -> Result<Report, main_error::MainError> {
    let mut records: Vec<Record> = records_from_file(file_path)?
        .into_iter()
        .filter(|r| r.date.starts_with(date_filter))
        .collect();

    records.sort_by(|a, b| a.date.cmp(&b.date));

    if records.is_empty() {
        return Err(format!("No records for the given filter: '{date_filter}'").into());
    }

    Ok(Report {
        filter: Some(String::from(date_filter)),
        records,
    })
}

fn generate_report_for_all(file_path: &Path) -> Result<Report, main_error::MainError> {
    let mut records = records_from_file(file_path)?;
    records.sort_by(|a, b| a.date.cmp(&b.date));

    if records.is_empty() {
        return Err(String::from("No records").into());
    }

    Ok(Report {
        filter: None,
        records,
    })
}

struct Report {
    filter: Option<String>,
    records: Vec<Record>,
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let records: Vec<(String, String)> = self
            .records
            .iter()
            .map(|record| (format!("{}:", record.date), record.amount.human_readable()))
            .collect();

        let final_line_prefix: String = if let Some(filter) = self.filter.as_ref() {
            format!("Total amount for filter '{filter}':")
        } else {
            "Total amount:".to_string()
        };
        let total: Decimal = self.records.iter().map(|record| record.amount).sum();
        let final_line_suffix: String = total.human_readable();
        let mut max_prefix_len = records.iter().map(|tuple| tuple.0.len()).max().unwrap();
        let mut max_suffix_len = records.iter().map(|tuple| tuple.1.len()).max().unwrap();
        max_prefix_len = max_prefix_len.max(final_line_prefix.len());
        max_suffix_len = max_suffix_len.max(final_line_suffix.len()) + 1;

        for (prefix, suffix) in records {
            write!(f, "{prefix:>max_prefix_len$}")?;
            writeln!(f, "{suffix:>max_suffix_len$}")?;
        }

        write!(f, "{final_line_prefix:>max_prefix_len$}")?;
        writeln!(f, "{final_line_suffix:>max_suffix_len$}")?;

        Ok(())
    }
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
