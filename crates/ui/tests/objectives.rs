//! The objectives panel checked as rectangles and as pixels: full and title-
//! only, at every scale, staying inside the rectangle layout.rs reserved.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{objectives, sample};
use rift_ui::raster;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const GROUND: [u8; 4] = [7, 9, 14, 255];

fn numbered(width: usize, height: usize) -> Image {
    Image::new(
        width,
        height,
        (0..width * height)
            .map(|i| [(i % width) as u8, (i / width) as u8, 80, 255])
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
        .panel("panel.dark", Image::filled(1, 1, [8, 8, 12, 220]), [0; 4])
        .unwrap();
    builder
        .panel("panel.slot", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder.font("font.body", boxes(), 8, 12);
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.bold", boxes(), 8, 12);
    for role in [
        "text",
        "text.quiet",
        "text.title",
        "text.good",
        "text.team.blue",
        "text.team.red",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_full_panel_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(
            10,
            10,
            272 * s as i32,
            state.objectives.wanted_height() * s as i32,
        );
        let mut list = DrawList::new((1200, 1700), s, &theme.atlas);
        objectives::draw(&theme, &mut list, area, &state.objectives);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        for vertex in vertices {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}

#[test]
fn the_title_only_panel_draws_only_in_its_24_pixel_rectangle() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(10, 10, 272 * s as i32, 24 * s as i32);
        let mut list = DrawList::new((1200, 200), s, &theme.atlas);
        objectives::draw(&theme, &mut list, area, &state.objectives);
        let (vertices, _) = list.finish();
        assert!(!vertices.is_empty());
        for vertex in vertices {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}

#[test]
fn nothing_is_drawn_outside_the_full_rectangle() {
    let theme = theme();
    let state = sample::state();
    let area = Rect::new(20, 20, 272, state.objectives.wanted_height());
    let mut list = DrawList::new((340, 460), 1, &theme.atlas);
    objectives::draw(&theme, &mut list, area, &state.objectives);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 340 * 460];
    raster::render(&vertices, &indices, &theme.atlas, 340, 460, &mut picture);
    for y in 0..460 {
        for x in 0..340 {
            let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
            if !inside {
                assert_eq!(
                    picture[y as usize * 340 + x as usize],
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
    builder.font("font.body", boxes(), 8, 12);
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.bold", boxes(), 8, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let area = Rect::new(0, 0, 272, state.objectives.wanted_height());
    let mut list = DrawList::new(
        (272, state.objectives.wanted_height() as u32),
        1,
        &theme.atlas,
    );
    objectives::draw(&theme, &mut list, area, &state.objectives);
    assert!(!list.is_empty());
}
