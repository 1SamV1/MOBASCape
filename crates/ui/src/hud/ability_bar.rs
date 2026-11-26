//! The ability bar (docs/INTERFACE_STAGE3_PLAN.md, section 5.2), with health
//! and resource inside it: portrait, four ability slots, two battle spells,
//! four small item slots, and the two bars.
//!
//! Stage 3 draws labelled grey slots and sample numbers. It does not read a
//! champion's abilities, items or cooldown durations; stage 4 fills those.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::{AbilityState, SpellSlot};
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

/// The skill-point "+" over ability `slot`: a 16-pixel button centred over
/// the slot and floating 4 pixels above the bar, as Dota 2 shows it. It sits
/// outside the bar's own rectangle, so it is drawn without the bar's clip.
pub fn upgrade_button(area: Rect, scale: i32, slot: usize) -> Rect {
    let s = scale.max(1);
    let x = 92 + slot as i32 * 42 + 12;
    Rect::new(area.x + x * s, area.y - 20 * s, 16 * s, 16 * s)
}

/// The "+" under the mouse, among the slots that can take a point.
pub fn upgrade_at(
    area: Rect,
    scale: i32,
    mouse: (i32, i32),
    upgradable: &[bool; 4],
) -> Option<usize> {
    (0..4).filter(|&n| upgradable[n]).find(|&n| {
        let b = upgrade_button(area, scale, n);
        mouse.0 >= b.x && mouse.0 < b.right() && mouse.1 >= b.y && mouse.1 < b.bottom()
    })
}

/// The two talent buttons above the bar while a talent pick is open: left
/// half and right half of the bar's width, 22 pixels tall, clear of the
/// skill-point "+" row.
pub fn talent_button(area: Rect, scale: i32, choice: usize) -> Rect {
    let s = scale.max(1);
    let w = SIZE.0 / 2 - 6;
    let x = 4 + choice as i32 * (SIZE.0 / 2);
    Rect::new(area.x + x * s, area.y - 48 * s, w * s, 22 * s)
}

/// The talent button under the mouse (0 left, 1 right).
pub fn talent_at(area: Rect, scale: i32, mouse: (i32, i32)) -> Option<usize> {
    (0..2).find(|&n| {
        let b = talent_button(area, scale, n);
        mouse.0 >= b.x && mouse.0 < b.right() && mouse.1 >= b.y && mouse.1 < b.bottom()
    })
}

/// Where the battle spells (D, F) and the teleport scroll (T) sit: left,
/// top and side, at the interface's own scale. T takes the bottom right
/// small slot, where Dota 2 keeps its scroll.
const SPELL_SLOTS: [(i32, i32, i32); 3] = [(278, 10, 40), (324, 10, 40), (412, 50, 36)];

