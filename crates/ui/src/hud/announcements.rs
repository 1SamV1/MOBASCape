//! Announcements (docs/INTERFACE_STAGE3_PLAN.md, section 5.10): one line of
//! the title font, yellow with a dark outline, centred in the upper quarter
//! of the window. It belongs to the moment and is not placed by the layout.
//!
//! Stage 3 draws one sample line; stage 4 draws the match's queue.

use crate::draw::DrawList;
use crate::outline;
use crate::theme::Theme;

const LAYER: i32 = 40;
/// The measured baseline is 250 of 1080, so 23% of the height.
const BASELINE_PERCENT: i32 = 23;

pub fn draw(theme: &Theme, list: &mut DrawList, text: &str) {
    if text.is_empty() {
        return;
    }
    let small = theme.font("font.small");
    let font = theme
        .font("font.title")
        .or_else(|| theme.font("font.bold"))
        .or_else(|| theme.font("font.body"))
        .or(small);
    let Some(font) = font else {
        return;
    };

    let s = list.scale();
    let screen = list.screen();
    let width = font.width(text) * s;
    let x = (screen.w - width - s) / 2;
    let baseline = screen.h * BASELINE_PERCENT / 100;
    let previous = list.set_layer(LAYER);
    outline::draw(
        list,
        font,
        text,
        x,
        baseline,
        theme.colour("announcement"),
        theme.colour("text.outline"),
    );
    list.set_layer(previous);
}
