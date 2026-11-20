//! The kill feed checked as rectangles and pixels: two entries, their
//! portraits, names, swords and gold stay inside 286 by 58 at scales 1 to 4,
//! nothing is drawn outside, and a theme with only fonts still draws the
//! fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{kill_feed, sample};
use rift_ui::raster;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const GROUND: [u8; 4] = [8, 11, 17, 255];
const TWO_ENTRIES: i32 = kill_feed::PITCH * 2 - 2;

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
    builder.sprite("icon.kills", numbered(10, 10));
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    for role in [
        "text",
        "text.quiet",
        "text.team.blue",
        "text.team.red",
        "announcement",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_feed_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(10, 10, kill_feed::WIDTH * s as i32, TWO_ENTRIES * s as i32);
        let mut list = DrawList::new((1400, 700), s, &theme.atlas);
        kill_feed::draw(&theme, &mut list, area, &state.feed);
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
    let area = Rect::new(20, 20, kill_feed::WIDTH, TWO_ENTRIES);
    let mut list = DrawList::new((340, 120), 1, &theme.atlas);
    kill_feed::draw(&theme, &mut list, area, &state.feed);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 340 * 120];
    raster::render(&vertices, &indices, &theme.atlas, 340, 120, &mut picture);
    for y in 0..120 {
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
fn a_theme_with_only_fonts_still_draws_the_feed() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.body", boxes(), 10, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let area = Rect::new(0, 0, kill_feed::WIDTH, TWO_ENTRIES);
    let mut list = DrawList::new(
        (kill_feed::WIDTH as u32, TWO_ENTRIES as u32),
        1,
        &theme.atlas,
    );
    kill_feed::draw(&theme, &mut list, area, &state.feed);
    assert!(!list.is_empty());
}
