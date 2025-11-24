pub fn normalize_string(input: &str) -> String {
    let newline_normalized = normalize_raw_string_newlines(input);
    remove_trailing_commas(&newline_normalized)
}

pub fn remove_trailing_commas(s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i + 1 < chars.len() {
        if chars[i] == ',' {
            let mut j = i + 1;
            // Skip whitespace after the comma
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            // If next significant char is a closing delimiter → remove comma (+ whitespace)
            if j < chars.len() && matches!(chars[j], ')' | ']' | '}') {
                chars.drain(i..j);
                continue; // re-process index i since characters shifted
            }
        }
        i += 1;
    }

    chars.into_iter().collect()
}

fn normalize_raw_string_newlines(input: &str) -> String {
    #[derive(Clone, Copy, Debug)]
    enum State {
        Outside,
        Normal { escaped: bool }, // inside "..." ; escaped=true if previous char was '\'
        Raw { hashes: usize },    // inside r###"..."###
    }

    let mut out = String::with_capacity(input.len());
    let mut it = input.chars().peekable();
    let mut state = State::Outside;

    while let Some(ch) = it.next() {
        match state {
            State::Outside => {
                // Try raw string start: r[#*]"
                if ch == 'r' {
                    let mut la = it.clone();
                    let mut hashes = 0usize;
                    while matches!(la.peek(), Some('#')) {
                        la.next();
                        hashes += 1;
                    }
                    if matches!(la.peek(), Some('"')) {
                        // Consume what we looked ahead: the hashes and the opening quote.
                        out.push('r');
                        for _ in 0..hashes {
                            it.next();
                            out.push('#');
                        }
                        it.next(); // consume the opening '"'
                        out.push('"');
                        state = State::Raw { hashes };
                        continue;
                    }
                }
                // Try normal string start: "
                if ch == '"' {
                    out.push('"');
                    state = State::Normal { escaped: false };
                    continue;
                }

                out.push(ch);
            }

            State::Normal { ref mut escaped } => {
                if *escaped {
                    // Previous char was a backslash; current char is escaped.
                    if ch == '\n' {
                        out.push_str("\\n");
                    } else {
                        out.push(ch);
                    }
                    *escaped = false;
                } else {
                    match ch {
                        '\\' => {
                            out.push('\\');
                            *escaped = true;
                        }
                        '"' => {
                            out.push('"');
                            state = State::Outside;
                        }
                        '\n' => out.push_str("\\n"),
                        _ => out.push(ch),
                    }
                }
            }

            State::Raw { hashes } => {
                if ch == '"' {
                    // Possible end of raw string: must be followed by exactly `hashes` '#'s.
                    let mut la = it.clone();
                    let mut seen = 0usize;
                    while seen < hashes && matches!(la.peek(), Some('#')) {
                        la.next();
                        seen += 1;
                    }
                    if seen == hashes {
                        // Close it: emit '"' and those '#'s, consume them, and leave raw mode.
                        out.push('"');
                        for _ in 0..hashes {
                            it.next();
                            out.push('#');
                        }
                        state = State::Outside;
                        continue;
                    } else {
                        out.push('"'); // just a quote inside the raw body
                    }
                } else if ch == '\n' {
                    out.push_str("\\n");
                } else {
                    out.push(ch);
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        let input = r##"

let s = r#"line1
line2"#;

let b = "
";
"##;
        let expected = r##"

let s = r#"line1\nline2"#;

let b = "\n";
"##;
        assert_eq!(normalize_raw_string_newlines(input), expected);
    }

    #[test]
    fn test_multiple_hashes() {
        let input = r####"let s = r###"hello
world"###;

"####;
        let expected = r####"let s = r###"hello\nworld"###;

"####;
        assert_eq!(normalize_raw_string_newlines(input), expected);
    }
}
