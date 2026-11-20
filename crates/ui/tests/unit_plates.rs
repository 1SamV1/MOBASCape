//! Unit plates checked as rectangles and pixels: every kind stays inside the
//! room above its anchor at scales 1 to 4, nothing is drawn outside that
//! room, and a theme with only fonts still draws the fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::state::{PlateKind, UnitPlate};
use rift_ui::hud::{sample, unit_plates};
use rift_ui::raster;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const GROUND: [u8; 4] = [8, 11, 17, 255];

fn numbered(width: usize, height: usize) -> Image {
    Image::new(
        width,
        height,
        (0..width * height)
            .map(|i| [(i % width) as u8, (i / width) as u8, 100, 255])
            .collect(),
    )
}

fn boxes() -> Vec<GlyphSource> {
    (32u8..127)
        .map(|code| {
            let c = code as char;
            let w = (code % 3 + 2) as usize;
            GlyphSource {
                character: c,
                image: (c != ' ').then(|| Image::filled(w, 6, [0, 0, 0, 255])),
                left: 1,
                top: 2,
                advance: w as i32 + 2,
            }
        })
        .collect()
}

fn theme() -> Theme {
    let mut builder = ThemeBuilder::new();
    builder
        .panel("panel.slot", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    for role in [
        "text",
        "text.team.blue",
        "text.team.red",
        "bar.health",
        "bar.enemy",
        "bar.back",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

fn plate(at: (i32, i32), kind: PlateKind) -> UnitPlate {
    UnitPlate {
        at,
        kind,
        name: "Monkey".to_string(),
        level: 7,
        health: 438,
        max_health: 730,
        blue: true,
    }
}

#[test]
fn every_plate_stays_inside_its_room_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        for plate in &state.plates {
            let area = unit_plates::bounds(plate, s as i32);
            let mut list = DrawList::new((1920, 1080), s, &theme.atlas);
            unit_plates::draw(&theme, &mut list, std::slice::from_ref(plate));
            let (vertices, indices) = list.finish();
            assert!(!vertices.is_empty() && indices.len() % 6 == 0);
            for vertex in vertices {
                assert!(
                    vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32,
                    "{} at {s}: x {} outside {area:?}",
                    plate.name,
                    vertex.pos[0]
                );
                assert!(
                    vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32,
                    "{} at {s}: y {} outside {area:?}",
                    plate.name,
                    vertex.pos[1]
                );
            }
        }
    }
}

#[test]
fn nothing_is_drawn_outside_the_plates() {
    let theme = theme();
    let plates = [
        plate((200, 150), PlateKind::Hero),
        plate((80, 220), PlateKind::Minion),
        plate((320, 230), PlateKind::Structure),
    ];
    let mut list = DrawList::new((400, 300), 1, &theme.atlas);
    unit_plates::draw(&theme, &mut list, &plates);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 400 * 300];
    raster::render(&vertices, &indices, &theme.atlas, 400, 300, &mut picture);
    let rooms: Vec<Rect> = plates
        .iter()
        .map(|plate| unit_plates::bounds(plate, 1))
        .collect();
    for y in 0..300 {
        for x in 0..400 {
            let inside = rooms
                .iter()
                .any(|room| x >= room.x && x < room.right() && y >= room.y && y < room.bottom());
            if !inside {
                assert_eq!(
                    picture[y as usize * 400 + x as usize],
                    GROUND,
                    "pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_plates() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.small", boxes(), 8, 10);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let plates = [
        plate((100, 100), PlateKind::Hero),
        plate((200, 100), PlateKind::Minion),
        plate((300, 100), PlateKind::Structure),
    ];
    let mut list = DrawList::new((400, 200), 1, &theme.atlas);
    unit_plates::draw(&theme, &mut list, &plates);
    assert!(!list.is_empty());
}
