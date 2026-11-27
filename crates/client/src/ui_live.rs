//! Stage 4's live bridges: slices of `HudState` read from the running match.
//! Every slice now comes from the match; the stage 3 sample only fills a
//! field when its live source is absent (for example before the first frame).

use crate::animated::ranks::{UpgradeBlock, rank_level};
use crate::animated::{AnimatedScene, UnitKind};
use crate::champion::{AbilityRank, ChampionAbility};
use crate::hud::{PlayerHud, UnitFrame};
use crate::items::{CATEGORY_LABELS, ItemConfig, ItemShop};
use crate::scene::{CampBuff, SceneData};
use glam::{Vec2, Vec3};
use rift_ui::hud::state::{
    AbilityNumbers, AbilityState, AbilityTooltipState, AbilityUpgrade, AllyState, BagItem,
    CampState, ChatKind, ChatLine, DamageSource, DeathState, HeroMark, HitmarkKind, HitmarkState,
    InventoryItem, ItemLink, KillFeedEntry, LaneState, MapPoint, MinimapState, NexusState,
    ObjectivesState, OverheadState, PlateKind, ScoreState, ShopItem, ShopState, SpellSlot,
    StructureMark, TargetStat, TargetState, TooltipState, UnitPlate, VisionMask,
};

/// The Tab scoreboard: every hero in the match, blue then red, the player
/// first on their team.
pub fn scoreboard(
    animated: &AnimatedScene,
    shop: Option<&ItemShop>,
    match_time: f32,
) -> rift_ui::hud::state::ScoreboardState {
    let mut blue = Vec::new();
    let mut red = Vec::new();
    let mut order: Vec<usize> = (0..animated.units.len())
        .filter(|&i| animated.units[i].is_champion && animated.units[i].team < 2)
        .collect();
    order.sort_by_key(|&i| i != animated.player_index);
    for index in order {
        let unit = &animated.units[index];
        let me = index == animated.player_index;
        let items: Vec<&ItemConfig> = if me {
            shop.map(|shop| {
                shop.owned
                    .iter()
                    .filter_map(|slot| slot.and_then(|i| shop.items.get(i)))
                    .collect()
            })
            .unwrap_or_default()
        } else {
            animated
                .bot_of(index)
                .map(|bot| bot.items().iter().collect())
                .unwrap_or_default()
        };
        // A bot's own name keeps its case; `unit_info` shouts it for plates.
        let name = animated
            .bot_of(index)
            .map(|bot| bot.name.clone())
            .or_else(|| animated.unit_info(index).map(|(name, _, _)| name))
            .unwrap_or_else(|| "Hero".to_string());
        let row = score_row(
            name,
            unit.level,
            animated.score.get(index),
            unit.gold,
            &items,
            (unit.health <= 0.0).then_some(unit.dead_timer),
            me,
        );
        if unit.team == 0 {
            blue.push(row);
        } else {
            red.push(row);
        }
    }
    rift_ui::hud::state::ScoreboardState {
        clock: crate::hud::format_clock(match_time),
        blue,
        red,
    }
}

/// One scoreboard line. Net worth is the gold in hand plus what the items
/// cost; the item icons are the shop's `icon.item.<id>` roles.
pub fn score_row(
    name: String,
    level: u32,
    score: crate::animated::score::HeroScore,
    gold: u32,
    items: &[&ItemConfig],
    dead_timer: Option<f32>,
    me: bool,
) -> rift_ui::hud::state::ScoreRow {
    rift_ui::hud::state::ScoreRow {
        name,
        level: level.min(u8::MAX as u32) as u8,
        kills: score.kills,
        deaths: score.deaths,
        assists: score.assists,
        creep_score: score.cs,
        gold: gold + items.iter().map(|item| item.cost).sum::<u32>(),
        items: items
            .iter()
            .filter(|item| item.obj.is_some())
            .map(|item| format!("icon.item.{}", item.id))
            .collect(),
        portrait: me.then(|| "portrait.self".to_string()),
        respawn_seconds: dead_timer.map(|t| t.max(0.0).ceil() as u32),
        me,
    }
}

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
    footprint: Option<[Vec3; 4]>,
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
    let mut heroes = Vec::new();
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
        if unit.kind.is_champion() {
            heroes.push(HeroMark {
                at: point(unit.pos),
                blue: unit.team == 0,
                player: unit.kind == UnitKind::Player,
                // Only the player's own portrait is baked for the live theme;
                // bots have no hero identity in the match yet.
                portrait: (unit.kind == UnitKind::Player).then_some("portrait.map"),
            });
            continue;
        }
        if matches!(unit.kind, UnitKind::MeleeMinion | UnitKind::RangedMinion) {
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
        heroes,
        vision: Some(vision_mask(animated, min, max)),
        camera,
        view: footprint.map(|corners| corners.map(|p| map_point_free(p, min, max))),
        pings: animated
            .scroll_pings
            .iter()
            .map(|&(pos, team, _)| StructureMark {
                at: map_point(pos, min, max),
                blue: team == 0,
            })
            .collect(),
    }
}

