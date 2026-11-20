//! Made-up values for stage 3 (docs/INTERFACE_STAGE3_PLAN.md, section 3).
//! Every one is static and plainly a sample: the screen marks them, and no
//! component reads the match until stage 4.

use crate::hud::state::{
    AbilityState, AllyState, CampState, ChatKind, ChatLine, HudState, InventoryItem, KillFeedEntry,
    LaneState, MapPoint, MinimapState, NexusState, ObjectivesState, PlateKind, ScoreState,
    ShopItem, ShopState, StructureMark, TargetStat, TargetState, TooltipState, UnitPlate,
};

pub fn state() -> HudState {
    state_for((1920, 1080))
}

/// The sample at this window's size. Stage 3 has no live units, so the unit
/// plates' positions are made up, as every other value is; they are placed
/// in fractions of the window only so that the sample looks like the
/// reference at any size the layout screen is run at.
pub fn state_for(window: (u32, u32)) -> HudState {
    let mut state = base();
    state.plates = plates(window);
    if let Some(tooltip) = state.tooltip.as_mut() {
        tooltip.at = ((window.0 / 2) as i32, (window.1 * 35 / 100) as i32);
    }
    state
}

fn base() -> HudState {
    HudState {
        score: ScoreState {
            clock: "12:34".to_string(),
            blue_kills: 7,
            red_kills: 5,
            kills: 3,
            deaths: 1,
            assists: 4,
            creep_score: 86,
        },
        ability: AbilityState {
            health: 438,
            max_health: 730,
            resource: 116,
            max_resource: 260,
            level: 7,
            experience: 62,
            cooldowns: [0.0, 3.2, 0.0, 18.5],
        },
        minimap: MinimapState {
            lanes: vec![
                vec![
                    MapPoint { x: 42, y: 210 },
                    MapPoint { x: 44, y: 70 },
                    MapPoint { x: 100, y: 33 },
                    MapPoint { x: 205, y: 42 },
                ],
                vec![
                    MapPoint { x: 42, y: 210 },
                    MapPoint { x: 126, y: 126 },
                    MapPoint { x: 205, y: 42 },
                ],
                vec![
                    MapPoint { x: 42, y: 210 },
                    MapPoint { x: 190, y: 208 },
                    MapPoint { x: 212, y: 150 },
                    MapPoint { x: 205, y: 42 },
                ],
            ],
            river: vec![
                MapPoint { x: 0, y: 42 },
                MapPoint { x: 58, y: 72 },
                MapPoint { x: 126, y: 126 },
                MapPoint { x: 196, y: 182 },
                MapPoint { x: 254, y: 218 },
            ],
            bases: vec![
                StructureMark {
                    at: MapPoint { x: 38, y: 216 },
                    blue: true,
                },
                StructureMark {
                    at: MapPoint { x: 214, y: 36 },
                    blue: false,
                },
            ],
            allies: vec![
                MapPoint { x: 74, y: 158 },
                MapPoint { x: 112, y: 108 },
                MapPoint { x: 160, y: 92 },
                MapPoint { x: 198, y: 58 },
            ],
            enemies: vec![
                MapPoint { x: 102, y: 150 },
                MapPoint { x: 138, y: 120 },
                MapPoint { x: 166, y: 72 },
                MapPoint { x: 176, y: 178 },
                MapPoint { x: 214, y: 116 },
            ],
            camps: vec![MapPoint { x: 82, y: 92 }, MapPoint { x: 170, y: 156 }],
            structures: vec![
                StructureMark {
                    at: MapPoint { x: 48, y: 166 },
                    blue: true,
                },
                StructureMark {
                    at: MapPoint { x: 86, y: 68 },
                    blue: true,
                },
                StructureMark {
                    at: MapPoint { x: 204, y: 88 },
                    blue: false,
                },
                StructureMark {
                    at: MapPoint { x: 164, y: 190 },
                    blue: false,
                },
            ],
            camera: (96, 92, 58, 48),
        },
        objectives: ObjectivesState {
            lanes: vec![
                LaneState {
                    name: "Top".to_string(),
                    blue_towers: [true, false],
                    blue_inhibitor: true,
                    red_towers: [true, true],
                    red_inhibitor: true,
                },
                LaneState {
                    name: "Mid".to_string(),
                    blue_towers: [true, true],
                    blue_inhibitor: true,
                    red_towers: [true, false],
                    red_inhibitor: true,
                },
                LaneState {
                    name: "Bot".to_string(),
                    blue_towers: [true, true],
                    blue_inhibitor: false,
                    red_towers: [true, true],
                    red_inhibitor: true,
                },
            ],
            nexuses: vec![
                NexusState {
                    name: "Blue nexus".to_string(),
                    blue: true,
                    state: "Shielded".to_string(),
                },
                NexusState {
                    name: "Red nexus".to_string(),
                    blue: false,
                    state: "620".to_string(),
                },
            ],
            second_lines: 4,
            usual_second_lines: 0,
            camps: vec![
                CampState {
                    name: "Might camp".to_string(),
                    grant: "Might".to_string(),
                    state: "Taken".to_string(),
                    detail: Some("Blue Might 58s".to_string()),
                },
                CampState {
                    name: "Swift camp".to_string(),
                    grant: "Swift".to_string(),
                    state: "Alive".to_string(),
                    detail: Some("Swift 12s".to_string()),
                },
                CampState {
                    name: "Might camp".to_string(),
                    grant: "Might".to_string(),
                    state: "0:42".to_string(),
                    detail: Some("Red Might 42s".to_string()),
                },
                CampState {
                    name: "Swift camp".to_string(),
                    grant: "Swift".to_string(),
                    state: "Alive".to_string(),
                    detail: Some("no buff".to_string()),
                },
            ],
        },
        allies: vec![
            AllyState {
                name: "Monkey".to_string(),
                health: 612,
                max_health: 700,
                resource: 180,
                max_resource: 260,
                level: 7,
                dead: false,
                respawn_seconds: None,
            },
            AllyState {
                name: "Jad".to_string(),
                health: 208,
                max_health: 700,
                resource: 90,
                max_resource: 260,
                level: 6,
                dead: false,
                respawn_seconds: None,
            },
            AllyState {
                name: "Kree".to_string(),
                health: 0,
                max_health: 420,
                resource: 0,
                max_resource: 240,
                level: 6,
                dead: true,
                respawn_seconds: Some(12),
            },
            AllyState {
                name: "Monkey".to_string(),
                health: 500,
                max_health: 700,
                resource: 210,
                max_resource: 260,
                level: 7,
                dead: false,
                respawn_seconds: None,
            },
        ],
        plates: Vec::new(),
        target: Some(TargetState {
            name: "Top outer statue".to_string(),
            kind: "Structure".to_string(),
            blue: false,
            description: "Outer tower - fires on enemy minions.".to_string(),
            health: 620,
            max_health: 1000,
            stats: vec![
                TargetStat {
                    label: "Attack".to_string(),
                    value: "92".to_string(),
                },
                TargetStat {
                    label: "Attack speed".to_string(),
                    value: "1.2".to_string(),
                },
                TargetStat {
                    label: "Defence melee".to_string(),
                    value: "60".to_string(),
                },
                TargetStat {
                    label: "Defence mage".to_string(),
                    value: "40".to_string(),
                },
                TargetStat {
                    label: "Power".to_string(),
                    value: "0".to_string(),
                },
                TargetStat {
                    label: "Range".to_string(),
                    value: "3".to_string(),
                },
                TargetStat {
                    label: "Defence ranged".to_string(),
                    value: "55".to_string(),
                },
                TargetStat {
                    label: "Move".to_string(),
                    value: "0".to_string(),
                },
            ],
        }),
        inventory: vec![
            Some(InventoryItem {
                name: "Bronze sword".to_string(),
                colour: [170, 112, 66],
            }),
            Some(InventoryItem {
                name: "Iron platebody".to_string(),
                colour: [150, 150, 160],
            }),
            Some(InventoryItem {
                name: "Amulet of fury".to_string(),
                colour: [210, 170, 60],
            }),
            None,
            None,
            None,
        ],
        gold: 1240,
        chat: vec![
            ChatLine {
                kind: ChatKind::System,
                text: "Welcome to the Rift.".to_string(),
            },
            ChatLine {
                kind: ChatKind::Player {
                    speaker: "Ava".to_string(),
                    hero: "Monkey".to_string(),
                    blue: true,
                },
                text: "top is pushed.".to_string(),
            },
            ChatLine {
                kind: ChatKind::Player {
                    speaker: "Rex".to_string(),
                    hero: "Jad".to_string(),
                    blue: true,
                },
                text: "rotating to mid.".to_string(),
            },
            ChatLine {
                kind: ChatKind::Event,
                text: "Blue team took a tower.".to_string(),
            },
            ChatLine {
                kind: ChatKind::Player {
                    speaker: "Kai".to_string(),
                    hero: "Kree".to_string(),
                    blue: false,
                },
                text: "need two more for drake.".to_string(),
            },
            ChatLine {
                kind: ChatKind::System,
                text: "Press Enter to Chat".to_string(),
            },
        ],
        feed: vec![
            KillFeedEntry {
                killer: "Jad".to_string(),
                victim: "Kree".to_string(),
                gold: 300,
                killer_blue: true,
                victim_blue: false,
            },
            KillFeedEntry {
                killer: "Kree".to_string(),
                victim: "Jad".to_string(),
                gold: 274,
                killer_blue: false,
                victim_blue: true,
            },
        ],
        announcement: Some("Giant Mole has appeared!".to_string()),
        tooltip: Some(TooltipState {
            title: "Berserker ring".to_string(),
            price: 700,
            stats: vec!["+8 damage".to_string(), "+15% ability power".to_string()],
            passive: Some("Berserk".to_string()),
            text: "Every fourth strike deals 15% more damage.".to_string(),
            at: (0, 0),
        }),
        shop: ShopState {
            open: true,
            category: "Items".to_string(),
            search: "Search items".to_string(),
            items: vec![
                shop_item("Bronze sword", 300, [180, 120, 60], &["+10 damage"]),
                shop_item("Iron platebody", 400, [160, 160, 170], &["+150 health"]),
                shop_item(
                    "Amulet of fury",
                    800,
                    [80, 200, 220],
                    &["+15 damage", "+50 health", "+20 mana"],
                ),
                shop_item(
                    "Berserker ring",
                    700,
                    [220, 80, 80],
                    &["+8 damage", "+15% ability power"],
                ),
                shop_item(
                    "Ahrim's staff",
                    900,
                    [120, 80, 200],
                    &["+40 mana", "+30% ability power"],
                ),
                shop_item(
                    "Dragon boots",
                    600,
                    [200, 60, 60],
                    &["+30 health", "+40 speed"],
                ),
            ],
            selected: 3,
            scroll: 0,
            hint: "Click an item to inspect it.".to_string(),
        },
    }
}

