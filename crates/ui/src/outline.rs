//! Text with a dark outline: the same line drawn once in the outline colour
//! one pixel down and right, then again in its own colour over it. It is the
//! two draws the design asks for, and needs nothing new in the raster.

use crate::{Colour, DrawList, Font};

/// Draw `text` with its outline at the interface's scale. Returns the width
/// of the text in window pixels.
pub fn draw(
    list: &mut DrawList,
    font: &Font,
    text: &str,
    x: i32,
    baseline: i32,
    colour: Colour,
    outline: Colour,
) -> i32 {
    let s = list.scale();
    list.text(font, text, x + s, baseline + s, outline);
    list.text(font, text, x, baseline, colour)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::Image;
    use crate::theme::{GlyphSource, ThemeBuilder};

    fn theme() -> crate::Theme {
        let mut builder = ThemeBuilder::new();
        let glyphs = ['A', 'B']
            .into_iter()
            .map(|character| GlyphSource {
                character,
                image: Some(Image::filled(3, 5, [0, 0, 0, 255])),
                left: 0,
                top: 0,
                advance: 4,
            })
            .collect();
        builder.font("font.title", glyphs, 5, 7);
        builder.build()
    }

    #[test]
    fn the_outline_is_one_pixel_down_and_right_of_the_text() {
        let theme = theme();
        let font = theme.font("font.title").expect("the font was added");
        let mut list = DrawList::new((40, 20), 1, &theme.atlas);
        let width = draw(
            &mut list,
            font,
            "AB",
            10,
            15,
            [1.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        );
        assert_eq!(width, font.width("AB"));
        let (vertices, _) = list.finish();
        assert_eq!(vertices.len(), 16, "two glyphs, two draws");
        let outline_left = vertices[..8]
            .iter()
            .map(|v| v.pos[0])
            .fold(f32::INFINITY, f32::min);
        let text_left = vertices[8..]
            .iter()
            .map(|v| v.pos[0])
            .fold(f32::INFINITY, f32::min);
        assert_eq!(outline_left, text_left + 1.0);
        assert_eq!(vertices[0].pos[1], vertices[8].pos[1] + 1.0);
    }
}
