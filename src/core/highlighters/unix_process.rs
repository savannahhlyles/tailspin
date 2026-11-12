use crate::core::config::UnixProcessConfig;
use crate::core::highlighter::Highlight;
use nu_ansi_term::Style as NuStyle;
use regex::{Error, Regex, RegexBuilder};
use std::borrow::Cow;

pub struct UnixProcessHighlighter {
    regex: Regex,
    name_style: NuStyle,
    id_style: NuStyle,
    bracket_style: NuStyle,
}

impl UnixProcessHighlighter {
    pub fn new(config: UnixProcessConfig) -> Result<Self, Error> {
        let pattern = r"(?P<process_name>\([A-Za-z0-9._ +:/-]+\)|[A-Za-z0-9_/-]+)\[(?P<process_id>\d+)]";
        let regex = RegexBuilder::new(pattern).unicode(false).build()?;

        Ok(Self {
            regex,
            name_style: config.name.into(),
            id_style: config.id.into(),
            bracket_style: config.bracket.into(),
        })
    }

    fn format_capture(&self, captures: &regex::Captures) -> String {
        let process_name = captures
            .name("process_name")
            .map(|p| self.name_style.paint(p.as_str()).to_string())
            .unwrap_or_default();
        let process_num = captures
            .name("process_id")
            .map(|n| self.id_style.paint(n.as_str()).to_string())
            .unwrap_or_default();

        format!(
            "{}{}{}{}",
            process_name,
            self.bracket_style.paint("["),
            process_num,
            self.bracket_style.paint("]")
        )
    }
}

impl Highlight for UnixProcessHighlighter {
    fn apply<'a>(&self, input: &'a str) -> Cow<'a, str> {
        self.regex
            .replace_all(input, |captures: &regex::Captures| self.format_capture(captures))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::escape_code_converter::ConvertEscapeCodes;
    use crate::style::{Color, Style};

    #[test]
    fn test_unix_process_highlighter() {
        let highlighter = UnixProcessHighlighter::new(UnixProcessConfig {
            name: Style::new().fg(Color::Magenta),
            id: Style::new().fg(Color::Green),
            bracket: Style::new().fg(Color::Blue),
        })
        .unwrap();

        let cases = vec![
            (
                "process[1]",
                "[magenta]process[reset][blue][[reset][green]1[reset][blue]][reset]",
            ),
            (
                "postfix/postscreen[1894]: CONNECT from [192.168.1.22]:12345 to [127.0.0.1]:25",
                "[magenta]postfix/postscreen[reset][blue][[reset][green]1894[reset][blue]][reset]: CONNECT from [192.168.1.22]:12345 to [127.0.0.1]:25",
            ),
            ("No process here!", "No process here!"),
        ];

        for (input, expected) in cases {
            let actual = highlighter.apply(input);
            assert_eq!(expected, actual.to_string().convert_escape_codes());
        }
    }
}
