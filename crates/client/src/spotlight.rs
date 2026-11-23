//! The hero-select spotlight: the selected hero, animated, in the middle of
//! the screen.
//!
//! The hero is drawn with the same tiny CPU rasteriser as the roster
//! portraits (flat-shaded cache faces and a z-buffer), so it stays
//! cache-native and needs no second render target. Each new frame is
//! streamed into a reserved slot of the HUD atlas
//! ([`crate::ui_assets::SPOTLIGHT_RECT`]). Picking a hero plays its intro (the
//! cast clip, else the attack clip) once, then the idle loop; the hero sways
//! slowly like an old character render on a turntable.

use anyhow::{Context, Result};
use glam::{Mat3, Vec3};
use rift_cache::{AnimationClip, Cache, Model};

use crate::champion::ChampionConfig;
use crate::ui_assets::SPOTLIGHT_RECT;

/// Supersampling factor of the CPU raster before the 2x2 box filter.
const SUPERSAMPLE: u32 = 2;
/// New frames per second (the HUD itself redraws every frame).
const FRAME_RATE: f32 = 24.0;
/// Fraction of the slot the hero's widest pose may fill.
const FILL_W: f32 = 0.80;
const FILL_H: f32 = 0.78;
/// Where the hero's feet stand, as a fraction of the slot height.
const FLOOR: f32 = 0.90;

/// How a hero's poses map onto the slot: fixed per hero so the intro does
/// not zoom the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Framing {
    /// Engine-space point drawn at the slot's floor centre.
    pub anchor: Vec3,
    /// Pixels (in the supersampled raster) per model unit.
    pub scale: f32,
}

/// Engine-space points of a pose (cache Y points down).
fn engine_points(positions: &[[f32; 3]]) -> Vec<Vec3> {
    positions
        .iter()
        .map(|p| Vec3::new(p[0], -p[1], p[2]))
        .collect()
}

/// The three-quarter view of an old character render, swayed by `yaw`.
fn view(yaw: f32) -> Mat3 {
    Mat3::from_rotation_y(0.42 + yaw) * Mat3::from_rotation_x(0.10)
}

/// Fit every pose in `poses` (seen from a range of sway angles) into a
/// `width` x `height` raster with the feet on the floor line.
pub fn frame_poses(poses: &[Vec<[f32; 3]>], width: f32, height: f32) -> Framing {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for pose in poses {
        for yaw in [-0.3, 0.0, 0.3] {
            let rot = view(yaw);
            for point in engine_points(pose) {
                let p = rot * point;
                min = min.min(p);
                max = max.max(p);
            }
        }
    }
    if min.x > max.x {
        return Framing {
            anchor: Vec3::ZERO,
            scale: 1.0,
        };
    }
    let extent = (max - min).max(Vec3::splat(1.0));
    let scale = (width * FILL_W / extent.x).min(height * FILL_H / extent.y);
    Framing {
        anchor: Vec3::new((min.x + max.x) * 0.5, min.y, 0.0),
        scale: scale.max(0.01),
    }
}

