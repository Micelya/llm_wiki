//! Small helpers that keep the text handling of the scripts of N (Python)
//! where it matters for the hints and the question: what is a space, how
//! lines are split and how a number is rounded.

pub use super::super::catalog::fold;

/// A space as Python's `str.isspace` sees it.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python's `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(is_space)
}

/// Python's `str.splitlines()`: no line for a final break.
pub fn splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let breaks = matches!(c, '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}');
        if !breaks {
            continue;
        }
        out.push(&s[start..i]);
        let mut end = i + c.len_utf8();
        if c == '\r' {
            if let Some((j, '\n')) = chars.peek().copied() {
                chars.next();
                end = j + 1;
            }
        }
        start = end;
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Python's `round(x)`: to the nearest integer, a half to the even one.
pub fn round(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// The last `n` characters of a text.
pub fn tail(s: &str, n: usize) -> String {
    let count = s.chars().count();
    s.chars().skip(count.saturating_sub(n)).collect()
}

/// The first `n` characters of a text.
pub fn head(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_spaces_and_rounding_as_python() {
        assert_eq!(splitlines("a\r\nb\rc\n"), vec!["a", "b", "c"]);
        assert_eq!(splitlines(""), Vec::<&str>::new());
        assert_eq!(splitlines("a\n\nb"), vec!["a", "", "b"]);
        assert_eq!(strip("\u{1f} hola \n"), "hola");
        assert_eq!((round(2.5), round(3.5), round(612.4)), (2, 4, 612));
        assert_eq!((tail("abcdef", 2), head("abcdef", 2)), ("ef".to_string(), "ab".to_string()));
    }
}
