//! The target panel (docs/INTERFACE_STAGE3_PLAN.md, section 5.6): a portrait
//! square, the target's name and kind, its health with figures, a description
//! line, and eight stat figures in two columns of four.
//!
//! Stage 3 draws a sample structure; stage 4 fills the same slice from the
//! selected unit.

use crate::draw::{DrawList, Rect};
use crate::hud::state::TargetState;
use crate::theme::Theme;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (332, 158);
const LAYER: i32 = 20;
const STAT_ROW: i32 = 15;

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, target: &TargetState) {
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
    let kind = theme.colour("announcement");
    let health = theme.colour("bar.health");
    let back = theme.colour("bar.back");
    let team = if target.blue {
        theme.colour("text.team.blue")
    } else {
        theme.colour("text.team.red")
    };
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body);

    // The portrait is a labelled square until portraits come from models.
    let portrait = Rect::new(area.x + 8 * s, area.y + 8 * s, 58 * s, 58 * s);
    match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, portrait),
        None => list.rect(portrait, [0.08, 0.07, 0.06, 0.95]),
    }
    list.rect(
        Rect::new(
            portrait.x + 5 * s,
            portrait.y + 5 * s,
            portrait.w - 10 * s,
            portrait.h - 10 * s,
        ),
        [0.20, 0.21, 0.24, 0.95],
    );
    if let Some(font) = bold {
        let initial = target.name.chars().next().unwrap_or('?').to_string();
        let width = font.width(&initial) * s;
        list.text(
            font,
            &initial,
            portrait.x + (portrait.w - width) / 2,
            portrait.y + portrait.h / 2 + font.ascent * s / 2,
            [1.0; 4],
        );
    }

    if let Some(font) = bold {
        list.text(font, &target.name, area.x + 76 * s, area.y + 26 * s, team);
    }
    if let Some(font) = small {
        let width = font.width(&target.kind) * s;
        list.text(
            font,
            &target.kind,
            area.right() - 8 * s - width,
            area.y + 26 * s,
            kind,
        );
    }

    // Health, green on dark, with "current / maximum" centred.
    let bar = Rect::new(area.x + 76 * s, area.y + 34 * s, 240 * s, 14 * s);
    list.rect(bar, back);
    let fraction = if target.max_health == 0 {
        0.0
    } else {
        (target.health as f32 / target.max_health as f32).clamp(0.0, 1.0)
    };
    list.rect(
        Rect::new(bar.x, bar.y, (bar.w as f32 * fraction) as i32, bar.h),
        health,
    );
    if let Some(font) = small {
        let figures = format!("{} / {}", target.health, target.max_health);
        let width = font.width(&figures) * s;
        list.text(
            font,
            &figures,
            bar.x + (bar.w - width) / 2,
            bar.y + (bar.h + font.ascent * s) / 2,
            text,
        );
    }

    if let Some(font) = body {
        list.text(
            font,
            &target.description,
            area.x + 12 * s,
            area.y + 82 * s,
            title,
        );
    }

    // Eight figures, two columns of four: the first four on the left, the
    // rest on the right. Labels are quiet, values white at the column edge.
    if let Some(font) = small {
        for (n, stat) in target.stats.iter().take(8).enumerate() {
            let column = n / 4;
            let row = n % 4;
            let left = 12 + column as i32 * 160;
            let right = left + 148;
            let baseline = area.y + (104 + row as i32 * STAT_ROW) * s;
            list.text(font, &stat.label, area.x + left * s, baseline, quiet);
            let width = font.width(&stat.value) * s;
            list.text(
                font,
                &stat.value,
                area.x + right * s - width,
                baseline,
                text,
            );
        }
    }

    list.pop_clip();
    list.set_layer(previous);
}