/// Cells across the minimap's vision veil (each about 2 by 2 picture
/// pixels, near the 128-unit tile size on the current maps).
const VISION_CELLS: (usize, usize) = (127, 126);

/// Sample the team's line-of-sight grid at each minimap cell centre.
fn vision_mask(animated: &AnimatedScene, min: Vec2, max: Vec2) -> VisionMask {
    let (cols, rows) = VISION_CELLS;
    let (ox, oy) = animated.vision_origin;
    let mut seen = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let u = (col as f32 + 0.5) / cols as f32;
            let v = (row as f32 + 0.5) / rows as f32;
            let (x, z) = world_from_map(u, v, min, max);
            let tx = (x / 128.0).floor() as i32 - ox;
            let ty = (z / 128.0).floor() as i32 - oy;
            seen.push(animated.visibility.is_visible(tx, ty));
        }
    }
    VisionMask { cols, rows, seen }
}

/// Like `map_point` but without clamping, for shapes that may run off the
/// picture (the drawing is clipped to the map instead).
fn map_point_free(p: Vec3, min: Vec2, max: Vec2) -> MapPoint {
    let span = |value: f32, low: f32, high: f32, size: f32| {
        if high > low {
            ((value - low) / (high - low) * size).round() as i32
        } else {
            (size * 0.5) as i32
        }
    };
    MapPoint {
        x: span(p.x, min.x, max.x, MAP.0),
        y: span(p.z, min.y, max.y, MAP.1),
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

/// The world x/z under a minimap fraction (0..1 across the picture): the
/// inverse of `map_point`, used by the live minimap's click-to-pan.
pub fn world_from_map(u: f32, v: f32, min: Vec2, max: Vec2) -> (f32, f32) {
    (
        min.x + u.clamp(0.0, 1.0) * (max.x - min.x),
        min.y + v.clamp(0.0, 1.0) * (max.y - min.y),
    )
}

fn is_objective_camp(kind: UnitKind, buff: Option<CampBuff>) -> bool {
    kind == UnitKind::Camp && buff.is_some()
}

/// How to spend a point on `slot`: an unlearned ability learns on a click;
/// a learned one casts on a click, so its "+" or Ctrl + key ranks it up.
fn skill_point_hint(slot: usize, rank: u8, points: u32) -> String {
    if rank == 0 {
        format!("Click to learn ({points} left)")
    } else {
        let key = ["Q", "W", "E", "R"][slot.min(3)];
        format!("Click + or Ctrl+{key} to rank up ({points} left)")
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

    let snapshots: Vec<CampSnapshot> = animated
        .units
        .iter()
        // Scattered critter camps are gameplay camps internally but carry no
        // team buff. The objectives panel reads the buff camps only.
        .filter(|unit| is_objective_camp(unit.kind, unit.camp_buff))
        .map(|unit| CampSnapshot {
            name: unit.camp_name.clone(),
            buff: unit.camp_buff.unwrap_or(CampBuff::Might),
            boss: unit.camp_boss,
            respawn: (unit.health <= 0.0).then(|| unit.dead_timer.max(0.0)),
        })
        .collect();
    let held: Vec<(u8, CampBuff, f32)> = (0..2u8)
        .flat_map(|team| {
            animated
                .team_buff_list(team)
                .into_iter()
                .map(move |(remaining, kind)| (team, kind, remaining))
        })
        .collect();
    let camps = camp_lines(&snapshots, &held);
    let second_lines = camps.iter().filter(|c| c.detail.is_some()).count();

    ObjectivesState {
        lanes,
        nexuses,
        camps,
        second_lines,
        usual_second_lines: 0,
    }
}

/// One buff camp as the objectives panel needs it.
#[derive(Debug, Clone, PartialEq)]
struct CampSnapshot {
    name: Option<String>,
    buff: CampBuff,
    boss: bool,
    /// Seconds until it respawns, while it is dead.
    respawn: Option<f32>,
}

fn grant_name(buff: CampBuff) -> &'static str {
    buff.name()
}

/// The objective lines below the nexuses (roadmap 6.3): every region boss on
/// a line of its own under its creature's name with its respawn timer, then
/// one line per buff for the plain camps, "Alive" when all stand, else how
/// many stand and the soonest respawn. A team holding a buff shows it as the
/// second line of that buff's camp line; a buff only a boss grants (Speed
/// Demon, Brood Venom) shows its holder under that boss instead. `held` is
/// every (team, buff, seconds left) in force.
fn camp_lines(camps: &[CampSnapshot], held: &[(u8, CampBuff, f32)]) -> Vec<CampState> {
    let timer = |seconds: f32| format!("{:.0}s", seconds.ceil());
    let holder = |buff: CampBuff| {
        held.iter().find_map(|&(team, kind, remaining)| {
            (kind == buff).then(|| {
                format!(
                    "{} holds {} {}",
                    if team == 0 { "Blue" } else { "Red" },
                    grant_name(buff),
                    timer(remaining)
                )
            })
        })
    };
    let mut lines: Vec<CampState> = camps
        .iter()
        .filter(|camp| camp.boss)
        .map(|camp| CampState {
            name: camp
                .name
                .clone()
                .unwrap_or_else(|| format!("{} boss", grant_name(camp.buff))),
            grant: grant_name(camp.buff).to_string(),
            state: camp.respawn.map_or_else(|| "Alive".to_string(), timer),
            detail: camps
                .iter()
                .all(|other| other.boss || other.buff != camp.buff)
                .then(|| holder(camp.buff))
                .flatten(),
        })
        .collect();
    for buff in [CampBuff::Might, CampBuff::Swift] {
        let group: Vec<&CampSnapshot> = camps
            .iter()
            .filter(|camp| !camp.boss && camp.buff == buff)
            .collect();
        if group.is_empty() {
            continue;
        }
        let standing = group.iter().filter(|camp| camp.respawn.is_none()).count();
        let soonest = group
            .iter()
            .filter_map(|camp| camp.respawn)
            .min_by(|a, b| a.total_cmp(b));
        lines.push(CampState {
            name: "Jungle camps".to_string(),
            grant: grant_name(buff).to_string(),
            state: match soonest {
                None => "Alive".to_string(),
                Some(seconds) => format!("{standing}/{}  {}", group.len(), timer(seconds)),
            },
            detail: holder(buff),
        });
    }
    lines
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
    let index = shop.shown()?;
    let item = shop.items.get(index)?;
    let stats = shop_item(item).stats;
    Some(TooltipState {
        title: item.name.clone(),
        // What it costs now: an upgrade's price when parts are owned.
        price: shop.upgrade(index).price,
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
        UnitKind::Player | UnitKind::BotChampion => "Hero",
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
    let link = |i: usize| ItemLink {
        name: shop.items[i].name.clone(),
        icon: shop.items[i]
            .obj
            .map(|_| format!("icon.item.{}", shop.items[i].id)),
        owned: shop.is_owned(i),
    };
    let items = filtered
        .iter()
        .map(|&index| {
            // Prices and recipe names depend on the bag.
            let mut entry = shop_item(&shop.items[index]);
            entry.price = shop.upgrade(index).price;
            let parts: Vec<usize> = shop.items[index]
                .builds_from
                .iter()
                .filter_map(|id| shop.items.iter().position(|item| &item.id == id))
                .collect();
            entry.builds_from = parts.iter().map(|&i| shop.items[i].name.clone()).collect();
            entry.from = parts.iter().map(|&i| link(i)).collect();
            // Recipe rows link parts that are in this same list.
            entry.recipe = parts
                .iter()
                .filter_map(|part| filtered.iter().position(|i| i == part))
                .collect();
            let into = shop.builds_into(index);
            let names: Vec<&str> = into.iter().map(|&i| shop.items[i].name.as_str()).collect();
            entry.builds_into = (!names.is_empty()).then(|| names.join(", "));
            entry.into = into.into_iter().map(link).collect();
            entry.owned = shop.is_owned(index);
            entry
        })
        .collect();
    let shown = shop.shown();
    let selected = shown
        .and_then(|wanted| filtered.iter().position(|&index| index == wanted))
        .unwrap_or(0);
    ShopState {
        open: shop.open,
        categories: CATEGORY_LABELS
            .iter()
            .map(|label| (*label).to_string())
            .collect(),
        category: shop.category.min(CATEGORY_LABELS.len() - 1),
        search: if shop.query.is_empty() && !shop.typing {
            "Search items".to_string()
        } else {
            shop.query.clone()
        },
        searching: shop.typing,
        items,
        selected,
        scroll: shop.scroll,
        hint: "Owned parts make upgrades cheaper.".to_string(),
        delivery: None,
        sell_price: shown
            .filter(|&index| shop.is_owned(index))
            .and_then(|index| shop.items.get(index))
            .map(|item| crate::items::sell_price(item.cost)),
        owned: shown
            .map(|index| shop.owned_count(index) as u32)
            .unwrap_or(0),
        undo: shop
            .undo
            .last()
            .and_then(|purchase| shop.items.get(purchase.index))
            .map(|item| item.name.clone()),
        bag: shop
            .owned
            .iter()
            .enumerate()
            .filter_map(|(slot, index)| {
                let index = (*index)?;
                let item = shop.items.get(index)?;
                Some(BagItem {
                    slot,
                    name: item.name.clone(),
                    icon: item.obj.map(|_| format!("icon.item.{}", item.id)),
                    sell: crate::items::sell_price(item.cost),
                    selected: shown == Some(index),
                })
            })
            .collect(),
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
    stats.extend(item.use_lines());
    ShopItem {
        name: item.name.clone(),
        price: item.cost,
        colour: item.color,
        icon: item.obj.map(|_| format!("icon.item.{}", item.id)),
        stats,
        builds_into: None,
        builds_from: Vec::new(),
        full_price: item.cost,
        tier: item.tier(),
        recipe: Vec::new(),
        owned: false,
        from: Vec::new(),
        into: Vec::new(),
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
    crate::hud::nudge_unit_frames(player.unit_frames.clone())
        .iter()
        .filter_map(plate)
        .collect()
}

/// One already-projected unit frame as a plate. Neutral camps are left to
/// the old HUD; the three plate kinds are hero, minion and structure.
pub fn plate(frame: &UnitFrame) -> Option<UnitPlate> {
    // Neutral camps and dummies are plate-eligible team 2 units; with the
    // legacy HUD suppressed they need their own small bars in the assembled
    // interface.
    let kind = if frame.hero {
        PlateKind::Hero
    } else if frame.camp {
        PlateKind::Camp
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
        name_colour: Some(frame.name_color),
        pk: frame.pk,
        tag: frame.tag.clone(),
    })
}

/// Projected OSRS hitmarks as the assembled interface's plain state.
pub fn hitmarks(player: &PlayerHud) -> Vec<HitmarkState> {
    player
        .hitsplats
        .iter()
        .map(|hit| HitmarkState {
            at: (hit.x.round() as i32, hit.y.round() as i32),
            text: hit.text.clone(),
            kind: match hit.kind {
                crate::animated::HitsplatKind::Damage => HitmarkKind::Damage,
                crate::animated::HitsplatKind::Poison => HitmarkKind::Poison,
                crate::animated::HitsplatKind::Venom => HitmarkKind::Venom,
                crate::animated::HitsplatKind::Disease => HitmarkKind::Disease,
                crate::animated::HitsplatKind::Heal => HitmarkKind::Heal,
                crate::animated::HitsplatKind::Miss => HitmarkKind::Miss,
                crate::animated::HitsplatKind::MaxHit => HitmarkKind::Max,
            },
            alpha: hit.alpha,
            stack: hit.stack,
            tint: hit.tint,
        })
        .collect()
}

/// Projected OSRS overhead chat as the assembled interface's plain state.
/// Overhead chat, plus the economy labels (the gold "+59" of a last hit),
/// which the interface draws the same way: yellow over a black shadow.
pub fn overhead(player: &PlayerHud) -> Vec<OverheadState> {
    let chat = player
        .overhead
        .iter()
        .map(|(x, y, text, alpha)| OverheadState {
            at: (x.round() as i32, y.round() as i32),
            text: text.clone(),
            alpha: *alpha,
        });
    let labels = player
        .damage_numbers
        .iter()
        .map(|(x, y, text, _, _)| OverheadState {
            at: (x.round() as i32, y.round() as i32),
            text: text.clone(),
            alpha: 1.0,
        });
    chat.chain(labels).collect()
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
    state.spells = std::array::from_fn(|n| spell_slot(&player.spells[n]));
    state.scroll = spell_slot(&player.scroll);
    state
}

fn spell_slot(spell: &crate::hud::SpellHud) -> SpellSlot {
    SpellSlot {
        icon: spell.icon.clone(),
        cooldown: spell.cooldown.max(0.0),
        channel: spell.channel,
        usable: spell.usable,
    }
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
        ranks: [0; 4],
        max_ranks: [4, 4, 4, 3],
        upgradable: [false; 4],
        time: 0.0,
        spells: Default::default(),
        scroll: Default::default(),
        talent_pick: None,
        charge: None,
    }
}

/// Fill the bar's rank pips and upgrade glow from the match.
pub fn ability_ranks(state: &mut AbilityState, animated: &AnimatedScene) {
    state.ranks = animated.ability_ranks();
    state.max_ranks = std::array::from_fn(|slot| animated.max_rank(slot));
    state.upgradable = std::array::from_fn(|slot| animated.can_upgrade(slot));
    state.talent_pick = animated.open_talent_tier().map(|tier| {
        let names = animated.player_ability_names();
        let pair = &animated.player_talents()[tier];
        let line = |n: usize| format!("{}: {}", pair[n].name, pair[n].effect.describe(&names));
        (crate::talents::TALENT_LEVELS[tier], [line(0), line(1)])
    });
    state.charge = animated
        .kit_charging()
        .map(|(slot, held, full)| (slot, held / full));
    state.time = animated.hud_time;
}

/// The tooltip for a hovered battle spell (0 for D, 1 for F) or the
/// teleport scroll (2, T).
pub fn spell_tooltip(
    animated: &AnimatedScene,
    slot: usize,
    mouse: (i32, i32),
) -> Option<TooltipState> {
    if slot >= 2 {
        let scroll = &animated.scroll_config;
        return Some(TooltipState {
            title: title_case(&scroll.name),
            price: 0,
            stats: vec![
                "Hotkey T".to_string(),
                format!("Channel {:.0} s", scroll.channel),
                format!("Cooldown {:.0} s", scroll.cooldown),
            ],
            passive: None,
            text: "Arrive beside the allied tower, inhibitor or nexus nearest the cursor \
                   or minimap point. Crowd control and Tele Block break the channel; \
                   moving or casting cancels it."
                .to_string(),
            at: mouse,
        });
    }
    let key = ["D", "F"][slot];
    let player = animated.units.get(animated.player_index)?;
    let Some(spell) = player.spells.picks[slot].and_then(|n| animated.spell_book.get(n)) else {
        return Some(TooltipState {
            title: "Battle spell".to_string(),
            price: 0,
            stats: vec![format!("Hotkey {key}")],
            passive: None,
            text: "Magic heroes only.".to_string(),
            at: mouse,
        });
    };
    Some(TooltipState {
        title: title_case(&spell.name),
        price: 0,
        stats: vec![
            format!("Hotkey {key}"),
            format!("Cooldown {:.0} s", spell.cooldown),
            format!("{} spellbook", spell.book),
        ],
        passive: None,
        text: spell.describe(),
        at: mouse,
    })
}

/// The tooltip for a hovered ability slot.
pub fn ability_tooltip(
    animated: &AnimatedScene,
    slot: usize,
    mouse: (i32, i32),
) -> Option<AbilityTooltipState> {
    let info = animated.ability_info(slot)?;
    let rank = animated.ability_rank(slot);
    let max_rank = animated.max_rank(slot);
    let power = animated.ability_power_mult();
    let level = animated.player_level();
    let numbers = |rank: u8| {
        animated
            .rank_entry(slot, rank)
            .map(|entry| ability_numbers(info, &entry, power))
            .unwrap_or_else(|| AbilityNumbers {
                damage_label: damage_label(info),
                damage: "-".to_string(),
                range: "-".to_string(),
                cooldown: "-".to_string(),
            })
    };
    let upgrades = (rank + 1..=max_rank)
        .map(|next| AbilityUpgrade {
            rank: next,
            numbers: numbers(next),
            needs_level: Some(rank_level(slot, next)).filter(|needs| *needs > level),
        })
        .collect();
    let hint = match animated.upgrade_block(slot) {
        None => Some(skill_point_hint(slot, rank, animated.skill_points)),
        Some(UpgradeBlock::NeedsLevel(needs)) if animated.skill_points > 0 => {
            Some(format!("Next rank unlocks at level {needs}"))
        }
        _ => None,
    };
    Some(AbilityTooltipState {
        key: ["Q", "W", "E", "R"][slot.min(3)].to_string(),
        name: title_case(&info.name),
        summary: info.description.clone(),
        rank,
        max_rank,
        current: numbers(rank),
        upgrades,
        note: info
            .damage_note
            .as_ref()
            .map(|note| format!("{}: {note}", damage_label(info))),
        hint,
        at: mouse,
    })
}

fn damage_label(info: &ChampionAbility) -> String {
    info.damage_label
        .clone()
        .unwrap_or_else(|| "Damage".to_string())
}

/// One rank's numbers as the tooltip shows them: damage with the hero's
/// current power applied, range in world units ("Self" for abilities
/// centred on the caster), cooldown in seconds.
pub fn ability_numbers(info: &ChampionAbility, rank: &AbilityRank, power: f32) -> AbilityNumbers {
    AbilityNumbers {
        damage_label: damage_label(info),
        damage: rank
            .damage
            .map(|damage| format!("{}", (damage * power).round() as i64))
            .unwrap_or_else(|| "-".to_string()),
        range: if rank.range <= 0.0 {
            "Self".to_string()
        } else {
            format!("{}", rank.range.round() as i64)
        },
        cooldown: seconds(rank.cooldown),
    }
}

fn seconds(value: f32) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}s")
}

/// "PLAGUE OF FROGS" -> "Plague of Frogs"; "BANDOS' WRATH" -> "Bandos' Wrath".
pub fn title_case(name: &str) -> String {
    const SMALL: [&str; 8] = ["of", "the", "a", "an", "and", "to", "in", "on"];
    name.split(' ')
        .enumerate()
        .map(|(n, word)| {
            let lower = word.to_lowercase();
            if n > 0 && SMALL.contains(&lower.as_str()) {
                return lower;
            }
            let mut chars = lower.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn count(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_score_row_counts_item_cost_in_net_worth_and_rounds_the_respawn_up() {
        let item = |id: &str, cost: u32, obj: Option<u32>| ItemConfig {
            id: id.to_string(),
            name: id.to_string(),
            obj,
            category: "melee".to_string(),
            color: [0, 0, 0],
            cost,
            damage: 0.0,
            health: 0.0,
            mana: 0.0,
            speed: 0.0,
            ability: 0.0,
            tier: None,
            builds_from: Vec::new(),
            thrall: None,
        };
        let sword = item("sword", 1200, Some(1277));
        let charm = item("charm", 300, None);
        let score = crate::animated::score::HeroScore {
            kills: 2,
            deaths: 1,
            assists: 3,
            cs: 40,
        };
        let row = score_row(
            "Cerberus".to_string(),
            300,
            score,
            450,
            &[&sword, &charm],
            Some(6.2),
            false,
        );
        assert_eq!(row.gold, 1950);
        // Only items with a cache model have an icon to draw.
        assert_eq!(row.items, vec!["icon.item.sword".to_string()]);
        assert_eq!(row.respawn_seconds, Some(7));
        assert_eq!(row.level, 255);
        assert_eq!(
            (row.kills, row.deaths, row.assists, row.creep_score),
            (2, 1, 3, 40)
        );
        assert!(!row.me && row.portrait.is_none());
        let me = score_row("You".into(), 5, score, 0, &[], None, true);
        assert_eq!(me.portrait.as_deref(), Some("portrait.self"));
        assert_eq!(me.respawn_seconds, None);
    }

    #[test]
    fn ability_names_and_numbers_read_cleanly() {
        assert_eq!(title_case("PLAGUE OF FROGS"), "Plague of Frogs");
        assert_eq!(title_case("BANDOS' WRATH"), "Bandos' Wrath");
        assert_eq!(title_case("YARRRRRRR!"), "Yarrrrrrr!");
        assert_eq!(seconds(6.5), "6.5s");
        assert_eq!(seconds(7.0), "7s");
        assert_eq!(seconds(4.75), "4.75s");
        let info: ChampionAbility = serde_json::from_str(
            r#"{"key":"E","name":"SWAMP CROAK","description":"Croak.","damage_label":"Shield",
                "ranks":[{"damage":90,"range":0,"cooldown":12}]}"#,
        )
        .unwrap();
        let numbers = ability_numbers(&info, &info.ranks[0], 1.5);
        assert_eq!(numbers.damage_label, "Shield");
        assert_eq!(numbers.damage, "135");
        assert_eq!(numbers.range, "Self");
        assert_eq!(numbers.cooldown, "12s");
    }

    #[test]
    fn minimap_click_inverts_the_dot_mapping() {
        let min = Vec2::new(340_000.0, 371_000.0);
        let max = Vec2::new(382_000.0, 408_000.0);
        for (x, z) in [
            (350_000.0, 380_000.0),
            (381_000.0, 372_000.0),
            (361_000.0, 399_500.0),
        ] {
            let dot = map_point(Vec3::new(x, 0.0, z), min, max);
            let (wx, wz) = world_from_map(dot.x as f32 / MAP.0, dot.y as f32 / MAP.1, min, max);
            // One map pixel is (max - min) / 254 world units.
            assert!((wx - x).abs() <= (max.x - min.x) / MAP.0, "x {wx} vs {x}");
            assert!((wz - z).abs() <= (max.y - min.y) / MAP.1, "z {wz} vs {z}");
        }
        assert_eq!(world_from_map(-1.0, 2.0, min, max), (min.x, max.y));
    }

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
    fn skill_point_hint_names_the_way_to_spend_it() {
        assert_eq!(skill_point_hint(1, 0, 2), "Click to learn (2 left)");
        assert_eq!(
            skill_point_hint(3, 1, 1),
            "Click + or Ctrl+R to rank up (1 left)"
        );
    }

    #[test]
    fn bosses_get_named_lines_and_plain_camps_group_by_buff() {
        let camp = |name: &str, buff, boss, respawn| CampSnapshot {
            name: Some(name.to_string()),
            buff,
            boss,
            respawn,
        };
        let camps = [
            camp("JalTok-Jad", CampBuff::Might, false, None),
            camp("Crypt spider", CampBuff::Swift, true, Some(36.2)),
            camp("Spider", CampBuff::Might, false, Some(31.4)),
            camp("Giant bat", CampBuff::Swift, false, None),
            camp("Albino bat", CampBuff::Might, true, None),
            camp("Kree'arra Jr.", CampBuff::Might, false, Some(12.0)),
        ];
        let lines = camp_lines(&camps, &[(0, CampBuff::Might, 41.5)]);
        let view: Vec<(&str, &str, &str, Option<&str>)> = lines
            .iter()
            .map(|l| {
                (
                    l.name.as_str(),
                    l.grant.as_str(),
                    l.state.as_str(),
                    l.detail.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            view,
            vec![
                ("Crypt spider", "Swift", "37s", None),
                ("Albino bat", "Might", "Alive", None),
                (
                    "Jungle camps",
                    "Might",
                    "1/3  12s",
                    Some("Blue holds Might 42s")
                ),
                ("Jungle camps", "Swift", "Alive", None),
            ]
        );
        // Six objective lines with the two nexuses: the panel's usual height.
        assert_eq!(lines.len() + 2, 6);

        // A boss with a buff of its own names its holder on its own line;
        // a team can hold several buffs at once.
        let mut camps = camps.to_vec();
        camps.push(camp("Sarachnis", CampBuff::BroodVenom, true, Some(80.0)));
        let held = [(0, CampBuff::Might, 41.5), (1, CampBuff::BroodVenom, 20.2)];
        let lines = camp_lines(&camps, &held);
        let sarachnis = lines.iter().find(|l| l.name == "Sarachnis").unwrap();
        assert_eq!(sarachnis.grant, "Brood Venom");
        assert_eq!(
            sarachnis.detail.as_deref(),
            Some("Red holds Brood Venom 21s")
        );
        // Albino bat's Might is also a plain-camp buff, so its holder stays
        // on the Jungle camps line.
        let albino = lines.iter().find(|l| l.name == "Albino bat").unwrap();
        assert_eq!(albino.detail, None);
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
            camp: false,
            structure: false,
            name_color: [255, 105, 95],
            name_scale: 2.0,
            pk: true,
            tag: Some("NO SAFE".to_string()),
            width: 184.0,
        };
        let hero = plate(&frame).expect("a hero plate");
        assert_eq!(hero.at, (400, 251));
        assert_eq!(hero.kind, PlateKind::Hero);
        assert_eq!((hero.health, hero.max_health), (208, 420));
        assert_eq!((hero.level, hero.blue), (7, false));
        assert!(hero.pk);
        assert_eq!(hero.tag.as_deref(), Some("NO SAFE"));

        frame.team = 2;
        frame.hero = false;
        frame.camp = true;
        let camp = plate(&frame).expect("a neutral camp plate");
        assert_eq!(camp.kind, PlateKind::Camp);
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
            tier: None,
            builds_from: Vec::new(),
            thrall: None,
        };
        let converted = shop_item(&item);
        assert_eq!(converted.name, "Test blade");
        assert_eq!(converted.price, 700);
        assert_eq!(converted.colour, [10, 20, 30]);
        assert_eq!(converted.stats, ["+8 damage", "+15% ability power"]);

        let catalog = ItemShop {
            items: vec![item],
            owned: [None; 6],
            undo: Vec::new(),
            open: true,
            selected: Some(0),
            category: 2,
            scroll: 0,
            query: String::new(),
            typing: false,
            recommended: Vec::new(),
            recommendations: Default::default(),
        };
        let state = shop(&catalog);
        assert!(state.open);
        assert_eq!(state.category, 2);
        assert_eq!(state.categories[2], "MELEE");
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
            tier: None,
            builds_from: Vec::new(),
            thrall: None,
        };
        let catalog = ItemShop {
            items: vec![item("Sword", [1, 2, 3]), item("Shield", [4, 5, 6])],
            owned: [Some(1), None, Some(0), Some(99), None, None],
            undo: Vec::new(),
            open: false,
            selected: None,
            category: 0,
            scroll: 0,
            query: String::new(),
            typing: false,
            recommended: Vec::new(),
            recommendations: Default::default(),
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
            tier: None,
            builds_from: Vec::new(),
            thrall: None,
        };
        let catalog = ItemShop {
            items: vec![item],
            owned: [None; 6],
            undo: Vec::new(),
            open: true,
            selected: Some(0),
            category: 1,
            scroll: 0,
            query: String::new(),
            typing: false,
            recommended: Vec::new(),
            recommendations: Default::default(),
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
        assert_eq!(kind_name(UnitKind::BotChampion), "Hero");
        assert_eq!(kind_name(UnitKind::MeleeMinion), "Minion");
        assert_eq!(kind_name(UnitKind::Tower), "Structure");
        assert_eq!(kind_name(UnitKind::Camp), "Camp");
        assert_eq!(kind_name(UnitKind::Ambient), "Unit");
    }
}