/// Rasterise one pose into an RGBA `width` x `height` image (premultiplied
/// box filter of a supersampled raster), transparent around the hero.
pub fn render_pose(
    model: &Model,
    positions: &[[f32; 3]],
    yaw: f32,
    framing: &Framing,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let sw = width * SUPERSAMPLE;
    let sh = height * SUPERSAMPLE;
    let rot = view(yaw);
    let floor = Vec3::new(sw as f32 * 0.5, sh as f32 * FLOOR, 0.0);
    let s = framing.scale * SUPERSAMPLE as f32;
    let screen: Vec<Vec3> = engine_points(positions)
        .into_iter()
        .map(|point| {
            let p = rot * point - framing.anchor;
            Vec3::new(floor.x + p.x * s, floor.y - p.y * s, p.z * s)
        })
        .collect();

    let mut color = vec![[0u8; 4]; (sw * sh) as usize];
    let mut depth = vec![f32::NEG_INFINITY; (sw * sh) as usize];
    // Warm key light from the upper left, cool fill: the lighting of the
    // old pre-rendered character art.
    let key = Vec3::new(-0.45, 0.75, 0.55).normalize();
    let fill = Vec3::new(0.7, 0.1, 0.4).normalize();

    for (face, &[a, b, c]) in model.faces.iter().enumerate() {
        let (Some(&pa), Some(&pb), Some(&pc)) = (
            screen.get(a as usize),
            screen.get(b as usize),
            screen.get(c as usize),
        ) else {
            continue;
        };
        let area = edge(pa, pb, pc);
        if area.abs() < 1e-5 {
            continue;
        }
        // Screen Y points down, so flip it back for lighting.
        let mut normal = Vec3::new(pb.x - pa.x, pa.y - pb.y, pb.z - pa.z)
            .cross(Vec3::new(pc.x - pa.x, pa.y - pc.y, pc.z - pa.z))
            .normalize_or_zero();
        if normal.z < 0.0 {
            normal = -normal;
        }
        let shade = (0.42 + normal.dot(key).max(0.0) * 0.78 + normal.dot(fill).max(0.0) * 0.18)
            .clamp(0.35, 1.3);
        let base = face_color(model, face);
        let warm = [1.04, 1.0, 0.92];
        let rgba = [
            (base[0] as f32 * shade * warm[0]).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * shade * warm[1]).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * shade * warm[2]).clamp(0.0, 255.0) as u8,
            255,
        ];
        let x0 = pa.x.min(pb.x).min(pc.x).floor().max(0.0) as u32;
        let x1 = pa.x.max(pb.x).max(pc.x).ceil().min(sw as f32 - 1.0);
        let y0 = pa.y.min(pb.y).min(pc.y).floor().max(0.0) as u32;
        let y1 = pa.y.max(pb.y).max(pc.y).ceil().min(sh as f32 - 1.0);
        if x1 < 0.0 || y1 < 0.0 {
            continue;
        }
        let inv = 1.0 / area;
        for py in y0..=y1 as u32 {
            for px in x0..=x1 as u32 {
                let p = Vec3::new(px as f32 + 0.5, py as f32 + 0.5, 0.0);
                let w0 = edge(pb, pc, p) * inv;
                let w1 = edge(pc, pa, p) * inv;
                let w2 = edge(pa, pb, p) * inv;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * pa.z + w1 * pb.z + w2 * pc.z;
                let index = (py * sw + px) as usize;
                if z > depth[index] {
                    depth[index] = z;
                    color[index] = rgba;
                }
            }
        }
    }
    downsample(&color, sw, sh)
}

fn face_color(model: &Model, face: usize) -> [u8; 3] {
    let textured = model
        .face_textures
        .as_ref()
        .and_then(|t| t.get(face))
        .is_some_and(|&t| t >= 0);
    if textured || model.face_colors.get(face).copied().unwrap_or(0) == 127 {
        return [126, 142, 110];
    }
    let rgb = model.face_rgb(face, 1.0);
    let peak = rgb[0].max(rgb[1]).max(rgb[2]) as f32;
    if peak < 70.0 {
        let lift = 70.0 / peak.max(1.0);
        rgb.map(|c| (c as f32 * lift).min(255.0) as u8)
    } else {
        rgb
    }
}

