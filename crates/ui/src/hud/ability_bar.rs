//! The ability bar (docs/INTERFACE_STAGE3_PLAN.md, section 5.2), with health
//! and resource inside it: portrait, four ability slots, two battle spells,
//! four small item slots, and the two bars.
//!
//! Stage 3 draws labelled grey slots and sample numbers. It does not read a
//! champion's abilities, items or cooldown durations; stage 4 fills those.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::AbilityState;
use crate::theme::Theme;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (458, 94);

const LAYER: i32 = 20;

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, state: &AbilityState) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.04, 0.04, 0.06, 0.88]),
    }

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let title = theme.colour("text.title");
    let health = theme.colour("bar.health");
    let resource = theme.colour("bar.resource");
    let back = theme.colour("bar.back");
    let at = |x: i32, y: i32| (area.x + x * s, area.y + y * s);
    let slot_font = theme.font("font.bold").or_else(|| theme.font("font.body"));
    let small = theme.font("font.small").or(slot_font);

    // A slot with its label centred in it, or a dark rectangle when the
    // theme has no slot frame.
    let slot =
        |list: &mut DrawList, x: i32, y: i32, w: i32, h: i32, label: &str, colour: Colour| {
            let (sx, sy) = at(x, y);
            let rect = Rect::new(sx, sy, w * s, h * s);
            match theme.panel("panel.slot") {
                Some(panel) => list.panel(panel, rect),
                None => list.rect(rect, [0.09, 0.08, 0.07, 0.95]),
            }
            if let Some(font) = slot_font {
                let width = font.width(label) * s;
                list.text(
                    font,
                    label,
                    rect.x + (rect.w - width) / 2,
                    rect.y + rect.h / 2 + font.ascent * s / 2,
                    colour,
                );
            }
        };

    // A theme sprite fitted inside a slot's frame, when the role exists.
    let icon =
        |list: &mut DrawList, role: Option<&String>, x: i32, y: i32, w: i32, h: i32| match role
            .filter(|role| theme.has_sprite(role))
        {
            Some(role) => {
                let (sx, sy) = at(x + 3, y + 3);
                list.sprite_fit(
                    theme.sprite(role),
                    Rect::new(sx, sy, (w - 6) * s, (h - 6) * s),
                );
                true
            }
            None => false,
        };

    // Portrait: the cache-baked champion face, or a labelled square when the
    // theme has none. The level sits in a small box at its lower right.
    let has_portrait = state
        .portrait
        .as_ref()
        .is_some_and(|role| theme.has_sprite(role));
    slot(
        list,
        9,
        10,
        76,
        76,
        if has_portrait { "" } else { "HERO" },
        quiet,
    );
    icon(list, state.portrait.as_ref(), 9, 10, 76, 76);
    slot(list, 68, 68, 18, 18, &state.level.to_string(), title);

    // Four ability slots and the two battle spells.
    for (n, label) in ["Q", "W", "E", "R"].iter().enumerate() {
        let x = 92 + n as i32 * 42;
        let role = state.icons.get(n).and_then(Option::as_ref);
        let has_icon = role.is_some_and(|role| theme.has_sprite(role));
        slot(list, x, 10, 40, 40, if has_icon { "" } else { label }, text);
        if icon(list, role, x, 10, 40, 40) {
            // The hotkey stays readable as a small corner badge.
            if let Some(font) = small {
                let (kx, ky) = at(x + 3, 10 + 2);
                list.text(font, label, kx, ky + font.ascent * s, title);
            }
        }
        let cooldown = state.cooldowns.get(n).copied().unwrap_or(0.0);
        if cooldown > 0.0 {
            let (ox, oy) = at(x, 10);
            list.rect(Rect::new(ox, oy, 40 * s, 40 * s), [0.0, 0.0, 0.0, 0.48]);
            if let Some(font) = slot_font {
                let secs = cooldown.ceil().to_string();
                let width = font.width(&secs) * s;
                list.text(
                    font,
                    &secs,
                    ox + (40 * s - width) / 2,
                    oy + 40 * s / 2 + font.ascent * s / 2,
                    [1.0, 0.85, 0.3, 1.0],
                );
            }
        }
        // A thin rank strip under each ability.
        let (rx, ry) = at(x, 50);
        list.rect(Rect::new(rx, ry, 40 * s, 6 * s), back);
        list.rect(Rect::new(rx, ry, 20 * s, 6 * s), [0.62, 0.52, 0.24, 1.0]);
    }
    slot(list, 278, 10, 40, 40, "D", quiet);
    slot(list, 324, 10, 40, 40, "F", quiet);

    // The two by two block of small item slots.
    for (n, label) in ["1", "2", "3", "4"].iter().enumerate() {
        let x = 372 + (n as i32 % 2) * 40;
        let y = 10 + (n as i32 / 2) * 40;
        slot(list, x, y, 36, 36, label, quiet);
    }

    // Health and resource, with a paler stretch at the end of health for
    // recent damage, and the figures centred on each bar.
    let bars = [
        (60, state.health, state.max_health, health, 0.10),
        (76, state.resource, state.max_resource, resource, 0.0),
    ];
    for (y, value, max, colour, damage) in bars {
        let (bx, by) = at(93, y);
        let bar = Rect::new(bx, by, 270 * s, 14 * s);
        list.rect(bar, back);
        let fraction = if max == 0 {
            0.0
        } else {
            (value as f32 / max as f32).clamp(0.0, 1.0)
        };
        if damage > 0.0 {
            let stretch = (fraction + damage).min(1.0);
            list.rect(
                Rect::new(bar.x, bar.y, (bar.w as f32 * stretch) as i32, bar.h),
                [0.85, 0.20, 0.20, 0.42],
            );
        }
        list.rect(
            Rect::new(bar.x, bar.y, (bar.w as f32 * fraction) as i32, bar.h),
            colour,
        );
        if let Some(font) = small {
            let words = format!("{value} / {max}");
            let width = font.width(&words) * s;
            list.text(
                font,
                &words,
                bar.x + (bar.w - width) / 2,
                bar.y + bar.h / 2 + font.ascent * s / 2,
                text,
            );
        }
    }

    list.pop_clip();
    list.set_layer(previous);
}
