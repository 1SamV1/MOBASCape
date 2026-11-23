//! Stage 4's live bridges: slices of `HudState` read from the running match.
//! Every slice now comes from the match; the stage 3 sample only fills a
//! field when its live source is absent (for example before the first frame).

use crate::animated::{AnimatedScene, UnitKind};
use crate::hud::{PlayerHud, UnitFrame};
use crate::items::{CATEGORY_LABELS, ItemConfig, ItemShop};
use crate::scene::{CampBuff, SceneData};
use glam::{Vec2, Vec3};
use rift_ui::hud::state::{
    AbilityState, AllyState, CampState, ChatKind, ChatLine, DamageSource, DeathState,
    InventoryItem, KillFeedEntry, LaneState, MapPoint, MinimapState, NexusState, ObjectivesState,
    PlateKind, ScoreState, ShopItem, ShopState, StructureMark, TargetStat, TargetState,
    TooltipState, UnitPlate,
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

fn is_objective_camp(kind: UnitKind, buff: Option<CampBuff>) -> bool {
    kind == UnitKind::Camp && buff.is_some()
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
        // Scattered critter camps are gameplay camps internally but carry no
        // team buff. The objectives panel lists the eight buff camps only.
        .filter(|unit| is_objective_camp(unit.kind, unit.camp_buff))
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

pub fn tooltip(shop: &ItemShop, mouse: (i32, i32)) -> Option<TooltipState> {
    if !shop.open {
        return None;
    }
    let item = shop.items.get(shop.selected?)?;
    let stats = shop_item(item).stats;
    Some(TooltipState {
        title: item.name.clone(),
        price: item.cost,
        stats,
        passive: None,
        text: String::new(),
        at: mouse,
    })
}

/// The selected unit as the target panel's slice. Gone or dead units are
/// not shown.
pub fn target(animated: &AnimatedScene, selected: Option<usize>) -> Option<TargetState> {
    let unit = animated.units.get(selected?)?;
    if unit.health <= 0.0 {
        return None;
    }
    let (name, _, description) = animated.unit_info(selected?)?;
    let stat = |label: &str, value: String| TargetStat {
        label: label.to_string(),
        value,
    };
    Some(TargetState {
        name,
        kind: kind_name(unit.kind).to_string(),
        blue: unit.team == 0,
        description: description.to_string(),
        health: count(unit.health),
        max_health: count(unit.max_health),
        stats: vec![
            stat("Attack", format!("{:.0}", unit.attack_damage)),
            stat(
                "Attack speed",
                format!("{:.2}", 1.0 / unit.attack_interval.max(0.01)),
            ),
            stat("Defence melee", format!("{:.0}", unit.armor)),
            stat("Defence mage", format!("{:.0}", unit.armor)),
            stat("Power", format!("{:.0}", unit.ability_power)),
            stat("Range", format!("{:.1}", unit.attack_range / 128.0)),
            stat("Defence ranged", format!("{:.0}", unit.armor)),
            stat("Move", format!("{:.1}", unit.speed / 128.0)),
        ],
    })
}

fn kind_name(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Player | UnitKind::EnemyChampion => "Hero",
        UnitKind::MeleeMinion | UnitKind::RangedMinion => "Minion",
        UnitKind::Tower | UnitKind::Inhibitor | UnitKind::Nexus | UnitKind::Watchtower => {
            "Structure"
        }
        UnitKind::Camp => "Camp",
        _ => "Unit",
    }
}

/// The team frames: the player first, then any other allied champions. The
/// current match is one hero a side, so the player is the row that keeps the
/// panel populated; the live theme carries a cache-baked `portrait.self`.
pub fn allies(animated: &AnimatedScene) -> Vec<AllyState> {
    let mut rows = Vec::new();
    if let Some(unit) = animated.units.get(animated.player_index) {
        let name = animated
            .unit_info(animated.player_index)
            .map(|(name, _, _)| name)
            .unwrap_or_else(|| "You".to_string());
        let mut row = ally_from(
            name,
            unit.health,
            unit.max_health,
            unit.mana,
            unit.max_mana,
            unit.level,
            unit.dead_timer,
        );
        row.portrait = Some("portrait.self".to_string());
        rows.push(row);
    }
    for (index, unit) in animated.units.iter().enumerate() {
        if index == animated.player_index || !unit.is_champion || unit.team != 0 {
            continue;
        }
        let name = animated
            .unit_info(index)
            .map(|(name, _, _)| name)
            .unwrap_or_else(|| "Ally".to_string());
        rows.push(ally_from(
            name,
            unit.health,
            unit.max_health,
            unit.mana,
            unit.max_mana,
            unit.level,
            unit.dead_timer,
        ));
    }
    rows
}

fn ally_from(
    name: String,
    health: f32,
    max_health: f32,
    resource: f32,
    max_resource: f32,
    level: u32,
    dead_timer: f32,
) -> AllyState {
    let dead = health <= 0.0;
    AllyState {
        name,
        health: count(health).min(count(max_health)),
        max_health: count(max_health),
        resource: count(resource).min(count(max_resource)),
        max_resource: count(max_resource),
        level: level.clamp(0, u8::MAX as u32) as u8,
        dead,
        respawn_seconds: dead.then(|| dead_timer.max(0.0).ceil() as u32),
        portrait: None,
    }
}

pub fn announcement(player: &PlayerHud) -> Option<String> {
    player.announcement.clone()
}

pub fn death(player: &PlayerHud, animated: &AnimatedScene) -> Option<DeathState> {
    let mut death = death_from(player.respawn_remaining, player.announcement.as_deref())?;
    death.damage = animated
        .damage_sources()
        .into_iter()
        .map(|(name, amount)| DamageSource { name, amount })
        .collect();
    Some(death)
}

/// The respawn count and killer are live; the damage breakdown is not
/// recorded by the match yet, so that panel is omitted.
pub fn death_from(respawn: Option<f32>, announcement: Option<&str>) -> Option<DeathState> {
    let seconds = respawn?;
    let killer = announcement
        .and_then(|text| text.strip_prefix("YOU HAVE BEEN SLAIN BY "))
        .unwrap_or("The Rift");
    Some(DeathState {
        respawn_seconds: seconds.max(0.0).ceil() as u32,
        killer: killer.to_string(),
        killer_blue: false,
        damage: Vec::new(),
    })
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
        categories: CATEGORY_LABELS
            .iter()
            .map(|label| (*label).to_string())
            .collect(),
        category: shop.category.min(CATEGORY_LABELS.len() - 1),
        search: "Search items".to_string(),
        items,
        selected,
        scroll: shop.scroll,
        hint: "Click a row, then Buy.".to_string(),
        delivery: None,
    }
}

