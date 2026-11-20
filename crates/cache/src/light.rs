//! Lighting a model the way the game lights it.
//!
//! The game does not paint a triangle one colour. It works out how much light
//! reaches each *corner* of the model, darkens or brightens the face's colour
//! by that amount at each corner, and lets the colour run smoothly across the
//! triangle between them. That is what gives its models their rounded look.
//!
//! The amount of light is a "lightness" where 64 is a surface edge-on to the
//! sun, higher is brighter, lower is darker. It multiplies the lightness part
//! of the face's own colour, so a surface in shadow keeps its hue and only
//! grows darker.
//!
//! Points given to [`light`] are in the client's world axes: y up, with the
//! model's own y already flipped. That flip mirrors the model, so a face's
//! outward side is the *negative* of the usual cross product of its edges.

use crate::model::hsl_to_rgb;

/// Light that reaches a surface edge-on to the sun.
pub const AMBIENT: f32 = 64.0;
/// How strongly facing the sun matters. The game's own default; a larger
/// number is a softer light.
pub const CONTRAST: f32 = 768.0;
/// The direction *towards* the sun, in world axes (y up). The game's own
/// default: low in the sky, off one corner of the map.
pub const SUN: [f32; 3] = [-50.0, 10.0, -50.0];
/// The game's brightness setting, as the power its palette is raised to.
/// Smaller is brighter. 0.8 is the game's second-darkest of four.
pub const GAMMA: f32 = 0.8;

/// How much a corner's lightness moves between facing the sun and facing away.
const SMOOTH_SWING: f32 = 256.0 / (CONTRAST / 256.0);
/// Flat faces are lit more gently, as the game does.
const FLAT_SWING: f32 = SMOOTH_SWING / 1.5;

/// The lightness of every corner and every face of one model.
pub struct Lit {
    vertex: Vec<f32>,
    face: Vec<f32>,
}

impl Lit {
    /// The lightness at the three corners of a face. A face the model marks
    /// as flat gets one value for all three.
    pub fn corners(&self, face_idx: usize, face: [u32; 3], render_type: u8) -> [f32; 3] {
        if render_type & 1 == 1 {
            let flat = self.face.get(face_idx).copied().unwrap_or(AMBIENT);
            return [flat; 3];
        }
        face.map(|v| self.vertex.get(v as usize).copied().unwrap_or(AMBIENT))
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f32; 3]) -> [f32; 3] {
    let len = dot(a, a).sqrt();
    if len > 0.0 {
        [a[0] / len, a[1] / len, a[2] / len]
    } else {
        [0.0; 3]
    }
}

/// The outward side of a face, for points in the client's mirrored axes.
fn outward(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let (u, v) = (sub(b, a), sub(c, a));
    unit([
        -(u[1] * v[2] - u[2] * v[1]),
        -(u[2] * v[0] - u[0] * v[2]),
        -(u[0] * v[1] - u[1] * v[0]),
    ])
}

/// Light a model. `points` are its corners turned to face the way it is drawn
/// (where it stands does not matter); `faces` index into them.
///
/// A corner's light comes from the average of the faces that meet there, as in
/// the game, so a sharp edge is lit a little less strongly than a smooth curve.
pub fn light(points: &[[f32; 3]], faces: &[[u32; 3]]) -> Lit {
    let sun = unit(SUN);
    let mut sum = vec![[0.0f32; 3]; points.len()];
    let mut count = vec![0u32; points.len()];
    let mut face = Vec::with_capacity(faces.len());

    for &[a, b, c] in faces {
        let (ia, ib, ic) = (a as usize, b as usize, c as usize);
        if ia >= points.len() || ib >= points.len() || ic >= points.len() {
            face.push(AMBIENT);
            continue;
        }
        let n = outward(points[ia], points[ib], points[ic]);
        face.push(AMBIENT + FLAT_SWING * dot(sun, n));
        for i in [ia, ib, ic] {
            sum[i] = [sum[i][0] + n[0], sum[i][1] + n[1], sum[i][2] + n[2]];
            count[i] += 1;
        }
    }

    let vertex = sum
        .iter()
        .zip(&count)
        .map(|(n, &k)| {
            if k == 0 {
                AMBIENT
            } else {
                AMBIENT + SMOOTH_SWING * dot(sun, *n) / k as f32
            }
        })
        .collect();

    Lit { vertex, face }
}