/// Made-up unit plates at fractions of this window, drawn in the order they
/// are listed: minions under structures under heroes.
fn plates(window: (u32, u32)) -> Vec<UnitPlate> {
    let x = |f: f32| (window.0 as f32 * f).round() as i32;
    let y = |f: f32| (window.1 as f32 * f).round() as i32;
    let at = |fx: f32, fy: f32| (x(fx), y(fy));
    vec![
        plate(
            at(0.450, 0.610),
            PlateKind::Minion,
            "Blue melee",
            1,
            55,
            55,
            true,
        ),
        plate(
            at(0.475, 0.620),
            PlateKind::Minion,
            "Blue melee",
            1,
            41,
            55,
            true,
        ),
        plate(
            at(0.530, 0.600),
            PlateKind::Minion,
            "Red melee",
            1,
            55,
            55,
            false,
        ),
        plate(
            at(0.555, 0.610),
            PlateKind::Minion,
            "Red melee",
            1,
            28,
            55,
            false,
        ),
        plate(
            at(0.340, 0.330),
            PlateKind::Structure,
            "Top tower",
            1,
            1000,
            1000,
            true,
        ),
        plate(
            at(0.670, 0.310),
            PlateKind::Structure,
            "Top outer statue",
            1,
            620,
            1000,
            false,
        ),
        plate(at(0.430, 0.500), PlateKind::Hero, "Jad", 6, 208, 700, true),
        plate(
            at(0.500, 0.400),
            PlateKind::Hero,
            "Monkey",
            7,
            438,
            730,
            true,
        ),
        plate(
            at(0.640, 0.360),
            PlateKind::Hero,
            "Kree",
            6,
            208,
            420,
            false,
        ),
    ]
}

fn plate(
    at: (i32, i32),
    kind: PlateKind,
    name: &str,
    level: u8,
    health: u32,
    max_health: u32,
    blue: bool,
) -> UnitPlate {
    UnitPlate {
        at,
        kind,
        name: name.to_string(),
        level,
        health,
        max_health,
        blue,
    }
}

fn shop_item(name: &str, price: u32, colour: [u8; 3], stats: &[&str]) -> ShopItem {
    ShopItem {
        name: name.to_string(),
        price,
        colour,
        stats: stats.iter().map(|line| (*line).to_string()).collect(),
        builds_into: None,
    }
}