/// The battle spell or scroll slot under the mouse: 0 for D, 1 for F, 2 for
/// the scroll on T.
pub fn spell_at(area: Rect, scale: i32, mouse: (i32, i32)) -> Option<usize> {
    let s = scale.max(1);
    let x = mouse.0 - area.x;
    let y = mouse.1 - area.y;
    SPELL_SLOTS.iter().position(|&(left, top, side)| {
        x >= left * s && x < (left + side) * s && y >= top * s && y < (top + side) * s
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
    round_portrait(list, at(9, 10), s);
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
    // Battle spells and the teleport scroll: a picture, a cooldown, a gold
    // fill along the bottom while channelling, and a greyed slot when the
    // hero has none.
    let spells = [
        (&state.spells[0], "D"),
        (&state.spells[1], "F"),
        (&state.scroll, "T"),
    ];
    for ((spell, label), &(x, y, side)) in spells.into_iter().zip(&SPELL_SLOTS) {
        draw_spell(theme, list, at(x, y), side * s, label, spell, s);
    }

    // The rest of the two by two block of small item slots.
    for (n, label) in ["1", "2", "3"].iter().enumerate() {
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

    // The skill-point "+" buttons float above the bar, outside its clip.
    for n in (0..4).filter(|&n| state.upgradable[n]) {
        let pulse = 0.5 + 0.5 * (state.time * 5.0 + n as f32 * 0.7).sin();
        draw_plus(list, upgrade_button(area, s, n), s, pulse);
    }

    // An open talent pick: two gold-edged buttons, Ctrl+1 / Ctrl+2.
    if let Some((level, lines)) = &state.talent_pick {
        let pulse = 0.5 + 0.5 * (state.time * 4.0).sin();
        if let Some(font) = small {
            let header = format!("TALENT (LEVEL {level}): CTRL+1 OR CTRL+2");
            let b = talent_button(area, s, 0);
            list.text(font, &header, b.x, b.y - 4 * s, title);
        }
        for (n, line) in lines.iter().enumerate() {
            let b = talent_button(area, s, n);
            list.rect(b, [0.55 + 0.3 * pulse, 0.42 + 0.2 * pulse, 0.12, 1.0]);
            let face = Rect::new(b.x + s, b.y + s, b.w - 2 * s, b.h - 2 * s);
            list.rect(face, [0.07, 0.06, 0.05, 0.96]);
            if let Some(font) = small {
                let mut label = format!("{}. {line}", n + 1);
                while font.width(&label) * s > face.w - 6 * s && label.len() > 4 {
                    label.pop();
                }
                list.push_clip(face);
                list.text(
                    font,
                    &label,
                    face.x + 3 * s,
                    face.y + face.h / 2 + font.ascent * s / 2,
                    text,
                );
                list.pop_clip();
            }
        }
    }
    list.set_layer(previous);
}

/// The portrait square's side and the ring's thickness, in interface pixels.
const PORTRAIT: i32 = 76;
const RING: f32 = 3.0;

/// Mask the portrait square to a circle and ring it in gold. Drawn as one
/// horizontal span per interface pixel row (the layer has no circles), so the
/// ring has no gaps at any slope.
fn round_portrait(list: &mut DrawList, origin: (i32, i32), s: i32) {
    let socket: Colour = [0.05, 0.045, 0.04, 1.0];
    let gold: Colour = [0.86, 0.68, 0.26, 1.0];
    let shadow: Colour = [0.32, 0.22, 0.06, 1.0];
    let half = PORTRAIT as f32 / 2.0;
    let outer = half - 1.0;
    let inner = outer - RING;
    let span = |list: &mut DrawList, row: i32, from: f32, to: f32, colour: Colour| {
        let (a, b) = (from.round() as i32, to.round() as i32);
        if b > a {
            list.rect(
                Rect::new(origin.0 + a * s, origin.1 + row * s, (b - a) * s, s),
                colour,
            );
        }
    };
    for row in 0..PORTRAIT {
        let dy = row as f32 + 0.5 - half;
        let ho = (outer * outer - dy * dy).max(0.0).sqrt();
        // Corners outside the ring become the dark socket.
        span(list, row, 0.0, half - ho, socket);
        span(list, row, half + ho, PORTRAIT as f32, socket);
        // The ring: the whole chord where the inner circle has ended.
        let colour = if dy > 0.0 { shadow } else { gold };
        if dy.abs() >= inner {
            span(list, row, half - ho, half + ho, colour);
        } else {
            let hi = (inner * inner - dy * dy).sqrt();
            span(list, row, half - ho, half - hi, colour);
            span(list, row, half + hi, half + ho, colour);
        }
    }
}

/// A gold "+" button with a dark edge; `pulse` (0 to 1) brightens it.
fn draw_plus(list: &mut DrawList, rect: Rect, s: i32, pulse: f32) {
    list.rect(rect, [0.12, 0.08, 0.02, 0.95]);
    let face = Rect::new(rect.x + s, rect.y + s, rect.w - 2 * s, rect.h - 2 * s);
    list.rect(face, [0.78 + 0.18 * pulse, 0.58 + 0.16 * pulse, 0.16, 1.0]);
    let ink: Colour = [0.16, 0.10, 0.02, 1.0];
    let (cx, cy) = (rect.x + rect.w / 2, rect.y + rect.h / 2);
    list.rect(Rect::new(cx - 5 * s, cy - s, 10 * s, 2 * s), ink);
    list.rect(Rect::new(cx - s, cy - 5 * s, 2 * s, 10 * s), ink);
}

/// One battle spell or scroll slot, `side` window pixels square at `at`.
fn draw_spell(
    theme: &Theme,
    list: &mut DrawList,
    at: (i32, i32),
    side: i32,
    label: &str,
    spell: &SpellSlot,
    s: i32,
) {
    let rect = Rect::new(at.0, at.1, side, side);
    match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, rect),
        None => list.rect(rect, [0.09, 0.08, 0.07, 0.95]),
    }
    let font = theme.font("font.bold").or_else(|| theme.font("font.body"));
    let small = theme.font("font.small").or(font);
    let role = spell.icon.as_ref().filter(|role| theme.has_sprite(role));
    if let Some(role) = role {
        let inset = 3 * s;
        list.sprite_fit(
            theme.sprite(role),
            Rect::new(
                rect.x + inset,
                rect.y + inset,
                side - 2 * inset,
                side - 2 * inset,
            ),
        );
    }
    let quiet = theme.colour("text.quiet");
    if !spell.usable {
        list.rect(rect, [0.0, 0.0, 0.0, 0.62]);
    }
    if spell.cooldown > 0.0 && spell.usable {
        list.rect(rect, [0.0, 0.0, 0.0, 0.48]);
        if let Some(font) = font {
            let secs = spell.cooldown.ceil().to_string();
            let width = font.width(&secs) * s;
            list.text(
                font,
                &secs,
                rect.x + (side - width) / 2,
                rect.y + side / 2 + font.ascent * s / 2,
                [1.0, 0.85, 0.3, 1.0],
            );
        }
    }
    if let Some(progress) = spell.channel {
        let gold = [1.0, 0.82, 0.30, 1.0];
        list.rect(rect, [1.0, 0.82, 0.30, 0.12]);
        let bar = 4 * s;
        list.rect(
            Rect::new(rect.x, rect.bottom() - bar, side, bar),
            [0.0, 0.0, 0.0, 0.7],
        );
        list.rect(
            Rect::new(
                rect.x,
                rect.bottom() - bar,
                (side as f32 * progress.clamp(0.0, 1.0)) as i32,
                bar,
            ),
            gold,
        );
    }
    // The hotkey: centred when there is no picture, else a corner badge.
    let colour = if spell.usable {
        theme.colour("text.title")
    } else {
        quiet
    };
    match (role, font, small) {
        (None, Some(font), _) => {
            let width = font.width(label) * s;
            list.text(
                font,
                label,
                rect.x + (side - width) / 2,
                rect.y + side / 2 + font.ascent * s / 2,
                colour,
            );
        }
        (Some(_), _, Some(small)) => {
            list.text(
                small,
                label,
                rect.x + 3 * s,
                rect.y + 2 * s + small.ascent * s,
                colour,
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_and_scroll_slots_are_hit_where_they_are_drawn() {
        let area = Rect::new(100, 200, SIZE.0 * 2, SIZE.1 * 2);
        assert_eq!(spell_at(area, 2, (100 + 279 * 2, 200 + 11 * 2)), Some(0));
        assert_eq!(spell_at(area, 2, (100 + 363 * 2, 200 + 49 * 2)), Some(1));
        assert_eq!(spell_at(area, 2, (100 + 430 * 2, 200 + 70 * 2)), Some(2));
        // The gap between D and F, and the item slot above T, miss.
        assert_eq!(spell_at(area, 2, (100 + 320 * 2, 200 + 30 * 2)), None);
        assert_eq!(spell_at(area, 2, (100 + 430 * 2, 200 + 30 * 2)), None);
    }

    #[test]
    fn plus_buttons_float_over_upgradable_slots_only() {
        let area = Rect::new(100, 200, SIZE.0 * 2, SIZE.1 * 2);
        let up = [false, true, false, true];
        // Centre of the W button: x 92 + 42 + 12 + 8, 12 above the bar.
        let w = (100 + (92 + 42 + 20) * 2, 200 - 12 * 2);
        assert_eq!(upgrade_at(area, 2, w, &up), Some(1));
        // Q's button spot is empty while Q cannot take a point.
        let q = (100 + (92 + 20) * 2, 200 - 12 * 2);
        assert_eq!(upgrade_at(area, 2, q, &up), None);
        // The button is above the bar, never on the slot itself.
        let b = upgrade_button(area, 2, 3);
        assert!(b.bottom() <= area.y && b.y >= area.y - 24 * 2);
        assert_eq!(upgrade_at(area, 2, (w.0, 200 + 30 * 2), &up), None);
    }

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
