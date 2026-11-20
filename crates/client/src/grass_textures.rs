use std::path::Path;

use anyhow::{Context, Result};
use image::RgbaImage;
use image::imageops::FilterType;

const SIZE: u32 = 512;
const MIPS: u32 = 10;
const LAYERS: u32 = 6;
const SOURCE: u32 = 128;

/// Six cache-scraped grass variants at 512px, built by mirror-tiling the
/// original 128px sprite four times per axis. Mirror edges always match, so
/// the larger texture has no seams while keeping the original pixel density
/// and repeating four times less often than a raw 128px tile.
pub struct GrassTextures {
    pub width: u32,
    pub height: u32,
    pub layers: u32,
    pub mip_levels: u32,
    pub pixels: Vec<u8>,
}

impl GrassTextures {
    pub fn load() -> Result<Self> {
        let names = [
            "grass",
            "grass_lush",
            "grass_dry",
            "grass_steppe",
            "grass_dire",
            "grass_straw",
        ];
        let mut pixels = Vec::new();
        for (index, name) in names.iter().enumerate() {
            let path = format!("assets/osrs-ground/{name}.png");
            let source = if Path::new(&path).exists() {
                image::open(&path)
                    .with_context(|| format!("loading grass texture {path}"))?
                    .to_rgba8()
            } else {
                fallback(index)
            };
            let source = if source.width() == SOURCE && source.height() == SOURCE {
                source
            } else {
                image::imageops::resize(&source, SOURCE, SOURCE, FilterType::Nearest)
            };
            let mut level = mirror_tile(&source);
            pixels.extend_from_slice(level.as_raw());
            for _ in 1..MIPS {
                let w = (level.width() / 2).max(1);
                let h = (level.height() / 2).max(1);
                level = image::imageops::resize(&level, w, h, FilterType::Triangle);
                pixels.extend_from_slice(level.as_raw());
            }
        }
        tracing::info!(
            "grass textures: {} variants, {}px, {} mips, {} bytes",
            LAYERS,
            SIZE,
            MIPS,
            pixels.len()
        );
        Ok(Self {
            width: SIZE,
            height: SIZE,
            layers: LAYERS,
            mip_levels: MIPS,
            pixels,
        })
    }
}

fn mirror_tile(source: &RgbaImage) -> RgbaImage {
    let copies = SIZE / SOURCE;
    let mut image = RgbaImage::new(SIZE, SIZE);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let cx = x / SOURCE;
            let cy = y / SOURCE;
            let mut sx = x % SOURCE;
            let mut sy = y % SOURCE;
            if cx % 2 == 1 {
                sx = SOURCE - 1 - sx;
            }
            if cy % 2 == 1 {
                sy = SOURCE - 1 - sy;
            }
            debug_assert!(cx < copies && cy < copies);
            image.put_pixel(x, y, *source.get_pixel(sx, sy));
        }
    }
    image
}

fn fallback(layer: usize) -> RgbaImage {
    let base: [u8; 3] = match layer {
        0 => [89, 113, 44],
        1 => [104, 99, 47],
        2 => [157, 148, 95],
        3 => [98, 84, 57],
        4 => [52, 78, 52],
        _ => [97, 79, 55],
    };
    let mut image = RgbaImage::new(SOURCE, SOURCE);
    for y in 0..SOURCE {
        for x in 0..SOURCE {
            let n = hash(x, y) * 10.0 - 5.0;
            let c = [
                (base[0] as f32 + n).clamp(0.0, 255.0) as u8,
                (base[1] as f32 + n).clamp(0.0, 255.0) as u8,
                (base[2] as f32 + n).clamp(0.0, 255.0) as u8,
                255,
            ];
            image.put_pixel(x, y, image::Rgba(c));
        }
    }
    image
}

fn hash(x: u32, y: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 255) as f32 / 255.0
}
