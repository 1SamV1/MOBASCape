//! Standalone 3D spectator arena for the authoritative sim.
//!
//! The prototype client still renders its own map and combat. This module is
//! the bridge: it steps `rift_sim::Match`, builds a flat arena platform plus
//! cache-model actors for heroes, minions and structures, and hands the mesh
//! to the existing renderer. It deliberately lives away from the world map so
//! it cannot be confused with live gameplay.

use anyhow::Result;
use glam::Vec3;
use rift_cache::{Cache, Model};
use rift_sim::{EntityKind, Match};

use crate::scene::{MeshData, SceneVertex, append_sim_actor};

const POS: f32 = 128.0;
const PLATFORM_LIFT: f32 = 2.0;

struct SimModels {
    blue_hero: Option<Model>,
    red_hero: Option<Model>,
    minion: Option<Model>,
    tower: Option<Model>,
    inhibitor: Option<Model>,
    nexus: Option<Model>,
}

impl SimModels {
    fn load(cache: &Cache) -> Self {
        let load = |id: u32| -> Option<Model> {
            let group = cache.group(7, id).ok()?;
            Model::load(id, &group.data).ok()
        };
        Self {
            // King Awowogei and Kree'arra Jr. — the two champions the client
            // already uses for the player and rival.
            blue_hero: load(4816),
            red_hero: load(28019),
            // OSRS imp used by the prototype lane minions.
            minion: load(19982),
            tower: load(1761),
            inhibitor: load(12953),
            nexus: load(36053),
        }
    }
}

pub struct Sim3d {
    pub game: Match,
    models: SimModels,
    pub mesh: MeshData,
    accumulator: f32,
    center: Vec3,
}

impl Sim3d {
    pub fn new(cache: &Cache, seed: u32, size: usize) -> Result<Self> {
        let game = Match::new(seed, size);
        let models = SimModels::load(cache);
        let span = game.arena_size() as f32 * POS;
        let center = Vec3::new(span * 0.5, 0.0, span * 0.5);
        let mut arena = Self {
            game,
            models,
            mesh: MeshData::new(),
            accumulator: 0.0,
            center,
        };
        arena.rebuild();
        Ok(arena)
    }

    pub fn center(&self) -> Vec3 {
        self.center
    }

    /// Advance the fixed 30 Hz sim with a frame-time accumulator and rebuild
    /// the actor mesh only when at least one tick ran.
    pub fn advance(&mut self, dt: f32) {
        self.accumulator += dt.max(0.0);
        let fixed = 1.0 / rift_sim::TICK_HZ as f32;
        let mut steps = 0u32;
        while self.accumulator >= fixed && steps < 12 {
            self.game.step();
            self.accumulator -= fixed;
            steps += 1;
        }
        if steps > 0 {
            self.rebuild();
        }
    }

