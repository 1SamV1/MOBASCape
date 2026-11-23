//! The ability tooltip: a dark panel above a hovered Q, W, E or R slot.
//!
//! ```text
//! Tongue Lash: Flicks its tongue down a line, dragging
//! the first enemy hit toward the frog.
//! Q  Rank 2 / 4                              ■ ■ □ □
//!              Damage    Range    Cooldown
//! Rank 2       65        850      6.5s        (green: now)
//! Rank 3       85        900      6s          (gold where it changes)
//! Rank 4       105       950      5.5s
//! Click to spend a skill point (1 left)
//! ```
//!
//! Like the item tooltip it is not part of the placing pass or the budget.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::font::Font;
use crate::hud::state::{AbilityNumbers, AbilityTooltipState};
use crate::theme::Theme;
use crate::wrap;

const LAYER: i32 = 50;
const WIDTH: i32 = 300;
const PADDING: i32 = 8;
const INNER: i32 = WIDTH - 2 * PADDING;
const GAP: i32 = 5;
/// Left edge of the rank, damage, range and cooldown columns.
const COLUMNS: [i32; 4] = [0, 92, 166, 226];

struct Metrics {
    body_lh: i32,
    small_lh: i32,
    lines: Vec<String>,
}

fn line_height(font: Option<&Font>, fallback: i32) -> i32 {
    font.map(|font| font.line_height.max(font.ascent + 1))
        .unwrap_or(fallback)
}

fn metrics(theme: &Theme, tip: &AbilityTooltipState) -> Metrics {
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let heading = format!("{}: {}", tip.name, tip.summary);
    Metrics {
        body_lh: line_height(body, 14),
        small_lh: line_height(small, 12),
        lines: body
            .map(|font| wrap::wrap(font, &heading, INNER))
            .unwrap_or_else(|| vec![heading]),
    }
}

/// The tooltip's own size at the interface's scale.
pub fn size(theme: &Theme, tip: &AbilityTooltipState) -> (i32, i32) {
    let m = metrics(theme, tip);
    let mut h = PADDING + m.lines.len() as i32 * m.body_lh;
    if tip.note.is_some() {
        h += m.small_lh;
    }
    // Rank line, column headings, the current row.
    h += GAP + m.body_lh + GAP + m.small_lh + m.small_lh;
    h += tip.upgrades.len() as i32 * m.small_lh;
    if tip.upgrades.is_empty() {
        h += m.small_lh;
    }
    if tip.hint.is_some() {
        h += GAP + m.small_lh;
    }
    (WIDTH, h + PADDING)
}

/// Where the tooltip goes: centred above `at`, kept on screen.
pub fn rect(theme: &Theme, tip: &AbilityTooltipState, screen: Rect, scale: i32) -> Rect {
    let (w, h) = size(theme, tip);
    let s = scale.max(1);
    let x = (tip.at.0 - w * s / 2)
        .min(screen.right() - w * s - 4 * s)
        .max(screen.x + 4 * s);
    let y = (tip.at.1 - h * s - 6 * s)
        .max(screen.y + 4 * s)
        .min(screen.bottom() - h * s - 4 * s);
    Rect::new(x, y, w * s, h * s)
}

