//! Spot animation ("graphic") definitions, config archive 13.
//!
//! A spot animation is a model plus a sequence: spell projectiles, arrows,
//! cast flashes and impact splashes. Ported from RuneLite's
//! `SpotAnimLoader` (BSD-2-Clause); field names follow the reference.
//! [`SpotAnimDefinition::parse`] stops cleanly at the first opcode it does not
//! know, the same way the item decoder does, and records that in `complete`.

use crate::buffer::Buffer;
use crate::error::Result;

/// Config archive that holds spot animations.
pub const SPOTANIM_ARCHIVE: u32 = 13;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotAnimDefinition {
    pub id: u32,
    /// Model id. Zero means none.
    pub model_id: u32,
    /// Sequence id, or -1 for a static model.
    pub animation_id: i32,
    /// Horizontal and vertical scale; 128 is 1:1.
    pub resize_x: u16,
    pub resize_y: u16,
    /// Rotation around the vertical axis in 1/2048 turns (multiples of 512).
    pub rotation: u16,
    pub ambient: u8,
    pub contrast: u8,
    /// `(find, replace)` palette pairs.
    pub recolors: Vec<(u16, u16)>,
    /// `(find, replace)` texture pairs.
    pub retextures: Vec<(u16, u16)>,
    /// True when the stream ended in the terminator, false when it stopped at
    /// an opcode this decoder does not understand.
    pub complete: bool,
}

impl Default for SpotAnimDefinition {
    fn default() -> Self {
        Self {
            id: 0,
            model_id: 0,
            animation_id: -1,
            resize_x: 128,
            resize_y: 128,
            rotation: 0,
            ambient: 0,
            contrast: 0,
            recolors: Vec::new(),
            retextures: Vec::new(),
            complete: false,
        }
    }
}

impl SpotAnimDefinition {
    pub fn parse(id: u32, data: &[u8]) -> Result<Self> {
        let mut def = Self {
            id,
            ..Default::default()
        };
        let mut b = Buffer::new(data);
        loop {
            let opcode = b.read_u8()?;
            match opcode {
                0 => {
                    def.complete = true;
                    break;
                }
                1 => def.model_id = b.read_u16()? as u32,
                2 => {
                    let raw = b.read_u16()?;
                    def.animation_id = if raw == u16::MAX { -1 } else { raw as i32 };
                }
                4 => def.resize_x = b.read_u16()?,
                5 => def.resize_y = b.read_u16()?,
                6 => def.rotation = b.read_u16()?,
                7 => def.ambient = b.read_u8()?,
                8 => def.contrast = b.read_u8()?,
                40 | 41 => {
                    let count = b.read_u8()? as usize;
                    let mut pairs = Vec::with_capacity(count);
                    for _ in 0..count {
                        let find = b.read_u16()?;
                        let replace = b.read_u16()?;
                        pairs.push((find, replace));
                    }
                    if opcode == 40 {
                        def.recolors = pairs;
                    } else {
                        def.retextures = pairs;
                    }
                }
                _ => break,
            }
        }
        Ok(def)
    }

    /// Scale factors relative to 1:1 (`resize / 128`).
    pub fn scale(&self) -> (f32, f32) {
        (self.resize_x as f32 / 128.0, self.resize_y as f32 / 128.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(extra: &[u8]) -> Vec<u8> {
        let mut data = vec![
            1, 0x0d, 0x8c, // model 3468
            2, 0x00, 0x8e, // sequence 142
            4, 0x00, 0xa0, // resize x 160
            5, 0x00, 0x60, // resize y 96
            6, 0x02, 0x00, // rotation 512
            7, 30, // ambient
            8, 60, // contrast
            40, 2, 0x00, 0x05, 0x00, 0x09, 0x01, 0x00, 0x02, 0x00, // two recolors
            41, 1, 0x00, 0x28, 0x00, 0x2a, // one retexture
        ];
        data.extend_from_slice(extra);
        data.push(0);
        data
    }

    #[test]
    fn every_known_field_is_read_and_the_terminator_is_complete() {
        let def = SpotAnimDefinition::parse(130, &sample(&[])).unwrap();
        assert_eq!(def.id, 130);
        assert_eq!(def.model_id, 3468);
        assert_eq!(def.animation_id, 142);
        assert_eq!((def.resize_x, def.resize_y), (160, 96));
        assert_eq!(def.rotation, 512);
        assert_eq!((def.ambient, def.contrast), (30, 60));
        assert_eq!(def.recolors, [(5, 9), (256, 512)]);
        assert_eq!(def.retextures, [(40, 42)]);
        assert_eq!(def.scale(), (1.25, 0.75));
        assert!(def.complete);
    }

    #[test]
    fn defaults_hold_when_the_stream_is_only_a_model() {
        let def = SpotAnimDefinition::parse(1, &[1, 0x00, 0x10, 0]).unwrap();
        assert_eq!(def.model_id, 16);
        assert_eq!(def.animation_id, -1);
        assert_eq!(def.scale(), (1.0, 1.0));
        assert!(def.complete);
    }

    #[test]
    fn a_missing_sequence_reads_as_minus_one() {
        let def = SpotAnimDefinition::parse(2, &[2, 0xff, 0xff, 0]).unwrap();
        assert_eq!(def.animation_id, -1);
    }

    #[test]
    fn an_unknown_opcode_stops_without_losing_the_fields_already_read() {
        let def = SpotAnimDefinition::parse(3, &sample(&[250, 1, 2])).unwrap();
        assert_eq!(def.model_id, 3468);
        assert_eq!(def.animation_id, 142);
        assert!(!def.complete);
    }

    #[test]
    fn a_truncated_stream_is_an_error() {
        assert!(SpotAnimDefinition::parse(4, &[1, 0x00]).is_err());
    }
}
