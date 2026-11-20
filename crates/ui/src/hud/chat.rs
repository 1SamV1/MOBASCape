//! The chat and event log (docs/INTERFACE_STAGE3_PLAN.md, section 5.8): a
//! parchment panel of wrapped lines, a scrollbar, and the All, Game and
//! Public tabs. A narrow chat may be cut to its last line by the placing
//! pass, and then only that line is drawn.
//!
//! Stage 3 draws six sample lines; stage 4 fills the same slice from the
//! match and accepts what the player types.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::{ChatKind, ChatLine};
use crate::theme::Theme;
use crate::wrap;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (440, 140);
const LAYER: i32 = 20;
/// The parchment's height; the tabs take the rest.
const PARCHMENT: i32 = 115;
const MAX_LINES: usize = 6;
const LEFT: i32 = 8;
/// Room kept at the right for the scrollbar.
const RIGHT: i32 = 26;

struct Row {
    text: String,
    /// Drawn before `text` on the first line of a player's message.
    prefix: Option<String>,
    prefix_colour: Colour,
    body_colour: Colour,
}

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, lines: &[ChatLine]) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    let compact = area.h / s < SIZE.1;

    match theme.panel("panel.parchment") {
        Some(panel) => {
            let parchment = if compact {
                area
            } else {
                Rect::new(area.x, area.y, area.w, PARCHMENT * s)
            };
            list.panel(panel, parchment);
        }
        None => list.rect(
            Rect::new(
                area.x,
                area.y,
                area.w,
                if compact { area.h } else { PARCHMENT * s },
            ),
            [0.66, 0.58, 0.42, 0.98],
        ),
    }

    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme
        .font("font.bold")
        .or_else(|| theme.font("font.body"))
        .or(small);

    let rows = body
        .map(|font| rows(theme, font, lines, area.w / s - LEFT - RIGHT))
        .unwrap_or_default();

    if compact {
        if let (Some(font), Some(row)) = (body, rows.last()) {
            draw_row(
                list,
                font,
                area.x + LEFT * s,
                area.y + (area.h + font.ascent * s) / 2,
                row,
            );
        }
        list.pop_clip();
        list.set_layer(previous);
        return;
    }

    let sb_x = area.right() - 18 * s;
    let sb_y = area.y + 8 * s;
    if theme.has_sprite("scrollbar") {
        list.sprite(theme.sprite("scrollbar"), sb_x, sb_y);
    } else {
        list.rect(
            Rect::new(sb_x, sb_y, 8 * s, 16 * s),
            [0.35, 0.28, 0.18, 0.85],
        );
    }

    if let Some(font) = body {
        let pitch = font.line_height.max(font.ascent + 2).max(10);
        let first = rows.len().saturating_sub(MAX_LINES);
        for (n, row) in rows[first..].iter().enumerate() {
            let baseline = area.y + (8 + font.ascent + n as i32 * pitch) * s;
            draw_row(list, font, area.x + LEFT * s, baseline, row);
        }
    }

    let title = theme.colour("text.title");
    let quiet = theme.colour("text.quiet");
    for (n, label) in ["All", "Game", "Public"].iter().enumerate() {
        let tab = Rect::new(
            area.x + (4 + n as i32 * 62) * s,
            area.y + 116 * s,
            60 * s,
            22 * s,
        );
        match theme.panel("button.stone") {
            Some(panel) => list.panel(panel, tab),
            None => list.rect(tab, [0.20, 0.17, 0.12, 0.95]),
        }
        if let Some(font) = bold {
            let width = font.width(label) * s;
            list.text(
                font,
                label,
                tab.x + (tab.w - width) / 2,
                tab.y + (tab.h + font.ascent * s) / 2,
                if n == 0 { title } else { quiet },
            );
        }
    }

    list.pop_clip();
    list.set_layer(previous);
}

fn rows(theme: &Theme, font: &crate::font::Font, lines: &[ChatLine], width: i32) -> Vec<Row> {
    let dark = theme.colour("text.outline");
    let blue = theme.colour("text.team.blue");
    let red = theme.colour("text.team.red");
    let event = theme.colour("text.team.blue");
    let mut out = Vec::new();
    for line in lines {
        match &line.kind {
            ChatKind::System => {
                for part in wrap::wrap(font, &line.text, width) {
                    out.push(Row {
                        text: part,
                        prefix: None,
                        prefix_colour: dark,
                        body_colour: dark,
                    });
                }
            }
            ChatKind::Event => {
                for part in wrap::wrap(font, &line.text, width) {
                    out.push(Row {
                        text: part,
                        prefix: None,
                        prefix_colour: event,
                        body_colour: event,
                    });
                }
            }
            ChatKind::Player {
                speaker,
                hero,
                blue: is_blue,
            } => {
                let prefix = format!("{speaker} [{hero}]: ");
                let full = format!("{prefix}{}", line.text);
                let team = if *is_blue { blue } else { red };
                for (n, part) in wrap::wrap(font, &full, width).into_iter().enumerate() {
                    let prefix = (n == 0 && part.starts_with(&prefix)).then(|| prefix.clone());
                    out.push(Row {
                        text: part,
                        prefix,
                        prefix_colour: team,
                        body_colour: dark,
                    });
                }
            }
        }
    }
    out
}

fn draw_row(list: &mut DrawList, font: &crate::font::Font, x: i32, baseline: i32, row: &Row) {
    if let Some(prefix) = &row.prefix
        && row.text.starts_with(prefix)
    {
        let width = list.text(font, prefix, x, baseline, row.prefix_colour);
        list.text(
            font,
            &row.text[prefix.len()..],
            x + width,
            baseline,
            row.body_colour,
        );
        return;
    }
    list.text(font, &row.text, x, baseline, row.body_colour);
}
