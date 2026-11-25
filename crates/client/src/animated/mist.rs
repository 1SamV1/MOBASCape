//! Ground mist over ground the team cannot see (roadmap 5.7).
//!
//! Puffs of the cache smoke cloud (spot animation 354, every face
//! translucent, so it draws in the additive glow pass) sit on a seeded,
//! jittered grid inside the arena walls. Each fades in while its tile is
//! out of team vision and out again once an ally sees it, so the fog of
//! war reads as drifting mist instead of only a darker floor. Only the
//! nearest puffs around the camera are drawn, which keeps the effect
//! inside the fx vertex budget.

use glam::{Mat3, Vec3};

use super::{AnimatedScene, SpotDraw};
use crate::scene::HeightGrid;

/// Grey smoke cloud (model 6288, 27 frames over 1.88 s).
pub(super) const MIST_SPOT: u32 = 354;
/// Grid spacing between puffs, in tiles.
const SPACING: f32 = 5.0;
/// Puffs drawn per frame at most, nearest to the camera first.
const MAX_DRAWN: usize = 48;
/// Share of the fx vertex budget the mist may fill.
const MIST_BUDGET: f32 = 0.5;
/// Puffs farther than this from the camera target are not drawn.
const DRAW_RADIUS: f32 = 22.0 * 128.0;
/// Alpha change per second while fading in or out.
const FADE_RATE: f32 = 1.4;
/// Scale on the cache model, which is under a tile wide at 1.
const PUFF_SCALE: f32 = 6.5;
/// Glow intensity at full alpha. The cloud is additive, so this is how
/// bright the mist gets over dark unseen ground.
const PUFF_GLOW: f32 = 0.06;
/// Vertical squash: a flat bank lying on the ground, not a cloud ball.
const PUFF_FLATTEN: f32 = 0.28;

