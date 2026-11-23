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

/// The ability slot (0 for Q to 3 for R) under the mouse, if any. Matches
/// the slot geometry `draw` uses: 40 by 40 at x = 92 + 42n, y = 10.
pub fn ability_at(area: Rect, scale: i32, mouse: (i32, i32)) -> Option<usize> {
    let s = scale.max(1);
    let x = mouse.0 - area.x;
    let y = mouse.1 - area.y;
    if y < 10 * s || y >= 50 * s {
        return None;
    }
    (0..4).find(|n| {
        let left = (92 + *n as i32 * 42) * s;
        x >= left && x < left + 40 * s
    })
}

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

    // Four ability slots and the two battle spells. A slot that would take
    // a skill point pulses a gold border, shimmers and lights its hotkey.
    let gold: Colour = [1.0, 0.82, 0.30, 1.0];
    for (n, label) in ["Q", "W", "E", "R"].iter().enumerate() {
        let x = 92 + n as i32 * 42;
        let role = state.icons.get(n).and_then(Option::as_ref);
        let has_icon = role.is_some_and(|role| theme.has_sprite(role));
        let upgradable = state.upgradable.get(n).copied().unwrap_or(false);
        let pulse = 0.5 + 0.5 * (state.time * 5.0 + n as f32 * 0.7).sin();
        let letter = if upgradable {
            [1.0, 0.78 + 0.18 * pulse, 0.25 + 0.35 * pulse, 1.0]
        } else {
            title
        };
        slot(
            list,
            x,
            10,
            40,
            40,
            if has_icon { "" } else { label },
            if upgradable { letter } else { text },
        );
        let drew_icon = icon(list, role, x, 10, 40, 40);
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
        if upgradable {
            let (ox, oy) = at(x, 10);
            let frame = Rect::new(ox, oy, 40 * s, 40 * s);
            // A soft wash, then a band of light sweeping across.
            list.rect(frame, [1.0, 0.78, 0.25, 0.08 + 0.10 * pulse]);
            list.push_clip(frame);
            let sweep = (state.time * 0.9 + n as f32 * 0.23).fract();
            let band_x = ox + ((sweep * 64.0 - 12.0) * s as f32) as i32;
            list.rect(Rect::new(band_x, oy, 9 * s, 40 * s), [1.0, 0.9, 0.55, 0.22]);
            list.rect(
                Rect::new(band_x + 3 * s, oy, 3 * s, 40 * s),
                [1.0, 0.97, 0.8, 0.30],
            );
            list.pop_clip();
            let edge = [1.0, 0.80, 0.28, 0.55 + 0.45 * pulse];
            let t = 2 * s;
            list.rect(Rect::new(ox, oy, frame.w, t), edge);
            list.rect(Rect::new(ox, oy + frame.h - t, frame.w, t), edge);
            list.rect(Rect::new(ox, oy, t, frame.h), edge);
            list.rect(Rect::new(ox + frame.w - t, oy, t, frame.h), edge);
        }
        if drew_icon {
            // The hotkey stays readable as a small corner badge; it glows
            // gold while the slot can take a point.
            if let Some(font) = small {
                let (kx, ky) = at(x + 3, 10 + 2);
                if upgradable {
                    let w = font.width(label) * s;
                    list.rect(
                        Rect::new(kx - s, ky, w + 3 * s, font.ascent * s + 3 * s),
                        [0.35, 0.22, 0.02, 0.55 + 0.35 * pulse],
                    );
                }
                list.text(font, label, kx, ky + font.ascent * s, letter);
            }
        }
        // Rank pips under the slot: gold for each rank learned.
        let ranks = state.max_ranks.get(n).copied().unwrap_or(0).max(1) as i32;
        let rank = state.ranks.get(n).copied().unwrap_or(0) as i32;
        let gap = 2;
        let pip_w = (40 - gap * (ranks - 1)) / ranks;
        for pip in 0..ranks {
            let (px, py) = at(x + pip * (pip_w + gap), 50);
            let cell = Rect::new(px, py, pip_w * s, 6 * s);
            list.rect(cell, back);
            if pip < rank {
                list.rect(
                    Rect::new(cell.x + s, cell.y + s, cell.w - 2 * s, cell.h - 2 * s),
                    gold,
                );
            } else if upgradable && pip == rank {
                list.rect(
                    Rect::new(cell.x + s, cell.y + s, cell.w - 2 * s, cell.h - 2 * s),
                    [1.0, 0.82, 0.30, 0.25 + 0.45 * pulse],
                );
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_slots_are_hit_where_they_are_drawn() {
        let area = Rect::new(100, 200, SIZE.0 * 2, SIZE.1 * 2);
        assert_eq!(ability_at(area, 2, (100 + 93 * 2, 200 + 11 * 2)), Some(0));
        assert_eq!(
            ability_at(area, 2, (100 + (92 + 3 * 42 + 39) * 2, 200 + 49 * 2)),
            Some(3)
        );
        // The gap between W and E, the portrait and the bars miss.
        assert_eq!(
            ability_at(area, 2, (100 + (92 + 40) * 2 + 1, 200 + 30 * 2)),
            None
        );
        assert_eq!(ability_at(area, 2, (100 + 20 * 2, 200 + 30 * 2)), None);
        assert_eq!(ability_at(area, 2, (100 + 100 * 2, 200 + 70 * 2)), None);
    }
}
