//! The objectives panel (docs/INTERFACE_STAGE3_PLAN.md, section 5.4): title,
//! lane table, rule, nexuses and jungle camps, with a 24-pixel line pitch.
//!
//! Stage 3 draws the game's own list with sample states. The height follows
//! the counts in `ObjectivesState`, never a fixed 302 or 398, and the panel
//! draws only inside the rectangle it is given.

use crate::draw::{DrawList, Rect};
use crate::hud::state::{LaneState, ObjectivesState};
use crate::theme::Theme;

const LAYER: i32 = 20;
const LINE: i32 = 24;
const HEAD: i32 = 158;

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, state: &ObjectivesState) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.dark") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.02, 0.02, 0.03, 0.88]),
    }

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let title = theme.colour("text.title");
    let good = theme.colour("text.good");
    let blue = theme.colour("text.team.blue");
    let red = theme.colour("text.team.red");
    let body = theme.font("font.body");
    let small = theme.font("font.small").or(body);
    let bold = theme.font("font.bold").or(body);
    let at = |x: i32, y: i32| (area.x + x * s, area.y + y * s);
    let full = area.h / s >= state.wanted_height();

    // Title and the minimise placeholder. In a title-only rectangle nothing
    // below it is drawn.
    if let Some(font) = bold {
        let (tx, ty) = at(12, if full { 22 } else { 17 });
        list.text(font, "Objectives", tx, ty, title);
    }
    let button = Rect::new(area.right() - 22 * s, area.y + 5 * s, 14 * s, 14 * s);
    match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, button),
        None => list.rect(button, [0.10, 0.09, 0.08, 0.95]),
    }
    list.rect(
        Rect::new(button.x + 3 * s, button.y + 6 * s, 8 * s, 2 * s),
        text,
    );
    if !full {
        list.pop_clip();
        list.set_layer(previous);
        return;
    }

    if let Some(font) = small {
        let blue_x = area.x + 102 * s;
        let red_x = area.x + 176 * s;
        list.text(font, "BLUE", blue_x, area.y + 48 * s, blue);
        list.text(font, "RED", red_x, area.y + 48 * s, red);
    }

    // The lane table: a label, two tower pips and an inhibitor pip per team.
    let pip = |list: &mut DrawList, x: i32, y: i32, size: i32, on: bool, colour: crate::Colour| {
        let (px, py) = at(x, y);
        let rect = Rect::new(px, py, size * s, size * s);
        list.rect(rect, [0.0, 0.0, 0.0, 0.82]);
        if on {
            list.rect(rect, colour);
        } else {
            list.outline(rect, s.max(1), quiet);
        }
    };
    for (row, lane) in state.lanes.iter().enumerate() {
        let lane: &LaneState = lane;
        let y = 56 + row as i32 * LINE;
        if let Some(font) = body {
            list.text(
                font,
                &lane.name,
                area.x + 12 * s,
                area.y + (y + 17) * s,
                text,
            );
        }
        for (n, alive) in lane.blue_towers.iter().enumerate() {
            pip(list, 104 + n as i32 * 12, y + 7, 6, *alive, blue);
        }
        pip(list, 128, y + 6, 8, lane.blue_inhibitor, blue);
        for (n, alive) in lane.red_towers.iter().enumerate() {
            pip(list, 176 + n as i32 * 12, y + 7, 6, *alive, red);
        }
        pip(list, 200, y + 6, 8, lane.red_inhibitor, red);
    }

    // A thin rule, then the objective lines.
    list.rect(
        Rect::new(
            area.x + 12 * s,
            area.y + 132 * s,
            (area.w - 24 * s).max(0),
            1,
        ),
        [0.55, 0.50, 0.38, 0.55],
    );
    let mut y = HEAD;
    for nexus in &state.nexuses {
        let name = &nexus.name;
        let state_text = &nexus.state;
        let colour = if nexus.blue { blue } else { red };
        objective_line(
            list,
            &at,
            body,
            small,
            y,
            name,
            state_text,
            colour,
            state_colour(state_text, good, quiet, title),
            &[],
            area.right(),
        );
        y += LINE;
    }
    for camp in &state.camps {
        let second: Vec<&str> = camp.detail.iter().map(String::as_str).collect();
        objective_line(
            list,
            &at,
            body,
            small,
            y,
            &camp.name,
            &format!("{}  {}", camp.grant, camp.state),
            camp_colour(&camp.grant, good, title, quiet),
            state_colour(&camp.state, good, quiet, title),
            &second,
            area.right(),
        );
        y += LINE + if camp.detail.is_some() { LINE } else { 0 };
    }

    list.pop_clip();
    list.set_layer(previous);
}

/// Draw one line with a small team/camp mark, its name, its right-aligned
/// state, and any quieter second lines.
#[allow(clippy::too_many_arguments)]
fn objective_line(
    list: &mut DrawList,
    at: &impl Fn(i32, i32) -> (i32, i32),
    body: Option<&crate::font::Font>,
    small: Option<&crate::font::Font>,
    y: i32,
    name: &str,
    state: &str,
    mark: crate::Colour,
    state_colour: crate::Colour,
    second: &[&str],
    right: i32,
) {
    let s = list.scale();
    let (mx, my) = at(12, y + 8);
    list.rect(Rect::new(mx, my, 6 * s, 6 * s), mark);
    if let Some(font) = body {
        list.text(font, name, mx + 10 * s, my + 6 * s, mark);
    }
    if let Some(font) = small {
        let width = font.width(state) * s;
        let (_, baseline) = at(0, y + 17);
        list.text(font, state, right - 12 * s - width, baseline, state_colour);
        for (n, line) in second.iter().enumerate() {
            let (x, baseline) = at(22, y + 17 + (n as i32 + 1) * 24);
            list.text(font, line, x, baseline, [0.72, 0.72, 0.66, 1.0]);
        }
    }
}

fn state_colour(
    state: &str,
    good: crate::Colour,
    quiet: crate::Colour,
    title: crate::Colour,
) -> crate::Colour {
    if state == "Alive" || state == "Open" {
        good
    } else if state.chars().all(|c| c.is_ascii_digit()) {
        title
    } else {
        quiet
    }
}

fn camp_colour(
    grant: &str,
    good: crate::Colour,
    title: crate::Colour,
    quiet: crate::Colour,
) -> crate::Colour {
    match grant {
        "Might" => title,
        "Swift" => good,
        _ => quiet,
    }
}