/// One of the game's colours under a given lightness.
pub fn shade(hsl: u16, lightness: f32) -> [u8; 3] {
    let own = (hsl & 127) as f32;
    let lit = (own * lightness / 128.0).round().clamp(2.0, 126.0) as u16;
    hsl_to_rgb((hsl & !127) | lit, GAMMA)
}

/// A plain colour under a given lightness, for the stand-in colours the client
/// still uses where a texture belongs. `rgb` is the colour edge-on to the sun.
pub fn tint(rgb: [f32; 3], lightness: f32) -> [u8; 3] {
    let k = (lightness / AMBIENT).clamp(0.25, 1.75);
    rgb.map(|v| (v * k).round().clamp(0.0, 255.0) as u8)
}

/// The starting light for the ground, before the sun is counted.
pub const GROUND_AMBIENT: f32 = 96.0;

/// The lightness of one corner of the ground, from the heights of the four
/// corners around it. `west`/`east` are one tile either side along x,
/// `south`/`north` one tile either side along z, all in world units, as is
/// `tile`, the width of a tile. A slope facing the sun brightens and one
/// facing away darkens, by the same rule and the same sun as the models.
///
/// One thing differs from a model, and it is the game's own habit: the sun's
/// height counts *against* the ground, not for it. Level ground therefore
/// sits a little under its starting light (about 84), which is what keeps the
/// game's grass a deep green and not a bright one.
pub fn ground_lightness(west: f32, east: f32, south: f32, north: f32, tile: f32) -> f32 {
    let normal = unit([west - east, 2.0 * tile, south - north]);
    let sun = unit(SUN);
    let across = sun[0] * normal[0] + sun[2] * normal[2];
    GROUND_AMBIENT + SMOOTH_SWING * (across - sun[1] * normal[1])
}

/// A plain colour (0 to 255 a channel) under a given lightness, by the same
/// rule as [`shade`]: its lightness is scaled, its hue and richness kept. Used
/// for the ground, whose colours are blends and so are not held to the game's
/// coarse colour steps, which would show as bands across a smooth blend.
pub fn shade_rgb(rgb: [f32; 3], lightness: f32) -> [u8; 3] {
    let [r, g, b] = rgb.map(|v| (v / 255.0).clamp(0.0, 1.0));
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let own = (max + min) / 2.0;
    let spread = max - min;
    let (hue, richness) = if spread <= f32::EPSILON {
        (0.0, 0.0)
    } else {
        let richness = if own > 0.5 {
            spread / (2.0 - max - min)
        } else {
            spread / (max + min)
        };
        let hue = if max == r {
            ((g - b) / spread).rem_euclid(6.0)
        } else if max == g {
            (b - r) / spread + 2.0
        } else {
            (r - g) / spread + 4.0
        };
        (hue, richness)
    };

    let lit = (own * lightness / 128.0).clamp(2.0 / 128.0, 126.0 / 128.0);
    let chroma = (1.0 - (2.0 * lit - 1.0).abs()) * richness;
    let x = chroma * (1.0 - (hue % 2.0 - 1.0).abs());
    let floor = lit - chroma / 2.0;
    let (r, g, b) = match hue as i32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    [r, g, b].map(|v| ((v + floor).clamp(0.0, 1.0).powf(GAMMA) * 255.0).round() as u8)
}

