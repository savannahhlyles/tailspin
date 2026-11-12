use crate::core::config::DateTimeConfig;
use crate::core::highlighter::Highlight;
use nu_ansi_term::Style as NuStyle;
use regex::{Captures, Error, Regex, RegexBuilder};
use std::borrow::Cow;
use std::fmt::Write as _;

pub struct TimeHighlighter {
    regex: Regex,
    parts: Vec<(usize, PartKind)>,
    time_style: NuStyle,
    zone_style: NuStyle,
    sep_style: NuStyle,
}

#[derive(Copy, Clone)]
enum PartKind {
    Time,
    Zone,
    Separator,
}

impl TimeHighlighter {
    pub fn new(time_config: DateTimeConfig) -> Result<Self, Error> {
        let pattern = r"(?x)
            (?P<T>[T\s])?
            (?P<hours>[01]?\d|2[0-3])(?P<colon1>:)
            (?P<minutes>[0-5]\d)(?P<colon2>:)
            (?P<seconds>[0-5]\d)
            (?P<frac_sep>[.,:])?(?P<frac_digits>\d+)?
            (?P<tz>Z)?
        ";

        let regex = RegexBuilder::new(pattern).unicode(false).build()?;

        let mut name_map = std::collections::HashMap::new();
        for (i, name_opt) in regex.capture_names().enumerate() {
            if let Some(name) = name_opt {
                name_map.insert(name.to_string(), i);
            }
        }

        let parts = vec![
            (name_map["T"], PartKind::Zone),
            (name_map["hours"], PartKind::Time),
            (name_map["colon1"], PartKind::Separator),
            (name_map["minutes"], PartKind::Time),
            (name_map["colon2"], PartKind::Separator),
            (name_map["seconds"], PartKind::Time),
            (name_map["frac_sep"], PartKind::Separator),
            (name_map["frac_digits"], PartKind::Time),
            (name_map["tz"], PartKind::Zone),
        ];

        Ok(Self {
            regex,
            parts,
            time_style: time_config.time.into(),
            zone_style: time_config.zone.into(),
            sep_style: time_config.separator.into(),
        })
    }

    fn style_for(&self, kind: PartKind) -> &NuStyle {
        match kind {
            PartKind::Time => &self.time_style,
            PartKind::Zone => &self.zone_style,
            PartKind::Separator => &self.sep_style,
        }
    }

    fn write_parts(&self, caps: &Captures<'_>, buf: &mut String) {
        for &(idx, kind) in &self.parts {
            if let Some(m) = caps.get(idx) {
                let _ = write!(buf, "{}", self.style_for(kind).paint(m.as_str()));
            }
        }
    }
}

impl Highlight for TimeHighlighter {
    fn apply<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut iter = self.regex.captures_iter(input).peekable();
        if iter.peek().is_none() {
            return Cow::Borrowed(input);
        }

        let mut result = String::with_capacity(input.len() + 32);
        let mut tail = 0usize;

        for caps in self.regex.captures_iter(input) {
            let m = caps.get(0).unwrap();
            result.push_str(&input[tail..m.start()]);
            self.write_parts(&caps, &mut result);
            tail = m.end();
        }

        result.push_str(&input[tail..]);
        Cow::Owned(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::DateTimeConfig;
    use crate::core::tests::escape_code_converter::ConvertEscapeCodes;
    use crate::style::{Color, Style};

    #[test]
    fn test_time_highlighter() {
        let config = DateTimeConfig {
            date: Style::new(),
            time: Style::new().fg(Color::Red),
            zone: Style::new().fg(Color::Blue),
            separator: Style::new().fg(Color::Yellow),
        };
        let highlighter = TimeHighlighter::new(config).unwrap();

        let cases = vec![
            (
                "07:46:34",
                "[red]07[reset][yellow]:[reset][red]46[reset][yellow]:[reset][red]34[reset]",
            ),
            (
                "10:51:19.251",
                "[red]10[reset][yellow]:[reset][red]51[reset][yellow]:[reset][red]19[reset][yellow].[reset][red]251[reset]",
            ),
            (
                "11:47:39:850",
                "[red]11[reset][yellow]:[reset][red]47[reset][yellow]:[reset][red]39[reset][yellow]:[reset][red]850[reset]",
            ),
            (
                "3:33:30",
                "[red]3[reset][yellow]:[reset][red]33[reset][yellow]:[reset][red]30[reset]",
            ),
            (
                "2022-09-09 11:48:34,534",
                "2022-09-09[blue] [reset][red]11[reset][yellow]:[reset][red]48[reset][yellow]:[reset][red]34[reset][yellow],[reset][red]534[reset]",
            ),
            (
                "2022-09-22T07:46:34.171800155Z",
                "2022-09-22[blue]T[reset][red]07[reset][yellow]:[reset][red]46[reset][yellow]:[reset][red]34[reset][yellow].[reset][red]171800155[reset][blue]Z[reset]",
            ),
            (
                "2024-09-14T07:57:30.659+02:00",
                "2024-09-14[blue]T[reset][red]07[reset][yellow]:[reset][red]57[reset][yellow]:[reset][red]30[reset][yellow].[reset][red]659[reset]+02:00",
            ),
            ("No time here!", "No time here!"),
            ("2001:db8::ff00:42:8329", "2001:db8::ff00:42:8329"),
        ];

        for (input, expected) in cases {
            let actual = highlighter.apply(input);
            assert_eq!(expected, actual.to_string().convert_escape_codes());
        }
    }
}
