//! Item definitions (config archive 10).
//!
//! The first fields are the ones the interface needs for an inventory icon:
//! the model, its zoom, its three rotations, its two pixel offsets, the
//! resize and the palette swaps. The rest of the modern item stream is not
//! decoded yet; [`ItemDefinition::parse`] stops cleanly at the first opcode
//! it does not know instead of guessing a length and losing the stream.

use crate::buffer::Buffer;
use crate::error::Result;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemDefinition {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// Model id used for the inventory picture. Zero means none.
    pub model: u32,
    pub zoom: u16,
    pub rotation_x: u16,
    pub rotation_y: u16,
    pub rotation_z: u16,
    pub offset_x: i16,
    pub offset_y: i16,
    pub scale_x: u16,
    pub scale_y: u16,
    pub scale_z: u16,
    pub ambient: i8,
    pub contrast: i16,
    /// `(find, replace)` palette pairs.
    pub recolors: Vec<(u16, u16)>,
    pub value: i32,
    pub weight: i16,
    pub category: u16,
    pub stackable: bool,
    pub members: bool,
    /// True when the stream ended in the terminator, false when it stopped at
    /// an opcode this decoder does not understand.
    pub complete: bool,
}

impl ItemDefinition {
    pub fn parse(id: u32, data: &[u8]) -> Result<Self> {
        let mut def = Self {
            id,
            zoom: 2000,
            scale_x: 128,
            scale_y: 128,
            scale_z: 128,
            ..Default::default()
        };
        let mut b = Buffer::new(data);
        loop {
            let opcode = b.read_u8()?;
            let ok = match opcode {
                0 => {
                    def.complete = true;
                    break;
                }
                1 => {
                    def.model = b.read_u16()? as u32;
                    true
                }
                2 => {
                    def.name = b.read_string()?;
                    true
                }
                3 => {
                    def.description = b.read_string()?;
                    true
                }
                4 => {
                    def.zoom = b.read_u16()?;
                    true
                }
                5 => {
                    def.rotation_x = b.read_u16()?;
                    true
                }
                6 => {
                    def.rotation_y = b.read_u16()?;
                    true
                }
                7 => {
                    def.offset_x = b.read_u16()? as i16;
                    true
                }
                8 => {
                    def.offset_y = b.read_u16()? as i16;
                    true
                }
                9 => {
                    b.read_string()?;
                    true
                }
                10 => {
                    b.read_u16()?;
                    true
                }
                11 => {
                    def.stackable = true;
                    true
                }
                12 => {
                    def.value = b.read_i32()?;
                    true
                }
                13 | 14 | 27 => {
                    b.read_i8()?;
                    true
                }
                15 => true,
                16 => {
                    def.members = true;
                    true
                }
                23 | 25 => {
                    b.read_u16()?;
                    b.read_u8()?;
                    true
                }
                24 | 26 => {
                    b.read_u16()?;
                    true
                }
                45 | 48 => {
                    b.read_i32()?;
                    b.read_u8()?;
                    true
                }
                30..=39 => {
                    b.read_string()?;
                    true
                }
                40 | 41 => {
                    let count = b.read_u8()? as usize;
                    if opcode == 40 {
                        def.recolors.clear();
                    }
                    for _ in 0..count {
                        let find = b.read_u16()?;
                        let replace = b.read_u16()?;
                        if opcode == 40 {
                            def.recolors.push((find, replace));
                        }
                    }
                    true
                }
                42 => {
                    b.read_i8()?;
                    true
                }
                43 => {
                    b.read_u8()?;
                    loop {
                        let sub = b.read_u8()?;
                        if sub == 0 {
                            break;
                        }
                        b.read_string()?;
                    }
                    true
                }
                44 | 46 | 47 | 49 | 50 | 51 | 52 | 53 | 54 => {
                    b.read_i32()?;
                    true
                }
                65 | 160 | 251 => true,
                75 => {
                    def.weight = b.read_i16()?;
                    true
                }
                78 | 79 | 90..=93 | 95 | 97 | 98 | 99 | 139 | 140 | 148 | 149 => {
                    b.read_u16()?;
                    true
                }
                94 => {
                    def.category = b.read_u16()?;
                    true
                }
                100..=109 => {
                    b.read_u16()?;
                    b.read_u16()?;
                    true
                }
                110 => {
                    def.scale_x = b.read_u16()?;
                    true
                }
                111 => {
                    def.scale_y = b.read_u16()?;
                    true
                }
                112 => {
                    def.scale_z = b.read_u16()?;
                    true
                }
                113 => {
                    def.ambient = b.read_i8()?;
                    true
                }
                114 => {
                    def.contrast = b.read_i8()? as i16 * 5;
                    true
                }
                115 => {
                    b.read_u8()?;
                    true
                }
                161 => {
                    let count = b.read_u16()? as usize;
                    for _ in 0..count {
                        b.read_u16()?;
                    }
                    true
                }
                249 => {
                    let count = b.read_u8()? as usize;
                    for _ in 0..count {
                        let kind = b.read_u8()?;
                        b.read_u24()?;
                        if kind == 1 {
                            b.read_string()?;
                        } else {
                            b.read_i32()?;
                        }
                    }
                    true
                }
                _ => false,
            };
            if !ok {
                break;
            }
        }
        Ok(def)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(extra: &[u8]) -> Vec<u8> {
        let mut data = vec![
            1, 0x0a, 0x2c, // model 2604
            2, b'S', b'w', b'o', b'r', b'd', 0, // name
            3, b'A', b' ', b'b', b'l', b'a', b'd', b'e', 0, // description
            4, 0x04, 0x10, // zoom 1040
            5, 0x01, 0x60, // rotation x
            6, 0x04, 0x84, // rotation y
            7, 0xff, 0xfd, // offset x -3
            8, 0x00, 0x10, // offset y 16
            40, 1, 0x00, 0x05, 0x00, 0x09, // one recolor
        ];
        data.extend_from_slice(extra);
        data.push(0);
        data
    }

    #[test]
    fn the_icon_fields_are_read_and_the_terminator_is_complete() {
        let def = ItemDefinition::parse(7, &sample(&[])).unwrap();
        assert_eq!(def.model, 2604);
        assert_eq!(def.name, "Sword");
        assert_eq!(def.description, "A blade");
        assert_eq!(
            (def.zoom, def.rotation_x, def.rotation_y),
            (1040, 352, 1156)
        );
        assert_eq!((def.offset_x, def.offset_y), (-3, 16));
        assert_eq!(def.recolors, [(5, 9)]);
        assert!(def.complete);
    }

    #[test]
    fn an_unknown_opcode_stops_without_losing_the_fields_already_read() {
        let def = ItemDefinition::parse(8, &sample(&[250, 1, 2])).unwrap();
        assert_eq!(def.model, 2604);
        assert_eq!(def.name, "Sword");
        assert!(!def.complete);
    }
}
