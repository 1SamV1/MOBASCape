//! Breaking a line of text into lines that fit a width, at a font's own
//! size. It is pure and has no idea where the text will be drawn.

use crate::font::Font;

/// Greedy word wrap. Words are kept whole where they fit; a word longer than
/// the width is broken between characters. A trailing space is never kept,
/// and empty text is one empty line.
pub fn wrap(font: &Font, text: &str, width: i32) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if width <= 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            take_word(font, word, width, &mut line, &mut lines);
            continue;
        }
        let mut candidate = line.clone();
        candidate.push(' ');
        candidate.push_str(word);
        if font.width(&candidate) <= width {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            take_word(font, word, width, &mut line, &mut lines);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Put one word on the current line, breaking it between characters if it
/// cannot fit whole.
fn take_word(font: &Font, word: &str, width: i32, line: &mut String, lines: &mut Vec<String>) {
    if font.width(word) <= width {
        *line = word.to_string();
        return;
    }
    for c in word.chars() {
        let mut candidate = line.clone();
        candidate.push(c);
        if !line.is_empty() && font.width(&candidate) > width {
            lines.push(std::mem::take(line));
        }
        line.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::Glyph;

    fn font() -> Font {
        let glyph = Glyph {
            advance: 5,
            ..Glyph::default()
        };
        Font::new((32u8..127).map(|c| (c as char, glyph)).collect(), 8, 10)
    }

    #[test]
    fn words_wrap_at_the_last_space_that_fits() {
        let lines = wrap(&font(), "aaa bbb ccc", 35);
        assert_eq!(lines, ["aaa bbb", "ccc"]);
        let lines = wrap(&font(), "aaa bbb ccc", 20);
        assert_eq!(lines, ["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn a_word_longer_than_the_width_is_broken_between_characters() {
        let lines = wrap(&font(), "abcdefgh", 20);
        assert_eq!(lines, ["abcd", "efgh"]);
    }

    #[test]
    fn an_empty_line_is_one_empty_string() {
        assert_eq!(wrap(&font(), "", 100), [""]);
        assert_eq!(wrap(&font(), "   ", 100), [""]);
    }

    #[test]
    fn text_exactly_the_width_stays_on_one_line() {
        assert_eq!(wrap(&font(), "aaa bbb", 35), ["aaa bbb"]);
    }

    #[test]
    fn a_width_of_zero_leaves_the_text_alone() {
        assert_eq!(wrap(&font(), "aaa bbb", 0), ["aaa bbb"]);
    }
}
