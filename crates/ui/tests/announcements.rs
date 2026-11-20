//! Announcements checked as rectangles and pixels: the outlined title line
//! is centred on the window, sits on the 23% baseline, stays inside the
//! window at scales 1 to 4, and a theme without a font draws nothing.

use rift_ui::atlas::Image;
use rift_ui::draw::DrawList;
use rift_ui::hud::announcements;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const TEXT: &str = "Giant Mole has appeared!";

fn boxes() -> Vec<GlyphSource> {
    (32u8..127)
        .map(|code| {
            let c = code as char;
            GlyphSource {
                character: c,
                image: (c != ' ').then(|| Image::filled(3, 5, [0, 0, 0, 255])),
                left: 0,
                top: 0,
                advance: 3,
            }
        })
        .collect()
}

fn theme() -> Theme {
    let mut builder = ThemeBuilder::new();
    builder.font("font.title", boxes(), 5, 7);
    builder.colour("announcement", [1.0, 1.0, 0.0, 1.0]);
    builder.colour("text.outline", [0.0, 0.0, 0.0, 1.0]);
    builder.build()
}

#[test]
fn the_announcement_is_centred_on_the_23_percent_baseline_at_every_scale() {
    let theme = theme();
    let font = theme.font("font.title").expect("the font was added");
    let width = font.width(TEXT) as f32;
    for s in 1..=4u32 {
        let (w, h) = (1920u32, 1080u32);
        let mut list = DrawList::new((w, h), s, &theme.atlas);
        announcements::draw(&theme, &mut list, TEXT);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for vertex in &vertices {
            assert!(vertex.pos[0] >= 0.0 && vertex.pos[0] <= w as f32);
            assert!(vertex.pos[1] >= 0.0 && vertex.pos[1] <= h as f32);
            min_x = min_x.min(vertex.pos[0]);
            max_x = max_x.max(vertex.pos[0]);
            max_y = max_y.max(vertex.pos[1]);
        }
        let centre = (min_x + max_x) / 2.0;
        assert!(
            (centre - w as f32 / 2.0).abs() <= s as f32,
            "scale {s}: centre {centre}"
        );
        let baseline = (h * 23 / 100) as f32;
        assert!(
            max_y <= baseline + s as f32 + 0.5,
            "scale {s}: bottom {max_y}, baseline {baseline}"
        );
        assert!(min_x <= (w as f32 - width * s as f32) / 2.0 + 1.0);
    }
}

#[test]
fn a_theme_without_a_font_draws_nothing() {
    let theme = ThemeBuilder::new().build();
    let mut list = DrawList::new((400, 300), 1, &theme.atlas);
    announcements::draw(&theme, &mut list, TEXT);
    assert!(list.is_empty());
}
