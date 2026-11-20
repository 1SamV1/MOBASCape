//! Stage 4's live bridges: slices of `HudState` read from the running match.
//! Each component still falls back to the stage 3 sample until its own
//! bridge is added.

use crate::animated::{AnimatedScene, UnitKind};
use crate::hud::{PlayerHud, UnitFrame};
use crate::items::{CATEGORY_LABELS, ItemConfig, ItemShop};
use crate::scene::{CampBuff, SceneData};
use glam::{Vec2, Vec3};
use rift_ui::hud::state::{
    AbilityState, CampState, ChatKind, ChatLine, KillFeedEntry, LaneState, MapPoint, MinimapState,
    NexusState, ObjectivesState, PlateKind, ScoreState, ShopItem, ShopState, StructureMark,
    UnitPlate,
};

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

pub fn objectives(scene: &SceneData, animated: &AnimatedScene) -> ObjectivesState {
    let lanes = animated
        .lanes
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let name = scene
                .map
                .lanes
                .get(index)
                .map(|lane| lane_name(&lane.id))
                .unwrap_or_else(|| format!("Lane {}", index + 1));
            let blue_towers = tower_slots(animated, index, 0);
            let red_towers = tower_slots(animated, index, 1);
            LaneState {
                name,
                blue_towers,
                blue_inhibitor: inhibitor_alive(animated, index, 0),
                red_towers,
                red_inhibitor: inhibitor_alive(animated, index, 1),
            }
        })
        .collect();

    let nexuses = [0u8, 1]
        .into_iter()
        .map(|team| {
            let health = animated
                .units
                .iter()
                .find(|unit| unit.kind == UnitKind::Nexus && unit.team == team)
                .map_or(0.0, |unit| unit.health);
            NexusState {
                name: if team == 0 { "Blue nexus" } else { "Red nexus" }.to_string(),
                blue: team == 0,
                state: nexus_state(health, animated.nexus_vulnerable(team)),
            }
        })
        .collect();

    let mut second_lines = 0;
    let camps = animated
        .units
        .iter()
        .filter(|unit| unit.kind == UnitKind::Camp)
        .map(|unit| {
            let grant = match unit.camp_buff.unwrap_or(CampBuff::Might) {
                CampBuff::Might => "Might",
                CampBuff::Swift => "Swift",
            };
            let state = if unit.health > 0.0 {
                "Alive".to_string()
            } else {
                format!("{:.0}s", unit.dead_timer.max(0.0))
            };
            let detail = (0..2).find_map(|team| {
                let (remaining, _, buff) = animated.team_buff(team)?;
                (buff == unit.camp_buff.unwrap_or(CampBuff::Might)).then(|| {
                    format!(
                        "{} {} {:.0}s",
                        if team == 0 { "Blue" } else { "Red" },
                        grant,
                        remaining
                    )
                })
            });
            if detail.is_some() {
                second_lines += 1;
            }
            CampState {
                name: format!("{grant} camp"),
                grant: grant.to_string(),
                state,
                detail,
            }
        })
        .collect();

    ObjectivesState {
        lanes,
        nexuses,
        camps,
        second_lines,
        usual_second_lines: 0,
    }
}

fn tower_slots(animated: &AnimatedScene, lane: usize, team: u8) -> [bool; 2] {
    let home = animated.nexus_positions[team as usize];
    let mut towers: Vec<(f32, bool)> = animated
        .units
        .iter()
        .filter(|unit| {
            unit.kind == UnitKind::Tower && unit.team == team && unit.structure_lane == Some(lane)
        })
        .map(|unit| (unit.pos.distance(home), unit.health > 0.0))
        .collect();
    // The tower farthest from the team's nexus is the outer one.
    towers.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut slots = [false; 2];
    for (slot, (_, alive)) in towers.into_iter().take(2).enumerate() {
        slots[slot] = alive;
    }
    slots
}

