use insta::assert_snapshot;
use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn new_entry_subtract() {
    let test_context = TestContext::new();

    let args = vec!["new-entry", "--amount", "-900"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
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
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["new-entry", "--amount", "42.42"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
           3 510.42
              42.42
    Total: 3 552.84

    ----- stderr -----
    ");
}

#[test]
fn new_entry_with_date_into_existing_file() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["new-entry", "--amount", "42.42", "--date", "2024-09-12"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
           3 510.42
              42.42
    Total: 3 552.84

    ----- stderr -----
    ");

    assert_snapshot!(test_context.content(), @r"
    date;amount
    2024-10-01;-200
    2024-09-11;700
    2024-10-02;3000.42
    2025-01-01;10
    2024-09-12;42.42
    ");
}

#[test]
fn new_entry_with_invalid_date_error() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["new-entry", "--amount", "42.42", "--date", "2024-12"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: Invalid date format: 2024-12 (premature end of input)
    caused by: premature end of input
    ");
}

#[test]
fn report_without_filter() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["report"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
      2024-09-11:   700.00
      2024-10-01:  -200.00
      2024-10-02: 3 000.42
      2025-01-01:    10.00
    Total amount: 3 510.42

    ----- stderr -----
    ");
}

#[test]
fn report_filter_year() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["report", "--filter", "2024"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
                        2024-09-11:   700.00
                        2024-10-01:  -200.00
                        2024-10-02: 3 000.42
    Total amount for filter '2024': 3 500.42

    ----- stderr -----
    ");
}

#[test]
fn report_filter_year_no_entries_error() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["report", "--filter", "2020"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: No entries matching filter: 2020
    ");
}

#[test]
fn report_filter_year_month() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["report", "--filter", "2024-10"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
                           2024-10-01:  -200.00
                           2024-10-02: 3 000.42
    Total amount for filter '2024-10': 2 800.42

    ----- stderr -----
    ");
}

#[test]
fn report_filter_year_month_no_entries_error() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["report", "--filter", "2020-01"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: No entries matching filter: 2020-01
    ");
}

#[test]
fn report_no_file_error() {
    let mut test_context = TestContext::new();
    test_context.setup_insta_filter();

    let args = vec!["report"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: I/O error: Failed to access file: [TEMP_DIR]/test.csv
    caused by: No such file or directory (os error 2)
    ");
}

#[test]
fn report_no_entries_error() {
    let test_context = TestContext::new();
    test_context.setup_empty_test_content();

    let args = vec!["report"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    Error: No entries found
    ");
}

#[test]
fn sort() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    let args = vec!["sort"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    assert_snapshot!(test_context.content(), @r"
    date;amount
    2024-09-11;700
    2024-10-01;-200
    2024-10-02;3000.42
    2025-01-01;10
    ");
}

#[test]
fn test_version() {
    let args = vec!["--version"];
    assert_cmd_snapshot!(Cli::with_args(args).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    mfinance 0.1.0

    ----- stderr -----
    ");
}

#[test]
fn test_config_warning_on_invalid_config() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    // Create an invalid config file in the same directory
    let config_path = test_context.tempdir.child("mfinance.toml");
    fs::write(
        &config_path,
        r#"
        [formatting]
        thousands_separator = "invalid"  # char expects single character
        "#,
    )
    .expect("write invalid config");

    let args = vec!["report"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r###"
    success: true
    exit_code: 0
    ----- stdout -----
      2024-09-11:   700.00
      2024-10-01:  -200.00
      2024-10-02: 3 000.42
      2025-01-01:    10.00
    Total amount: 3 510.42

    ----- stderr -----
    Warning: Failed to load config: invalid value: string "invalid", expected a character for key `formatting.thousands_separator`
    "###);
}

#[test]
fn test_config_with_only_data() {
    let test_context = TestContext::new();
    test_context.setup_test_content();

    // Create data config
    let data_config_path = test_context.tempdir.child("mfinance.toml");
    fs::write(
        &data_config_path,
        r#"
        [formatting]
        currency_symbol = " $"
        currency_position = "Suffix"
        thousands_separator = ","
        "#,
    )
    .expect("write data config");

    let args = vec!["report"];
    assert_cmd_snapshot!(Cli::with_args(args).path(test_context.content_path()).cmd(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
      2024-09-11:   700.00 $
      2024-10-01:  -200.00 $
      2024-10-02: 3,000.42 $
      2025-01-01:    10.00 $
    Total amount: 3,510.42 $

    ----- stderr -----
    ");
}

struct Cli {
    command: Command,
}

impl Cli {
    fn with_args(args: Vec<&str>) -> Self {
        let mut command = Command::new(get_cargo_bin("mfinance"));
        command.env("MFINANCE_TEST_MODE", "1");
        command.args(&args);
        Self { command }
    }

    fn path(mut self, path: impl AsRef<Path>) -> Self {
        self.command.arg(path.as_ref().as_os_str());
        self
    }

    fn cmd(self) -> Command {
        self.command
    }
}

struct TestContext {
    tempdir: temp_dir::TempDir,
    #[allow(dyn_drop)]
    insta_settings_bind_drop_guard: Option<Box<dyn Drop>>,
}

impl TestContext {
    fn new() -> Self {
        TestContext {
            tempdir: temp_dir::TempDir::with_prefix("mfinance-").unwrap(),
            insta_settings_bind_drop_guard: None,
        }
    }

    fn setup_insta_filter(&mut self) {
        let mut settings = insta::Settings::clone_current();
        settings.add_filter(&self.tempdir.path().to_string_lossy(), "[TEMP_DIR]");
        self.insta_settings_bind_drop_guard = Some(Box::new(settings.bind_to_scope()));
    }

    fn content_path(&self) -> PathBuf {
        self.tempdir.child("test.csv")
    }

    fn setup_test_content(&self) {
        // The content is intentionally unsorted.
        fs::write(
            self.content_path(),
            "date;amount\n2024-10-01;-200\n2024-09-11;700\n2024-10-02;3000.42\n2025-01-01;10\n",
        )
        .expect("write test.csv");
    }

    fn setup_empty_test_content(&self) {
        fs::write(self.content_path(), "date;amount\n").expect("write empty test.csv");
    }

    fn content(&self) -> String {
        fs::read_to_string(self.content_path()).expect("read test.csv")
    }
}
