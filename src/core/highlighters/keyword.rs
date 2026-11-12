use crate::core::config::KeywordConfig;
use crate::core::highlighter::Highlight;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use nu_ansi_term::Style as NuStyle;
use regex::Error;
use std::borrow::Cow;

pub struct KeywordHighlighter {
    ac: AhoCorasick,
    match_style: NuStyle,
}

impl KeywordHighlighter {
    pub fn new(keyword_config: KeywordConfig) -> Result<Self, Error> {
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostFirst)
            .build(&keyword_config.words)
            .map_err(|e| Error::Syntax(e.to_string()))?;

        Ok(Self {
            ac,
            match_style: keyword_config.style.into(),
        })
    }

    fn paint_keyword(&self, text: &str) -> String {
        if self.match_style.background.is_none() {
            format!("{}", self.match_style.paint(text))
        } else {
            let padded = format!(" {} ", text);
            format!("{}", self.match_style.paint(padded))
        }
    }
}

fn is_word_char(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
}

fn at_word_boundary(hay: &[u8], start: usize, end: usize) -> bool {
    let left_ok = start == 0 || !is_word_char(hay[start - 1]);
    let right_ok = end == hay.len() || !is_word_char(hay[end]);
    left_ok && right_ok
}

impl Highlight for KeywordHighlighter {
    fn apply<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let bytes = input.as_bytes();
        let mut result: Option<String> = None;
        let mut tail = 0;

        for m in self.ac.find_iter(bytes) {
            let (start, end) = (m.start(), m.end());
            if !at_word_boundary(bytes, start, end) {
                continue;
            }

            let buf = result.get_or_insert_with(|| String::with_capacity(input.len() + 16));
            buf.push_str(&input[tail..start]);
            buf.push_str(&self.paint_keyword(&input[start..end]));

            tail = end;
        }

        match result {
            None => Cow::Borrowed(input),
            Some(mut s) => {
                s.push_str(&input[tail..]);
                Cow::Owned(s)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::escape_code_converter::ConvertEscapeCodes;
    use crate::style::{Color, Style};

    #[test]
    fn test_foreground_keyword_highlighter() {
        let config = KeywordConfig {
            words: vec!["null".to_string()],
            style: Style::new().fg(Color::Red),
        };
        let highlighter = KeywordHighlighter::new(config).unwrap();

        let cases = vec![
            ("Hello null world", "Hello [red]null[reset] world"),
            (
                "There are 1001 nights in the tale.",
                "There are 1001 nights in the tale.",
            ),
            ("No numbers here!", "No numbers here!"),
        ];

        for (input, expected) in cases {
            let actual = highlighter.apply(input);
            assert_eq!(expected, actual.to_string().convert_escape_codes());
        }
    }

    #[test]
    fn test_background_keyword_highlighter() {
        let config = KeywordConfig {
            words: vec!["null".to_string()],
            style: Style::new().on(Color::Red),
        };
        let highlighter = KeywordHighlighter::new(config).unwrap();

        let cases = vec![
            ("Hello null world", "Hello [bg_red] null [reset] world"),
            (
                "There are 1001 nights in the tale.",
                "There are 1001 nights in the tale.",
            ),
            ("No numbers here!", "No numbers here!"),
        ];

        for (input, expected) in cases {
            let actual = highlighter.apply(input);
            assert_eq!(expected, actual.to_string().convert_escape_codes());
        }
    }
}
