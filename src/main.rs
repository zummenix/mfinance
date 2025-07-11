use axum::http::StatusCode;
use axum::{
    Extension, Router,
    extract::Path as AxumPath,
    response::{Html, Json},
    routing::get,
};
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
    /// Start web server to view CSV files
    Server {
        /// Directory containing CSV files
        dir: PathBuf,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Entry {
    date: String,
    amount: Decimal,
}

#[tokio::main]
async fn main() -> Result<(), main_error::MainError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::NewEntry { amount, date, file } => {
            let date: NaiveDate = if let Some(date) = date {
                date.parse()
                    .map_err(|err| format!("failed to parse date, {err}"))?
            } else {
                chrono::Local::now().date_naive()
            };
            let info = add_entry(&file, date, amount)?;
            print!("{info}");
        }
        Commands::Report { filter, file } => {
            let report = if let Some(filter) = filter {
                generate_report(&file, &filter)?
            } else {
                generate_report_for_all(&file)?
            };
            print!("{report}");
        }
        Commands::Sort { file } => {
            let mut entries = entries_from_file(&file)?;
            entries.sort_by(|a, b| a.date.cmp(&b.date));
            let mut writer = WriterBuilder::new()
                .delimiter(DELIMITER)
                .from_writer(OpenOptions::new().write(true).truncate(true).open(&file)?);

            for entry in entries {
                writer.serialize(entry)?;
            }
            writer.flush()?;
        }
        Commands::Server { dir } => server_main(dir).await?,
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct EntryResponse {
    date: String,
    amount: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FileResponse {
    years: Vec<YearGroupResponse>,
    total: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct YearGroupResponse {
    year: String,
    entries: Vec<EntryResponse>,
    subtotal: String,
}

async fn server_main(dir: PathBuf) -> Result<(), main_error::MainError> {
    use axum::Extension;

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/files", get(list_files_handler))
        .route("/api/files/{filename}", get(file_handler))
        .layer(Extension(dir.clone()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Server running at http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index_handler() -> Html<&'static str> {
    Html(include_str!("templates/index.html"))
}

async fn list_files_handler(Extension(dir): Extension<PathBuf>) -> Json<Vec<String>> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&*dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "csv" {
                    if let Some(name) = entry.file_name().to_str() {
                        files.push(name.to_string());
                    }
                }
            }
        }
    }

    Json(files)
}

async fn file_handler(
    Extension(dir): Extension<PathBuf>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<FileResponse>, (StatusCode, String)> {
    struct YearGroup {
        entries: Vec<EntryResponse>,
        subtotal: Decimal,
    }

    use std::collections::HashMap;
    let path = dir.join(&filename);

    let entries = entries_from_file(&path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")))?;
    let mut year_groups: HashMap<String, YearGroup> = HashMap::new();
    let mut total = Decimal::default();

    for entry in entries {
        let date = NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d").map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Invalid date format: {}, {e:?}", entry.date),
            )
        })?;

        let year = date.format("%Y").to_string();
        let amount = entry.amount;
        total += amount;

        let group = year_groups
            .entry(year.clone())
            .or_insert_with(|| YearGroup {
                entries: Vec::new(),
                subtotal: Decimal::default(),
            });

        group.entries.push(EntryResponse {
            date: date.format("%B %-d").to_string(),
            amount: amount.human_readable(),
        });
        group.subtotal += amount;
    }

    let mut years: Vec<YearGroupResponse> = year_groups
        .into_iter()
        .map(|(year, group)| YearGroupResponse {
            year,
            entries: group.entries,
            subtotal: group.subtotal.human_readable(),
        })
        .collect();
    years.sort_by(|a, b| a.year.cmp(&b.year));

    Ok(Json(FileResponse {
        years,
        total: total.human_readable(),
    }))
}

fn add_entry(
    file_path: &Path,
    date: NaiveDate,
    amount: Decimal,
) -> Result<NewEntryInfo, main_error::MainError> {
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
                .open(file_path)?,
        );

    writer.serialize(new_entry)?;
    writer.flush()?;

    Ok(NewEntryInfo {
        total_before,
        total_after: entries_from_file(file_path)?
            .iter()
            .map(|entry| entry.amount)
            .sum(),
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
            .map(|s| s.chars().count())
            .max()
            .unwrap();

        writeln!(f, "{total_before_line:>max_len$}")?;
        writeln!(f, "{diff_line:>max_len$}")?;
        writeln!(f, "{total_after_line:>max_len$}")?;
        Ok(())
    }
}

