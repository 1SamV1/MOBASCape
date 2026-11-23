//! A tiny flat-shaded software rasteriser for contact sheets: one model pose
//! per labelled tile. Used to pick spot animations and attack sequences by
//! eye without starting the client.

use rift_cache::Model;
use rift_cache::model::hsl_to_rgb;

use crate::picture::{Picture, Rgba, WHITE};

/// Draw `positions` (cache space, Y down) of `model` into a square picture,
/// fitted to the tile and seen from a three-quarter angle.
pub fn render_pose(model: &Model, positions: &[[f32; 3]], size: usize, ground: Rgba) -> Picture {
    let mut picture = Picture::new(size, size, ground);
    if positions.is_empty() || model.faces.is_empty() {
        return picture;
    }
    // Yaw 35 degrees, pitch 20 degrees, engine Y up.
    let (sy, cy) = 0.61f32.sin_cos();
    let (sp, cp) = 0.35f32.sin_cos();
    let view = |p: &[f32; 3]| -> [f32; 3] {
        let (x, y, z) = (p[0], -p[1], p[2]);
        let rx = x * cy + z * sy;
        let rz = -x * sy + z * cy;
        let ry = y * cp - rz * sp;
        let rz = y * sp + rz * cp;
        [rx, ry, rz]
    };
    let points: Vec<[f32; 3]> = positions.iter().map(view).collect();
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in &points {
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    let extent = (max[0] - min[0]).max(max[1] - min[1]).max(1.0);
    let scale = size as f32 * 0.86 / extent;
    let mid = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let half = size as f32 * 0.5;
    let screen: Vec<[f32; 3]> = points
        .iter()
        .map(|p| {
            [
                half + (p[0] - mid[0]) * scale,
                half - (p[1] - mid[1]) * scale,
                p[2],
            ]
        })
        .collect();

    let mut depth = vec![f32::MIN; size * size];
    let light = normalise([0.4, 0.8, 0.45]);
    for (face, &[a, b, c]) in model.faces.iter().enumerate() {
        let (Some(pa), Some(pb), Some(pc)) = (
            screen.get(a as usize),
            screen.get(b as usize),
            screen.get(c as usize),
        ) else {
            continue;
        };
        let (Some(wa), Some(wb), Some(wc)) = (
            points.get(a as usize),
            points.get(b as usize),
            points.get(c as usize),
        ) else {
            continue;
        };
        let normal = normalise(cross(sub(*wb, *wa), sub(*wc, *wa)));
        let shade = 0.45 + 0.55 * dot(normal, light).abs();
        let rgb = hsl_to_rgb(model.face_colors.get(face).copied().unwrap_or(0), 1.0);
        let colour = [
            (rgb[0] as f32 * shade) as u8,
            (rgb[1] as f32 * shade) as u8,
            (rgb[2] as f32 * shade) as u8,
            255,
        ];
        fill_triangle(&mut picture, &mut depth, *pa, *pb, *pc, colour);
    }
    picture
}

/// Lay tiles out in a grid with a label under each one.
pub fn sheet(tiles: &[(String, Picture)], columns: usize, tile: usize) -> Picture {
    let columns = columns.max(1);
    let rows = tiles.len().div_ceil(columns).max(1);
    let label_h = 12;
    let mut out = Picture::new(
        columns * (tile + 4),
        rows * (tile + label_h + 4),
        [20, 20, 24, 255],
    );
    for (i, (label, picture)) in tiles.iter().enumerate() {
        let x = (i % columns) * (tile + 4) + 2;
        let y = (i / columns) * (tile + label_h + 4) + 2;
        out.draw(picture, x as i32, y as i32);
        out.label(label, x as i32 + 2, (y + tile + 2) as i32, WHITE);
    }
    out
}

fn fill_triangle(
    picture: &mut Picture,
    depth: &mut [f32],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    colour: Rgba,
) {
    let width = picture.width as i32;
    let height = picture.height as i32;
    let min_x = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i32;
    let max_x = (a[0].max(b[0]).max(c[0]).ceil() as i32).min(width - 1);
    let min_y = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i32;
    let max_y = (a[1].max(b[1]).max(c[1]).ceil() as i32).min(height - 1);
    let area = edge(a, b, c[0], c[1]);
    if area.abs() < 1e-6 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = a[2] * w0 + b[2] * w1 + c[2] * w2;
            let index = y as usize * picture.width + x as usize;
            // View Z points away from the camera after the pitch, so the
            // larger value is nearer.
            if z > depth[index] {
                depth[index] = z;
                picture.set(x, y, colour);
            }
        }
    }
}

fn edge(a: [f32; 3], b: [f32; 3], px: f32, py: f32) -> f32 {
    (b[0] - a[0]) * (py - a[1]) - (b[1] - a[1]) * (px - a[0])
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalise(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length < 1e-9 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / length, v[1] / length, v[2] / length]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_triangle_fills_only_inside_its_edges() {
        let mut picture = Picture::new(8, 8, [0, 0, 0, 255]);
        let mut depth = vec![f32::MIN; 64];
        fill_triangle(
            &mut picture,
            &mut depth,
            [0.0, 0.0, 0.0],
            [8.0, 0.0, 0.0],
            [0.0, 8.0, 0.0],
            [255, 0, 0, 255],
        );
        assert_eq!(picture.get(1, 1), [255, 0, 0, 255]);
        assert_eq!(picture.get(7, 7), [0, 0, 0, 255]);
    }

    #[test]
    fn the_nearer_face_wins_the_depth_test() {
        let mut picture = Picture::new(4, 4, [0, 0, 0, 255]);
        let mut depth = vec![f32::MIN; 16];
        let quad = |z| ([-1.0, -1.0, z], [9.0, -1.0, z], [-1.0, 9.0, z]);
        let (a, b, c) = quad(5.0);
        fill_triangle(&mut picture, &mut depth, a, b, c, [0, 255, 0, 255]);
        let (a, b, c) = quad(1.0);
        fill_triangle(&mut picture, &mut depth, a, b, c, [255, 0, 0, 255]);
        assert_eq!(picture.get(1, 1), [0, 255, 0, 255]);
    }
}
