//! JSON string escaping shared by transactional artifact encoders.

pub fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value <= '\u{1f}' => {
                use std::fmt::Write as _;
                write!(&mut escaped, "\\u{:04x}", value as u32).expect("String writes cannot fail");
            }
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes_quotes_newlines_and_control_characters() {
        assert_eq!(
            super::json_string("quoted \"line\"\nslash\\tab\t\u{1}"),
            r#""quoted \"line\"\nslash\\tab\t\u0001""#
        );
    }
}