pub fn draw(theme: &Theme, list: &mut DrawList, tip: &AbilityTooltipState) {
    let s = list.scale();
    let area = rect(theme, tip, list.screen(), s);
    let m = metrics(theme, tip);
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.dark") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.02, 0.02, 0.03, 0.92]),
    }

    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body);
    let yellow = theme.colour("announcement");
    let green = theme.colour("text.good");
    let orange = theme.colour("text.title");
    let white = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let gold: Colour = [1.0, 0.82, 0.30, 1.0];
    let x = area.x + PADDING * s;
    let mut cursor = PADDING;
    let baseline = |cursor: i32, font: &Font| area.y + (cursor + font.ascent) * s;

    // "Name: summary", the name in yellow, the summary wrapping in white.
    if let Some(font) = body {
        let prefix = format!("{}:", tip.name);
        for (n, line) in m.lines.iter().enumerate() {
            let y = baseline(cursor, font);
            match line.strip_prefix(&prefix).filter(|_| n == 0) {
                Some(rest) => {
                    let width = list.text(font, &prefix, x, y, yellow);
                    list.text(font, rest, x + width, y, white);
                }
                None => {
                    list.text(font, line, x, y, white);
                }
            }
            cursor += m.body_lh;
        }
    } else {
        cursor += m.lines.len() as i32 * m.body_lh;
    }
    if let Some(note) = &tip.note {
        if let Some(font) = small {
            list.text(font, note, x, baseline(cursor, font), green);
        }
        cursor += m.small_lh;
    }

    // Hotkey and rank, with rank pips on the right.
    cursor += GAP;
    if let Some(font) = bold {
        let words = format!("{}   Rank {} / {}", tip.key, tip.rank, tip.max_rank);
        list.text(font, &words, x, baseline(cursor, font), orange);
        let pip_w = 10;
        let gap = 3;
        let count = tip.max_rank.max(1) as i32;
        let left = area.right() - PADDING * s - (count * pip_w + (count - 1) * gap) * s;
        let top = area.y + (cursor + (m.body_lh - 6) / 2) * s;
        for n in 0..count {
            let pip = Rect::new(left + n * (pip_w + gap) * s, top, pip_w * s, 6 * s);
            list.rect(pip, [0.0, 0.0, 0.0, 0.7]);
            let fill = if n < tip.rank as i32 {
                gold
            } else {
                [0.22, 0.2, 0.17, 1.0]
            };
            list.rect(
                Rect::new(pip.x + s, pip.y + s, pip.w - 2 * s, pip.h - 2 * s),
                fill,
            );
        }
    }
    cursor += m.body_lh + GAP;

    // The table: headings, the current rank, then every rank still to come.
    if let Some(font) = small {
        let headings = ["", tip.current.damage_label.as_str(), "Range", "Cooldown"];
        row(
            list,
            font,
            x,
            baseline(cursor, font),
            s,
            &headings,
            [quiet; 4],
        );
        cursor += m.small_lh;
        let now = format!("Rank {}", tip.rank);
        let current = cells(&now, &tip.current);
        row(
            list,
            font,
            x,
            baseline(cursor, font),
            s,
            &current,
            [green; 4],
        );
        cursor += m.small_lh;
        let mut previous = tip.current.clone();
        for upgrade in &tip.upgrades {
            let label = match upgrade.needs_level {
                Some(level) => format!("Rank {} (Lv {level})", upgrade.rank),
                None => format!("Rank {}", upgrade.rank),
            };
            let base = if upgrade.needs_level.is_some() {
                quiet
            } else {
                white
            };
            let changed = |a: &str, b: &str| if a != b { gold } else { base };
            let colours = [
                base,
                changed(&upgrade.numbers.damage, &previous.damage),
                changed(&upgrade.numbers.range, &previous.range),
                changed(&upgrade.numbers.cooldown, &previous.cooldown),
            ];
            row(
                list,
                font,
                x,
                baseline(cursor, font),
                s,
                &cells(&label, &upgrade.numbers),
                colours,
            );
            previous = upgrade.numbers.clone();
            cursor += m.small_lh;
        }
        if tip.upgrades.is_empty() {
            list.text(font, "Max rank", x, baseline(cursor, font), orange);
            cursor += m.small_lh;
        }
        if let Some(hint) = &tip.hint {
            cursor += GAP;
            list.text(font, hint, x, baseline(cursor, font), gold);
        }
    }

    list.pop_clip();
    list.set_layer(previous);
}

fn cells<'a>(label: &'a str, numbers: &'a AbilityNumbers) -> [&'a str; 4] {
    [
        label,
        numbers.damage.as_str(),
        numbers.range.as_str(),
        numbers.cooldown.as_str(),
    ]
}

fn row(
    list: &mut DrawList,
    font: &Font,
    x: i32,
    baseline: i32,
    s: i32,
    cells: &[&str; 4],
    colours: [Colour; 4],
) {
    for ((cell, column), colour) in cells.iter().zip(COLUMNS).zip(colours) {
        if !cell.is_empty() {
            list.text(font, cell, x + column * s, baseline, colour);
        }
    }
}
