//! Bitmap fonts (docs/INTERFACE.md, stage 1).
//!
//! A font is two things in the cache: a sprite sheet of 256 frames, one for
//! each character, and a small file of measurements saying how far to move
//! on after each character and where the baseline is.
//!
//! Untested against real data until the dump tool's pictures have been
//! looked at.

use crate::error::{Error, Result};
use crate::sprite::SpriteSheet;

/// The cache index font measurements are kept in. The letters themselves are
/// the sprite with the same archive id.
pub const FONT_INDEX: u32 = 13;

/// The size of the plain form of the measurements: 256 advances and the
/// height of the baseline.
const PLAIN: usize = 257;

/// One character of a font.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Glyph {
    /// Where its picture starts, from the pen and from the top of the line.
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub height: usize,
    /// How far the pen moves on after it.
    pub advance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    pub id: u32,
    pub glyphs: Vec<Glyph>,
    /// From the top of a line down to the baseline.
    pub ascent: i32,
    /// The furthest any letter reaches above and below the baseline.
    pub max_ascent: i32,
    pub max_descent: i32,
    /// Anything about the measurements the decoder could not read.
    pub note: Option<String>,
}

/// A character placed on a line: which glyph, and where its picture's top
/// left corner goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub glyph: usize,
    pub x: i32,
    pub y: i32,
}

impl Font {
    /// Put the measurements together with the letters' sheet.
    pub fn decode(id: u32, measurements: &[u8], letters: &SpriteSheet) -> Result<Self> {
        if measurements.len() < 256 {
            return Err(Error::parse(format!(
                "font {id}: {} bytes of measurements, fewer than one advance for each character",
                measurements.len()
            )));
        }
        let mut glyphs: Vec<Glyph> = (0..256)
            .map(|i| {
                let advance = measurements[i] as i32;
                match letters.frames.get(i) {
                    Some(frame) => Glyph {
                        left: frame.offset_x as i32,
                        top: frame.offset_y as i32,
                        width: frame.width,
                        height: frame.height,
                        advance,
                    },
                    None => Glyph {
                        advance,
                        ..Glyph::default()
                    },
                }
            })
            .collect();
        glyphs.truncate(256);

        let drawn = || glyphs.iter().filter(|g| g.height != 0);
        let highest = drawn().map(|g| g.top).min().unwrap_or(0);
        let lowest = drawn().map(|g| g.top + g.height as i32).max().unwrap_or(0);

        let mut note = None;
        let ascent = if measurements.len() == PLAIN {
            measurements[256] as i32
        } else {
            // A longer form exists that also holds how letters tuck together.
            // It is not read: the advances are taken, and the baseline is
            // guessed as the foot of a capital H.
            note = Some(format!(
                "{} bytes of measurements, not {PLAIN}: only the advances were read and the baseline is a guess",
                measurements.len()
            ));
            let h = glyphs[b'H' as usize];
            h.top + h.height as i32
        };
        if letters.frames.len() != 256 {
            let said = format!("{} letters in its sheet, not 256", letters.frames.len());
            note = Some(match note {
                Some(other) => format!("{other}; {said}"),
                None => said,
            });
        }

        Ok(Self {
            id,
            glyphs,
            ascent,
            max_ascent: ascent - highest,
            max_descent: lowest - ascent,
            note,
        })
    }

    /// The width of a line of text.
    pub fn width(&self, text: &str) -> i32 {
        text.chars()
            .map(|c| self.glyphs[code_of(c) as usize].advance)
            .sum()
    }

    /// Where each character's picture goes for a line of text whose pen
    /// starts at `x` on a baseline at `y`.
    pub fn place(&self, text: &str, x: i32, y: i32) -> Vec<Placed> {
        let top = y - self.ascent;
        let mut pen = x;
        let mut placed = Vec::new();
        for c in text.chars() {
            let code = code_of(c) as usize;
            let glyph = self.glyphs[code];
            if glyph.width != 0 && glyph.height != 0 {
                placed.push(Placed {
                    glyph: code,
                    x: pen + glyph.left,
                    y: top + glyph.top,
                });
            }
            pen += glyph.advance;
        }
        placed
    }
}

/// Windows-1252's characters from 0x80 to 0x9F, the only stretch where it
/// and Unicode part company.
const HIGH: [char; 32] = [
    '\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}',
];

/// The character a number stands for in the fonts: the other way round from
/// [`code_of`].
pub fn char_of(code: u8) -> char {
    match code {
        0x80..=0x9F => HIGH[(code - 0x80) as usize],
        other => other as char,
    }
}

