//! Sprites: the panels, buttons, icons and letters of the interface
//! (docs/INTERFACE.md, stage 1).
//!
//! One sprite archive is a sheet of one or more frames that share a palette.
//! The file is read from both ends: the pixels of every frame come first,
//! then the palette, then a table of each frame's place and size, and the
//! last two bytes say how many frames there are.
//!
//! Until the dump tool has been run on the real cache and its pictures looked
//! at, this decoder is untested against real data, whatever its unit tests
//! say: they only show it reads the format its author believes in.

use crate::buffer::Buffer;
use crate::error::{Error, Result};

/// The cache index sprites are kept in.
pub const SPRITE_INDEX: u32 = 8;

/// A frame's pixels are stored a column at a time instead of a row at a time.
const BY_COLUMN: u8 = 1;
/// A frame carries an opacity for each pixel as well as a colour.
const HAS_ALPHA: u8 = 2;

/// One picture of a sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteFrame {
    /// Where the picture sits inside the sheet's full size.
    pub offset_x: usize,
    pub offset_y: usize,
    pub width: usize,
    pub height: usize,
    /// One palette entry for each pixel, row by row. 0 is "nothing here".
    pub indices: Vec<u8>,
    /// One opacity for each pixel, row by row: 0 clear, 255 solid.
    pub alpha: Vec<u8>,
    /// Whether the file stored this frame a column at a time.
    pub by_column: bool,
    /// Whether the file gave opacities, or they were made from the indices.
    pub has_alpha: bool,
}

/// A decoded sprite archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheet {
    pub id: u32,
    /// The full size every frame is placed within.
    pub width: usize,
    pub height: usize,
    /// Colours as 0xRRGGBB. Entry 0 is never drawn.
    pub palette: Vec<u32>,
    pub frames: Vec<SpriteFrame>,
    /// Bytes between the end of the last frame's pixels and the start of the
    /// palette. A right reading leaves none; anything else means the frames
    /// were not where or what the decoder thought.
    pub unread: i64,
}

impl SpriteFrame {
    /// The frame as red, green, blue, opacity for each pixel, row by row.
    pub fn rgba(&self, palette: &[u32]) -> Vec<[u8; 4]> {
        self.indices
            .iter()
            .zip(&self.alpha)
            .map(|(&index, &alpha)| {
                let colour = palette.get(index as usize).copied().unwrap_or(0xFF00FF);
                [
                    (colour >> 16) as u8,
                    (colour >> 8) as u8,
                    colour as u8,
                    if index == 0 { 0 } else { alpha },
                ]
            })
            .collect()
    }

    /// How many of its pixels are fully clear.
    pub fn clear_pixels(&self) -> usize {
        self.indices
            .iter()
            .zip(&self.alpha)
            .filter(|&(&index, &alpha)| index == 0 || alpha == 0)
            .count()
    }
}

