//! The tooltip checked as rectangles and pixels: it sits beside the cursor,
//! stays inside the window at scales 1 to 4, its outlined contents stay
//! inside its own rectangle, longer text makes it taller and not wider, and
//! a theme with only fonts still draws the fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{sample, tooltip};
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
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    for role in ["text", "text.good", "text.title", "announcement"] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_tooltip_is_beside_the_cursor_and_inside_the_window_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    let tooltip = state.tooltip.as_ref().expect("the sample has a tooltip");
    let window = Rect::new(0, 0, 1920, 1080);
    for s in 1..=4u32 {
        let area = tooltip::rect(&theme, tooltip, window, s as i32);
        assert!(area.x >= window.x && area.y >= window.y);
        assert!(area.right() <= window.right() && area.bottom() <= window.bottom());
        let mut list = DrawList::new((1920, 1080), s, &theme.atlas);
        tooltip::draw(&theme, &mut list, tooltip);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        for vertex in vertices {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}

#[test]
fn the_tooltip_turns_away_from_both_bottom_corners() {
    let theme = theme();
    let mut tooltip = sample::state().tooltip.expect("the sample has a tooltip");
    let window = Rect::new(0, 0, 1280, 720);
    for at in [(1279, 719), (0, 0), (1279, 0), (0, 719)] {
        tooltip.at = at;
        let area = tooltip::rect(&theme, &tooltip, window, 1);
        assert!(area.x >= window.x && area.y >= window.y);
        assert!(area.right() <= window.right() && area.bottom() <= window.bottom());
    }
}

#[test]
fn nothing_is_drawn_outside_the_rectangle() {
    let theme = theme();
    let mut tooltip = sample::state().tooltip.expect("the sample has a tooltip");
    tooltip.at = (100, 100);
    let window = Rect::new(0, 0, 500, 400);
    let area = tooltip::rect(&theme, &tooltip, window, 1);
    let mut list = DrawList::new((500, 400), 1, &theme.atlas);
    tooltip::draw(&theme, &mut list, &tooltip);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 500 * 400];
    raster::render(&vertices, &indices, &theme.atlas, 500, 400, &mut picture);
    for y in 0..400 {
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
fn longer_passive_text_makes_the_tooltip_taller_not_wider() {
    let theme = theme();
    let sample = sample::state().tooltip.expect("the sample has a tooltip");
    let mut longer = sample.clone();
    longer.text = "Every fourth strike deals bonus damage and heals the bearer. ".repeat(4);
    let (w, h) = tooltip::size(&theme, &sample);
    let (long_w, long_h) = tooltip::size(&theme, &longer);
    assert_eq!(w, long_w);
    assert!(long_h > h);
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_tooltip() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.body", boxes(), 10, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let tooltip = state.tooltip.as_ref().expect("the sample has a tooltip");
    let mut list = DrawList::new((1920, 1080), 1, &theme.atlas);
    tooltip::draw(&theme, &mut list, tooltip);
    assert!(!list.is_empty());
}
