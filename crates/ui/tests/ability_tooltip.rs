//! The ability tooltip as rectangles and draw output: it sits above the
//! cursor, stays inside the window at scales 1 to 4, every upgrade adds a
//! row, and its contents stay inside its own rectangle.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::ability_tooltip;
use rift_ui::hud::state::{AbilityNumbers, AbilityTooltipState, AbilityUpgrade};
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

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
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    for role in [
        "text",
        "text.good",
        "text.title",
        "text.quiet",
        "announcement",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

fn numbers(damage: &str, range: &str, cooldown: &str) -> AbilityNumbers {
    AbilityNumbers {
        damage_label: "Damage".to_string(),
        damage: damage.to_string(),
        range: range.to_string(),
        cooldown: cooldown.to_string(),
    }
}

fn frog_q(upgrades: usize) -> AbilityTooltipState {
    let rows = [
        numbers("85", "900", "6s"),
        numbers("105", "950", "5.5s"),
        numbers("125", "950", "5s"),
    ];
    AbilityTooltipState {
        key: "Q".to_string(),
        name: "Tongue Lash".to_string(),
        summary: "Flicks its tongue down a line, dragging the first enemy hit toward the frog."
            .to_string(),
        rank: 2,
        max_rank: 4,
        current: numbers("65", "850", "6.5s"),
        upgrades: rows
            .into_iter()
            .take(upgrades)
            .enumerate()
            .map(|(n, numbers)| AbilityUpgrade {
                rank: 3 + n as u8,
                numbers,
                needs_level: None,
            })
            .collect(),
        note: None,
        hint: Some("Click to spend a skill point (1 left)".to_string()),
        at: (700, 1000),
    }
}

#[test]
fn it_sits_above_the_cursor_inside_the_window_at_every_scale() {
    let theme = theme();
    let window = Rect::new(0, 0, 1920, 1080);
    for s in 1..=4 {
        let tip = frog_q(2);
        let area = ability_tooltip::rect(&theme, &tip, window, s);
        assert!(area.bottom() <= tip.at.1, "scale {s}: {area:?}");
        assert!(area.x >= 0 && area.right() <= window.right(), "scale {s}");
        assert!(area.y >= 0 && area.bottom() <= window.bottom(), "scale {s}");
    }
}

#[test]
fn every_upgrade_left_adds_a_row() {
    let theme = theme();
    let (w2, h2) = ability_tooltip::size(&theme, &frog_q(2));
    let (w1, h1) = ability_tooltip::size(&theme, &frog_q(1));
    assert_eq!(w1, w2, "rows make it taller, not wider");
    assert_eq!(h2 - h1, 10, "one small line per upgrade");
}

#[test]
fn everything_drawn_stays_inside_the_tooltip() {
    let theme = theme();
    let window = Rect::new(0, 0, 1280, 720);
    for s in 1..=3u32 {
        let mut tip = frog_q(3);
        tip.at = (60, 300);
        let area = ability_tooltip::rect(&theme, &tip, window, s as i32);
        let mut list = DrawList::new((1280, 720), s, &theme.atlas);
        ability_tooltip::draw(&theme, &mut list, &tip);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        for vertex in vertices {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}
