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
           3 510.42
              42.42
    Total: 3 552.84

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
           3 510.42
              42.42
    Total: 3 552.84

    ----- stderr -----
    ");

    assert_snapshot!(csv_file.content(), @r"
    date;amount
    2024-09-11;700
    2024-09-12;42.42
    2024-10-01;-200
    2024-10-02;3000.42
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
      2024-09-11:   700.00
      2024-10-01:  -200.00
      2024-10-02: 3 000.42
      2025-01-01:    10.00
    Total amount: 3 510.42

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
               2024-09-11:   700.00
               2024-10-01:  -200.00
               2024-10-02: 3 000.42
    Total amount for 2024: 3 500.42

    ----- stderr -----
    ");
}

#[test]
fn report_period_year_no_records() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = ReportArgs::new().period("2020");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: No records for the given period: 2020
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
                  2024-10-01:  -200.00
                  2024-10-02: 3 000.42
    Total amount for 2024-10: 2 800.42

    ----- stderr -----
    ");
}

#[test]
fn report_period_year_month_no_records() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_test_content();

    let args = ReportArgs::new().period("2020-01");
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: No records for the given period: 2020-01
    ");
}

#[test]
fn report_no_file() {
    let mut csv_file = TempCsvFile::new();
    csv_file.setup_insta_filter();

    let args = ReportArgs::new();
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: File '[TEMP_DIR]/test.csv' does not exist
    ");
}

#[test]
fn report_no_records() {
    let csv_file = TempCsvFile::new();
    csv_file.setup_empty_test_content();

    let args = ReportArgs::new();
    assert_cmd_snapshot!(args.cmd(&csv_file.path()), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: No records
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
    #[allow(dyn_drop)]
    insta_settings_bind_drop_guard: Option<Box<dyn Drop>>,
}

impl TempCsvFile {
    fn new() -> Self {
        TempCsvFile {
            tempdir: temp_dir::TempDir::with_prefix("mfinance-").unwrap(),
            insta_settings_bind_drop_guard: None,
        }
    }

    fn setup_insta_filter(&mut self) {
        let mut settings = insta::Settings::clone_current();
        settings.add_filter(&self.tempdir.path().to_string_lossy(), "[TEMP_DIR]");
        self.insta_settings_bind_drop_guard = Some(Box::new(settings.bind_to_scope()));
    }

    fn path(&self) -> PathBuf {
        self.tempdir.child("test.csv")
    }

    fn setup_test_content(&self) {
        fs::write(
            self.path(),
            "date;amount\n2024-09-11;700\n2024-10-01;-200\n2024-10-02;3000.42\n2025-01-01;10",
        )
        .expect("write test.csv");
    }

    fn setup_empty_test_content(&self) {
        fs::write(self.path(), "date;amount\n").expect("write empty test.csv");
    }

    fn content(&self) -> String {
        fs::read_to_string(self.path()).expect("read test.csv")
    }
}
