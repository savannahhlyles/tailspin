use nu_ansi_term::Style as NuStyle;
use std::borrow::Cow;

use crate::core::config::QuotesConfig;
use crate::core::highlighter::Highlight;
use crate::style::Style;

const RESET_SEQ: &str = "\x1b[0m";

pub struct QuoteHighlighter {
    token: char,
    style_code: String,
}

impl QuoteHighlighter {
    pub fn new(config: QuotesConfig) -> Self {
        let style_code = extract_color_prefix(config.style);

        Self {
            token: config.quotes_token,
            style_code,
        }
    }

    fn has_valid_quotes(&self, input: &str) -> bool {
        let count = input.chars().filter(|&ch| ch == self.token).count();
        count > 0 && count % 2 == 0
    }

    fn process_quoted_char(ch: char, style_code: &str, reset_buffer: &mut String, output: &mut String) {
        reset_buffer.push(ch);
        if reset_buffer.as_str() == RESET_SEQ {
            output.push_str(reset_buffer);
            output.push_str(style_code);
            reset_buffer.clear();
        } else if !RESET_SEQ.starts_with(reset_buffer.as_str()) {
            output.push_str(reset_buffer);
            reset_buffer.clear();
        }
    }
}

fn extract_color_prefix(style: Style) -> String {
    let nu_style = NuStyle::from(style);
    let styled = format!("{}", nu_style.paint(""));
    styled.replace(RESET_SEQ, "")
}

impl Highlight for QuoteHighlighter {
    fn apply<'a>(&self, input: &'a str) -> Cow<'a, str> {
        if !self.has_valid_quotes(input) {
            return Cow::Borrowed(input);
        }

        let mut inside = false;
        let mut output = String::new();
        let mut reset_buffer = String::new();

        for ch in input.chars() {
            if inside {
                if ch == self.token {
                    output.push(ch);
                    output.push_str(RESET_SEQ);
                    inside = false;
                    continue;
                }

                Self::process_quoted_char(ch, &self.style_code, &mut reset_buffer, &mut output);
            } else {
                if ch == self.token {
                    output.push_str(&self.style_code);
                    output.push(ch);
                    inside = true;
                    reset_buffer.clear();
                    continue;
                }
                output.push(ch);
            }
        }

        Cow::Owned(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::escape_code_converter::{ConvertEscapeCodes, ConvertHighlightCodes};
    use crate::style::{Color, Style};

    #[test]
    fn test_multiple() {
        let highlighter = QuoteHighlighter::new(QuotesConfig {
            quotes_token: '"',
            style: Style::new().fg(Color::Yellow),
        });

        let cases = vec![
            (
                r#"Lorem ipsum "dolor" sit amet"#,
                r#"Lorem ipsum [yellow]"dolor"[reset] sit amet"#,
            ),
            (
                r#"Lorem ipsum dolor sit amet, consectetur adipiscing elit"#,
                r#"Lorem ipsum dolor sit amet, consectetur adipiscing elit"#,
            ),
        ];

        for (input, expected) in cases {
            let actual = highlighter.apply(input);
            assert_eq!(expected, actual.to_string().convert_escape_codes());
        }
    }

    #[test]
    fn test_no_overwrite() {
        let highlighter = QuoteHighlighter::new(QuotesConfig {
            quotes_token: '"',
            style: Style::new().fg(Color::Yellow),
        });

        let input = r#"Hello "abc [red]def[reset] ghi" World"#.to_string().convert_highlight_codes();
        let expected = r#"Hello [yellow]"abc [red]def[reset][yellow] ghi"[reset] World"#;

        let actual = highlighter.apply(input.as_str());

        assert_eq!(actual.to_string().convert_escape_codes(), expected);
    }

    #[test]
    fn test_odd_number_of_highlight_tokens() {
        let highlighter = QuoteHighlighter::new(QuotesConfig {
            quotes_token: '"',
            style: Style::new().fg(Color::Yellow),
        });

        let input = r#"Hello "abc def ghi World"#;
        let expected = r#"Hello "abc def ghi World"#;

        let actual = highlighter.apply(input);

        assert_eq!(actual.to_string().convert_escape_codes(), expected);
    }
}
