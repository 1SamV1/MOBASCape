//! OSRS hitmarks for the assembled interface: the exact cache splat under
//! the bold cache font, stacked down the unit like the legacy HUD.
//!
//! The theme carries one sprite per kind (`icon.hitmark.*`). A theme without
//! them still draws a coloured splat behind the number, so a missing cache
//! does not remove combat feedback.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::{HitmarkKind, HitmarkState};
use crate::theme::Theme;

/// Cache splats are 24x23, drawn at their native size like the OSRS client
/// (two times was too big over a busy fight).
const SPLAT_TIMES: u32 = 1;
/// Room for one splat plus a small gap, in window pixels.
const STACK_STEP: i32 = 23 + 2;
const LAYER: i32 = 2;

impl HitmarkKind {
    /// Flat colour behind the number if the theme has no splat art.
    fn fallback_colour(self) -> [u8; 3] {
        match self {
            Self::Damage => [195, 30, 28],
            Self::Poison => [35, 160, 45],
            Self::Venom => [35, 120, 112],
            Self::Disease => [220, 145, 20],
            Self::Heal => [165, 45, 190],
            Self::Miss => [45, 65, 220],
            Self::Max => [210, 45, 25],
        }
    }
}

fn colour(rgb: [u8; 3], alpha: f32) -> Colour {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        alpha,
    ]
}

/// Draw every live hitmark, bottom of the stack last.
pub fn draw(theme: &Theme, list: &mut DrawList, hits: &[HitmarkState]) {
    if hits.is_empty() {
        return;
    }
    let previous = list.set_layer(LAYER);
    let font = theme.font("font.bold").or_else(|| theme.font("font.body"));
    for hit in hits {
        if hit.alpha <= 0.01 {
            continue;
        }
        let role = format!("icon.hitmark.{}", hit.kind.role());
        let region = theme.sprite(&role);
        let (sw, sh) = if theme.has_sprite(&role) {
            (
                region.w as i32 * SPLAT_TIMES as i32,
                region.h as i32 * SPLAT_TIMES as i32,
            )
        } else {
            (24 * SPLAT_TIMES as i32, 23 * SPLAT_TIMES as i32)
        };
        let cx = hit.at.0;
        let cy = hit.at.1 + hit.stack as i32 * STACK_STEP;
        let x = cx - sw / 2;
        let y = cy - sh / 2;
        if theme.has_sprite(&role) {
            let tint = hit
                .tint
                .map_or([1.0, 1.0, 1.0, hit.alpha], |rgb| colour(rgb, hit.alpha));
            list.sprite_times(region, x, y, SPLAT_TIMES, tint);
        } else {
            list.rect(
                Rect::new(x, y, sw, sh),
                colour(hit.kind.fallback_colour(), hit.alpha * 0.92),
            );
        }

        let Some(font) = font else {
            continue;
        };
        // The bold cache font at its own size, as in the OSRS client; long
        // numbers simply run past the splat there too.
        let times = 1;
        let width = font.width(&hit.text) * times;
        let baseline = cy + 5 * times;
        let shadow = (times / 2).max(1);
        let tx = cx - width / 2;
        list.text_times(
            font,
            &hit.text,
            tx + shadow,
            baseline + shadow,
            times as u32,
            [0.0, 0.0, 0.0, hit.alpha],
        );
        list.text_times(
            font,
            &hit.text,
            tx,
            baseline,
            times as u32,
            [1.0, 1.0, 1.0, hit.alpha],
        );
    }
    list.set_layer(previous);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::Image;
    use crate::theme::{GlyphSource, ThemeBuilder};

    fn theme() -> Theme {
        let mut builder = ThemeBuilder::new();
        builder.sprite(
            "icon.hitmark.damage",
            Image::filled(24, 23, [170, 20, 20, 255]),
        );
        builder.font(
            "font.bold",
            (32u8..127)
                .map(|code| GlyphSource {
                    character: code as char,
                    image: Some(Image::filled(5, 8, [255, 255, 255, 255])),
                    left: 1,
                    top: 1,
                    advance: 6,
                })
                .collect(),
            12,
            14,
        );
        builder.build()
    }

    #[test]
    fn a_hitmark_draws_a_splat_and_number() {
        let theme = theme();
        let hits = [HitmarkState {
            at: (100, 100),
            text: "78".to_string(),
            kind: HitmarkKind::Damage,
            alpha: 1.0,
            stack: 0,
            tint: None,
        }];
        let mut list = DrawList::new((400, 300), 1, &theme.atlas);
        draw(&theme, &mut list, &hits);
        assert!(!list.is_empty());
    }

    #[test]
    fn a_tint_changes_the_splat_colour() {
        let theme = theme();
        let hit = |tint| HitmarkState {
            at: (100, 100),
            text: "1".to_string(),
            kind: HitmarkKind::Damage,
            alpha: 1.0,
            stack: 0,
            tint,
        };
        let mut plain = DrawList::new((400, 300), 1, &theme.atlas);
        draw(&theme, &mut plain, &[hit(None)]);
        let mut tinted = DrawList::new((400, 300), 1, &theme.atlas);
        draw(&theme, &mut tinted, &[hit(Some([200, 200, 220]))]);
        let (plain_v, _) = plain.finish();
        let (tinted_v, _) = tinted.finish();
        assert_ne!(plain_v[0].colour, tinted_v[0].colour);
    }
}
