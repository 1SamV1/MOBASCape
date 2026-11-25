//! The item tooltip (docs/INTERFACE_STAGE3_PLAN.md, section 5.10): a dark
//! panel beside the cursor with the title and price in yellow, green stat
//! lines, and a passive whose name is orange and whose text wraps in white.
//!
//! It is not part of the placing pass or the budget. Stage 3 shows a sample
//! item; stage 4 shows what is under the cursor.

use crate::draw::{DrawList, Rect};
use crate::hud::state::TooltipState;
use crate::theme::Theme;
use crate::wrap;

const LAYER: i32 = 50;
const WIDTH: i32 = 310;
const PADDING: i32 = 8;
const INNER: i32 = WIDTH - 2 * PADDING;
const GAP: i32 = 4;

struct Metrics {
    title_lh: i32,
    small_lh: i32,
    body_lh: i32,
    wrapped: Vec<String>,
}

/// The tooltip's own size at the interface's scale.
pub fn size(theme: &Theme, tooltip: &TooltipState) -> (i32, i32) {
    let m = metrics(theme, tooltip);
    (WIDTH, height(tooltip, &m))
}

/// Where the tooltip goes, beside the cursor and kept inside the window.
pub fn rect(theme: &Theme, tooltip: &TooltipState, screen: Rect, scale: i32) -> Rect {
    let (w, h) = size(theme, tooltip);
    let s = scale.max(1);
    let x = (tooltip.at.0 + 12 * s)
        .min(screen.right() - w * s - 4 * s)
        .max(screen.x + 4 * s);
    let y = (tooltip.at.1 + 12 * s)
        .min(screen.bottom() - h * s - 4 * s)
        .max(screen.y + 4 * s);
    Rect::new(x, y, w * s, h * s)
}

pub fn draw(theme: &Theme, list: &mut DrawList, tooltip: &TooltipState) {
    let s = list.scale();
    let screen = list.screen();
    let area = rect(theme, tooltip, screen, s);
    let m = metrics(theme, tooltip);
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.dark") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.02, 0.02, 0.03, 0.92]),
    }

    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body);
    let title_font = bold.or(body);
    let yellow = theme.colour("announcement");
    let green = theme.colour("text.good");
    let orange = theme.colour("text.title");
    let white = theme.colour("text");
    let x = area.x + PADDING * s;
    let mut cursor = PADDING;

    if let Some(font) = title_font {
        let baseline = area.y + (cursor + font.ascent) * s;
        list.text(font, &tooltip.title, x, baseline, yellow);
        // Spells and the scroll cost nothing; only items show a price.
        if tooltip.price > 0 {
            let price = tooltip.price.to_string();
            let width = font.width(&price) * s;
            list.text(
                font,
                &price,
                area.right() - PADDING * s - width,
                baseline,
                yellow,
            );
        }
    }
    cursor += m.title_lh;

    if !tooltip.stats.is_empty() {
        cursor += GAP;
        if let Some(font) = small {
            for stat in &tooltip.stats {
                let baseline = area.y + (cursor + font.ascent) * s;
                list.text(font, stat, x, baseline, green);
                cursor += m.small_lh;
            }
        } else {
            cursor += tooltip.stats.len() as i32 * m.small_lh;
        }
    }

    if tooltip.passive.is_some() || !m.wrapped.is_empty() {
        cursor += 6;
        if let Some(name) = &tooltip.passive {
            if let Some(font) = body {
                let baseline = area.y + (cursor + font.ascent) * s;
                list.text(font, name, x, baseline, orange);
            }
            cursor += m.body_lh;
        }
        if let Some(font) = body {
            for line in &m.wrapped {
                let baseline = area.y + (cursor + font.ascent) * s;
                list.text(font, line, x, baseline, white);
                cursor += m.body_lh;
            }
        }
    }

    list.pop_clip();
    list.set_layer(previous);
}

fn metrics(theme: &Theme, tooltip: &TooltipState) -> Metrics {
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body);
    let title_lh = line_height(bold.or(body), 16);
    let small_lh = line_height(small, 12);
    let body_lh = line_height(body, 14);
    let wrapped = if tooltip.text.is_empty() {
        Vec::new()
    } else {
        body.map(|font| wrap::wrap(font, &tooltip.text, INNER))
            .unwrap_or_default()
    };
    Metrics {
        title_lh,
        small_lh,
        body_lh,
        wrapped,
    }
}

fn height(tooltip: &TooltipState, m: &Metrics) -> i32 {
    let stats = if tooltip.stats.is_empty() {
        0
    } else {
        GAP + tooltip.stats.len() as i32 * m.small_lh
    };
    let passive = if tooltip.passive.is_some() || !m.wrapped.is_empty() {
        6 + if tooltip.passive.is_some() {
            m.body_lh
        } else {
            0
        } + m.wrapped.len() as i32 * m.body_lh
    } else {
        0
    };
    2 * PADDING + m.title_lh + stats + passive
}

fn line_height(font: Option<&crate::font::Font>, fallback: i32) -> i32 {
    font.map(|font| font.line_height.max(font.ascent + 1))
        .unwrap_or(fallback)
}
