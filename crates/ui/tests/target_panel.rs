//! The target panel checked as rectangles and as pixels: the portrait, name,
//! health, description and eight stat figures stay inside their 332 by 158
//! rectangle, and a theme with only fonts still draws the fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{sample, target_panel};
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
        .panel("panel.dark", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder
        .panel("panel.slot", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    for role in [
        "text",
        "text.quiet",
        "text.title",
        "text.team.blue",
        "text.team.red",
        "announcement",
        "bar.health",
        "bar.back",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_target_panel_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    let target = state.target.as_ref().expect("the sample selects a target");
    for s in 1..=4u32 {
        let area = Rect::new(
            10,
            10,
            target_panel::SIZE.0 * s as i32,
            target_panel::SIZE.1 * s as i32,
        );
        let mut list = DrawList::new((1400, 700), s, &theme.atlas);
        target_panel::draw(&theme, &mut list, area, target);
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
    let target = state.target.as_ref().expect("the sample selects a target");
    let area = Rect::new(20, 20, target_panel::SIZE.0, target_panel::SIZE.1);
    let mut list = DrawList::new((380, 200), 1, &theme.atlas);
    target_panel::draw(&theme, &mut list, area, target);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 380 * 200];
    raster::render(&vertices, &indices, &theme.atlas, 380, 200, &mut picture);
    for y in 0..200 {
        for x in 0..380 {
            let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
            if !inside {
                assert_eq!(
                    picture[y as usize * 380 + x as usize],
                    GROUND,
                    "pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_panel() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.small", boxes(), 8, 10);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let target = state.target.as_ref().expect("the sample selects a target");
    let area = Rect::new(0, 0, target_panel::SIZE.0, target_panel::SIZE.1);
    let mut list = DrawList::new(
        (target_panel::SIZE.0 as u32, target_panel::SIZE.1 as u32),
        1,
        &theme.atlas,
    );
    target_panel::draw(&theme, &mut list, area, target);
    assert!(!list.is_empty());
}
