//! The kill feed (docs/INTERFACE_STAGE3_PLAN.md, section 5.9): one dark
//! entry per kill, with a bar in the killer's team colour, both portraits,
//! both names in their team colours, crossed swords and the gold won.
//!
//! Stage 3 draws two sample entries; stage 4 fills the same slice from the
//! match's kill events and fades old ones out.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::KillFeedEntry;
use crate::theme::Theme;

/// The width of one entry, and the room it takes with the gap below it.
pub const WIDTH: i32 = 286;
pub const ENTRY: i32 = 28;
pub const PITCH: i32 = 30;
const LAYER: i32 = 30;

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, entries: &[KillFeedEntry]) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    for (n, entry) in entries.iter().enumerate() {
        let top = n as i32 * PITCH;
        if (top + ENTRY) * s > area.h {
            break;
        }
        let entry_area = Rect::new(area.x, area.y + top * s, area.w, ENTRY * s);
        draw_entry(theme, list, entry_area, entry);
    }
    list.pop_clip();
    list.set_layer(previous);
}

fn draw_entry(theme: &Theme, list: &mut DrawList, area: Rect, entry: &KillFeedEntry) {
    let s = list.scale();
    match theme.panel("panel.dark") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.02, 0.02, 0.03, 0.90]),
    }

    let quiet = theme.colour("text.quiet");
    let blue = theme.colour("text.team.blue");
    let red = theme.colour("text.team.red");
    let gold = theme.colour("announcement");
    let killer_team = if entry.killer_blue { blue } else { red };
    let victim_team = if entry.victim_blue { blue } else { red };
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body).or(small);

    // The killer's team, down the left edge.
    list.rect(Rect::new(area.x, area.y, 3 * s, area.h), killer_team);

    let portrait = |list: &mut DrawList, x: i32, name: &str, team: Colour| {
        let frame = Rect::new(area.x + x * s, area.y + 4 * s, 20 * s, 20 * s);
        match theme.panel("panel.slot") {
            Some(panel) => list.panel(panel, frame),
            None => list.rect(frame, [0.08, 0.07, 0.06, 0.95]),
        }
        list.rect(
            Rect::new(
                frame.x + 3 * s,
                frame.y + 3 * s,
                frame.w - 6 * s,
                frame.h - 6 * s,
            ),
            [0.18, 0.19, 0.22, 0.95],
        );
        if let Some(font) = bold {
            let initial = name.chars().next().unwrap_or('?').to_string();
            let width = font.width(&initial) * s;
            list.text(
                font,
                &initial,
                frame.x + (frame.w - width) / 2,
                frame.y + (frame.h + font.ascent * s) / 2,
                team,
            );
        }
        frame
    };

    portrait(list, 7, &entry.killer, killer_team);
    let killer_x = 31;
    let baseline = area.y + 18 * s;
    let mut killer_width = 0;
    if let Some(font) = body {
        killer_width = list.text(
            font,
            &entry.killer,
            area.x + killer_x * s,
            baseline,
            killer_team,
        );
    }

    let swords_x = killer_x + killer_width / s + 8;
    let swords_width = if theme.has_sprite("icon.kills") {
        let icon = theme.sprite("icon.kills");
        list.sprite(icon, area.x + swords_x * s, area.y + 4 * s);
        icon.w as i32
    } else {
        if let Some(font) = small {
            list.text(font, "x", area.x + (swords_x + 3) * s, baseline, quiet);
        }
        12
    };

    let victim_portrait = portrait(
        list,
        swords_x + swords_width + 6,
        &entry.victim,
        victim_team,
    );
    let victim_x = victim_portrait.right() + 6 * s;
    if let Some(font) = body {
        list.text(font, &entry.victim, victim_x, baseline, victim_team);
    }

    if let Some(font) = body {
        let won = format!("+{}", entry.gold);
        let width = font.width(&won) * s;
        list.text(font, &won, area.right() - 8 * s - width, baseline, gold);
    }
}