fn inhibitor_alive(animated: &AnimatedScene, lane: usize, team: u8) -> bool {
    animated.units.iter().any(|unit| {
        unit.kind == UnitKind::Inhibitor
            && unit.team == team
            && unit.structure_lane == Some(lane)
            && unit.health > 0.0
    })
}

fn lane_name(id: &str) -> String {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Lane".to_string(),
    }
}

fn nexus_state(health: f32, vulnerable: bool) -> String {
    if health <= 0.0 {
        "0".to_string()
    } else if vulnerable {
        format!("{health:.0}")
    } else {
        "Shielded".to_string()
    }
}

pub fn shop(shop: &ItemShop) -> ShopState {
    let filtered = shop.filtered_items();
    let items = filtered
        .iter()
        .map(|&index| shop_item(&shop.items[index]))
        .collect();
    let selected = shop
        .selected
        .and_then(|wanted| filtered.iter().position(|&index| index == wanted))
        .unwrap_or(0);
    ShopState {
        open: shop.open,
        category: CATEGORY_LABELS
            .get(shop.category)
            .copied()
            .unwrap_or("ALL")
            .to_string(),
        search: "Search items".to_string(),
        items,
        selected,
        scroll: 0,
        hint: "Buying stays in the old HUD for now.".to_string(),
    }
}

pub fn shop_item(item: &ItemConfig) -> ShopItem {
    let mut stats = Vec::new();
    if item.damage != 0.0 {
        stats.push(format!("+{:.0} damage", item.damage));
    }
    if item.health != 0.0 {
        stats.push(format!("+{:.0} health", item.health));
    }
    if item.mana != 0.0 {
        stats.push(format!("+{:.0} mana", item.mana));
    }
    if item.speed != 0.0 {
        stats.push(format!("+{:.0} speed", item.speed));
    }
    if item.ability != 0.0 {
        stats.push(format!("+{:.0}% ability power", item.ability));
    }
    ShopItem {
        name: item.name.clone(),
        price: item.cost,
        colour: item.color,
        stats,
        builds_into: None,
    }
}

pub fn chat(player: &PlayerHud) -> Vec<ChatLine> {
    chat_lines(&player.events)
}

/// The event log is newest first; the chat panel reads oldest at the top.
pub fn chat_lines(events: &[String]) -> Vec<ChatLine> {
    events
        .iter()
        .rev()
        .map(|text| ChatLine {
            kind: ChatKind::Event,
            text: text.clone(),
        })
        .collect()
}

pub fn feed(player: &PlayerHud) -> Vec<KillFeedEntry> {
    player
        .kill_feed
        .iter()
        .rev()
        .take(4)
        .map(|line| feed_entry(line))
        .collect()
}

/// The live feed only carries allied kills, and its lines are written by
/// `AnimatedScene` as "YOU SLAIN X" or "KILLER HAS SLAIN VICTIM".
pub fn feed_entry(line: &str) -> KillFeedEntry {
    let (killer, victim) = if let Some(victim) = line.strip_prefix("YOU SLAIN ") {
        ("You", victim)
    } else if let Some((killer, victim)) = line.split_once(" HAS SLAIN ") {
        (killer, victim)
    } else {
        ("The Rift", line)
    };
    KillFeedEntry {
        killer: killer.to_string(),
        victim: victim.to_string(),
        gold: reward(victim),
        killer_blue: true,
        victim_blue: false,
    }
}

fn reward(victim: &str) -> u32 {
    match victim {
        "A MINION" => 10,
        "A TOWER" => 100,
        "AN INHIBITOR" => 150,
        "A CAMP" => 50,
        "A DUMMY" => 20,
        _ => 0,
    }
}

pub fn plates(player: &PlayerHud) -> Vec<UnitPlate> {
    player.unit_frames.iter().filter_map(plate).collect()
}

