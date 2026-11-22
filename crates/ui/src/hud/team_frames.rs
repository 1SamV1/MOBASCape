//! Team frames (docs/INTERFACE_STAGE3_PLAN.md, section 5.5): four rows for
//! the allies but the player, each with a portrait and level, name, health,
//! resource and two battle-spell marks.
//!
//! Stage 3 draws labelled portrait squares and sample allies; stage 4 fills
//! the same slice from the match.

use crate::draw::{DrawList, Rect};
use crate::hud::state::AllyState;
use crate::theme::Theme;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (180, 188);
const ROW: i32 = 47;
const LAYER: i32 = 20;

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, allies: &[AllyState]) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.05, 0.05, 0.07, 0.95]),
    }

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let blue = theme.colour("text.team.blue");
    let health = theme.colour("bar.health");
    let resource = theme.colour("bar.resource");
    let back = theme.colour("bar.back");
    let small = theme.font("font.small");
    let bold = theme.font("font.bold").or(small);
    let at = |x: i32, y: i32| (area.x + x * s, area.y + y * s);

    let slot = |list: &mut DrawList, x: i32, y: i32, w: i32, h: i32| {
        let (sx, sy) = at(x, y);
        let rect = Rect::new(sx, sy, w * s, h * s);
        match theme.panel("panel.slot") {
            Some(panel) => list.panel(panel, rect),
            None => list.rect(rect, [0.08, 0.07, 0.06, 0.95]),
        }
        rect
    };

    for (n, ally) in allies.iter().take(4).enumerate() {
        let y = n as i32 * ROW;
        let portrait = slot(list, 6, y + 5, 36, 36);
        let colour = if ally.dead { quiet } else { blue };
        let inner = Rect::new(
            portrait.x + 4 * s,
            portrait.y + 4 * s,
            portrait.w - 8 * s,
            portrait.h - 8 * s,
        );
        list.rect(
            inner,
            if ally.dead {
                [0.18, 0.18, 0.18, 0.95]
            } else {
                [0.24, 0.38, 0.68, 0.95]
            },
        );
        // A cache-baked champion portrait fills the slot when the live theme
        // carries one. The sample theme and unpainted allies keep the
        // team-coloured initial so the row never reads as a missing image.
        let mut drew_portrait = false;
        if !ally.dead
            && let Some(role) = ally.portrait.as_deref()
            && theme.has_sprite(role)
        {
            list.sprite_fit(theme.sprite(role), inner);
            drew_portrait = true;
        }
        if !drew_portrait && let Some(font) = bold {
            let initial = ally.name.chars().next().unwrap_or('?').to_string();
            let width = font.width(&initial) * s;
            list.text(
                font,
                if ally.dead { "X" } else { &initial },
                portrait.x + (portrait.w - width) / 2,
                portrait.y + portrait.h / 2 + font.ascent * s / 2,
                if ally.dead { quiet } else { [1.0; 4] },
            );
        }

        // Level in a small box at the portrait's lower right.
        let level = slot(list, 28, y + 27, 12, 12);
        if let Some(font) = small {
            let words = ally.level.to_string();
            let width = font.width(&words) * s;
            list.text(
                font,
                &words,
                level.x + (level.w - width) / 2,
                level.y + level.h / 2 + font.ascent * s / 2,
                text,
            );
        }

        if let Some(font) = small {
            let name = if ally.dead {
                format!("{}  {}s", ally.name, ally.respawn_seconds.unwrap_or(0))
            } else {
                ally.name.clone()
            };
            list.text(font, &name, area.x + 48 * s, area.y + (y + 15) * s, colour);
        }

        // Health and resource, green and blue, with no figures at this size.
        for (bar_y, value, max, colour) in [
            (y + 21, ally.health, ally.max_health, health),
            (y + 30, ally.resource, ally.max_resource, resource),
        ] {
            let (bx, by) = at(48, bar_y);
            let bar = Rect::new(bx, by, 100 * s, 6 * s);
            list.rect(bar, back);
            let fraction = if ally.dead || max == 0 {
                0.0
            } else {
                (value as f32 / max as f32).clamp(0.0, 1.0)
            };
            list.rect(
                Rect::new(bar.x, bar.y, (bar.w as f32 * fraction) as i32, bar.h),
                colour,
            );
        }

        // Two battle-spell marks at the right, one over the other.
        for (spell_y, label) in [(y + 4, "D"), (y + 23, "F")] {
            let spell = slot(list, 154, spell_y, 14, 14);
            if let Some(font) = small {
                let width = font.width(label) * s;
                list.text(
                    font,
                    label,
                    spell.x + (spell.w - width) / 2,
                    spell.y + spell.h / 2 + font.ascent * s / 2,
                    quiet,
                );
            }
        }

        list.rect(
            Rect::new(area.x, area.y + (y + 46) * s, area.w, s.max(1)),
            [0.32, 0.28, 0.20, 0.45],
        );
    }

    list.pop_clip();
    list.set_layer(previous);
}
