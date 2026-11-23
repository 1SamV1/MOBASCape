use std::path::Path;

use anyhow::{Context, Result};
use image::RgbaImage;
use image::imageops::FilterType;
use rift_cache::Cache;

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
    /// Prefer the cache's own texture definitions. The six documented
    /// grass IDs cover classic, lush, dry, steppe, dire and straw.
    pub fn load_from_cache(cache: &Cache) -> Result<Self> {
        const IDS: [u32; LAYERS as usize] = [129, 203, 18, 197, 195, 205];
        let files = crate::model_textures::texture_files(cache)?;
        let mut sources = Vec::with_capacity(IDS.len());
        for id in IDS {
            let source = crate::model_textures::texture_image(&files, cache, id)?
                .with_context(|| format!("grass texture {id} missing from cache"))?;
            sources.push(source);
        }
        Self::from_sources(sources)
    }

    /// Cache first, then the local override/procedural fallback. A clean
    /// checkout still renders grass even if a texture definition is missing.
    pub fn load_with_fallback(cache: &Cache) -> Result<Self> {
        match Self::load_from_cache(cache) {
            Ok(textures) => Ok(textures),
            Err(err) => {
                tracing::warn!("cache grass textures: {err:#}; using local/fallback");
                Self::load()
            }
        }
    }

    /// Local PNG override or procedural fallback, used when a cache texture
    /// definition is unavailable.
    pub fn load() -> Result<Self> {
        let names = [
            "grass",
            "grass_lush",
            "grass_dry",
            "grass_steppe",
            "grass_dire",
            "grass_straw",
        ];
        let mut sources = Vec::with_capacity(names.len());
        for (index, name) in names.iter().enumerate() {
            let path = format!("assets/osrs-ground/{name}.png");
            let source = if Path::new(&path).exists() {
                image::open(&path)
                    .with_context(|| format!("loading grass texture {path}"))?
                    .to_rgba8()
            } else {
                fallback(index)
            };
            sources.push(source);
        }
        Self::from_sources(sources)
    }

    fn from_sources(sources: Vec<RgbaImage>) -> Result<Self> {
        debug_assert_eq!(sources.len(), LAYERS as usize);
        let mut sources: Vec<RgbaImage> = sources
            .into_iter()
            .map(|source| {
                if source.width() == SOURCE && source.height() == SOURCE {
                    source
                } else {
                    image::imageops::resize(&source, SOURCE, SOURCE, FilterType::Nearest)
                }
            })
            .collect();
        normalize_brightness(&mut sources);
        let mut pixels = Vec::new();
        for source in sources {
            let mut level = mirror_tile(&source);
            bake_alpha_noise(&mut level);
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

/// Pull every variant's mean brightness most of the way to the classic
/// grass (layer 0), in linear light, keeping each sprite's hue and texture.
/// The cache "lush" grass (203) is several times darker than the classic
/// sprite; a biome pocket using it read as a flat dark slab across the
/// jungle. `KEEP` retains part of each variant's own brightness so biomes
/// still differ in tone.
fn normalize_brightness(sources: &mut [RgbaImage]) {
    const KEEP: f32 = 0.3;
    let to_linear = |v: u8| crate::scene::srgb_to_linear(v as f32 / 255.0);
    let to_srgb = |v: f32| {
        let v = v.clamp(0.0, 1.0);
        let s = if v <= 0.003_130_8 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round().clamp(0.0, 255.0) as u8
    };
    let mean_luma = |image: &RgbaImage| {
        let mut sum = 0.0f64;
        for p in image.pixels() {
            let [r, g, b, _] = p.0;
            sum += (0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)) as f64;
        }
        (sum / (image.width() * image.height()) as f64).max(1e-4) as f32
    };
    let Some(target) = sources.first().map(mean_luma) else {
        return;
    };
    for (index, image) in sources.iter_mut().enumerate().skip(1) {
        let mean = mean_luma(image);
        let gain = (target / mean).powf(1.0 - KEEP).clamp(0.5, 3.0);
        for p in image.pixels_mut() {
            for c in 0..3 {
                p.0[c] = to_srgb(to_linear(p.0[c]) * gain);
            }
        }
        tracing::info!("grass variant {index}: mean luma {mean:.3} -> gain {gain:.2}");
    }
}

/// Write tileable multi-octave value noise into the alpha channel. The grass
/// sprites are opaque and the shader only reads `.rgb`, so alpha is free:
/// the terrain shader reads it from the grass samples it already takes and
/// gets mottle and macro patches without evaluating procedural noise per
/// pixel. The noise is periodic over the 512px layer, so it tiles exactly
/// like the colour does, and the mip chain filters it for distance.
fn bake_alpha_noise(image: &mut RgbaImage) {
    const OCTAVES: [(u32, f32); 3] = [(4, 0.55), (9, 0.30), (23, 0.15)];
    let size = image.width();
    for y in 0..size {
        for x in 0..size {
            let mut value = 0.0f32;
            for (index, (cells, weight)) in OCTAVES.iter().enumerate() {
                value += periodic_noise(x, y, size, *cells, index as u32) * weight;
            }
            image.get_pixel_mut(x, y).0[3] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Smooth value noise with `cells` lattice cells across `size` pixels that
/// wraps at the edge, so the layer tiles seamlessly.
fn periodic_noise(x: u32, y: u32, size: u32, cells: u32, seed: u32) -> f32 {
    let fx = x as f32 * cells as f32 / size as f32;
    let fy = y as f32 * cells as f32 / size as f32;
    let ix = fx.floor() as u32;
    let iy = fy.floor() as u32;
    let tx = fx - ix as f32;
    let ty = fy - iy as f32;
    let ux = tx * tx * (3.0 - 2.0 * tx);
    let uy = ty * ty * (3.0 - 2.0 * ty);
    let corner = |cx: u32, cy: u32| {
        hash(
            (cx % cells).wrapping_add(seed * 7_919),
            (cy % cells).wrapping_add(seed * 104_729),
        )
    };
    let a = corner(ix, iy);
    let b = corner(ix + 1, iy);
    let c = corner(ix, iy + 1);
    let d = corner(ix + 1, iy + 1);
    let top = a + (b - a) * ux;
    let bottom = c + (d - c) * ux;
    top + (bottom - top) * uy
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_noise_tiles_seamlessly_and_spans_a_range() {
        let mut image = RgbaImage::from_pixel(SIZE, SIZE, image::Rgba([10, 20, 30, 255]));
        bake_alpha_noise(&mut image);
        let alpha = |x: u32, y: u32| image.get_pixel(x, y).0[3] as i32;
        // The last column/row must lead smoothly into the first.
        for k in (0..SIZE).step_by(17) {
            assert!(
                (alpha(SIZE - 1, k) - alpha(0, k)).abs() <= 12,
                "x seam at {k}"
            );
            assert!(
                (alpha(k, SIZE - 1) - alpha(k, 0)).abs() <= 12,
                "y seam at {k}"
            );
        }
        let values: Vec<i32> = image.pixels().map(|p| p.0[3] as i32).collect();
        let min = *values.iter().min().unwrap();
        let max = *values.iter().max().unwrap();
        assert!(max - min > 100, "noise too flat: {min}..{max}");
        // Colour is untouched.
        assert_eq!(&image.get_pixel(5, 5).0[..3], &[10, 20, 30]);
    }

    #[test]
    fn brightness_normalisation_lifts_dark_variants_toward_the_first() {
        let bright = RgbaImage::from_pixel(8, 8, image::Rgba([120, 170, 60, 255]));
        let dark = RgbaImage::from_pixel(8, 8, image::Rgba([30, 50, 20, 255]));
        let mut layers = vec![bright.clone(), dark.clone()];
        normalize_brightness(&mut layers);
        assert_eq!(layers[0], bright, "the reference layer is untouched");
        let lifted = layers[1].get_pixel(0, 0).0;
        assert!(lifted[1] > 50 && lifted[1] < 170, "green {}", lifted[1]);
        // Hue survives: green stays the dominant channel.
        assert!(lifted[1] > lifted[0] && lifted[0] > lifted[2]);
    }
}