/// Label for the shop's delivery control. `state` is the `PlayerHud`
/// delivery state: 0 idle, 1 flying to the hero, 2 returning.
pub fn delivery_label(pending: usize, state: u8) -> Option<String> {
    if pending == 0 {
        return None;
    }
    Some(match state {
        0 => format!("DELIVER ({pending})"),
        1 => "DELIVERING".to_string(),
        _ => "RETURNING".to_string(),
    })
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
        icon: item.obj.map(|_| format!("icon.item.{}", item.id)),
        stats,
        builds_into: None,
    }
}

/// The six inventory slots, in order. Purchases fill `ItemShop::owned` from
/// the first free slot, so empty slots stay empty and shift nothing.
pub fn inventory(shop: &ItemShop) -> Vec<Option<InventoryItem>> {
    shop.owned
        .iter()
        .map(|slot| {
            slot.and_then(|index| shop.items.get(index))
                .map(inventory_item)
        })
        .collect()
}

pub fn inventory_item(item: &ItemConfig) -> InventoryItem {
    InventoryItem {
        name: item.name.clone(),
        colour: item.color,
        icon: item.obj.map(|_| format!("icon.item.{}", item.id)),
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
    // Neutral camps and dummies are plate-eligible team 2 units; with the
    // legacy HUD suppressed they need their own small bars in the assembled
    // interface.
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
    let mut state = ability_from(
        player.health,
        player.max_health,
        player.mana,
        player.max_mana,
        player.level,
        player.xp,
        player.xp_next,
        player.cooldowns,
    );
    state.portrait = Some("portrait.self".to_string());
    state.icons = std::array::from_fn(|n| Some(ability_icon_role(&player.champion_name, n)));
    state
}

/// Theme role of a champion's ability icon: `icon.ability.<name>.<slot>`,
/// with the display name reduced to lowercase letters and digits so it is
/// stable across punctuation ("Kree'arra Jr." -> `kreearrajr`).
pub fn ability_icon_role(champion_name: &str, slot: usize) -> String {
    let key: String = champion_name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    format!("icon.ability.{key}.{slot}")
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
        portrait: None,
        icons: Default::default(),
    }
}