impl SpriteSheet {
    pub fn decode(id: u32, data: &[u8]) -> Result<Self> {
        let bad = |what: &str| Error::parse(format!("sprite {id}: {what}"));
        if data.len() < 2 {
            return Err(bad("too short to hold a frame count"));
        }
        let mut buf = Buffer::new(data);
        buf.set_pos(data.len() - 2);
        let count = buf.read_u16()? as usize;

        // the table at the end: full size, palette length, then four lists
        let table = data
            .len()
            .checked_sub(7 + count * 8)
            .ok_or_else(|| bad("too short for its table of frames"))?;
        buf.set_pos(table);
        let width = buf.read_u16()? as usize;
        let height = buf.read_u16()? as usize;
        let palette_len = buf.read_u8()? as usize + 1;

        let list = |buf: &mut Buffer| -> Result<Vec<usize>> {
            (0..count).map(|_| Ok(buf.read_u16()? as usize)).collect()
        };
        let offsets_x = list(&mut buf)?;
        let offsets_y = list(&mut buf)?;
        let widths = list(&mut buf)?;
        let heights = list(&mut buf)?;

        // the palette, just before the table; entry 0 is not stored
        let palette_at = table
            .checked_sub((palette_len - 1) * 3)
            .ok_or_else(|| bad("too short for its palette"))?;
        buf.set_pos(palette_at);
        let mut palette = vec![0u32; palette_len];
        for entry in palette.iter_mut().skip(1) {
            *entry = buf.read_u24()?;
            // pure black is kept apart from "nothing here"
            if *entry == 0 {
                *entry = 1;
            }
        }

        // the pixels, from the start
        buf.set_pos(0);
        let mut frames = Vec::with_capacity(count);
        for i in 0..count {
            let (w, h) = (widths[i], heights[i]);
            let size = w * h;
            let flags = buf.read_u8()?;
            let by_column = flags & BY_COLUMN != 0;
            let has_alpha = flags & HAS_ALPHA != 0;
            let plane = |buf: &mut Buffer| -> Result<Vec<u8>> {
                if buf.pos() + size > palette_at {
                    return Err(bad("a frame's pixels run into the palette"));
                }
                let stored = buf.read_bytes(size)?;
                if !by_column {
                    return Ok(stored.to_vec());
                }
                let mut rows = vec![0u8; size];
                for x in 0..w {
                    for y in 0..h {
                        rows[y * w + x] = stored[x * h + y];
                    }
                }
                Ok(rows)
            };
            let indices = plane(&mut buf)?;
            let alpha = if has_alpha {
                plane(&mut buf)?
            } else {
                indices
                    .iter()
                    .map(|&index| if index == 0 { 0 } else { 255 })
                    .collect()
            };
            frames.push(SpriteFrame {
                offset_x: offsets_x[i],
                offset_y: offsets_y[i],
                width: w,
                height: h,
                indices,
                alpha,
                by_column,
                has_alpha,
            });
        }

        Ok(Self {
            id,
            width,
            height,
            palette,
            frames,
            unread: palette_at as i64 - buf.pos() as i64,
        })
    }
}