#[derive(Debug, Clone, PartialEq)]
pub struct MistPuff {
    /// Ground position; `y` is filled from the height grid when drawn.
    pub pos: Vec3,
    pub yaw: f32,
    /// Offset into the loop so neighbours do not billow in step.
    pub phase_ms: f32,
    pub scale: f32,
    /// 0 = gone (the tile is seen), 1 = full mist.
    pub alpha: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Mist {
    pub puffs: Vec<MistPuff>,
    time_ms: f32,
}

/// A small integer hash for deterministic jitter (no RNG state touched).
fn hash(x: i32, z: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (z as u32).wrapping_mul(0x85EB_CA77)
        ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

impl Mist {
    /// Lay puffs over `bounds` (tiles: x0, z0, x1, z1), keeping a two
    /// tile margin inside the walls. Every puff starts at full mist.
    pub fn new(bounds: (f32, f32, f32, f32)) -> Self {
        let (x0, z0, x1, z1) = bounds;
        let margin = 2.0;
        let cols = ((x1 - x0 - 2.0 * margin) / SPACING).floor().max(0.0) as i32;
        let rows = ((z1 - z0 - 2.0 * margin) / SPACING).floor().max(0.0) as i32;
        let mut puffs = Vec::with_capacity(((cols + 1) * (rows + 1)) as usize);
        for gz in 0..=rows {
            for gx in 0..=cols {
                let jx = (hash(gx, gz, 1) - 0.5) * SPACING * 0.8;
                let jz = (hash(gx, gz, 2) - 0.5) * SPACING * 0.8;
                let tx = x0 + margin + gx as f32 * SPACING + jx;
                let tz = z0 + margin + gz as f32 * SPACING + jz;
                puffs.push(MistPuff {
                    pos: Vec3::new(tx * 128.0, 0.0, tz * 128.0),
                    yaw: hash(gx, gz, 3) * std::f32::consts::TAU,
                    phase_ms: hash(gx, gz, 4) * 1880.0,
                    scale: PUFF_SCALE * (0.8 + 0.4 * hash(gx, gz, 5)),
                    alpha: 1.0,
                });
            }
        }
        Self {
            puffs,
            time_ms: 0.0,
        }
    }

    /// Fade each puff towards mist where `seen` says its spot is unseen.
    pub fn update(&mut self, dt: f32, seen: impl Fn(Vec3) -> bool) {
        self.time_ms += dt * 1000.0;
        let step = FADE_RATE * dt;
        for puff in &mut self.puffs {
            let target = if seen(puff.pos) { 0.0 } else { 1.0 };
            puff.alpha += (target - puff.alpha).clamp(-step, step);
        }
    }

    /// Indices of the puffs to draw this frame: visible enough, within
    /// range of `focus`, nearest first, at most `MAX_DRAWN`.
    pub fn drawn(&self, focus: Vec3) -> Vec<usize> {
        let mut near: Vec<(usize, f32)> = self
            .puffs
            .iter()
            .enumerate()
            .filter(|(_, puff)| puff.alpha > 0.02)
            .map(|(i, puff)| {
                let d = (puff.pos.x - focus.x).powi(2) + (puff.pos.z - focus.z).powi(2);
                (i, d)
            })
            .filter(|&(_, d)| d <= DRAW_RADIUS * DRAW_RADIUS)
            .collect();
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        near.truncate(MAX_DRAWN);
        near.into_iter().map(|(i, _)| i).collect()
    }
}

impl AnimatedScene {
    pub(super) fn update_mist(&mut self, dt: f32) {
        let mut mist = std::mem::take(&mut self.mist);
        mist.update(dt, |pos| self.in_team_vision(pos));
        self.mist = mist;
    }

    /// Add this frame's mist puffs to the glow pass.
    pub(super) fn draw_mist(&mut self, height: &HeightGrid, focus: Vec3) {
        let Some(spot) = self.spot_models.get(&MIST_SPOT).cloned() else {
            return;
        };
        let time = self.mist.time_ms;
        let puffs: Vec<_> = self
            .mist
            .drawn(focus)
            .into_iter()
            .map(|i| self.mist.puffs[i].clone())
            .collect();
        for puff in puffs {
            // Mist is scenery: it never takes more than half the fx budget,
            // so spell graphics and projectiles drawn after it still fit.
            if self.fx.used_fraction() > MIST_BUDGET {
                break;
            }
            let pos = Vec3::new(
                puff.pos.x,
                height.height_at(puff.pos.x, puff.pos.z),
                puff.pos.z,
            );
            // Ease the fade so the edge of vision is soft, not a pop.
            let a = puff.alpha * puff.alpha * (3.0 - 2.0 * puff.alpha);
            self.append_spot_model(
                &spot,
                time + puff.phase_ms,
                pos,
                Mat3::from_rotation_y(puff.yaw)
                    * Mat3::from_diagonal(Vec3::new(1.0, PUFF_FLATTEN, 1.0)),
                puff.scale,
                PUFF_GLOW * a,
                SpotDraw::Held,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puffs_cover_the_arena_inside_the_walls() {
        let mist = Mist::new((0.0, 0.0, 100.0, 100.0));
        assert!(mist.puffs.len() >= 300, "{}", mist.puffs.len());
        for puff in &mist.puffs {
            let (tx, tz) = (puff.pos.x / 128.0, puff.pos.z / 128.0);
            assert!((0.0..=100.0).contains(&tx) && (0.0..=100.0).contains(&tz));
        }
        // Seeded: the same bounds lay the same mist.
        assert_eq!(mist.puffs, Mist::new((0.0, 0.0, 100.0, 100.0)).puffs);
    }

    #[test]
    fn mist_fades_out_where_seen_and_back_where_unseen() {
        let mut mist = Mist::new((0.0, 0.0, 40.0, 40.0));
        let seen_left = |pos: Vec3| pos.x < 20.0 * 128.0;
        for _ in 0..60 {
            mist.update(1.0 / 30.0, seen_left);
        }
        for puff in &mist.puffs {
            if puff.pos.x < 20.0 * 128.0 {
                assert_eq!(puff.alpha, 0.0);
            } else {
                assert_eq!(puff.alpha, 1.0);
            }
        }
        // Not a pop: one frame moves alpha by a small step only.
        mist.update(1.0 / 30.0, |_| false);
        assert!(
            mist.puffs
                .iter()
                .all(|p| p.alpha <= FADE_RATE / 30.0 + 1e-6 || p.alpha == 1.0)
        );
    }

    #[test]
    fn only_the_nearest_unseen_puffs_are_drawn() {
        let mut mist = Mist::new((0.0, 0.0, 200.0, 200.0));
        let focus = Vec3::new(100.0 * 128.0, 0.0, 100.0 * 128.0);
        let drawn = mist.drawn(focus);
        assert_eq!(drawn.len(), MAX_DRAWN);
        let far = |i: usize| mist.puffs[i].pos.distance(focus);
        assert!(drawn.windows(2).all(|w| far(w[0]) <= far(w[1])));
        // Everything seen: nothing to draw.
        for _ in 0..60 {
            mist.update(1.0 / 30.0, |_| true);
        }
        assert!(mist.drawn(focus).is_empty());
    }
}
