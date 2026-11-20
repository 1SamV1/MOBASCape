//! The ability bar checked as rectangles and as pixels: it stays inside the
//! place layout.rs gave it, draws its slots, portrait and bars, and touches
//! nothing outside.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{ability_bar, sample};
use rift_ui::raster;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const GROUND: [u8; 4] = [9, 12, 18, 255];

fn numbered(width: usize, height: usize) -> Image {
    Image::new(
        width,
        height,
        (0..width * height)
            .map(|i| [(i % width) as u8, (i / width) as u8, 110, 255])
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
        .panel("panel.stone", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder
        .panel("panel.slot", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder.font("font.bold", boxes(), 8, 12);
    builder.font("font.small", boxes(), 8, 10);
    builder.colour("text", [1.0; 4]);
    builder.colour("text.quiet", [0.7, 0.7, 0.7, 1.0]);
    builder.colour("text.title", [1.0, 0.6, 0.1, 1.0]);
    builder.colour("bar.health", [0.0, 0.8, 0.0, 1.0]);
    builder.colour("bar.resource", [0.0, 0.8, 1.0, 1.0]);
    builder.colour("bar.back", [0.0, 0.0, 0.0, 0.8]);
    builder.build()
}

#[test]
fn the_ability_bar_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(
            10,
            10,
            ability_bar::SIZE.0 * s as i32,
            ability_bar::SIZE.1 * s as i32,
        );
        let mut list = DrawList::new((2000, 500), s, &theme.atlas);
        ability_bar::draw(&theme, &mut list, area, &state.ability);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        for vertex in vertices {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}

#[test]
fn nothing_is_drawn_outside_the_rectangle() {
    let theme = theme();
    let state = sample::state();
    let area = Rect::new(40, 30, ability_bar::SIZE.0, ability_bar::SIZE.1);
    let mut list = DrawList::new((600, 180), 1, &theme.atlas);
    ability_bar::draw(&theme, &mut list, area, &state.ability);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 600 * 180];
    raster::render(&vertices, &indices, &theme.atlas, 600, 180, &mut picture);
    for y in 0..180 {
        for x in 0..600 {
            let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
            if !inside {
                assert_eq!(
                    picture[y as usize * 600 + x as usize],
                    GROUND,
                    "pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn a_theme_with_only_a_font_still_draws_the_bar() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.bold", boxes(), 8, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let area = Rect::new(0, 0, ability_bar::SIZE.0, ability_bar::SIZE.1);
    let mut list = DrawList::new(
        (ability_bar::SIZE.0 as u32, ability_bar::SIZE.1 as u32),
        1,
        &theme.atlas,
    );
    ability_bar::draw(&theme, &mut list, area, &state.ability);
    assert!(!list.is_empty());
}
