//! Bakes a top-down terrain image for the HUD minimap.
//!
//! The minimap is a real map of the world. Each terrain vertex is coloured
//! the way the world shader paints it: the grass layer, lane soil, cache
//! overlay, rock on steep faces and water come from the vertex's own packed
//! data, and each takes the mean colour of the same cache texture the
//! world samples ([`Palette`]). The colours are binned into a small RGBA
//! image that the UI atlas uploads once at startup. Without a cache the bake
//! falls back to the raw floor colours.

use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use glam::Vec2;
use image::{Rgba, RgbaImage};
use rift_cache::Cache;

use crate::scene::{SceneData, SceneVertex};

/// Edge length of the baked minimap texture in pixels.
pub const MINIMAP_SIZE: u32 = 256;

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// The grass layers in the world shader's order (`grass_textures.rs`).
pub const GRASS_TEXTURES: [u32; 6] = [129, 203, 18, 197, 195, 205];
/// Lane and bare-ground soil, rock faces and water (`terrain_textures.rs`).
pub const SOIL_TEXTURE: u32 = 32;
pub const STONE_TEXTURE: u32 = 10;
pub const WATER_TEXTURE: u32 = 1;

/// Mean display colours (sRGB, 0 to 1) of the cache textures the world
/// shader paints the ground with.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub grass: [[f32; 3]; 6],
    pub soil: [f32; 3],
    pub stone: [f32; 3],
    pub water: [f32; 3],
    /// Overlay textures by id, for tiles that carry a textured overlay.
    pub overlays: HashMap<u32, [f32; 3]>,
}

impl Palette {
    /// Average the textures from the cache, including every overlay
    /// texture the terrain uses.
    pub fn from_cache(cache: &Cache, scene: &SceneData) -> Result<Self> {
        let files = crate::model_textures::texture_files(cache)?;
        let mean = |id: u32| -> Result<[f32; 3]> {
            let image = crate::model_textures::texture_image(&files, cache, id)?
                .ok_or_else(|| anyhow::anyhow!("texture {id} missing from cache"))?;
            mean_colour(&image).ok_or_else(|| anyhow::anyhow!("texture {id} is empty"))
        };
        let mut grass = [[0.0; 3]; 6];
        for (slot, id) in grass.iter_mut().zip(GRASS_TEXTURES) {
            *slot = mean(id)?;
        }
        let used: BTreeSet<u32> = scene
            .terrain
            .vertices
            .iter()
            .map(|v| Packed::of(v).overlay)
            .filter(|&id| id > 0)
            .collect();
        let mut overlays = HashMap::new();
        for id in used {
            // An overlay without art keeps the classified ground, as in
            // the shader.
            if let Ok(colour) = mean(id) {
                overlays.insert(id, colour);
            }
        }
        Ok(Self {
            grass,
            soil: mean(SOIL_TEXTURE)?,
            stone: mean(STONE_TEXTURE)?,
            water: mean(WATER_TEXTURE)?,
            overlays,
        })
    }
}

/// The mean colour of an image's opaque pixels, in sRGB 0 to 1.
pub fn mean_colour(image: &RgbaImage) -> Option<[f32; 3]> {
    let mut sum = [0.0f64; 3];
    let mut n = 0u64;
    for p in image.pixels() {
        if p[3] < 128 {
            continue;
        }
        for k in 0..3 {
            sum[k] += p[k] as f64;
        }
        n += 1;
    }
    (n > 0).then(|| sum.map(|c| (c / n as f64 / 255.0) as f32))
}

/// A terrain vertex's packed shader data (see `scene.wgsl`, `albedo`).
struct Packed {
    variant: usize,
    overlay: u32,
    overlay_weight: f32,
}

impl Packed {
    fn of(v: &SceneVertex) -> Self {
        Self {
            variant: (v.texture_id & 0xFF).min(5) as usize,
            overlay: (v.texture_id >> 8) & 0xFF,
            overlay_weight: ((v.texture_id >> 16) & 0xFF) as f32 / 255.0,
        }
    }
}