fn entries_from_file(path: &Path) -> Result<Vec<Entry>, main_error::MainError> {
    if !path.exists() {
        return Err(format!("File '{}' does not exist", path.to_string_lossy()).into());
    }

    let mut reader = ReaderBuilder::new().delimiter(DELIMITER).from_path(path)?;
    let entries = reader
        .deserialize::<Entry>()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

fn generate_report(file_path: &Path, date_filter: &str) -> Result<Report, main_error::MainError> {
    let mut entries: Vec<Entry> = entries_from_file(file_path)?
        .into_iter()
        .filter(|entry| entry.date.starts_with(date_filter))
        .collect();
    if entries.is_empty() {
        return Err(format!("No entries for the given filter: '{date_filter}'").into());
    }

    entries.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(Report {
        filter: Some(String::from(date_filter)),
        entries,
    })
}

fn generate_report_for_all(file_path: &Path) -> Result<Report, main_error::MainError> {
    let mut entries = entries_from_file(file_path)?;
    if entries.is_empty() {
        return Err(String::from("No entries").into());
    }

    entries.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(Report {
        filter: None,
        entries,
    })
}

struct Report {
    filter: Option<String>,
    entries: Vec<Entry>,
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rows: Vec<(String, String)> = self
            .entries
            .iter()
            .map(|entry| (format!("{}:", entry.date), entry.amount.human_readable()))
            .collect();

        let final_line_prefix: String = if let Some(filter) = self.filter.as_ref() {
            format!("Total amount for filter '{filter}':")
        } else {
            "Total amount:".to_string()
        };
        let total: Decimal = self.entries.iter().map(|entry| entry.amount).sum();
        let final_line_suffix: String = total.human_readable();
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
                result.push('\u{a0}');
                group_separator_index += 3;
            }
            result.push(ch);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::{Decimal, prelude::FromPrimitive};

    use super::HumanReadable;

    #[test]
    fn format_fractions() {
        insta::assert_snapshot!(Decimal::from_f32(0.006).unwrap().human_readable(), @r"0.01");
    }

    #[test]
    fn format_fractions_negative() {
        insta::assert_snapshot!(Decimal::from_f32(-0.006).unwrap().human_readable(), @r"-0.01");
    }

    #[test]
    fn format_singles() {
        insta::assert_snapshot!(Decimal::from_i8(1).unwrap().human_readable(), @r"1.00");
    }

    #[test]
    fn format_singles_negative() {
        insta::assert_snapshot!(Decimal::from_i8(-1).unwrap().human_readable(), @r"-1.00");
    }

    #[test]
    fn format_tens() {
        insta::assert_snapshot!(Decimal::from_i8(10).unwrap().human_readable(), @r"10.00");
    }

    #[test]
    fn format_tens_negative() {
        insta::assert_snapshot!(Decimal::from_i8(-10).unwrap().human_readable(), @r"-10.00");
    }

    #[test]
    fn format_hundreds() {
        insta::assert_snapshot!(Decimal::from_i8(100).unwrap().human_readable(), @r"100.00");
    }

    #[test]
    fn format_hundreds_negative() {
        insta::assert_snapshot!(Decimal::from_i8(-100).unwrap().human_readable(), @r"-100.00");
    }

    #[test]
    fn format_thousands() {
        insta::assert_snapshot!(Decimal::from_f32(1999.99).unwrap().human_readable(), @r"1 999.99");
    }

    #[test]
    fn format_thousands_negative() {
        insta::assert_snapshot!(Decimal::from_f32(-1999.99).unwrap().human_readable(), @r"-1 999.99");
    }

    #[test]
    fn format_ten_thousands() {
        insta::assert_snapshot!(Decimal::from_f32(19999.99).unwrap().human_readable(), @r"19 999.99");
    }

    #[test]
    fn format_ten_thousands_negative() {
        insta::assert_snapshot!(Decimal::from_f32(-19999.99).unwrap().human_readable(), @r"-19 999.99");
    }

    #[test]
    fn format_hundred_thousands() {
        insta::assert_snapshot!(Decimal::from_f64(199999.99).unwrap().human_readable(), @r"199 999.99");
    }

    #[test]
    fn format_hundred_thousands_negative() {
        insta::assert_snapshot!(Decimal::from_f64(-199999.99).unwrap().human_readable(), @r"-199 999.99");
    }

    #[test]
    fn format_million() {
        insta::assert_snapshot!(Decimal::from_f64(1999999.99).unwrap().human_readable(), @r"1 999 999.99");
    }

    #[test]
    fn format_million_negative() {
        insta::assert_snapshot!(Decimal::from_f64(-1999999.99).unwrap().human_readable(), @r"-1 999 999.99");
    }
}
