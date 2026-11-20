//! The chat checked as rectangles and pixels: wrapped lines, scrollbar and
//! tabs stay inside 440 by 140, the collapsed last-line chat stays inside
//! 440 by 24, and a theme with only fonts still draws the fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{chat, sample};
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
        .panel("panel.parchment", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder
        .panel("button.stone", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder.sprite("scrollbar", numbered(8, 8));
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    for role in [
        "text",
        "text.quiet",
        "text.title",
        "text.outline",
        "text.team.blue",
        "text.team.red",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

fn check_contained(list: &DrawList, area: Rect) {
    let (vertices, indices) = list.finish();
    assert!(!vertices.is_empty() && indices.len() % 6 == 0);
    for vertex in vertices {
        assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
        assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
    }
}

#[test]
fn the_chat_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(10, 10, chat::SIZE.0 * s as i32, chat::SIZE.1 * s as i32);
        let mut list = DrawList::new((1400, 700), s, &theme.atlas);
        chat::draw(&theme, &mut list, area, &state.chat);
        check_contained(&list, area);
    }
}

#[test]
fn the_last_line_only_chat_stays_inside_its_rectangle() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(10, 10, chat::SIZE.0 * s as i32, 24 * s as i32);
        let mut list = DrawList::new((1400, 700), s, &theme.atlas);
        chat::draw(&theme, &mut list, area, &state.chat);
        check_contained(&list, area);
    }
}

#[test]
fn nothing_is_drawn_outside_the_rectangle() {
    let theme = theme();
    let state = sample::state();
    let area = Rect::new(20, 20, chat::SIZE.0, chat::SIZE.1);
    let mut list = DrawList::new((500, 200), 1, &theme.atlas);
    chat::draw(&theme, &mut list, area, &state.chat);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 500 * 200];
    raster::render(&vertices, &indices, &theme.atlas, 500, 200, &mut picture);
    for y in 0..200 {
        for x in 0..500 {
            let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
            if !inside {
                assert_eq!(
                    picture[y as usize * 500 + x as usize],
                    GROUND,
                    "pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_chat() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.body", boxes(), 10, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let area = Rect::new(0, 0, chat::SIZE.0, chat::SIZE.1);
    let mut list = DrawList::new((chat::SIZE.0 as u32, chat::SIZE.1 as u32), 1, &theme.atlas);
    chat::draw(&theme, &mut list, area, &state.chat);
    assert!(!list.is_empty());
}
