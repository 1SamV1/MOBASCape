//! Cache-native inventory icons.
//!
//! OSRS does not store item icons as sprites: the client renders the item's
//! inventory model into a 32 by 32 picture. This is the same idea, with the
//! existing cache model decoder and a small CPU rasteriser. The output is
//! uploaded to the interface theme under `icon.item.<key>`.

use anyhow::{Context, Result};
use glam::{Mat3, Vec3};
use rift_cache::{Cache, ItemDefinition, Model};
use rift_ui::atlas::Image;

/// Native inventory icon size. The interface draws it at whole-number scale.
pub const ICON_SIZE: u32 = 32;
const LIGHT: Vec3 = Vec3::new(-0.35, 0.78, 0.52);

pub fn bake(cache: &Cache, item_id: u32) -> Result<Image> {
    let data = cache
        .config(10, item_id)
        .with_context(|| format!("reading item {item_id}"))?;
    let def =
        ItemDefinition::parse(item_id, &data).with_context(|| format!("parsing item {item_id}"))?;
    if def.model == 0 {
        anyhow::bail!("item {item_id} ({}) has no inventory model", def.name);
    }
    let group = cache
        .group(7, def.model)
        .with_context(|| format!("reading item model {}", def.model))?;
    let mut model = Model::load(def.model, &group.data)
        .with_context(|| format!("decoding item model {}", def.model))?;

    if !def.recolors.is_empty() {
        let find: Vec<i16> = def.recolors.iter().map(|&(find, _)| find as i16).collect();
        let replace: Vec<i16> = def
            .recolors
            .iter()
            .map(|&(_, replace)| replace as i16)
            .collect();
        model.recolor(&find, &replace);
    }

    render(&model, &def)
}

fn render(model: &Model, def: &ItemDefinition) -> Result<Image> {
    let sx = def.scale_x.max(1) as f32 / 128.0;
    let sy = def.scale_y.max(1) as f32 / 128.0;
    let sz = def.scale_z.max(1) as f32 / 128.0;
    let points: Vec<Vec3> = (0..model.vertex_count)
        .map(|i| {
            Vec3::new(
                model.vertex_x[i] as f32 * sx,
                -(model.vertex_y[i] as f32) * sy,
                model.vertex_z[i] as f32 * sz,
            )
        })
        .collect();
    if points.is_empty() || model.faces.is_empty() {
        anyhow::bail!("item model has no geometry");
    }

    let turn = std::f32::consts::TAU / 2048.0;
    let rot = Mat3::from_rotation_y(def.rotation_y as f32 * turn)
        * Mat3::from_rotation_x(def.rotation_x as f32 * turn)
        * Mat3::from_rotation_z(def.rotation_z as f32 * turn);
    let rotated: Vec<Vec3> = points.iter().map(|point| rot * *point).collect();

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for point in &rotated {
        min = min.min(*point);
        max = max.max(*point);
    }
    let extent = (max - min).max(Vec3::splat(1.0));
    let scale = ((ICON_SIZE as f32 - 4.0) / extent.x)
        .min((ICON_SIZE as f32 - 4.0) / extent.y)
        .max(0.01);
    let mid = (min + max) * 0.5;
    let half = ICON_SIZE as f32 * 0.5;
    let project = |point: Vec3| {
        let p = point - mid;
        Vec3::new(half + p.x * scale, half - p.y * scale, p.z)
    };
    let screen: Vec<Vec3> = rotated.iter().map(|point| project(*point)).collect();

    let mut pixels = vec![[0u8; 4]; (ICON_SIZE * ICON_SIZE) as usize];
    let mut depth = vec![f32::NEG_INFINITY; (ICON_SIZE * ICON_SIZE) as usize];
    let light = LIGHT.normalize();

    for (face, &[a, b, c]) in model.faces.iter().enumerate() {
        let pa = screen[a as usize];
        let pb = screen[b as usize];
        let pc = screen[c as usize];
        let area = (pb.x - pa.x) * (pc.y - pa.y) - (pb.y - pa.y) * (pc.x - pa.x);
        if area.abs() < 1e-4 {
            continue;
        }
        let normal = (pb - pa).cross(pc - pa).normalize_or_zero();
        if normal.length_squared() < 1e-6 {
            continue;
        }
        let shade = (0.92 + 0.28 * normal.dot(light).abs()).clamp(0.78, 1.2);
        let colour = model.face_rgb(face, shade);
        let rgba = [colour[0], colour[1], colour[2], 255];

        let min_x = pa.x.min(pb.x).min(pc.x).floor().max(0.0) as i32;
        let max_x = pa.x.max(pb.x).max(pc.x).ceil().min(ICON_SIZE as f32) as i32;
        let min_y = pa.y.min(pb.y).min(pc.y).floor().max(0.0) as i32;
        let max_y = pa.y.max(pb.y).max(pc.y).ceil().min(ICON_SIZE as f32) as i32;
        for y in min_y..max_y {
            for x in min_x..max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = ((pb.x - px) * (pc.y - py) - (pb.y - py) * (pc.x - px)) / area;
                let w1 = ((pc.x - px) * (pa.y - py) - (pc.y - py) * (pa.x - px)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                    continue;
                }
                let z = w0 * pa.z + w1 * pb.z + w2 * pc.z;
                let index = y as usize * ICON_SIZE as usize + x as usize;
                if z > depth[index] {
                    depth[index] = z;
                    pixels[index] = rgba;
                }
            }
        }
    }

    // The game puts a one-pixel dark edge around an inventory icon.
    let filled = pixels.clone();
    let edge = [34, 28, 26, 255];
    for y in 0..ICON_SIZE as i32 {
        for x in 0..ICON_SIZE as i32 {
            let index = y as usize * ICON_SIZE as usize + x as usize;
            if filled[index][3] != 0 {
                continue;
            }
            let neighbour = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
            if neighbour.iter().any(|&(nx, ny)| {
                nx >= 0
                    && ny >= 0
                    && nx < ICON_SIZE as i32
                    && ny < ICON_SIZE as i32
                    && filled[ny as usize * ICON_SIZE as usize + nx as usize][3] != 0
            }) {
                pixels[index] = edge;
            }
        }
    }

    Ok(Image::new(ICON_SIZE as usize, ICON_SIZE as usize, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "writes item icon pictures for the eye"]
    fn item_icons_can_be_written_for_the_eye() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cache_path = root.join(".cache/osrs-235/cache");
        if !cache_path.join("main_file_cache.dat2").exists() {
            eprintln!("no pinned cache; nothing to draw");
            return;
        }
        let cache = Cache::open(&cache_path).expect("the pinned cache opens");
        for id in [1277u32, 1115, 6585, 6737, 4710, 11802, 4151, 20997] {
            match bake(&cache, id) {
                Ok(icon) => {
                    let mut out = image::RgbaImage::new(ICON_SIZE, ICON_SIZE);
                    for (x, y, pixel) in out.enumerate_pixels_mut() {
                        let at = (y * ICON_SIZE + x) as usize;
                        *pixel = image::Rgba(icon.pixels[at]);
                    }
                    let scale = 8;
                    let large = image::imageops::resize(
                        &out,
                        ICON_SIZE * scale,
                        ICON_SIZE * scale,
                        image::imageops::FilterType::Nearest,
                    );
                    let path = format!("/tmp/opencode/item-{id}.png");
                    large.save(&path).expect("write icon");
                    eprintln!("item icon {id} written to {path}");
                }
                Err(err) => eprintln!("item icon {id}: {err:#}"),
            }
        }
    }
}
