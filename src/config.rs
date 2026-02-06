use crate::number_formatter::{CurrencyPosition, FormatOptions};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, Eq, PartialEq)]
#[serde(default)]
pub struct Config {
    pub formatting: FormattingConfig,
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

    #[test]
    fn test_default_format_options() {
        let config = Config::default();
        let expected = FormattingConfig::default().format_options();
        assert_eq!(config.formatting.format_options(), expected);
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
        assert_eq!(
            format_options,
            FormatOptions {
                thousands_separator: '\u{a0}',
                decimal_separator: ',',
                currency: CurrencyPosition::Prefix(String::from("$"))
            }
        );
    }
}