    pub fn rebuild(&mut self) {
        let mesh = &mut self.mesh;
        mesh.vertices.clear();
        mesh.indices.clear();
        let span = self.game.arena_size() as f32 * POS;
        let half = span * 4.0;
        let floor_y = 0.0;
        // Large unlit platform. Unlit geometry ignores fog of war, so the
        // arena stays readable even though it sits outside the world map.
        push_quad(
            mesh,
            Vec3::new(-half, floor_y, -half),
            Vec3::new(-half, floor_y, half),
            Vec3::new(half, floor_y, half),
            Vec3::new(half, floor_y, -half),
            [54, 82, 50],
        );
        // Lanes.
        for lane in &self.game.lanes {
            for pair in lane.windows(2) {
                let a = Vec3::new(
                    pair[0].0 as f32 * POS,
                    floor_y + 1.0,
                    pair[0].1 as f32 * POS,
                );
                let b = Vec3::new(
                    pair[1].0 as f32 * POS,
                    floor_y + 1.0,
                    pair[1].1 as f32 * POS,
                );
                let dir = (b - a).normalize_or_zero();
                let normal = Vec3::new(-dir.z, 0.0, dir.x) * 3.0 * POS;
                push_quad(
                    mesh,
                    a - normal,
                    a + normal,
                    b + normal,
                    b - normal,
                    [112, 88, 58],
                );
            }
        }

        // Structures first so units overlap correctly in the depth buffer.
        for entity in &self.game.entities {
            let (model, scale) = match entity.kind {
                EntityKind::Tower => (self.models.tower.as_ref(), 1.5),
                EntityKind::Inhibitor => (self.models.inhibitor.as_ref(), 1.8),
                EntityKind::Nexus => (self.models.nexus.as_ref(), 2.15),
                _ => continue,
            };
            let base = entity_base(entity);
            let tint = if entity.alive {
                if entity.team == 0 {
                    [0.55, 0.75, 1.35]
                } else {
                    [1.35, 0.62, 0.55]
                }
            } else {
                [0.30, 0.28, 0.27]
            };
            if let Some(model) = model {
                append_sim_actor(mesh, model, base, 0.0, scale, tint, 46);
            } else {
                push_cube(
                    mesh,
                    base + Vec3::Y * 60.0,
                    Vec3::new(70.0, 120.0, 70.0),
                    [150, 150, 145],
                );
            }
        }

        // Units.
        for entity in &self.game.entities {
            if !entity.alive {
                continue;
            }
            let (model, scale, tint, texture_hint) = match entity.kind {
                EntityKind::Hero => {
                    let model = if entity.team == 0 {
                        self.models.blue_hero.as_ref()
                    } else {
                        self.models.red_hero.as_ref()
                    };
                    (
                        model,
                        if entity.team == 0 { 6.0 } else { 2.2 },
                        if entity.team == 0 {
                            [0.82, 0.92, 1.18]
                        } else {
                            [1.18, 0.84, 0.80]
                        },
                        46,
                    )
                }
                EntityKind::Minion => (
                    self.models.minion.as_ref(),
                    2.8,
                    if entity.team == 0 {
                        [0.80, 0.95, 1.20]
                    } else {
                        [1.20, 0.85, 0.80]
                    },
                    46,
                ),
                _ => continue,
            };
            let base = entity_base(entity);
            let yaw = if entity.team == 0 {
                0.0
            } else {
                std::f32::consts::PI
            };
            // Team ring on the platform makes the two sides readable even
            // when the camera is fully top-down.
            let ring = if entity.team == 0 {
                [70, 150, 245]
            } else {
                [240, 80, 70]
            };
            let radius = if entity.kind == EntityKind::Hero {
                105.0
            } else {
                45.0
            };
            push_disc(mesh, base + Vec3::Y * 1.0, radius, ring);
            if let Some(model) = model {
                append_sim_actor(mesh, model, base, yaw, scale, tint, texture_hint);
            } else {
                let color = if entity.team == 0 {
                    [90, 170, 255]
                } else {
                    [245, 90, 80]
                };
                push_cube(
                    mesh,
                    base + Vec3::Y * 55.0,
                    Vec3::new(38.0, 90.0, 38.0),
                    color,
                );
            }
        }
    }
}

fn entity_base(entity: &rift_sim::Entity) -> Vec3 {
    let tile = entity.tile();
    Vec3::new(
        (tile.0 as f32 + 0.5) * POS,
        PLATFORM_LIFT,
        (tile.1 as f32 + 0.5) * POS,
    )
}

fn push_quad(mesh: &mut MeshData, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: [u8; 3]) {
    let base = mesh.vertices.len() as u32;
    for point in [a, b, c, d] {
        mesh.vertices.push(SceneVertex::unlit(point, color));
    }
    mesh.indices
        .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
}

fn push_disc(mesh: &mut MeshData, center: Vec3, radius: f32, color: [u8; 3]) {
    let segments = 24u32;
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(SceneVertex::unlit(center, color));
    for step in 0..=segments {
        let angle = step as f32 / segments as f32 * std::f32::consts::TAU;
        mesh.vertices.push(SceneVertex::unlit(
            center + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius),
            color,
        ));
    }
    for step in 0..segments {
        mesh.indices
            .extend_from_slice(&[base, base + 1 + step, base + 2 + step]);
    }
}

fn push_cube(mesh: &mut MeshData, center: Vec3, half: Vec3, color: [u8; 3]) {
    let corners = [
        center + Vec3::new(-half.x, -half.y, -half.z),
        center + Vec3::new(half.x, -half.y, -half.z),
        center + Vec3::new(half.x, -half.y, half.z),
        center + Vec3::new(-half.x, -half.y, half.z),
        center + Vec3::new(-half.x, half.y, -half.z),
        center + Vec3::new(half.x, half.y, -half.z),
        center + Vec3::new(half.x, half.y, half.z),
        center + Vec3::new(-half.x, half.y, half.z),
    ];
    for (a, b, c, d) in [
        (0, 1, 2, 3),
        (4, 5, 6, 7),
        (0, 1, 5, 4),
        (2, 3, 7, 6),
        (1, 2, 6, 5),
        (3, 0, 4, 7),
    ] {
        push_quad(mesh, corners[a], corners[b], corners[c], corners[d], color);
    }
}
