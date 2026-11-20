//! The score strip (docs/INTERFACE_STAGE3_PLAN.md, section 5.1): blue and
//! red team marks and kills, the match clock, the player's kills/deaths/
//! assists and creep score, in one piece of stone.
//!
//! It is given its rectangle by `layout.rs` and draws nothing outside it. In
//! stage 3 the values come from `sample.rs`; stage 4 fills the same slice
//! from the match.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::ScoreState;
use crate::theme::Theme;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (534, 38);

const LAYER: i32 = 20;

/// Draw the score strip in `area`, which is the rectangle `layout.rs`
/// reserved for it.
pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, score: &ScoreState) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);

    // Stone, or a dark panel if this theme has no stone.
    match theme.panel("panel.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.04, 0.04, 0.06, 0.88]),
    }

    let font = theme.font("font.bold").or_else(|| theme.font("font.body"));
    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let blue = theme.colour("text.team.blue");
    let red = theme.colour("text.team.red");
    let clock = theme.colour("announcement");
    let at = |x: i32, y: i32| (area.x + x * s, area.y + y * s);
    let baseline = area.y + 25 * s;

    // A team mark, or a coloured square when the theme has no team orb.
    let mark = |list: &mut DrawList, role: &str, x: i32, fallback: Colour| {
        let (ax, ay) = at(x, 6);
        if theme.has_sprite(role) {
            list.sprite(theme.sprite(role), ax, ay);
        } else {
            list.rect(Rect::new(ax, ay, 26 * s, 26 * s), fallback);
        }
    };
    mark(list, "icon.team.blue", 8, blue);
    mark(list, "icon.team.red", 100, red);

    if theme.has_sprite("icon.kills") {
        let (kx, ky) = at(56, 7);
        list.sprite(theme.sprite("icon.kills"), kx, ky);
    }

    if let Some(font) = font {
        let write = |list: &mut DrawList, words: &str, x: i32, colour: Colour| {
            list.text(font, words, x, baseline, colour);
        };
        let width = |words: &str| font.width(words) * s;

        write(list, &score.blue_kills.to_string(), area.x + 38 * s, blue);
        write(list, &score.red_kills.to_string(), area.x + 84 * s, red);

        let clock_text = &score.clock;
        write(
            list,
            clock_text,
            area.x + (area.w - width(clock_text)) / 2,
            clock,
        );

        let creep = format!("{} CS", score.creep_score);
        let creep_x = area.right() - 8 * s - width(&creep);
        write(list, &creep, creep_x, quiet);

        let own = format!("{}/{}/{}", score.kills, score.deaths, score.assists);
        write(list, &own, creep_x - 12 * s - width(&own), text);
    }

    list.pop_clip();
    list.set_layer(previous);
}
