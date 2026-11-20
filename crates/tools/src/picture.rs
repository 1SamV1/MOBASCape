//! Pictures the dump tool draws and saves: a plain grid of pixels and a PNG
//! writer, with nothing borrowed from outside so that neither can be wrong
//! in a way that hides a wrong decoder.

use std::io::Write;
use std::path::Path;

pub type Rgba = [u8; 4];

pub const CLEAR: Rgba = [0, 0, 0, 0];
pub const MAGENTA: Rgba = [255, 0, 255, 255];
pub const DARK: Rgba = [32, 32, 36, 255];
pub const WHITE: Rgba = [255, 255, 255, 255];
pub const YELLOW: Rgba = [255, 255, 0, 255];
pub const RED: Rgba = [255, 64, 64, 255];
pub const GREY: Rgba = [96, 96, 104, 255];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Rgba>,
}

impl Picture {
    pub fn new(width: usize, height: usize, ground: Rgba) -> Self {
        Self {
            width,
            height,
            pixels: vec![ground; width * height],
        }
    }

    pub fn from_pixels(width: usize, height: usize, pixels: Vec<Rgba>) -> Self {
        assert_eq!(pixels.len(), width * height, "a picture has all its pixels");
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Rgba {
        self.pixels[y * self.width + x]
    }

    /// Set one pixel outright; off the picture does nothing.
    pub fn set(&mut self, x: i32, y: i32, colour: Rgba) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = colour;
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, width: usize, height: usize, colour: Rgba) {
        for dy in 0..height as i32 {
            for dx in 0..width as i32 {
                self.set(x + dx, y + dy, colour);
            }
        }
    }