fn count(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_icon_roles_are_stable_across_punctuation() {
        assert_eq!(
            ability_icon_role("Kree'arra Jr.", 0),
            "icon.ability.kreearrajr.0"
        );
        assert_eq!(
            ability_icon_role("King Awowogei", 3),
            "icon.ability.kingawowogei.3"
        );
        assert_eq!(
            ability_icon_role("JalRek-Jad", 2),
            "icon.ability.jalrekjad.2"
        );
    }

    #[test]
    fn every_champion_ships_four_ability_icons() {
        for name in ["monkey", "jad", "kree", "ranger"] {
            let path = format!(
                "{}/../../data/champions/{name}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let text = std::fs::read_to_string(&path).unwrap();
            let champion: crate::champion::ChampionConfig = serde_json::from_str(&text).unwrap();
            assert_eq!(champion.ability_icons.len(), 4, "{name}");
        }
    }

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
    fn only_buff_camps_are_objectives() {
        assert!(is_objective_camp(UnitKind::Camp, Some(CampBuff::Might)));
        assert!(is_objective_camp(UnitKind::Camp, Some(CampBuff::Swift)));
        assert!(!is_objective_camp(UnitKind::Camp, None));
        assert!(!is_objective_camp(UnitKind::Player, None));
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
    fn unit_frames_become_plates_and_neutral_camps_keep_their_bars() {
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
        frame.hero = false;
        let camp = plate(&frame).expect("a neutral camp plate");
        assert_eq!(camp.kind, PlateKind::Minion);
        assert!(!camp.blue);
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
    fn the_delivery_control_reads_pending_and_state() {
        assert_eq!(delivery_label(0, 0), None);
        assert_eq!(delivery_label(2, 0).as_deref(), Some("DELIVER (2)"));
        assert_eq!(delivery_label(2, 1).as_deref(), Some("DELIVERING"));
        assert_eq!(delivery_label(2, 2).as_deref(), Some("RETURNING"));
    }

    #[test]
    fn the_shop_reads_its_filtered_items_and_chosen_index() {
        let item = ItemConfig {
            id: "test".to_string(),
            name: "Test blade".to_string(),
            obj: None,
            category: "melee".to_string(),
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
            scroll: 0,
        };
        let state = shop(&catalog);
        assert!(state.open);
        assert_eq!(state.category, 1);
        assert_eq!(state.categories[1], "MELEE");
        assert_eq!(state.categories.len(), CATEGORY_LABELS.len());
        assert_eq!(state.items.len(), 1);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn the_inventory_reads_owned_slots_in_order_and_leaves_gaps_empty() {
        let item = |name: &str, color: [u8; 3]| ItemConfig {
            id: name.to_lowercase(),
            name: name.to_string(),
            obj: None,
            category: "weapon".to_string(),
            color,
            cost: 100,
            damage: 0.0,
            health: 0.0,
            mana: 0.0,
            speed: 0.0,
            ability: 0.0,
        };
        let catalog = ItemShop {
            items: vec![item("Sword", [1, 2, 3]), item("Shield", [4, 5, 6])],
            owned: [Some(1), None, Some(0), Some(99), None, None],
            open: false,
            selected: None,
            category: 0,
            scroll: 0,
        };
        let slots = inventory(&catalog);
        assert_eq!(slots.len(), 6);
        assert_eq!(
            slots[0],
            Some(InventoryItem {
                name: "Shield".to_string(),
                colour: [4, 5, 6],
                icon: None,
            })
        );
        assert_eq!(slots[1], None);
        assert_eq!(
            slots[2].as_ref().map(|item| item.name.as_str()),
            Some("Sword")
        );
        assert_eq!(slots[3], None);
        assert_eq!(slots[4], None);
        assert_eq!(slots[5], None);

        let sword = inventory_item(&catalog.items[0]);
        assert_eq!(sword.name, "Sword");
        assert_eq!(sword.colour, [1, 2, 3]);
    }

    #[test]
    fn the_death_overlay_reads_respawn_and_killer() {
        let death = death_from(Some(3.2), Some("YOU HAVE BEEN SLAIN BY A RIVAL CHAMPION"))
            .expect("a death state");
        assert_eq!(death.respawn_seconds, 4);
        assert_eq!(death.killer, "A RIVAL CHAMPION");
        assert!(death.damage.is_empty());
        assert!(death_from(None, None).is_none());
    }

    #[test]
    fn the_tooltip_follows_the_selected_shop_item_and_cursor() {
        let item = ItemConfig {
            id: "test".to_string(),
            name: "Test ring".to_string(),
            obj: None,
            category: "jewellery".to_string(),
            color: [1, 2, 3],
            cost: 900,
            damage: 0.0,
            health: 50.0,
            mana: 0.0,
            speed: 0.0,
            ability: 0.0,
        };
        let catalog = ItemShop {
            items: vec![item],
            owned: [None; 6],
            open: true,
            selected: Some(0),
            category: 0,
            scroll: 0,
        };
        let tooltip = tooltip(&catalog, (400, 250)).expect("a tooltip");
        assert_eq!(tooltip.title, "Test ring");
        assert_eq!(tooltip.price, 900);
        assert_eq!(tooltip.stats, ["+50 health"]);
        assert_eq!(tooltip.at, (400, 250));
        assert!(tooltip.passive.is_none() && tooltip.text.is_empty());
    }

    #[test]
    fn an_allied_champion_becomes_a_team_frame() {
        let ally = ally_from("Ally".to_string(), 208.4, 700.0, 90.6, 260.0, 6, 0.0);
        assert_eq!((ally.health, ally.max_health), (208, 700));
        assert_eq!((ally.resource, ally.max_resource), (91, 260));
        assert_eq!((ally.level, ally.dead), (6, false));
        assert_eq!(ally.respawn_seconds, None);

        let dead = ally_from("Ally".to_string(), 0.0, 700.0, 0.0, 260.0, 6, 4.2);
        assert!(dead.dead);
        assert_eq!(dead.respawn_seconds, Some(5));
    }

    #[test]
    fn unit_kinds_name_the_target_panel_tag() {
        assert_eq!(kind_name(UnitKind::EnemyChampion), "Hero");
        assert_eq!(kind_name(UnitKind::MeleeMinion), "Minion");
        assert_eq!(kind_name(UnitKind::Tower), "Structure");
        assert_eq!(kind_name(UnitKind::Camp), "Camp");
        assert_eq!(kind_name(UnitKind::Ambient), "Unit");
    }
}
