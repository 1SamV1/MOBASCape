//! The death overlay checked as rectangles and pixels: the damage panel is
//! centred at 330 by 80 for three sources, the dim covers the window, the
//! text and panel stay inside it at scales 1 to 4, and a theme with only
//! fonts still draws the fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{death, sample};
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

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
    builder.font("font.title", boxes(), 12, 14);
    for role in [
        "text",
        "text.quiet",
        "death",
        "announcement",
        "text.outline",
        "bar.enemy",
        "bar.back",
        "text.team.blue",
        "text.team.red",
        "dim",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_damage_panel_is_centred_and_counts_its_sources() {
    let death = sample::state().death.expect("the sample is dead");
    assert_eq!(death.total(), 730);
    assert_eq!(
        death::panel_rect(Rect::new(0, 0, 1920, 1080), 1, death.damage.len()),
        Rect::new(795, 442, 330, 80)
    );
}

#[test]
fn the_dim_covers_the_window_and_everything_else_stays_inside_it() {
    let theme = theme();
    let state = sample::state();
    let death = state.death.as_ref().expect("the sample is dead");
    for s in 1..=4u32 {
        let (w, h) = (1920u32, 1080u32);
        let mut list = DrawList::new((w, h), s, &theme.atlas);
        death::draw(&theme, &mut list, death);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        let dim = &vertices[..4];
        assert!(dim.iter().any(|v| v.pos == [0.0, 0.0]));
        assert!(dim.iter().any(|v| v.pos == [w as f32, h as f32]));
        for vertex in vertices {
            assert!(vertex.pos[0] >= 0.0 && vertex.pos[0] <= w as f32);
            assert!(vertex.pos[1] >= 0.0 && vertex.pos[1] <= h as f32);
        }
    }
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_overlay() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.body", boxes(), 10, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let death = state.death.as_ref().expect("the sample is dead");
    let mut list = DrawList::new((1920, 1080), 1, &theme.atlas);
    death::draw(&theme, &mut list, death);
    assert!(!list.is_empty());
}
