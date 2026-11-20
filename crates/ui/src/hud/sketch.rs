//! The labelled rectangles of stage 3 step B (docs/INTERFACE_STAGE3_PLAN.md,
//! section 8). Every piece is drawn as an empty, named rectangle so the
//! placing can be looked at over the game before any component is built.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::layout::{Layout, Piece};
use crate::theme::Theme;

const PANEL: i32 = 10;
const WRITING: i32 = 11;

pub fn build(theme: &Theme, layout: &Layout) -> DrawList {
    let mut list = DrawList::new(layout.window, layout.scale, &theme.atlas);
    let s = list.scale();
    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let font = theme.font("font.small");
    let fill: Colour = [0.04, 0.06, 0.10, 0.58];
    let border = |piece: Piece| -> Colour {
        match piece {
            Piece::Score | Piece::Feed => [1.0, 0.78, 0.2, 0.95],
            Piece::Team | Piece::Target => [1.0, 0.35, 0.3, 0.95],
            Piece::Objectives | Piece::Chat => [0.95, 0.55, 0.15, 0.95],
            Piece::Ability | Piece::Inventory | Piece::Minimap => [0.2, 0.7, 1.0, 0.95],
        }
    };

    for (piece, rect) in &layout.pieces {
        list.set_layer(PANEL);
        list.rect(*rect, fill);
        list.outline(*rect, s.max(1), border(*piece));
        if let Some(font) = font {
            list.set_layer(WRITING);
            list.push_clip(*rect);
            let name = piece.name().to_uppercase();
            list.text(
                font,
                &format!("{name}  {} x {}", rect.w / s, rect.h / s),
                rect.x + 4 * s,
                rect.y + font.ascent * s + 3 * s,
                text,
            );
            list.pop_clip();
        }
    }

    // The header names the scale and what the collapse gave way. A requested
    // scale that had to be lowered says so, and the log says it too.
    if let Some(font) = font {
        list.set_layer(WRITING);
        let mut line = format!(
            "layout {} x {}  scale {}",
            layout.window.0, layout.window.1, layout.scale
        );
        if let Some(requested) = layout.requested
            && requested != layout.scale
        {
            line.push_str(&format!(
                "  (requested {}, drawn at {})",
                requested, layout.scale
            ));
        }
        line.push_str(&format!("  usual {:.1}%", layout.usual_coverage * 100.0));
        let area = Rect::new(6 * s, 6 * s, 520 * s, 16 * s);
        list.rect(area, [0.0, 0.0, 0.0, 0.72]);
        list.text(
            font,
            &line,
            area.x + 4 * s,
            area.y + font.ascent * s + 2 * s,
            text,
        );
        if !layout.steps.is_empty() {
            let steps = layout.steps.join("; ");
            let y = area.bottom() + font.ascent * s + 4 * s;
            list.text(font, &steps, area.x + 4 * s, y, quiet);
        }
        if !layout.conflicts.is_empty() {
            list.text(
                font,
                "CLASH LEFT - see the log",
                area.x + 4 * s,
                area.bottom() + 2 * (font.ascent * s + 4 * s),
                [1.0, 0.2, 0.2, 1.0],
            );
        }
    }

    list
}