/// One already-projected unit frame as a plate. Neutral camps are left to
/// the old HUD; the three plate kinds are hero, minion and structure.
pub fn plate(frame: &UnitFrame) -> Option<UnitPlate> {
    if frame.team == 2 {
        return None;
    }
    let kind = if frame.hero {
        PlateKind::Hero
    } else if frame.structure {
        PlateKind::Structure
    } else {
        PlateKind::Minion
    };
    Some(UnitPlate {
        at: (frame.x.round() as i32, frame.y.round() as i32),
        kind,
        name: frame.name.clone(),
        level: frame.level.clamp(0, u8::MAX as u32) as u8,
        health: count(frame.health),
        max_health: count(frame.max_health),
        blue: frame.team == 0,
    })
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

    #[test]
    fn objective_names_and_nexus_states_read_naturally() {
        assert_eq!(lane_name("top"), "Top");
        assert_eq!(lane_name("mid"), "Mid");
        assert_eq!(nexus_state(0.0, true), "0");
        assert_eq!(nexus_state(620.0, true), "620");
        assert_eq!(nexus_state(620.0, false), "Shielded");
    }

    #[test]
    fn unit_frames_become_plates_and_neutral_camps_are_skipped() {
        let mut frame = UnitFrame {
            x: 400.4,
            y: 250.6,
            name: "Rival".to_string(),
            level: 7,
            health: 208.4,
            max_health: 420.0,
            mana: 0.0,
            max_mana: 0.0,
            team: 1,
            hero: true,
            structure: false,
            width: 184.0,
        };
        let hero = plate(&frame).expect("a hero plate");
        assert_eq!(hero.at, (400, 251));
        assert_eq!(hero.kind, PlateKind::Hero);
        assert_eq!((hero.health, hero.max_health), (208, 420));
        assert_eq!((hero.level, hero.blue), (7, false));

        frame.team = 2;
        assert!(plate(&frame).is_none());
    }

    #[test]
    fn the_event_log_reverses_into_chat_and_the_feed_parses_kills() {
        let events = vec!["NEWEST".to_string(), "OLDEST".to_string()];
        let chat = chat_lines(&events);
        assert_eq!(chat.len(), 2);
        assert_eq!(chat[0].text, "OLDEST");
        assert_eq!(chat[0].kind, ChatKind::Event);
        assert_eq!(chat[1].text, "NEWEST");

        let kill = feed_entry("YOU SLAIN A MINION");
        assert_eq!(
            (kill.killer.as_str(), kill.victim.as_str()),
            ("You", "A MINION")
        );
        assert_eq!(kill.gold, 10);
        assert!(kill.killer_blue && !kill.victim_blue);

        let structure = feed_entry("A TOWER HAS SLAIN A MINION");
        assert_eq!(structure.killer, "A TOWER");
        assert_eq!(structure.victim, "A MINION");
        assert_eq!(structure.gold, 10);
    }

    #[test]
    fn the_shop_reads_its_filtered_items_and_chosen_index() {
        let item = ItemConfig {
            id: "test".to_string(),
            name: "Test blade".to_string(),
            category: "weapon".to_string(),
            color: [10, 20, 30],
            cost: 700,
            damage: 8.0,
            health: 0.0,
            mana: 0.0,
            speed: 0.0,
            ability: 15.0,
        };
        let converted = shop_item(&item);
        assert_eq!(converted.name, "Test blade");
        assert_eq!(converted.price, 700);
        assert_eq!(converted.colour, [10, 20, 30]);
        assert_eq!(converted.stats, ["+8 damage", "+15% ability power"]);

        let catalog = ItemShop {
            items: vec![item],
            owned: [None; 6],
            open: true,
            selected: Some(0),
            category: 1,
        };
        let state = shop(&catalog);
        assert!(state.open);
        assert_eq!(state.category, "WEAPONS");
        assert_eq!(state.items.len(), 1);
        assert_eq!(state.selected, 0);
    }
}