/// Map paint for grass: the reference minimap's fields are a muted olive
/// green (median saturation about 0.4), so the texture mean keeps part of
/// its saturation and leans a little green.
const GRASS_KEEP: f32 = 0.6;
const GRASS_TINT: [f32; 3] = [0.90, 1.0, 0.86];
const GRASS_LIGHT: f32 = 0.92;

/// One terrain vertex's ground colour in sRGB 0 to 1, following the
/// world shader's choices: grass or soil by the floor colour's hue, cache
/// overlay art, rock on steep faces, then lane soil and water on top.
pub fn vertex_colour(v: &SceneVertex, palette: &Palette) -> [f32; 3] {
    let floor = [v.color[0], v.color[1], v.color[2]].map(linear_to_srgb);
    let packed = Packed::of(v);
    let slope = v.uv[0].clamp(0.0, 1.0);
    let water = v.uv[1].clamp(0.0, 1.0);
    let lane = v.color[3].clamp(0.0, 1.0);
    let [r, g, b] = floor;
    let green = (g - r).max(0.0) * 2.2 + (g - b).max(0.0);
    let brown = (r - g).max(0.0) * 2.0 + (g - b).max(0.0) * 0.15;
    let grass_share = if green + brown > 1e-4 {
        green / (green + brown)
    } else {
        0.5
    };
    let grass = {
        let base = palette.grass[packed.variant];
        let luma = 0.299 * base[0] + 0.587 * base[1] + 0.114 * base[2];
        std::array::from_fn(|k| {
            ((luma + (base[k] - luma) * GRASS_KEEP) * GRASS_TINT[k] * GRASS_LIGHT).clamp(0.0, 1.0)
        })
    };
    let dirt = palette.soil.map(|c| c * 0.82);
    let mut c = mix3(dirt, grass, grass_share);
    if packed.overlay > 0
        && let Some(&art) = palette.overlays.get(&packed.overlay)
    {
        c = mix3(c, art, packed.overlay_weight);
    }
    c = mix3(c, palette.stone, smoothstep(0.13, 0.32, slope));
    c = mix3(c, palette.soil.map(|v| (v * 1.12).min(1.0)), lane * 0.96);
    mix3(c, palette.water, water)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t)
}

/// Saturation boost of the baked ground: the colours are averaged per
/// pixel and read a little washed out at minimap size.
const SATURATION: f32 = 1.1;
/// Brightness kept on tiles nobody can walk (trees, rocks, walls), so the
/// jungle reads as dark masses between the lanes, as on the OSRS world map.
const BLOCKED_SHADE: f32 = 0.34;
/// Pixels either side over which the blocked share is averaged.
const BLOCKED_RADIUS: usize = 3;

/// Rasterise the terrain into a minimap image covering `bounds`, coloured
/// from `palette` when the cache provided one.
pub fn render(
    scene: &SceneData,
    bounds: (Vec2, Vec2),
    size: u32,
    palette: Option<&Palette>,
) -> RgbaImage {
    let mut image = render_ground(scene, bounds, size, palette);
    let (min, max) = bounds;
    let span = max - min;
    let n = size as usize;
    let mut blocked = vec![0.0f32; n * n];
    for y in 0..n {
        for x in 0..n {
            let wx = min.x + (x as f32 + 0.5) / size as f32 * span.x;
            let wz = min.y + (y as f32 + 0.5) / size as f32 * span.y;
            if !scene.collision.is_walkable_world(wx, wz) {
                blocked[y * n + x] = 1.0;
            }
        }
    }
    let density = box_blur(&blocked, n, BLOCKED_RADIUS);
    for y in 0..size {
        for x in 0..size {
            let p = image.get_pixel_mut(x, y);
            p.0 = grade(p.0, density[(y * size + x) as usize]);
        }
    }
    image
}

