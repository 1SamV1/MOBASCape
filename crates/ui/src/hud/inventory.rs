//! The inventory (docs/INTERFACE_STAGE3_PLAN.md, section 5.7): our six item
//! slots in two rows of three, the gold count and a Shop button.
//!
//! Stage 3 draws labelled squares and three sample items; stage 4 fills the
//! same slice from the match and routes the Shop button to the live overlay.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::InventoryItem;
use crate::theme::Theme;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (144, 160);
const LAYER: i32 = 20;
const CELL: i32 = 40;
const PITCH: i32 = 44;

/// The Shop button's rectangle inside the inventory's area.
pub fn shop_button(area: Rect, scale: i32) -> Rect {
    let s = scale.max(1);
    Rect::new(area.x + 8 * s, area.y + 128 * s, 128 * s, 24 * s)
}

/// Whether the pointer is on the Shop button.
pub fn shop_button_at(area: Rect, scale: i32, mouse: (i32, i32)) -> bool {
    let button = shop_button(area, scale);
    mouse.0 >= button.x
        && mouse.0 < button.right()
        && mouse.1 >= button.y
        && mouse.1 < button.bottom()
}

pub fn draw(
    theme: &Theme,
    list: &mut DrawList,
    area: Rect,
    items: &[Option<InventoryItem>],
    gold: u32,
) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.05, 0.05, 0.07, 0.95]),
    }

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let gold_colour = theme.colour("announcement");
    let small = theme.font("font.small");
    let bold = theme
        .font("font.bold")
        .or_else(|| theme.font("font.body"))
        .or(small);

    for n in 0..6 {
        let x = 8 + (n % 3) as i32 * PITCH;
        let y = 8 + (n / 3) as i32 * PITCH;
        let cell = Rect::new(area.x + x * s, area.y + y * s, CELL * s, CELL * s);
        match theme.panel("panel.slot") {
            Some(panel) => list.panel(panel, cell),
            None => list.rect(cell, [0.08, 0.07, 0.06, 0.95]),
        }
        match items.get(n).and_then(Option::as_ref) {
            Some(item) => {
                let icon = item
                    .icon
                    .as_deref()
                    .filter(|role| theme.has_sprite(role))
                    .map(|role| theme.sprite(role));
                if let Some(icon) = icon {
                    let (w, h) = list.size_of(icon);
                    list.sprite(icon, cell.x + (cell.w - w) / 2, cell.y + (cell.h - h) / 2);
                } else {
                    // The colour and initial stand in only when the cache
                    // could not bake this item's icon.
                    list.rect(
                        Rect::new(
                            cell.x + 4 * s,
                            cell.y + 4 * s,
                            (CELL - 8) * s,
                            (CELL - 8) * s,
                        ),
                        [
                            item.colour[0] as f32 / 255.0,
                            item.colour[1] as f32 / 255.0,
                            item.colour[2] as f32 / 255.0,
                            1.0,
                        ],
                    );
                    if let Some(font) = bold {
                        let initial = item.name.chars().next().unwrap_or('?').to_string();
                        let width = font.width(&initial) * s;
                        list.text(
                            font,
                            &initial,
                            cell.x + (cell.w - width) / 2,
                            cell.y + cell.h / 2 + font.ascent * s / 2,
                            ink(item.colour),
                        );
                    }
                }
            }
            None => {
                if let Some(font) = small {
                    let label = (n + 1).to_string();
                    let width = font.width(&label) * s;
                    list.text(
                        font,
                        &label,
                        cell.x + (cell.w - width) / 2,
                        cell.y + cell.h / 2 + font.ascent * s / 2,
                        quiet,
                    );
                }
            }
        }
    }

    // The gold pile and count, then the Shop button.
    let gold_x = if theme.has_sprite("icon.gold") {
        let icon = theme.sprite("icon.gold");
        list.sprite(icon, area.x + 8 * s, area.y + 96 * s);
        8 + icon.w as i32 + 4
    } else {
        list.rect(
            Rect::new(area.x + 8 * s, area.y + 102 * s, 14 * s, 14 * s),
            gold_colour,
        );
        26
    };
    if let Some(font) = bold {
        list.text(
            font,
            &gold.to_string(),
            area.x + gold_x * s,
            area.y + 116 * s,
            gold_colour,
        );
    }

    let button = shop_button(area, s);
    match theme.panel("button.stone") {
        Some(panel) => list.panel(panel, button),
        None => list.rect(button, [0.12, 0.10, 0.08, 0.95]),
    }
    if let Some(font) = bold {
        let width = font.width("Shop") * s;
        list.text(
            font,
            "Shop",
            button.x + (button.w - width) / 2,
            button.y + (button.h + font.ascent * s) / 2,
            text,
        );
    }

    list.pop_clip();
    list.set_layer(previous);
}

/// White on a dark item, near black on a light one, so the initial reads at
/// 40 pixels either way.
fn ink(colour: [u8; 3]) -> Colour {
    let light = 0.299 * colour[0] as f32 + 0.587 * colour[1] as f32 + 0.114 * colour[2] as f32;
    if light > 150.0 {
        [0.05, 0.05, 0.05, 1.0]
    } else {
        [1.0; 4]
    }
}