/// The hash the cache keeps in place of an archive's name.
pub fn name_hash(name: &str) -> i32 {
    name.to_lowercase().bytes().fold(0i32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as i32)
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Write a sheet the way the format is believed to be laid out.
    pub(crate) fn encode(sheet: &SpriteSheet) -> Vec<u8> {
        let mut out = Vec::new();
        for frame in &sheet.frames {
            let mut flags = 0;
            if frame.by_column {
                flags |= BY_COLUMN;
            }
            if frame.has_alpha {
                flags |= HAS_ALPHA;
            }
            out.push(flags);
            let mut plane = |values: &[u8]| {
                if frame.by_column {
                    for x in 0..frame.width {
                        for y in 0..frame.height {
                            out.push(values[y * frame.width + x]);
                        }
                    }
                } else {
                    out.extend_from_slice(values);
                }
            };
            plane(&frame.indices);
            if frame.has_alpha {
                plane(&frame.alpha);
            }
        }
        for &colour in sheet.palette.iter().skip(1) {
            out.extend_from_slice(&colour.to_be_bytes()[1..]);
        }
        out.extend_from_slice(&(sheet.width as u16).to_be_bytes());
        out.extend_from_slice(&(sheet.height as u16).to_be_bytes());
        out.push((sheet.palette.len() - 1) as u8);
        for pick in [
            |f: &SpriteFrame| f.offset_x,
            |f: &SpriteFrame| f.offset_y,
            |f: &SpriteFrame| f.width,
            |f: &SpriteFrame| f.height,
        ] {
            for frame in &sheet.frames {
                out.extend_from_slice(&(pick(frame) as u16).to_be_bytes());
            }
        }
        out.extend_from_slice(&(sheet.frames.len() as u16).to_be_bytes());
        out
    }

    fn frame(w: usize, h: usize, by_column: bool, has_alpha: bool) -> SpriteFrame {
        let indices: Vec<u8> = (0..w * h).map(|i| (i % 4) as u8).collect();
        let alpha = if has_alpha {
            (0..w * h).map(|i| (i * 37 % 256) as u8).collect()
        } else {
            indices
                .iter()
                .map(|&i| if i == 0 { 0 } else { 255 })
                .collect()
        };
        SpriteFrame {
            offset_x: 3,
            offset_y: 5,
            width: w,
            height: h,
            indices,
            alpha,
            by_column,
            has_alpha,
        }
    }

    fn sheet(frames: Vec<SpriteFrame>) -> SpriteSheet {
        SpriteSheet {
            id: 7,
            width: 40,
            height: 30,
            palette: vec![0, 0xFF0000, 0x00FF00, 0x0000FF],
            frames,
            unread: 0,
        }
    }

    #[test]
    fn every_way_a_frame_can_be_stored_reads_back_the_same() {
        for by_column in [false, true] {
            for has_alpha in [false, true] {
                // not square, so rows and columns cannot be mistaken for each other
                let made = sheet(vec![frame(5, 3, by_column, has_alpha)]);
                let read = SpriteSheet::decode(7, &encode(&made)).unwrap();
                assert_eq!(read, made, "by column {by_column}, alpha {has_alpha}");
            }
        }
    }

    #[test]
    fn several_frames_of_different_sizes_keep_their_own_places() {
        let made = sheet(vec![
            frame(5, 3, false, false),
            frame(2, 7, true, true),
            frame(1, 1, false, true),
            frame(0, 0, false, false),
        ]);
        let read = SpriteSheet::decode(7, &encode(&made)).unwrap();
        assert_eq!(read, made);
        assert_eq!(read.unread, 0);
    }

    #[test]
    fn red_is_red_and_nothing_is_clear() {
        let made = sheet(vec![frame(4, 1, false, false)]);
        let read = SpriteSheet::decode(7, &encode(&made)).unwrap();
        let pixels = read.frames[0].rgba(&read.palette);
        assert_eq!(pixels[0][3], 0, "entry 0 is clear");
        assert_eq!(pixels[1], [255, 0, 0, 255], "entry 1 is red, not blue");
        assert_eq!(pixels[2], [0, 255, 0, 255]);
        assert_eq!(pixels[3], [0, 0, 255, 255]);
        assert_eq!(read.frames[0].clear_pixels(), 1);
    }

    #[test]
    fn black_in_the_palette_is_kept_apart_from_nothing() {
        let mut made = sheet(vec![frame(4, 1, false, false)]);
        made.palette[2] = 0;
        let read = SpriteSheet::decode(7, &encode(&made)).unwrap();
        assert_eq!(read.palette[2], 1);
    }

    #[test]
    fn a_file_cut_short_is_an_error_and_not_a_crash() {
        let bytes = encode(&sheet(vec![frame(5, 3, false, true)]));
        for cut in 0..bytes.len() {
            // any prefix: either an error or some reading, never a panic
            let _ = SpriteSheet::decode(7, &bytes[..cut]);
        }
        assert!(SpriteSheet::decode(7, &[]).is_err());
        // a table that claims more pixels than the file holds
        let mut lying = bytes.clone();
        let at = lying.len() - 2 - 2;
        lying[at] = 0x7F;
        assert!(SpriteSheet::decode(7, &lying).is_err());
    }

    #[test]
    fn bytes_left_over_are_counted() {
        let made = sheet(vec![frame(5, 3, false, false)]);
        let mut bytes = encode(&made);
        // two stray bytes between the pixels and the palette
        let at = 1 + 15;
        bytes.splice(at..at, [9, 9]);
        let read = SpriteSheet::decode(7, &bytes).unwrap();
        assert_eq!(read.unread, 2);
    }

    #[test]
    fn names_hash_the_way_the_cache_hashes_them() {
        assert_eq!(name_hash(""), 0);
        assert_eq!(name_hash("a"), 97);
        assert_eq!(name_hash("AB"), 97 * 31 + 98);
        // long enough to wrap round
        assert_eq!(name_hash("p11_full"), name_hash("P11_FULL"));
    }
}