/// The mean of `values` (an `n` by `n` grid) over a square of `radius`
/// cells around each cell, clamped at the edges.
fn box_blur(values: &[f32], n: usize, radius: usize) -> Vec<f32> {
    let pass = |src: &[f32], horizontal: bool| {
        let mut out = vec![0.0f32; n * n];
        for a in 0..n {
            for b in 0..n {
                let lo = b.saturating_sub(radius);
                let hi = (b + radius).min(n - 1);
                let sum: f32 = (lo..=hi)
                    .map(|k| {
                        if horizontal {
                            src[a * n + k]
                        } else {
                            src[k * n + a]
                        }
                    })
                    .sum();
                let at = if horizontal { a * n + b } else { b * n + a };
                out[at] = sum / (hi - lo + 1) as f32;
            }
        }
        out
    };
    pass(&pass(values, true), false)
}

/// The minimap's colour grade for one pixel: more saturation, and ground
/// among blocked tiles (`blocked`, the blocked share nearby, 0 to 1)
/// pushed down towards a deep green, so dense jungle reads as dark masses
/// and a lone tree does not speckle the map.
pub fn grade(pixel: [u8; 4], blocked: f32) -> [u8; 4] {
    let [r, g, b, a] = pixel.map(|c| c as f32 / 255.0);
    let luma = 0.299 * r + 0.587 * g + 0.114 * b;
    let sat = |c: f32| (luma + (c - luma) * SATURATION).clamp(0.0, 1.0);
    let (mut r, mut g, mut b) = (sat(r), sat(g), sat(b));
    let t = smoothstep(0.2, 0.65, blocked);
    if t > 0.0 {
        let mix = |c: f32, dark: f32| c + (dark - c) * t;
        r = mix(r, r * BLOCKED_SHADE * 0.8);
        g = mix(g, g * BLOCKED_SHADE + 0.03);
        b = mix(b, b * BLOCKED_SHADE * 0.8);
    }
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(r), byte(g), byte(b), byte(a)]
}

fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
    let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The averaged terrain colours, before the grade.
fn render_ground(
    scene: &SceneData,
    bounds: (Vec2, Vec2),
    size: u32,
    palette: Option<&Palette>,
) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(size, size, Rgba([8, 11, 14, 255]));
    let mut accum = vec![[0.0f32; 3]; (size * size) as usize];
    let mut count = vec![0u32; (size * size) as usize];

    let (min, max) = bounds;
    let span_x = (max.x - min.x).max(1.0);
    let span_z = (max.y - min.y).max(1.0);

    for vertex in &scene.terrain.vertices {
        let nx = (vertex.position[0] - min.x) / span_x;
        let nz = (vertex.position[2] - min.y) / span_z;
        if !(0.0..1.0).contains(&nx) || !(0.0..1.0).contains(&nz) {
            continue;
        }
        let colour = match palette {
            Some(palette) => vertex_colour(vertex, palette),
            None => [vertex.color[0], vertex.color[1], vertex.color[2]]
                .map(|c| linear_to_srgb((c * 1.35).min(1.0))),
        };
        let px = (nx * (size - 1) as f32).round() as i32;
        // North (small z) is up, matching the world camera.
        let py = (nz * (size - 1) as f32).round() as i32;
        // Splat into a 3x3 neighbourhood: the terrain vertices are sparser
        // than the minimap pixels, so point binning would leave a grid of
        // unfilled background pixels.
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                let x = px + dx;
                let y = py + dy;
                if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
                    continue;
                }
                let index = y as usize * size as usize + x as usize;
                for k in 0..3 {
                    accum[index][k] += colour[k];
                }
                count[index] += 1;
            }
        }
    }

    for y in 0..size {
        for x in 0..size {
            let index = (y * size + x) as usize;
            if count[index] == 0 {
                continue;
            }
            let n = count[index] as f32;
            let [r, g, b] = accum[index].map(|c| ((c / n).clamp(0.0, 1.0) * 255.0) as u8);
            image.put_pixel(x, y, Rgba([r, g, b, 255]));
        }
    }

    // A subtle vignette keeps the map readable under the HUD frame.
    for y in 0..size {
        for x in 0..size {
            let nx = x as f32 / (size - 1) as f32 * 2.0 - 1.0;
            let ny = y as f32 / (size - 1) as f32 * 2.0 - 1.0;
            let d = (nx * nx + ny * ny).sqrt();
            let shade = 1.0 - (d * 0.16).clamp(0.0, 0.2);
            let p = image.get_pixel_mut(x, y);
            p.0[0] = (p.0[0] as f32 * shade) as u8;
            p.0[1] = (p.0[1] as f32 * shade) as u8;
            p.0[2] = (p.0[2] as f32 * shade) as u8;
        }
    }

    image
}

