use crate::number_formatter::{CurrencyPosition, FormatOptions};
use config;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Config {
    pub formatting: FormattingConfig,
}

impl Config {
    pub fn load(
        global_config_path: Option<impl AsRef<Path>>,
        data_config_path: Option<impl AsRef<Path>>,
    ) -> Self {
        let mut settings = config::Config::builder();

        if let Some(path) = global_config_path {
            settings = settings.add_source(config::File::from(path.as_ref()).required(false));
        }

        if let Some(path) = data_config_path {
            settings = settings.add_source(config::File::from(path.as_ref()).required(false));
        }

        match settings.build() {
            Ok(settings) => match settings.try_deserialize::<Config>() {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Warning! Failed to parse config: {e}");
                    Config::default()
                }
            },
            Err(e) => {
                eprintln!("Warning! Failed to load config: {e}");
                Config::default()
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FormattingConfig {
    #[serde(rename = "currency_symbol")]
    pub currency: Option<String>,
    #[serde(rename = "currency_position")]
    pub currency_position: Option<CurrencyPositionChoice>,
    #[serde(rename = "thousands_separator")]
    pub thousands_separator: char,
    #[serde(rename = "decimal_separator")]
    pub decimal_separator: char,
}

impl FormattingConfig {
    pub fn format_options(&self) -> FormatOptions {
        let currency = match (self.currency.as_ref(), self.currency_position) {
            (Some(symbol), Some(CurrencyPositionChoice::Prefix)) => {
                CurrencyPosition::Prefix(symbol.clone())
            }
            (Some(symbol), Some(CurrencyPositionChoice::Suffix)) => {
                CurrencyPosition::Suffix(symbol.clone())
            }
            _ => CurrencyPosition::None,
        };

        FormatOptions {
            thousands_separator: self.thousands_separator,
            decimal_separator: self.decimal_separator,
            currency,
        }
    }
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            currency: None,
            currency_position: None,
            thousands_separator: '\u{a0}',
            decimal_separator: '.',
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum CurrencyPositionChoice {
    Prefix,
    Suffix,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use temp_dir::TempDir;

    fn create_temp_config(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let file_path = dir.child("config.toml");
        std::fs::write(&file_path, content).unwrap();
        (dir, file_path)
    }

    #[test]
    fn test_default_format_options() {
        let config = Config::default();
        let expected = FormattingConfig::default().format_options();
        assert_eq!(config.formatting.format_options(), expected);
    }

    #[test]
    fn test_without_configs() {
        let config = Config::load(Option::<&Path>::None, Option::<&Path>::None);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_invalid_path() {
        let config = Config::load(Some("/nonexistent/path"), Option::<&Path>::None);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_from_file() {
        let (_dir, config_file) = create_temp_config(
            r#"
            [formatting]
            currency_symbol = "$"
            currency_position = "Prefix"
            thousands_separator = ","
            decimal_separator = "."
            "#,
        );

        let config = Config::load(Some(config_file.as_path()), Option::<&Path>::None);
        let expected = FormattingConfig {
            currency: Some("$".to_string()),
            currency_position: Some(CurrencyPositionChoice::Prefix),
            thousands_separator: ',',
            decimal_separator: '.',
        };

        assert_eq!(config.formatting, expected);
    }

    #[test]
    fn test_load_multiple_configs() {
        let (_global_dir, global_config) = create_temp_config(
            r#"
            [formatting]
            currency_symbol = "€"
            decimal_separator = ","
            "#,
        );

        let (_data_dir, data_config) = create_temp_config(
            r#"
            [formatting]
            currency_position = "Suffix"
            thousands_separator = "."
            "#,
        );

        let config = Config::load(Some(global_config.as_path()), Some(data_config.as_path()));
        let expected = FormattingConfig {
            currency: Some("€".to_string()),
            currency_position: Some(CurrencyPositionChoice::Suffix),
            thousands_separator: '.',
            decimal_separator: ',',
        };

        assert_eq!(config.formatting, expected);
    }

    #[test]
    fn test_format_options_conversion() {
        let mut config = Config::default();
        config.formatting = FormattingConfig {
            currency: Some("$".to_string()),
            currency_position: Some(CurrencyPositionChoice::Prefix),
            thousands_separator: '\u{a0}',
            decimal_separator: ',',
        };

        let format_options = config.formatting.format_options();
        assert!(matches!(
            format_options.currency,
            CurrencyPosition::Prefix(s) if s == "$"
        ));
    }
}
