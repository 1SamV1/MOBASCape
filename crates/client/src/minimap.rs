//! Bakes a top-down terrain image for the HUD minimap.
//!
//! The minimap is a real map of the world: it bins the terrain mesh colours
//! (which already contain the biome pockets, the river and the baked lanes)
//! into a small RGBA image that the UI atlas uploads once at startup.

use glam::Vec2;
use image::{Rgba, RgbaImage};

use crate::scene::SceneData;

/// Edge length of the baked minimap texture in pixels.
pub const MINIMAP_SIZE: u32 = 256;

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Saturation boost of the baked ground: the scene colours are blurred
/// per vertex and read washed out at minimap size.
const SATURATION: f32 = 1.35;
/// Brightness kept on tiles nobody can walk (trees, rocks, walls), so the
/// jungle reads as dark masses between the lanes, as on the OSRS world map.
const BLOCKED_SHADE: f32 = 0.42;
/// Pixels either side over which the blocked share is averaged.
const BLOCKED_RADIUS: usize = 3;

/// Rasterise the terrain into a minimap image covering `bounds`.
pub fn render(scene: &SceneData, bounds: (Vec2, Vec2), size: u32) -> RgbaImage {
    let mut image = render_ground(scene, bounds, size);
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
fn render_ground(scene: &SceneData, bounds: (Vec2, Vec2), size: u32) -> RgbaImage {
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
                accum[index][0] += vertex.color[0];
                accum[index][1] += vertex.color[1];
                accum[index][2] += vertex.color[2];
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
            let r = linear_to_srgb((accum[index][0] / n * 1.35).min(1.0));
            let g = linear_to_srgb((accum[index][1] / n * 1.35).min(1.0));
            let b = linear_to_srgb((accum[index][2] / n * 1.35).min(1.0));
            image.put_pixel(
                x,
                y,
                Rgba([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, 255]),
            );
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
    use super::{box_blur, grade};

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