#[cfg(test)]
mod tests {
    use super::{Palette, box_blur, grade, mean_colour, vertex_colour};
    use crate::scene::SceneVertex;
    use glam::Vec3;
    use std::collections::HashMap;

    fn palette() -> Palette {
        Palette {
            grass: [[0.30, 0.45, 0.20]; 6],
            soil: [0.60, 0.50, 0.35],
            stone: [0.45, 0.45, 0.45],
            water: [0.20, 0.35, 0.50],
            overlays: HashMap::from([(7, [0.9, 0.1, 0.1])]),
        }
    }

    fn vertex(floor: [u8; 3]) -> SceneVertex {
        let mut v = SceneVertex::new(Vec3::ZERO, Vec3::Y, floor);
        v.color[3] = 0.0;
        v
    }

    #[test]
    fn vertices_take_the_world_shaders_ground_classes() {
        let p = palette();
        let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.02);
        // A green floor becomes tinted grass: green leads clearly.
        let grass = vertex_colour(&vertex([70, 110, 40]), &p);
        assert!(
            grass[1] > grass[0] * 1.3 && grass[1] > grass[2] * 1.3,
            "{grass:?}"
        );
        // Full lane mask paints lane soil; full water mask paints water.
        let mut lane = vertex([70, 110, 40]);
        lane.color[3] = 1.0;
        let lane = vertex_colour(&lane, &p);
        assert!(lane[0] > grass[0] && lane[2] > grass[2], "{lane:?}");
        let mut wet = vertex([70, 110, 40]);
        wet.uv[1] = 1.0;
        assert!(near(vertex_colour(&wet, &p), p.water));
        // Steep faces are rock.
        let mut cliff = vertex([70, 110, 40]);
        cliff.uv[0] = 0.5;
        assert!(near(vertex_colour(&cliff, &p), p.stone));
        // A full-weight overlay with art takes the overlay's colour.
        let mut overlay = vertex([70, 110, 40]);
        overlay.texture_id = 7 << 8 | 255 << 16;
        assert!(near(vertex_colour(&overlay, &p), [0.9, 0.1, 0.1]));
    }

    #[test]
    fn the_mean_skips_transparent_pixels() {
        let mut image = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 0, 0, 255]));
        image.put_pixel(1, 0, image::Rgba([0, 0, 255, 0]));
        assert_eq!(mean_colour(&image), Some([1.0, 0.0, 0.0]));
        assert_eq!(mean_colour(&image::RgbaImage::new(1, 1)), None);
    }

    #[test]
    fn blocked_ground_is_darker_and_greener_than_open_ground() {
        let grass = [96, 120, 60, 255];
        let open = grade(grass, 0.0);
        let blocked = grade(grass, 1.0);
        let sum = |p: [u8; 4]| p[0] as u32 + p[1] as u32 + p[2] as u32;
        assert!(sum(blocked) * 2 < sum(open));
        assert!(blocked[1] > blocked[0] && blocked[1] > blocked[2]);
        // A lone blocked tile among open ground barely changes the colour.
        assert_eq!(grade(grass, 0.1), open);
        // The grade keeps grey grey and only stretches colour.
        assert_eq!(grade([100, 100, 100, 255], 0.0), [100, 100, 100, 255]);
    }

    #[test]
    fn the_blur_spreads_a_cell_and_keeps_the_total_in_the_middle() {
        let n = 9;
        let mut grid = vec![0.0; n * n];
        grid[4 * n + 4] = 1.0;
        let out = box_blur(&grid, n, 1);
        assert!((out[4 * n + 4] - 1.0 / 9.0).abs() < 1e-6);
        assert!((out[3 * n + 5] - 1.0 / 9.0).abs() < 1e-6);
        assert_eq!(out[0], 0.0);
        let total: f32 = out.iter().sum();
        assert!((total - 1.0).abs() < 1e-5);
    }
}
