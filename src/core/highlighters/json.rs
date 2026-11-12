use crate::core::config::JsonConfig;
use crate::core::highlighter::Highlight;
use nu_ansi_term::Style as NuStyle;
use serde_json::Value;
use std::borrow::Cow;
use std::fmt::Write;

pub struct JsonHighlighter {
    pub key: NuStyle,
    pub quote_token: NuStyle,
    pub curly_bracket: NuStyle,
    pub square_bracket: NuStyle,
    pub comma: NuStyle,
    pub colon: NuStyle,
}

impl JsonHighlighter {
    pub fn new(config: JsonConfig) -> Self {
        Self {
            key: config.key.into(),
            quote_token: config.quote_token.into(),
            curly_bracket: config.curly_bracket.into(),
            square_bracket: config.square_bracket.into(),
            comma: config.comma.into(),
            colon: config.colon.into(),
        }
    }

    fn format_value(&self, value: &Value, buf: &mut String) {
        match value {
            Value::Object(map) => {
                write!(buf, "{}", self.curly_bracket.paint("{")).unwrap();
                let mut first = true;
                for (key, val) in map {
                    if !first {
                        write!(buf, "{}", self.comma.paint(",")).unwrap();
                    }
                    first = false;

                    write!(
                        buf,
                        " {}{}{}",
                        self.quote_token.paint("\""),
                        self.key.paint(key),
                        self.quote_token.paint("\"")
                    )
                    .unwrap();
                    write!(buf, "{} ", self.colon.paint(":")).unwrap();

                    self.format_value(val, buf);
                }
                write!(buf, " {}", self.curly_bracket.paint("}")).unwrap();
            }
            Value::Array(arr) => {
                write!(buf, "{}", self.square_bracket.paint("[")).unwrap();
                let mut first = true;
                for item in arr {
                    if !first {
                        write!(buf, "{} ", self.comma.paint(",")).unwrap();
                    }
                    first = false;

                    self.format_value(item, buf);
                }
                write!(buf, "{}", self.square_bracket.paint("]")).unwrap();
            }
            Value::String(s) => {
                write!(
                    buf,
                    "{}{}{}",
                    self.quote_token.paint("\""),
                    s,
                    self.quote_token.paint("\"")
                )
                .unwrap();
            }
            Value::Number(n) => {
                write!(buf, "{}", n).unwrap();
            }
            Value::Bool(b) => {
                write!(buf, "{}", b).unwrap();
            }
            Value::Null => {
                write!(buf, "null").unwrap();
            }
        }
    }
}

fn scan_object_end(input: &[u8], pos: usize) -> Option<usize> {
    let mut i = pos + 1;
    while i < input.len() {
        match input[i] {
            b'{' => {
                i = scan_object_end(input, i)?;
                i += 1;
            }
            b'[' => {
                i = scan_array_end(input, i)?;
                i += 1;
            }
            b'"' => {
                i = skip_json_string(input, i);
            }
            b'}' => return Some(i),
            _ => {
                i += 1;
            }
        }
    }
    None
}

fn scan_array_end(input: &[u8], pos: usize) -> Option<usize> {
    let mut i = pos + 1;
    while i < input.len() {
        match input[i] {
            b'{' => {
                i = scan_object_end(input, i)?;
                i += 1;
            }
            b'[' => {
                i = scan_array_end(input, i)?;
                i += 1;
            }
            b'"' => {
                i = skip_json_string(input, i);
            }
            b']' => return Some(i),
            _ => {
                i += 1;
            }
        }
    }
    None
}

fn skip_json_string(input: &[u8], pos: usize) -> usize {
    let mut i = pos + 1;
    while i < input.len() {
        if input[i] == b'\\' {
            i += 2;
            continue;
        }
        if input[i] == b'"' {
            return i + 1;
        }
        i += 1;
    }
    i
}

fn locate_json_span(input: &str) -> Option<(usize, usize)> {
    let bytes = input.as_bytes();
    for (start, &b) in bytes.iter().enumerate() {
        let end = match b {
            b'{' => scan_object_end(bytes, start),
            b'[' => scan_array_end(bytes, start),
            _ => continue,
        };
        if let Some(close) = end {
            if serde_json::from_str::<Value>(&input[start..=close]).is_ok() {
                return Some((start, close + 1));
            }
        }
    }
    None
}

impl Highlight for JsonHighlighter {
    fn apply<'a>(&self, input: &'a str) -> Cow<'a, str> {
        if let Ok(value) = serde_json::from_str::<Value>(input) {
            let mut buf = String::new();
            self.format_value(&value, &mut buf);
            return Cow::Owned(buf);
        }

        if let Some((start, end)) = locate_json_span(input) {
            if let Ok(value) = serde_json::from_str::<Value>(&input[start..end]) {
                let mut buf = String::with_capacity(input.len() + 64);
                buf.push_str(&input[..start]);
                self.format_value(&value, &mut buf);
                buf.push_str(&input[end..]);
                return Cow::Owned(buf);
            }
        }

        Cow::Borrowed(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::escape_code_converter::ConvertEscapeCodes;
    use crate::style::{Color, Style};

    #[test]
    fn test_number_highlighter() {
        let config = JsonConfig {
            key: Style::new().fg(Color::Yellow),
            quote_token: Style::new().fg(Color::Blue),
            curly_bracket: Style::new().fg(Color::Cyan),
            square_bracket: Style::new().fg(Color::Green),
            comma: Style::new().fg(Color::Red),
            colon: Style::new().fg(Color::Magenta),
        };
        let highlighter = JsonHighlighter::new(config);

        let cases = vec![
            (
                r#"{ "name": "John Doe", "age": 43, "phones": [ "+44 1234567", "+44 2345678" ] }"#,
                r#"[cyan]{[reset] [blue]"[reset][yellow]name[reset][blue]"[reset][magenta]:[reset] [blue]"[reset]John Doe[blue]"[reset][red],[reset] [blue]"[reset][yellow]age[reset][blue]"[reset][magenta]:[reset] 43[red],[reset] [blue]"[reset][yellow]phones[reset][blue]"[reset][magenta]:[reset] [green][[reset][blue]"[reset]+44 1234567[blue]"[reset][red],[reset] [blue]"[reset]+44 2345678[blue]"[reset][green]][reset] [cyan]}[reset]"#,
            ),
            (
                r#"{ "name": "John", "age": 30 }"#,
                r#"[cyan]{[reset] [blue]"[reset][yellow]name[reset][blue]"[reset][magenta]:[reset] [blue]"[reset]John[blue]"[reset][red],[reset] [blue]"[reset][yellow]age[reset][blue]"[reset][magenta]:[reset] 30 [cyan]}[reset]"#,
            ),
            ("No jsons here!", "No jsons here!"),
        ];

        for (input, expected) in cases {
            let actual = highlighter.apply(input);
            assert_eq!(expected, actual.to_string().convert_escape_codes());
        }
    }
}
