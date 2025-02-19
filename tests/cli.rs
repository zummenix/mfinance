use insta::assert_snapshot;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn new_entry_subtract() {
    let csv_file = TempCsvFile::new();

    let args = NewEntryArgs::with_amount("-900");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: true
    exit_code: 0
    ----- stdout -----
              0.00
           -900.00
    Total: -900.00

    ----- stderr -----
    ");
}

#[test]
fn new_entry_into_existing_file() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = NewEntryArgs::with_amount("42.42");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: true
    exit_code: 0
    ----- stdout -----
           1 210.42
              42.42
    Total: 1 252.84

    ----- stderr -----
    ");
}

#[test]
fn new_entry_with_date_into_existing_file() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = NewEntryArgs::with_amount("42.42").date("2024-09-12");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: true
    exit_code: 0
    ----- stdout -----
           1 210.42
              42.42
    Total: 1 252.84

    ----- stderr -----
    ");

    assert_snapshot!(csv_file.content(), @r"
    date;amount
    2024-09-11;700
    2024-09-12;42.42
    2024-10-01;200
    2024-10-02;300.42
    2025-01-01;10
    ");
}

#[test]
fn report_without_period() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = ReportArgs::new();
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Total amount: 1 210.42

    ----- stderr -----
    ");
}

#[test]
fn report_period_year() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = ReportArgs::new().period("2024");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Total amount for 2024: 1 200.42

    ----- stderr -----
    ");
}

#[test]
fn report_period_year_month() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = ReportArgs::new().period("2024-10");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Total amount for 2024-10: 500.42

    ----- stderr -----
    ");
}

#[test]
fn test_version() {
    assert_cmd_snapshot!(mfinance().arg("--version"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    mfinance 0.1.0

    ----- stderr -----
    ");
}

fn mfinance() -> Command {
    Command::new(get_cargo_bin("mfinance"))
}

struct NewEntryArgs {
    amount: &'static str,
    date: Option<&'static str>,
}

impl NewEntryArgs {
    fn with_amount(amount: &'static str) -> Self {
        NewEntryArgs { amount, date: None }
    }

    fn date(mut self, date: &'static str) -> Self {
        self.date = Some(date);
        self
    }

    fn cmd(&self, file: &Path) -> Command {
        let mut cmd = mfinance();
        cmd.arg("new-entry").arg(format!("-a={}", self.amount));
        if let Some(date) = self.date {
            cmd.arg(format!("-d={}", date));
        }
        cmd.arg(file.as_os_str());
        cmd
    }
}

struct ReportArgs {
    period: Option<&'static str>,
}

impl ReportArgs {
    fn new() -> Self {
        ReportArgs { period: None }
    }

    fn period(mut self, period: &'static str) -> Self {
        self.period = Some(period);
        self
    }

    fn cmd(&self, file: &Path) -> Command {
        let mut cmd = mfinance();
        cmd.arg("report");
        if let Some(period) = self.period {
            cmd.arg(format!("-p={period}"));
        }
        cmd.arg(file.as_os_str());
        cmd
    }
}

struct TempCsvFile {
    tempdir: temp_dir::TempDir,
}

impl TempCsvFile {
    fn new() -> Self {
        TempCsvFile {
            tempdir: temp_dir::TempDir::with_prefix("mfinance-").unwrap(),
        }
    }

    fn path(&self) -> PathBuf {
        self.tempdir.child("test.csv")
    }

    fn setup_test_content(&self) {
        fs::write(
            self.path(),
            "date;amount\n2024-09-11;700\n2024-10-01;200\n2024-10-02;300.42\n2025-01-01;10",
        )
        .expect("write test.csv");
    }

    fn content(&self) -> String {
        fs::read_to_string(self.path()).expect("read test.csv")
    }
}
