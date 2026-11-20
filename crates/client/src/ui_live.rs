//! Stage 4's first live bridge: the score strip's slice of `HudState` from
//! the running match. Every other component still uses the stage 3 sample
//! until its own bridge is added.

use crate::animated::{AnimatedScene, UnitKind};
use crate::hud::PlayerHud;
use crate::scene::SceneData;
use glam::{Vec2, Vec3};
use rift_ui::hud::state::{AbilityState, MapPoint, MinimapState, ScoreState, StructureMark};

pub fn score(player: &PlayerHud) -> ScoreState {
    score_from(
        player.match_time,
        player.kills,
        (
            player.player_kills,
            player.player_deaths,
            player.player_assists,
        ),
        player.player_cs,
    )
}

/// The pure part, so the numbers and clock can be checked without a match.
pub fn score_from(
    match_time: f32,
    team_kills: (u32, u32),
    own: (u32, u32, u32),
    creep_score: u32,
) -> ScoreState {
    ScoreState {
        clock: crate::hud::format_clock(match_time),
        blue_kills: team_kills.0,
        red_kills: team_kills.1,
        kills: own.0,
        deaths: own.1,
        assists: own.2,
        creep_score,
    }
}

/// The minimap picture's own size, matching `crates/ui/src/hud/minimap.rs`.
const MAP: (f32, f32) = (254.0, 252.0);

pub fn minimap(
    scene: &SceneData,
    animated: &AnimatedScene,
    camera: (f32, f32, f32, f32),
    window: (u32, u32),
) -> MinimapState {
    let (min, max) = animated.minimap_bounds;
    let point = |p: Vec3| map_point(p, min, max);

    let lanes = animated
        .lanes
        .iter()
        .map(|lane| lane.iter().map(|p| point(*p)).collect())
        .collect();
    let river = scene
        .map
        .river
        .as_ref()
        .map(|river| {
            river
                .points
                .iter()
                .map(|(x, z)| point(Vec3::new(x * 128.0, 0.0, z * 128.0)))
                .collect()
        })
        .unwrap_or_default();

    let bases = [
        StructureMark {
            at: point(animated.nexus_positions[0]),
            blue: true,
        },
        StructureMark {
            at: point(animated.nexus_positions[1]),
            blue: false,
        },
    ];

    let mut allies = Vec::new();
    let mut enemies = Vec::new();
    let mut camps = Vec::new();
    let mut structures = Vec::new();
    for unit in &animated.units {
        if !unit.visible || unit.health <= 0.0 {
            continue;
        }
        if unit.kind.is_structure() {
            structures.push(StructureMark {
                at: point(unit.pos),
                blue: unit.team == 0,
            });
            continue;
        }
        if unit.kind == UnitKind::Camp {
            camps.push(point(unit.pos));
            continue;
        }
        if matches!(
            unit.kind,
            UnitKind::Player
                | UnitKind::EnemyChampion
                | UnitKind::MeleeMinion
                | UnitKind::RangedMinion
        ) {
            let mark = point(unit.pos);
            if unit.team == 0 {
                allies.push(mark);
            } else {
                enemies.push(mark);
            }
        }
    }

    let (cx, cz, distance, pitch) = camera;
    let aspect = window.0 as f32 / window.1.max(1) as f32;
    let half_w = distance * 0.466 * aspect;
    let half_h = distance * 0.466 / pitch.sin().max(0.2);
    let a = point(Vec3::new(cx - half_w, 0.0, cz - half_h));
    let b = point(Vec3::new(cx + half_w, 0.0, cz + half_h));
    let camera = (
        a.x.min(b.x),
        a.y.min(b.y),
        (b.x - a.x).abs().max(1),
        (b.y - a.y).abs().max(1),
    );

    MinimapState {
        lanes,
        river,
        bases: bases.to_vec(),
        allies,
        enemies,
        camps,
        structures,
        camera,
    }
}

fn map_point(p: Vec3, min: Vec2, max: Vec2) -> MapPoint {
    let normal = |value: f32, low: f32, high: f32| {
        if high > low {
            ((value - low) / (high - low)).clamp(0.0, 1.0)
        } else {
            0.5
        }
    };
    MapPoint {
        x: (normal(p.x, min.x, max.x) * MAP.0).round() as i32,
        y: (normal(p.z, min.y, max.y) * MAP.1).round() as i32,
    }
}

pub fn ability(player: &PlayerHud) -> AbilityState {
    ability_from(
        player.health,
        player.max_health,
        player.mana,
        player.max_mana,
        player.level,
        player.xp,
        player.xp_next,
        player.cooldowns,
    )
}

/// The pure part of the ability bar's bridge.
#[allow(clippy::too_many_arguments)]
pub fn ability_from(
    health: f32,
    max_health: f32,
    resource: f32,
    max_resource: f32,
    level: u32,
    experience: f32,
    experience_next: f32,
    cooldowns: [f32; 4],
) -> AbilityState {
    let max_health = count(max_health);
    let max_resource = count(max_resource);
    AbilityState {
        health: count(health).min(max_health),
        max_health,
        resource: count(resource).min(max_resource),
        max_resource,
        level: level.clamp(0, u8::MAX as u32) as u8,
        experience: if experience_next > 0.0 {
            (experience / experience_next * 100.0).clamp(0.0, 100.0) as u32
        } else {
            0
        },
        cooldowns: cooldowns.map(|seconds| seconds.max(0.0)),
    }
}

fn count(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_score_strip_reads_the_match_numbers_and_clock() {
        let score = score_from(125.0, (7, 5), (3, 1, 4), 86);
        assert_eq!(score.clock, "2:05");
        assert_eq!((score.blue_kills, score.red_kills), (7, 5));
        assert_eq!((score.kills, score.deaths, score.assists), (3, 1, 4));
        assert_eq!(score.creep_score, 86);
    }

    #[test]
    fn the_ability_bar_reads_health_resource_level_and_cooldowns() {
        let bar = ability_from(
            438.4,
            730.0,
            116.6,
            260.0,
            7,
            62.0,
            100.0,
            [1.2, 0.0, 3.9, 20.0],
        );
        assert_eq!((bar.health, bar.max_health), (438, 730));
        assert_eq!((bar.resource, bar.max_resource), (117, 260));
        assert_eq!(bar.level, 7);
        assert_eq!(bar.experience, 62);
        assert_eq!(bar.cooldowns, [1.2, 0.0, 3.9, 20.0]);

        // A stale health value cannot exceed its maximum.
        let bar = ability_from(900.0, 730.0, 0.0, 260.0, 300, 0.0, 0.0, [0.0; 4]);
        assert_eq!(bar.health, 730);
        assert_eq!(bar.level, 255);
        assert_eq!(bar.experience, 0);
    }

    #[test]
    fn minimap_points_land_in_the_picture_and_clamp_outside_it() {
        let min = Vec2::new(100.0, 200.0);
        let max = Vec2::new(300.0, 400.0);
        let middle = map_point(Vec3::new(200.0, 0.0, 300.0), min, max);
        assert_eq!(middle, MapPoint { x: 127, y: 126 });
        let outside = map_point(Vec3::new(0.0, 0.0, 9999.0), min, max);
        assert_eq!(outside, MapPoint { x: 0, y: 252 });
        let flat = map_point(Vec3::new(1.0, 0.0, 1.0), min, min);
        assert_eq!(flat, MapPoint { x: 127, y: 126 });
    }
}
