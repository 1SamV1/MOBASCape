//! A bitmap font: a picture for each character and how far to move on.

use std::collections::HashMap;

use crate::atlas::Region;

/// One character.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Glyph {
    /// Its picture in the atlas; none for a space.
    pub region: Option<Region>,
    /// Where the picture starts, from the pen and from the top of the line.
    pub left: i32,
    pub top: i32,
    /// How far the pen moves on after it.
    pub advance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    glyphs: HashMap<char, Glyph>,
    /// From the top of a line down to the baseline.
    pub ascent: i32,
    /// From one baseline to the next.
    pub line_height: i32,
}

impl Font {
    /// `glyphs` pairs each character the font has with its picture and
    /// measurements. Which characters those are is the theme's business.
    pub fn new(glyphs: Vec<(char, Glyph)>, ascent: i32, line_height: i32) -> Self {
        Self {
            glyphs: glyphs.into_iter().collect(),
            ascent,
            line_height,
        }
    }

    /// A character's glyph; a question mark's for one the font lacks, and
    /// nothing at all if it lacks that too.
    pub fn glyph(&self, c: char) -> Glyph {
        self.glyphs
            .get(&c)
            .or_else(|| self.glyphs.get(&'?'))
            .copied()
            .unwrap_or_default()
    }

    /// The width of a line of text, at the font's own size.
    pub fn width(&self, text: &str) -> i32 {
        text.chars().map(|c| self.glyph(c).advance).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_character_is_a_question_mark() {
        let q = Glyph {
            advance: 7,
            ..Glyph::default()
        };
        let a = Glyph {
            advance: 5,
            ..Glyph::default()
        };
        let font = Font::new(vec![('?', q), ('a', a)], 10, 12);
        assert_eq!(font.glyph('a').advance, 5);
        assert_eq!(font.glyph('\u{4E2D}').advance, 7);
        assert_eq!(font.width("a\u{4E2D}a"), 17);
        let bare = Font::new(vec![], 10, 12);
        assert_eq!(bare.width("abc"), 0);
    }
}
