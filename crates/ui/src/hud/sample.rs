//! Made-up values for stage 3 (docs/INTERFACE_STAGE3_PLAN.md, section 3).
//! Every one is static and plainly a sample: the screen marks them, and no
//! component reads the match until stage 4.

use crate::hud::state::{
    AbilityState, AllyState, CampState, HudState, InventoryItem, KillFeedEntry, MapPoint,
    MinimapState, ObjectivesState, ScoreState, StructureMark, TargetState,
};

pub fn state() -> HudState {
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
            lines: 6,
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
            },
            AllyState {
                name: "Jad".to_string(),
                health: 208,
                max_health: 700,
                resource: 90,
                max_resource: 260,
                level: 6,
                dead: false,
            },
            AllyState {
                name: "Kree".to_string(),
                health: 0,
                max_health: 420,
                resource: 0,
                max_resource: 240,
                level: 6,
                dead: true,
            },
            AllyState {
                name: "Monkey".to_string(),
                health: 500,
                max_health: 700,
                resource: 210,
                max_resource: 260,
                level: 7,
                dead: false,
            },
        ],
        target: Some(TargetState {
            name: "Top outer statue".to_string(),
            kind: "Structure".to_string(),
            health: 620,
            max_health: 1000,
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
            "Welcome to the Rift.".to_string(),
            "Monkey: top is pushed.".to_string(),
            "Jad: rotating to mid.".to_string(),
            "Game: blue team took a tower.".to_string(),
            "Kree: need two more for drake.".to_string(),
            "Press Enter to Chat".to_string(),
        ],
        feed: vec![
            KillFeedEntry {
                killer: "Jad".to_string(),
                victim: "Kree".to_string(),
                gold: 300,
                killer_blue: true,
            },
            KillFeedEntry {
                killer: "Kree".to_string(),
                victim: "Jad".to_string(),
                gold: 274,
                killer_blue: false,
            },
        ],
        announcement: Some("Giant Mole has appeared!".to_string()),
        tooltip: Some("Berserker ring: +8 damage".to_string()),
    }
}