    /// Put another picture on this one, mixing by its opacity. Where this
    /// picture is itself clear, the other's pixel is kept as it is.
    pub fn draw(&mut self, other: &Picture, x: i32, y: i32) {
        for oy in 0..other.height {
            for ox in 0..other.width {
                let (px, py) = (x + ox as i32, y + oy as i32);
                if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
                    continue;
                }
                let over = other.get(ox, oy);
                let at = py as usize * self.width + px as usize;
                self.pixels[at] = mix(self.pixels[at], over);
            }
        }
    }

    /// Put another picture on this one in a single colour, using only its
    /// opacity: how a letter is drawn.
    pub fn stamp(&mut self, other: &Picture, x: i32, y: i32, colour: Rgba) {
        for oy in 0..other.height {
            for ox in 0..other.width {
                let alpha = other.get(ox, oy)[3];
                if alpha == 0 {
                    continue;
                }
                let (px, py) = (x + ox as i32, y + oy as i32);
                if px < 0 || py < 0 || px as usize >= self.width || py as usize >= self.height {
                    continue;
                }
                let at = py as usize * self.width + px as usize;
                self.pixels[at] = mix(self.pixels[at], [colour[0], colour[1], colour[2], alpha]);
            }
        }
    }

    /// The same picture over a solid ground.
    pub fn over(&self, ground: Rgba) -> Picture {
        let mut out = Picture::new(self.width, self.height, ground);
        out.draw(self, 0, 0);
        out
    }

    /// The picture made smaller by whole steps until it fits, never larger.
    pub fn fitted(&self, width: usize, height: usize) -> Picture {
        let mut step = 1;
        while self.width.div_ceil(step) > width || self.height.div_ceil(step) > height {
            step += 1;
        }
        if step == 1 {
            return self.clone();
        }
        let (w, h) = (self.width.div_ceil(step), self.height.div_ceil(step));
        let mut out = Picture::new(w, h, CLEAR);
        for y in 0..h {
            for x in 0..w {
                out.pixels[y * w + x] = self.get(x * step, y * step);
            }
        }
        out
    }

    /// Write whole numbers in a small built-in face: for labels, so that a
    /// label can never be wrong because a font was decoded wrongly.
    pub fn label(&mut self, text: &str, x: i32, y: i32, colour: Rgba) {
        let mut pen = x;
        for c in text.chars() {
            if let Some(rows) = small_face(c) {
                for (dy, row) in rows.iter().enumerate() {
                    for dx in 0..3 {
                        if row & (0b100 >> dx) != 0 {
                            self.set(pen + dx, y + dy as i32, colour);
                        }
                    }
                }
            }
            pen += 4;
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::File::create(path)?.write_all(&self.png())
    }

    /// The picture as a PNG file: eight bits each of red, green, blue and
    /// opacity, not squeezed.
    pub fn png(&self) -> Vec<u8> {
        let mut raw = Vec::with_capacity(self.height * (1 + self.width * 4));
        for y in 0..self.height {
            raw.push(0); // this row is given as it is
            for x in 0..self.width {
                raw.extend_from_slice(&self.get(x, y));
            }
        }

        // the pixels in "stored" blocks: the format's own way of holding
        // bytes without squeezing them
        let mut packed = vec![0x78, 0x01];
        let mut blocks = raw.chunks(65535).peekable();
        if blocks.peek().is_none() {
            packed.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
        }
        while let Some(block) = blocks.next() {
            packed.push(if blocks.peek().is_none() { 1 } else { 0 });
            let len = block.len() as u16;
            packed.extend_from_slice(&len.to_le_bytes());
            packed.extend_from_slice(&(!len).to_le_bytes());
            packed.extend_from_slice(block);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &raw {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        packed.extend_from_slice(&((b << 16) | a).to_be_bytes());

        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut header = Vec::new();
        header.extend_from_slice(&(self.width as u32).to_be_bytes());
        header.extend_from_slice(&(self.height as u32).to_be_bytes());
        header.extend_from_slice(&[8, 6, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &header);
        chunk(&mut out, b"IDAT", &packed);
        chunk(&mut out, b"IEND", &[]);
        out
    }
}

fn mix(under: Rgba, over: Rgba) -> Rgba {
    let a = over[3] as u32;
    if a == 255 || under[3] == 0 {
        return over;
    }
    if a == 0 {
        return under;
    }
    let blend = |u: u8, o: u8| ((o as u32 * a + u as u32 * (255 - a) + 127) / 255) as u8;
    [
        blend(under[0], over[0]),
        blend(under[1], over[1]),
        blend(under[2], over[2]),
        under[3].max(over[3]),
    ]
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let from = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in &out[from..] {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

/// Digits and a few marks, three wide and five tall.
fn small_face(c: char) -> Option<[u8; 5]> {
    Some(match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'x' => [0b000, 0b101, 0b010, 0b101, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_has_its_marks_its_size_and_sums_that_add_up() {
        let mut picture = Picture::new(3, 2, CLEAR);
        picture.set(0, 0, [255, 0, 0, 255]);
        picture.set(2, 1, [0, 0, 255, 128]);
        let png = picture.png();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..24], &[0, 0, 0, 3, 0, 0, 0, 2]);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // the well-known sum of an empty IEND chunk
        assert_eq!(&png[png.len() - 4..], &[0xAE, 0x42, 0x60, 0x82]);
    }

    #[test]
    fn drawing_mixes_by_opacity_and_keeps_what_is_under_a_clear_pixel() {
        let mut ground = Picture::new(2, 1, MAGENTA);
        let over = Picture::from_pixels(2, 1, vec![[0, 255, 0, 255], CLEAR]);
        ground.draw(&over, 0, 0);
        assert_eq!(ground.get(0, 0), [0, 255, 0, 255]);
        assert_eq!(ground.get(1, 0), MAGENTA);
        // half-clear white over black is mid grey
        let mut black = Picture::new(1, 1, [0, 0, 0, 255]);
        black.draw(
            &Picture::from_pixels(1, 1, vec![[255, 255, 255, 128]]),
            0,
            0,
        );
        assert_eq!(black.get(0, 0), [128, 128, 128, 255]);
        // off the edge does nothing
        ground.draw(&over, 5, 5);
        ground.draw(&over, -5, -5);
    }

    #[test]
    fn a_picture_is_made_smaller_by_whole_steps_and_never_larger() {
        let big = Picture::new(100, 40, WHITE);
        let small = big.fitted(30, 30);
        assert_eq!((small.width, small.height), (25, 10));
        assert_eq!(Picture::new(8, 8, WHITE).fitted(30, 30).width, 8);
    }
}
