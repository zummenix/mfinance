use rust_decimal::Decimal;

pub trait NumberFormatter {
    fn format(&self, options: &FormatOptions) -> String;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum CurrencyPosition {
    None,
    Prefix(String),
    Suffix(String),
}

#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub thousands_separator: char,
    pub decimal_separator: char,
    pub currency: CurrencyPosition,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            thousands_separator: '\u{a0}', // Non-breaking space
            decimal_separator: '.',
            currency: CurrencyPosition::None,
        }
    }
}

impl NumberFormatter for Decimal {
    fn format(&self, options: &FormatOptions) -> String {
        let precision = 2;
        let decimal = self.round_dp(precision as u32);
        let decimal_string =
            format!("{decimal:.precision$}").replace(".", &String::from(options.decimal_separator));

        let sign_offset = usize::from(decimal.is_sign_negative());
        let len_till_dot = decimal_string.len() - 1 - precision;
        let mut group_separator_index = (len_till_dot - sign_offset) % 3 + sign_offset;
        if group_separator_index == sign_offset {
            group_separator_index = 3 + sign_offset;
        }
        let mut formatted = String::new();
        for (i, ch) in decimal_string.char_indices() {
            if group_separator_index == i && group_separator_index < len_till_dot {
                formatted.push(options.thousands_separator);
                group_separator_index += 3;
            }
            formatted.push(ch);
        }

        match &options.currency {
            CurrencyPosition::Prefix(symbol) => format!("{}{}", symbol, formatted),
            CurrencyPosition::Suffix(symbol) => format!("{}{}", formatted, symbol),
            CurrencyPosition::None => formatted,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::{Decimal, prelude::FromPrimitive};

    #[test]
    fn format_with_currency_prefix() {
        let options = FormatOptions {
            currency: CurrencyPosition::Prefix("€".to_string()),
            ..FormatOptions::default()
        };
        insta::assert_snapshot!(Decimal::from(1000).format(&options), @r"€1 000.00");
    }

    #[test]
    fn format_with_currency_suffix() {
        let options = FormatOptions {
            currency: CurrencyPosition::Suffix(" EUR".to_string()),
            ..FormatOptions::default()
        };
        insta::assert_snapshot!(Decimal::from(1000).format(&options), @"1 000.00 EUR");
    }

    #[test]
    fn format_with_thousands_separator() {
        let options = FormatOptions {
            thousands_separator: ',',
            ..FormatOptions::default()
        };
        insta::assert_snapshot!(Decimal::from(1000).format(&options), @"1,000.00");
    }

    #[test]
    fn format_with_decimal_separator() {
        let options = FormatOptions {
            decimal_separator: ',',
            ..FormatOptions::default()
        };
        insta::assert_snapshot!(Decimal::from(1000).format(&options), @"1 000,00");
    }

    #[test]
    fn format_fractions_negative() {
        insta::assert_snapshot!(Decimal::from_f32(-0.006).unwrap().format(&FormatOptions::default()), @r"-0.01");
    }

    #[test]
    fn format_fractions() {
        let options = FormatOptions::default();
        insta::assert_snapshot!(Decimal::from_f32(0.006).unwrap().format(&options), @r"0.01");
    }

    #[test]
    fn format_singles() {
        insta::assert_snapshot!(Decimal::from_i8(1).unwrap().format(&FormatOptions::default()), @r"1.00");
    }

    #[test]
    fn format_singles_negative() {
        insta::assert_snapshot!(Decimal::from_i8(-1).unwrap().format(&FormatOptions::default()), @r"-1.00");
    }

    #[test]
    fn format_tens() {
        insta::assert_snapshot!(Decimal::from_i8(10).unwrap().format(&FormatOptions::default()), @r"10.00");
    }

    #[test]
    fn format_tens_negative() {
        insta::assert_snapshot!(Decimal::from_i8(-10).unwrap().format(&FormatOptions::default()), @r"-10.00");
    }

    #[test]
    fn format_hundreds() {
        insta::assert_snapshot!(Decimal::from_i8(100).unwrap().format(&FormatOptions::default()), @r"100.00");
    }

    #[test]
    fn format_hundreds_negative() {
        insta::assert_snapshot!(Decimal::from_i8(-100).unwrap().format(&FormatOptions::default()), @r"-100.00");
    }

    #[test]
    fn format_thousands() {
        let options = FormatOptions::default();
        insta::assert_snapshot!(Decimal::from_f32(1999.99).unwrap().format(&options), @r"1 999.99");
    }

    #[test]
    fn format_thousands_negative() {
        insta::assert_snapshot!(Decimal::from_f32(-1999.99).unwrap().format(&FormatOptions::default()), @r"-1 999.99");
    }

    #[test]
    fn format_ten_thousands() {
        insta::assert_snapshot!(Decimal::from_f32(19999.99).unwrap().format(&FormatOptions::default()), @r"19 999.99");
    }

    #[test]
    fn format_ten_thousands_negative() {
        insta::assert_snapshot!(Decimal::from_f32(-19999.99).unwrap().format(&FormatOptions::default()), @r"-19 999.99");
    }

    #[test]
    fn format_hundred_thousands() {
        insta::assert_snapshot!(Decimal::from_f64(199999.99).unwrap().format(&FormatOptions::default()), @r"199 999.99");
    }

    #[test]
    fn format_hundred_thousands_negative() {
        insta::assert_snapshot!(Decimal::from_f64(-199999.99).unwrap().format(&FormatOptions::default()), @r"-199 999.99");
    }

    #[test]
    fn format_million() {
        insta::assert_snapshot!(Decimal::from_f64(1999999.99).unwrap().format(&FormatOptions::default()), @r"1 999 999.99");
    }

    #[test]
    fn format_million_negative() {
        insta::assert_snapshot!(Decimal::from_f64(-1999999.99).unwrap().format(&FormatOptions::default()), @r"-1 999 999.99");
    }
}