/// Soften a field of ground colours, as the game does: each tile takes the
/// average of the tiles within `radius` of it. A tile with no colour of its
/// own (`None`) adds nothing to its neighbours and stays `None`.
pub fn blend(
    field: &[Option<[f32; 3]>],
    width: usize,
    height: usize,
    radius: usize,
) -> Vec<Option<[f32; 3]>> {
    let mut out = vec![None; field.len()];
    for y in 0..height {
        for x in 0..width {
            if field[y * width + x].is_none() {
                continue;
            }
            let (mut sum, mut count) = ([0.0f32; 3], 0.0f32);
            for ny in y.saturating_sub(radius)..=(y + radius).min(height - 1) {
                for nx in x.saturating_sub(radius)..=(x + radius).min(width - 1) {
                    if let Some(c) = field[ny * width + nx] {
                        sum = [sum[0] + c[0], sum[1] + c[1], sum[2] + c[2]];
                        count += 1.0;
                    }
                }
            }
            out[y * width + x] = Some(sum.map(|v| v / count));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square lying flat, seen from above, wound the way the client's
    /// mirrored models are: its outward side is up.
    fn floor() -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        (
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
    }

    #[test]
    fn a_face_turned_to_the_sun_is_brighter_than_one_turned_away() {
        // two walls back to back, one facing the sun's corner, one facing away
        let toward = vec![[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 1.0]];
        let away = vec![toward[1], toward[0], [1.0, 1.0, 0.0]];
        let a = light(&toward, &[[0, 1, 2]]);
        let b = light(&away, &[[0, 1, 2]]);
        let (la, lb) = (a.corners(0, [0, 1, 2], 0)[0], b.corners(0, [0, 1, 2], 0)[0]);
        assert!(
            (la - AMBIENT).abs() > 40.0,
            "a wall square to the sun moves well off ambient"
        );
        assert!(
            (la - AMBIENT + lb - AMBIENT).abs() < 0.01,
            "back to back walls are lit opposite"
        );
        assert!(la.max(lb) > AMBIENT && la.min(lb) < AMBIENT);
    }

    #[test]
    fn the_top_of_a_thing_is_a_little_brighter_than_ambient() {
        let (points, faces) = floor();
        let lit = light(&points, &faces);
        for l in lit.corners(0, faces[0], 0) {
            assert!(l > AMBIENT && l < AMBIENT + 20.0, "the sun is low: {l}");
        }
    }

    #[test]
    fn a_smooth_face_blends_and_a_flat_face_does_not() {
        // a ridge: two faces meeting at an angle along one shared edge
        let points = vec![
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [-1.0, 0.0, 0.5],
            [1.0, 0.0, 0.5],
        ];
        let faces = vec![[0, 1, 2], [1, 0, 3]];
        let lit = light(&points, &faces);
        let smooth = lit.corners(0, faces[0], 0);
        assert!(
            (smooth[0] - smooth[2]).abs() > 1.0,
            "the shared edge differs from the far corner"
        );
        let flat = lit.corners(0, faces[0], 1);
        assert!(flat[0] == flat[1] && flat[1] == flat[2]);
    }

    #[test]
    fn where_a_model_stands_does_not_change_its_light() {
        let (points, faces) = floor();
        let far: Vec<[f32; 3]> = points
            .iter()
            .map(|p| [p[0] + 300.0, p[1] + 40.0, p[2] - 700.0])
            .collect();
        let (a, b) = (light(&points, &faces), light(&far, &faces));
        assert_eq!(a.corners(1, faces[1], 0), b.corners(1, faces[1], 0));
    }

    #[test]
    fn shadow_darkens_a_colour_and_keeps_its_hue() {
        let brown: u16 = (6 << 10) | (4 << 7) | 60;
        let (dark, mid, bright) = (shade(brown, 20.0), shade(brown, 64.0), shade(brown, 120.0));
        let total = |c: [u8; 3]| c.iter().map(|&v| v as u32).sum::<u32>();
        assert!(total(dark) < total(mid) && total(mid) < total(bright));
        for c in [dark, mid, bright] {
            assert!(c[0] >= c[1] && c[1] >= c[2], "still a brown: {c:?}");
        }
    }

    #[test]
    fn a_bad_face_is_lit_plainly_and_does_not_panic() {
        let (points, _) = floor();
        let lit = light(&points, &[[0, 1, 99]]);
        assert_eq!(lit.corners(0, [0, 1, 99], 0), [AMBIENT; 3]);
    }

    #[test]
    fn a_stand_in_colour_follows_the_light_too() {
        assert_eq!(tint([100.0, 80.0, 60.0], AMBIENT), [100, 80, 60]);
        assert!(tint([100.0, 80.0, 60.0], 30.0)[0] < 100);
        assert!(tint([100.0, 80.0, 60.0], 100.0)[0] > 100);
    }

    #[test]
    fn level_ground_is_evenly_lit_at_the_games_own_level() {
        let level = ground_lightness(40.0, 40.0, 40.0, 40.0, 128.0);
        assert!(
            (level - 84.0).abs() < 1.0,
            "the game lights level ground at about 84: {level}"
        );
        assert_eq!(level, ground_lightness(900.0, 900.0, 900.0, 900.0, 128.0));
    }

    #[test]
    fn a_slope_facing_the_sun_is_brighter_than_one_facing_away() {
        let level = ground_lightness(0.0, 0.0, 0.0, 0.0, 128.0);
        // the sun stands towards -x and -z: ground that rises towards +x faces it
        let facing = ground_lightness(0.0, 200.0, 0.0, 0.0, 128.0);
        let away = ground_lightness(200.0, 0.0, 0.0, 0.0, 128.0);
        assert!(facing > level && level > away, "{facing} {level} {away}");
        let facing_z = ground_lightness(0.0, 0.0, 0.0, 200.0, 128.0);
        assert!(
            (facing - facing_z).abs() < 0.01,
            "the sun is square to neither axis"
        );
    }

    #[test]
    fn the_ground_and_a_model_agree_about_a_surface() {
        // the same tilted square, once as ground heights and once as a model
        let rise = 100.0;
        let ground = ground_lightness(0.0, 2.0 * rise, 0.0, 0.0, 128.0);
        let points = vec![
            [0.0, 0.0, 0.0],
            [256.0, 2.0 * rise, 0.0],
            [256.0, 2.0 * rise, 256.0],
            [0.0, 0.0, 256.0],
        ];
        let faces = vec![[0, 1, 2], [0, 2, 3]];
        let model = light(&points, &faces).corners(0, faces[0], 0)[0];
        // measured from each one's own level surface, so the game's different
        // starting light for ground and model does not come into it; what is
        // left over is the small part the sun's height plays
        let level_ground = ground_lightness(0.0, 0.0, 0.0, 0.0, 128.0);
        let flat = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        let level_model = light(&flat, &faces).corners(0, faces[0], 0)[0];
        let (ground, model) = (ground - level_ground, model - level_model);
        assert!(
            ground > 20.0 && model > 20.0,
            "both brighten: {ground} {model}"
        );
        assert!(
            (ground - model).abs() < 6.0,
            "and by nearly the same: {ground} {model}"
        );
    }

    #[test]
    fn ground_colour_darkens_in_shadow_and_keeps_its_hue() {
        let grass = [60.0, 120.0, 40.0];
        let (dark, mid, bright) = (
            shade_rgb(grass, 40.0),
            shade_rgb(grass, 96.0),
            shade_rgb(grass, 150.0),
        );
        let total = |c: [u8; 3]| c.iter().map(|&v| v as u32).sum::<u32>();
        assert!(total(dark) < total(mid) && total(mid) < total(bright));
        for c in [dark, mid, bright] {
            assert!(c[1] > c[0] && c[0] > c[2], "still a grass green: {c:?}");
        }
        let grey = shade_rgb([128.0, 128.0, 128.0], 96.0);
        assert!(grey[0] == grey[1] && grey[1] == grey[2], "grey stays grey");
    }

    #[test]
    fn near_colours_shade_to_near_colours() {
        // no bands: a small step in colour is a small step after shading
        let mut last = shade_rgb([60.0, 120.0, 40.0], 100.0);
        for i in 1..=40 {
            let t = i as f32 / 40.0;
            let c = shade_rgb([60.0 + 60.0 * t, 120.0 - 50.0 * t, 40.0], 100.0);
            for k in 0..3 {
                assert!(
                    (c[k] as i32 - last[k] as i32).abs() <= 3,
                    "{last:?} -> {c:?}"
                );
            }
            last = c;
        }
    }

    #[test]
    fn blending_softens_an_edge_and_leaves_gaps_alone() {
        // a row: five green, a gap, five brown
        let (green, brown) = ([0.0, 100.0, 0.0], [100.0, 50.0, 0.0]);
        let mut row = vec![Some(green); 5];
        row.push(None);
        row.extend(vec![Some(brown); 5]);
        let out = blend(&row, 11, 1, 2);
        assert_eq!(out[5], None, "a tile with no colour stays without one");
        assert_eq!(out[0], Some(green), "far from the edge nothing changes");
        let near = out[4].unwrap();
        assert!(
            near[0] > 0.0 && near[0] < 50.0,
            "next to the edge it leans brown: {near:?}"
        );
        let one = blend(&[Some(green)], 1, 1, 5);
        assert_eq!(one[0], Some(green));
    }
}
