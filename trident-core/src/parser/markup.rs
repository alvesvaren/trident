//! Inline markdown-style text styling for node names, labels and class members.
//!
//! - `*text*` or `_text_`  => italic
//! - `__text__`            => underline
//! - `___text___`          => italic + underline (or nest, e.g. `__*text*__`)
//! - `\*`, `\_`, `\\`      => literal character
//!
//! Like CommonMark, underscores inside a word are not delimiters, so `snake_case_name`
//! renders as-is. Unmatched delimiters are kept as literal text.

use serde::Serialize;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TextStyle {
    pub italic: bool,
    pub underline: bool,
}

impl TextStyle {
    pub fn union(self, other: TextStyle) -> TextStyle {
        TextStyle {
            italic: self.italic || other.italic,
            underline: self.underline || other.underline,
        }
    }
}

/// A run of text with a single style.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StyledSpan {
    pub text: String,
    #[serde(flatten)]
    pub style: TextStyle,
}

/// Concatenate the text of spans, dropping styling.
pub fn plain_text(spans: &[StyledSpan]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

enum Piece {
    Text(String),
    Delim {
        raw: String,
        style: TextStyle,
        can_open: bool,
        can_close: bool,
    },
}

/// Style a delimiter run represents, or None if it isn't a supported delimiter.
fn delim_style(ch: char, len: usize) -> Option<TextStyle> {
    let italic = TextStyle { italic: true, underline: false };
    let underline = TextStyle { italic: false, underline: true };
    match (ch, len) {
        ('*', 1) | ('_', 1) => Some(italic),
        ('_', 2) => Some(underline),
        ('_', 3) => Some(italic.union(underline)),
        _ => None,
    }
}

/// Parse inline markup into styled spans. Adjacent spans with equal style are merged.
pub fn parse_inline(input: &str) -> Vec<StyledSpan> {
    let chars: Vec<char> = input.chars().collect();
    let mut pieces: Vec<Piece> = Vec::new();
    let mut text = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\\' && matches!(chars.get(i + 1), Some('*' | '_' | '\\')) {
            text.push(chars[i + 1]);
            i += 2;
            continue;
        }

        if c != '*' && c != '_' {
            text.push(c);
            i += 1;
            continue;
        }

        let start = i;
        while chars.get(i) == Some(&c) {
            i += 1;
        }
        let raw: String = chars[start..i].iter().collect();
        let Some(style) = delim_style(c, i - start) else {
            text.push_str(&raw);
            continue;
        };

        let prev = start.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i).copied();
        let mut can_open = next.is_some_and(|n| !n.is_whitespace());
        let mut can_close = prev.is_some_and(|p| !p.is_whitespace());
        if c == '_' {
            can_open &= !prev.is_some_and(char::is_alphanumeric);
            can_close &= !next.is_some_and(char::is_alphanumeric);
        }

        if !text.is_empty() {
            pieces.push(Piece::Text(std::mem::take(&mut text)));
        }
        pieces.push(Piece::Delim { raw, style, can_open, can_close });
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }

    // Match closers with the nearest identical opener; openers skipped over become literal.
    let mut openers: Vec<usize> = Vec::new();
    let mut pairs: Vec<(usize, usize, TextStyle)> = Vec::new();
    for (idx, piece) in pieces.iter().enumerate() {
        let Piece::Delim { raw, style, can_open, can_close } = piece else {
            continue;
        };
        if *can_close {
            let matching = openers.iter().rposition(|&o| {
                matches!(&pieces[o], Piece::Delim { raw: open_raw, .. } if open_raw == raw)
            });
            if let Some(pos) = matching {
                pairs.push((openers[pos], idx, *style));
                openers.truncate(pos);
                continue;
            }
        }
        if *can_open {
            openers.push(idx);
        }
    }

    let mut spans: Vec<StyledSpan> = Vec::new();
    for (idx, piece) in pieces.iter().enumerate() {
        let text = match piece {
            Piece::Text(t) => t,
            Piece::Delim { raw, .. } => {
                if pairs.iter().any(|&(o, c, _)| o == idx || c == idx) {
                    continue;
                }
                raw
            }
        };
        let style = pairs
            .iter()
            .filter(|&&(o, c, _)| o < idx && idx < c)
            .fold(TextStyle::default(), |acc, &(_, _, s)| acc.union(s));
        match spans.last_mut() {
            Some(last) if last.style == style => last.text.push_str(text),
            _ => spans.push(StyledSpan { text: text.clone(), style }),
        }
    }
    spans
}

/// Markup wrapped around a node name in its declaration, e.g. `__*Foo*__`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct NameMarkup {
    /// Delimiters before the identifier (e.g. `__*`)
    pub open: String,
    /// Delimiters after the identifier (e.g. `*__`)
    pub close: String,
    pub style: TextStyle,
}

/// Split a node name token like `*Foo*` into its identifier and markup.
/// Returns None when the token doesn't render as a single identifier.
pub fn parse_name(token: &str) -> Option<(String, NameMarkup)> {
    let spans = parse_inline(token);
    let [span] = spans.as_slice() else {
        return None;
    };
    let open_len = token.find(span.text.as_str())?;
    let close_start = open_len + span.text.len();
    Some((
        span.text.clone(),
        NameMarkup {
            open: token[..open_len].to_string(),
            close: token[close_start..].to_string(),
            style: span.style,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(text: &str, italic: bool, underline: bool) -> StyledSpan {
        StyledSpan { text: text.into(), style: TextStyle { italic, underline } }
    }

    #[test]
    fn plain_text_is_one_span() {
        assert_eq!(parse_inline("+ count: int"), vec![span("+ count: int", false, false)]);
    }

    #[test]
    fn italic_and_underline() {
        assert_eq!(
            parse_inline("+ *area*(): __int__"),
            vec![
                span("+ ", false, false),
                span("area", true, false),
                span("(): ", false, false),
                span("int", false, true),
            ]
        );
        assert_eq!(parse_inline("_whole line_"), vec![span("whole line", true, false)]);
        assert_eq!(parse_inline("___both___"), vec![span("both", true, true)]);
        assert_eq!(parse_inline("__*both*__"), vec![span("both", true, true)]);
    }

    #[test]
    fn snake_case_and_pointers_stay_literal() {
        assert_eq!(parse_inline("my_snake_case"), vec![span("my_snake_case", false, false)]);
        assert_eq!(parse_inline("+ name: char*"), vec![span("+ name: char*", false, false)]);
        assert_eq!(parse_inline("a * b * c"), vec![span("a * b * c", false, false)]);
        assert_eq!(parse_inline("+ _private"), vec![span("+ _private", false, false)]);
    }

    #[test]
    fn escapes() {
        assert_eq!(parse_inline(r"\*not italic\*"), vec![span("*not italic*", false, false)]);
        assert_eq!(parse_inline(r"\_\_init\_\_"), vec![span("__init__", false, false)]);
    }

    #[test]
    fn names() {
        let (id, m) = parse_name("*Foo*").unwrap();
        assert_eq!(id, "Foo");
        assert_eq!((m.open.as_str(), m.close.as_str()), ("*", "*"));
        assert_eq!(m.style, TextStyle { italic: true, underline: false });

        let (id, m) = parse_name("__my_class__").unwrap();
        assert_eq!(id, "my_class");
        assert_eq!(m.style, TextStyle { italic: false, underline: true });

        let (id, m) = parse_name("Plain").unwrap();
        assert_eq!(id, "Plain");
        assert_eq!(m, NameMarkup::default());

        assert!(parse_name("*Foo").is_some_and(|(id, _)| id == "*Foo"));
    }
}