/// 2x2 premultiplied box filter.
fn downsample(color: &[[u8; 4]], sw: u32, sh: u32) -> Vec<u8> {
    let (w, h) = (sw / SUPERSAMPLE, sh / SUPERSAMPLE);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let mut rgb = [0.0f32; 3];
            let mut alpha = 0.0f32;
            for sy in 0..SUPERSAMPLE {
                for sx in 0..SUPERSAMPLE {
                    let p = color[((y * SUPERSAMPLE + sy) * sw + x * SUPERSAMPLE + sx) as usize];
                    let a = p[3] as f32 / 255.0;
                    for i in 0..3 {
                        rgb[i] += p[i] as f32 * a;
                    }
                    alpha += a;
                }
            }
            let inv = if alpha > 1e-4 { 1.0 / alpha } else { 0.0 };
            let o = ((y * w + x) * 4) as usize;
            for i in 0..3 {
                out[o + i] = (rgb[i] * inv).clamp(0.0, 255.0) as u8;
            }
            let samples = (SUPERSAMPLE * SUPERSAMPLE) as f32;
            out[o + 3] = (alpha / samples * 255.0).clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// Twice the signed area of the triangle `(a, b, p)`.
fn edge(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

struct Loaded {
    id: String,
    model: Model,
    idle: AnimationClip,
    intro: Option<AnimationClip>,
    framing: Framing,
}

/// The animated spotlight for the hero-select screen.
#[derive(Default)]
pub struct Spotlight {
    loaded: Option<Loaded>,
    /// Seconds since the hero was picked.
    time: f32,
    /// Seconds until the next frame is due.
    frame_timer: f32,
    pixels: Vec<u8>,
}

impl Spotlight {
    /// The champion currently shown, by file id.
    pub fn showing(&self) -> Option<&str> {
        self.loaded.as_ref().map(|loaded| loaded.id.as_str())
    }

    /// Show `champion`, holding `weapon` (wield parts) when it has one.
    pub fn show(
        &mut self,
        cache: &Cache,
        champion: &ChampionConfig,
        weapon: &[Model],
    ) -> Result<()> {
        let mut parts = Vec::with_capacity(1 + champion.attachments.len() + weapon.len());
        for id in std::iter::once(champion.model).chain(champion.attachments.iter().copied()) {
            let group = cache
                .group(7, id)
                .with_context(|| format!("loading model {id}"))?;
            parts.push(Model::load(id, &group.data)?);
        }
        let idle = AnimationClip::load(cache, champion.idle)
            .with_context(|| format!("loading idle {}", champion.idle))?;
        if crate::wield::can_wield(&idle) {
            parts.extend(weapon.iter().cloned());
        }
        let model = Model::merge(&parts);
        let intro = champion
            .cast_anim
            .or(champion.attack_anim)
            .and_then(|id| AnimationClip::load(cache, id).ok());
        let mut poses = vec![model.animated_positions(&idle, 0.0)];
        if let Some(intro) = &intro {
            let length = intro.loop_ms();
            for i in 0..8 {
                poses.push(model.animated_positions(intro, length * i as f32 / 8.0));
            }
        }
        let (_, _, w, h) = SPOTLIGHT_RECT;
        let framing = frame_poses(&poses, (w * SUPERSAMPLE) as f32, (h * SUPERSAMPLE) as f32);
        let framing = Framing {
            scale: framing.scale / SUPERSAMPLE as f32,
            ..framing
        };
        self.loaded = Some(Loaded {
            id: champion.id.clone(),
            model,
            idle,
            intro,
            framing,
        });
        self.time = 0.0;
        self.frame_timer = 0.0;
        Ok(())
    }

    /// Advance the animation. Returns the new RGBA frame for the spotlight
    /// slot when one is due.
    pub fn update(&mut self, dt: f32) -> Option<&[u8]> {
        let loaded = self.loaded.as_ref()?;
        self.time += dt;
        self.frame_timer -= dt;
        if self.frame_timer > 0.0 {
            return None;
        }
        self.frame_timer += 1.0 / FRAME_RATE;
        if self.frame_timer < 0.0 {
            self.frame_timer = 1.0 / FRAME_RATE;
        }
        let ms = self.time * 1000.0;
        // A short beat before the intro, so the pick reads as an entrance.
        let intro_start = 150.0;
        let positions = match &loaded.intro {
            Some(intro) if ms >= intro_start && ms - intro_start < intro.loop_ms() => {
                loaded.model.animated_positions(intro, ms - intro_start)
            }
            _ => loaded.model.animated_positions(&loaded.idle, ms),
        };
        let yaw = (self.time * 0.55).sin() * 0.28;
        let (_, _, w, h) = SPOTLIGHT_RECT;
        self.pixels = render_pose(&loaded.model, &positions, yaw, &loaded.framing, w, h);
        Some(&self.pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_stands_the_tallest_pose_on_the_floor_inside_the_slot() {
        // A 100-unit tall, 40-wide box (cache Y is negative upward).
        let mut pose = Vec::new();
        for x in [-20.0, 20.0] {
            for y in [0.0, -100.0] {
                for z in [-10.0, 10.0] {
                    pose.push([x, y, z]);
                }
            }
        }
        let framing = frame_poses(&[pose], 400.0, 500.0);
        assert!(framing.scale > 0.0);
        assert!(100.0 * framing.scale <= 500.0 * FILL_H + 1.0);
        assert!(framing.anchor.y.abs() < 20.0, "feet anchor near y = 0");
    }

    #[test]
    fn an_empty_model_renders_a_transparent_frame_of_the_right_size() {
        let model = Model::merge(&[]);
        let framing = Framing {
            anchor: Vec3::ZERO,
            scale: 1.0,
        };
        let pixels = render_pose(&model, &[], 0.0, &framing, 8, 6);
        assert_eq!(pixels.len(), 8 * 6 * 4);
        assert!(pixels.chunks(4).all(|p| p[3] == 0));
    }
}
