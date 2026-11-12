use crate::core::config::IpV4Config;
use crate::core::highlighter::Highlight;
use nu_ansi_term::Style as NuStyle;
use regex::{Captures, Error, Regex, RegexBuilder};
use std::borrow::Cow;

const OCTET_NAMES: [&str; 4] = ["o1", "o2", "o3", "o4"];

pub struct IpV4Highlighter {
    regex: Regex,
    octet_style: NuStyle,
    dot_style: NuStyle,
}

impl IpV4Highlighter {
    pub fn new(config: IpV4Config) -> Result<Self, Error> {
        let pattern = r"(?x)\b
            (?P<o1>\d{1,3})\.
            (?P<o2>\d{1,3})\.
            (?P<o3>\d{1,3})\.
            (?P<o4>\d{1,3})
            (?:/(?P<mask>\d{1,2}))?
            \b";
        let regex = RegexBuilder::new(pattern).unicode(false).build()?;

        Ok(Self {
            regex,
            octet_style: config.number.into(),
            dot_style: config.separator.into(),
        })
    }

    fn validate_octets(caps: &Captures<'_>) -> bool {
        OCTET_NAMES.iter().all(|name| {
            caps.name(name)
                .expect("named octet group always present")
                .as_str()
                .parse::<u8>()
                .is_ok()
        })
    }

    fn validate_mask(caps: &Captures<'_>) -> bool {
        match caps.name("mask").map(|m| m.as_str()) {
            Some(mask_text) => mask_text.parse::<u8>().map(|v| v <= 32).unwrap_or(false),
            None => true,
        }
    }

    fn format_address(&self, caps: &Captures<'_>) -> String {
        let mut result = String::new();

        for (idx, &name) in OCTET_NAMES.iter().enumerate() {
            let text = caps.name(name).expect("named octet group always present").as_str();
            result.push_str(&self.octet_style.paint(text).to_string());
            if idx < 3 {
                result.push_str(&self.dot_style.paint(".").to_string());
            }
        }

        if let Some(mask_match) = caps.name("mask") {
            result.push_str(&self.dot_style.paint("/").to_string());
            result.push_str(&self.octet_style.paint(mask_match.as_str()).to_string());
        }

        result
    }
}

impl Highlight for IpV4Highlighter {
    fn apply<'a>(&self, input: &'a str) -> Cow<'a, str> {
        if !input.as_bytes().contains(&b'.') {
            return Cow::Borrowed(input);
        }

        self.regex.replace_all(input, |caps: &Captures<'_>| {
            let raw = caps.get(0).expect("full match always present").as_str();

            if !Self::validate_octets(caps) || !Self::validate_mask(caps) {
                return raw.to_string();
            }

            self.format_address(caps)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::escape_code_converter::ConvertEscapeCodes;
    use crate::style::{Color, Style};

    #[test]
    fn test_ipv4_highlighter_valid() {
        let h = IpV4Highlighter::new(IpV4Config {
            number: Style::new().fg(Color::Blue),
            separator: Style::new().fg(Color::Red),
        })
        .unwrap();

        let cases = vec![
            (
                "10.0.0.123",
                "[blue]10[reset][red].[reset][blue]0[reset][red].[reset][blue]0[reset][red].[reset][blue]123[reset]",
            ),
            (
                "192.168.0.1/24",
                "[blue]192[reset][red].[reset][blue]168[reset][red].[reset][blue]0[reset][red].[reset][blue]1[reset][red]/[reset][blue]24[reset]",
            ),
            (
                "0.0.0.0",
                "[blue]0[reset][red].[reset][blue]0[reset][red].[reset][blue]0[reset][red].[reset][blue]0[reset]",
            ),
        ];

        for (input, expect) in cases {
            let actual = h.apply(input);
            assert_eq!(expect, actual.to_string().convert_escape_codes());
        }
    }

    #[test]
    fn test_ipv4_highlighter_invalid_octet_or_mask() {
        let h = IpV4Highlighter::new(IpV4Config {
            number: Style::new().fg(Color::Blue),
            separator: Style::new().fg(Color::Red),
        })
        .unwrap();

        let cases = vec![
            ("256.1.1.1", "256.1.1.1"),
            ("999.999.999.999", "999.999.999.999"),
            ("192.168.0.1/33", "192.168.0.1/33"),
            ("1.2.3", "1.2.3"),
        ];

        for (input, expect) in cases {
            let actual = h.apply(input);
            assert_eq!(expect, actual);
        }
    }
}
