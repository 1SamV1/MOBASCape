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
}
