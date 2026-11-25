//! OSRS overhead chat for the assembled interface: bold yellow text with a
//! one-pixel black shadow, centred over the speaker ("Taste vengeance!").
//!
//! Like unit plates and hitmarks, the client's projection places each line
//! and this module only draws it.

use crate::draw::DrawList;
use crate::hud::state::OverheadState;
use crate::theme::Theme;

const LAYER: i32 = 2;
/// The reference client's overhead chat yellow.
const YELLOW: [f32; 3] = [1.0, 1.0, 0.0];

pub fn draw(theme: &Theme, list: &mut DrawList, lines: &[OverheadState]) {
    let Some(font) = theme.font("font.bold").or_else(|| theme.font("font.body")) else {
        return;
    };
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    for line in lines {
        if line.alpha <= 0.01 || line.text.is_empty() {
            continue;
        }
        let width = font.width(&line.text) * s;
        let x = line.at.0 - width / 2;
        let baseline = line.at.1;
        let shadow = [0.0, 0.0, 0.0, line.alpha];
        let text = [YELLOW[0], YELLOW[1], YELLOW[2], line.alpha];
        list.text(font, &line.text, x + s, baseline + s, shadow);
        list.text(font, &line.text, x, baseline, text);
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

    fn line(text: &str, alpha: f32) -> OverheadState {
        OverheadState {
            at: (200, 100),
            text: text.to_string(),
            alpha,
        }
    }

    #[test]
    fn faded_and_empty_lines_draw_nothing() {
        let theme = theme();
        let mut list = DrawList::new((400, 300), 1, &theme.atlas);
        draw(
            &theme,
            &mut list,
            &[line("", 1.0), line("Taste vengeance!", 0.0)],
        );
        assert!(list.is_empty());
    }

    #[test]
    fn a_line_is_yellow_over_a_black_shadow_and_centred() {
        let theme = theme();
        let mut list = DrawList::new((400, 300), 1, &theme.atlas);
        draw(&theme, &mut list, &[line("Hi", 1.0)]);
        let (vertices, _) = list.finish();
        // Two glyphs, each drawn twice (shadow first, then yellow).
        assert_eq!(vertices.len(), 4 * 4);
        assert_eq!(vertices[0].colour, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(vertices[8].colour, [1.0, 1.0, 0.0, 1.0]);
        // "Hi" is 12 pixels wide, so the pen starts 6 left of the anchor.
        let left = vertices[8..]
            .iter()
            .map(|v| v.pos[0])
            .fold(f32::MAX, f32::min);
        assert_eq!(left, 200.0 - 6.0 + 1.0);
    }
}