/// The number a character has in the fonts: its place in Windows-1252, the
/// character set the game's text is kept in. Anything outside it is a
/// question mark.
pub fn code_of(c: char) -> u8 {
    let n = c as u32;
    if n < 0x80 || (0xA0..=0xFF).contains(&n) {
        return n as u8;
    }
    match HIGH.iter().position(|&high| high == c) {
        Some(at) => 0x80 + at as u8,
        None => b'?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::SpriteFrame;

    /// A font whose every letter is a box `code % 5 + 1` wide and 8 tall,
    /// two down from the top of the line, one in from the pen.
    fn letters() -> SpriteSheet {
        let frames = (0..256usize)
            .map(|code| {
                let (w, h) = if code == 32 {
                    (0, 0)
                } else {
                    (code % 5 + 1, 8)
                };
                SpriteFrame {
                    offset_x: 1,
                    offset_y: if code == b'g' as usize { 5 } else { 2 },
                    width: w,
                    height: h,
                    indices: vec![1; w * h],
                    alpha: vec![255; w * h],
                    by_column: false,
                    has_alpha: false,
                }
            })
            .collect();
        SpriteSheet {
            id: 1,
            width: 16,
            height: 16,
            palette: vec![0, 0xFFFFFF],
            frames,
            unread: 0,
        }
    }

    fn measurements() -> Vec<u8> {
        let mut bytes: Vec<u8> = (0..256usize).map(|code| (code % 5 + 3) as u8).collect();
        bytes.push(10); // the baseline, ten down
        bytes
    }

    #[test]
    fn advances_and_the_baseline_come_from_the_measurements() {
        let font = Font::decode(1, &measurements(), &letters()).unwrap();
        assert_eq!(font.ascent, 10);
        assert_eq!(font.glyphs[b'A' as usize].advance, (65 % 5 + 3) as i32);
        assert_eq!(font.glyphs[b'A' as usize].width, 65 % 5 + 1);
        assert_eq!(font.note, None);
        // letters start 2 down and are 8 tall: they stand on the baseline;
        // the g starts 5 down and hangs 3 below it
        assert_eq!(font.max_ascent, 8);
        assert_eq!(font.max_descent, 3);
    }

    #[test]
    fn a_line_is_as_wide_as_its_advances_and_each_letter_sits_where_its_offsets_say() {
        let font = Font::decode(1, &measurements(), &letters()).unwrap();
        let advance = |c: u8| (c as i32) % 5 + 3;
        assert_eq!(
            font.width("Hi g"),
            advance(b'H') + advance(b'i') + advance(b' ') + advance(b'g')
        );
        let placed = font.place("Hi g", 100, 50);
        // the space has no picture
        assert_eq!(placed.len(), 3);
        // top of the line is the baseline less the ascent: 40
        assert_eq!(
            placed[0],
            Placed {
                glyph: b'H' as usize,
                x: 101,
                y: 42
            }
        );
        assert_eq!(placed[1].x, 100 + advance(b'H') + 1);
        assert_eq!(
            placed[2],
            Placed {
                glyph: b'g' as usize,
                x: 100 + advance(b'H') + advance(b'i') + advance(b' ') + 1,
                y: 45
            }
        );
    }

    #[test]
    fn measurements_of_another_length_are_said_to_be_so() {
        let mut long = measurements();
        long.extend([0u8; 40]);
        let font = Font::decode(1, &long, &letters()).unwrap();
        assert!(font.note.as_deref().unwrap().contains("297 bytes"));
        assert_eq!(font.ascent, 10, "guessed from the foot of the H: 2 + 8");
        assert!(Font::decode(1, &[0u8; 100], &letters()).is_err());
    }

    #[test]
    fn a_sheet_without_all_its_letters_is_said_to_be_so() {
        let mut few = letters();
        few.frames.truncate(100);
        let font = Font::decode(1, &measurements(), &few).unwrap();
        assert!(font.note.as_deref().unwrap().contains("100 letters"));
        assert_eq!(font.glyphs.len(), 256);
        assert_eq!(font.glyphs[200].width, 0);
    }

    #[test]
    fn characters_are_numbered_as_the_game_numbers_them() {
        assert_eq!(code_of('A'), 65);
        assert_eq!(code_of('\u{E9}'), 0xE9); // e acute
        assert_eq!(code_of('\u{20AC}'), 0x80); // euro
        assert_eq!(code_of('\u{2019}'), 0x92); // curly apostrophe
        assert_eq!(code_of('\u{4E2D}'), b'?');
    }

    #[test]
    fn every_number_comes_back_from_its_character() {
        for code in 0..=255u8 {
            assert_eq!(code_of(char_of(code)), code, "{code}");
        }
        assert_eq!(char_of(0x80), '\u{20AC}');
        assert_eq!(char_of(b'A'), 'A');
    }
}
