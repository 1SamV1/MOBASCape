//! The shop overlay (docs/INTERFACE_STAGE3_PLAN.md, section 5.11): a centred
//! 782 by 580 stone frame over a dimmed match, with one category, a search
//! line, the item rows, the chosen item's detail and a Buy button, and the
//! "Your items" foot line with the gold count.
//!
//! Stage 3 draws the frame and our six sample items; stage 4 fills it from
//! the item system and makes buying work. The item list is clipped and
//! scrolled, so a longer list cannot draw outside its panel.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::{ShopItem, ShopState};
use crate::scroll::Scroll;
use crate::theme::Theme;

/// The overlay's size at the interface's own scale.
pub const SIZE: (i32, i32) = (782, 580);
/// The dim goes under the fixed interface; the shop itself over it.
const DIM_LAYER: i32 = 5;
const LAYER: i32 = 45;
const ROW: i32 = 44;

/// Where the shop sits: centred, and never larger than the window.
pub fn rect(screen: Rect, scale: i32) -> Rect {
    let s = scale.max(1);
    let w = SIZE.0 * s;
    let h = SIZE.1 * s;
    let x = (screen.x + (screen.w - w) / 2).max(screen.x);
    let y = (screen.y + (screen.h - h) / 2).max(screen.y);
    Rect::new(x, y, w.min(screen.w), h.min(screen.h))
}

pub fn draw(theme: &Theme, list: &mut DrawList, shop: &ShopState, gold: u32) {
    let s = list.scale();
    let screen = list.screen();
    let area = rect(screen, s);

    let previous = list.set_layer(DIM_LAYER);
    list.rect(screen, theme.colour("dim"));
    list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.05, 0.05, 0.07, 0.95]),
    }

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let title = theme.colour("text.title");
    let gold_colour = theme.colour("announcement");
    let good = theme.colour("text.good");
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let bold = theme.font("font.bold").or(body);
    let heading = theme.font("font.title").or(bold);

    // Title bar and the close placeholder.
    if let Some(font) = heading {
        list.text(
            font,
            "Grand Exchange",
            area.x + 16 * s,
            area.y + 30 * s,
            title,
        );
    }
    let close = Rect::new(area.right() - 40 * s, area.y + 10 * s, 24 * s, 24 * s);
    slot(theme, list, close);
    if let Some(font) = bold {
        let width = font.width("X") * s;
        list.text(
            font,
            "X",
            close.x + (close.w - width) / 2,
            close.y + (close.h + font.ascent * s) / 2,
            quiet,
        );
    }

    // Categories on the left: ours has one, under its heading.
    let left = Rect::new(area.x + 14 * s, area.y + 48 * s, 150 * s, 432 * s);
    dark(theme, list, left);
    if let Some(font) = small {
        list.text(font, "Categories", left.x + 12 * s, left.y + 18 * s, quiet);
    }
    list.rect(
        Rect::new(left.x + 6 * s, left.y + 26 * s, left.w - 12 * s, 24 * s),
        [0.26, 0.21, 0.10, 0.90],
    );
    if let Some(font) = body {
        list.text(
            font,
            &shop.category,
            left.x + 12 * s,
            left.y + 44 * s,
            title,
        );
    }

    // The search line.
    let search = Rect::new(area.x + 178 * s, area.y + 48 * s, 360 * s, 26 * s);
    dark(theme, list, search);
    if let Some(font) = small {
        list.text(
            font,
            &shop.search,
            search.x + 8 * s,
            search.y + 18 * s,
            quiet,
        );
    }

    // The item rows, clipped and scrolled.
    let view = Rect::new(area.x + 178 * s, area.y + 82 * s, 360 * s, 398 * s);
    dark(theme, list, view);
    let content = shop.items.len().max(1) as i32 * ROW * s;
    let scroll = Scroll::new(view, content, shop.scroll * s);
    list.push_clip(view);
    for (n, item) in shop.items.iter().enumerate() {
        let y = scroll.place(n as i32 * ROW * s);
        if y + 42 * s < view.y || y > view.bottom() {
            continue;
        }
        let row = Rect::new(view.x, y, view.w, 42 * s);
        if n == shop.selected {
            list.rect(row, [0.16, 0.20, 0.27, 0.92]);
        }
        let frame = Rect::new(view.x + 8 * s, y + 5 * s, 36 * s, 32 * s);
        slot(theme, list, frame);
        list.rect(
            Rect::new(
                frame.x + 4 * s,
                frame.y + 4 * s,
                frame.w - 8 * s,
                frame.h - 8 * s,
            ),
            swatch(item),
        );
        if let Some(font) = bold {
            let initial = item.name.chars().next().unwrap_or('?').to_string();
            let width = font.width(&initial) * s;
            list.text(
                font,
                &initial,
                frame.x + (frame.w - width) / 2,
                frame.y + (frame.h + font.ascent * s) / 2,
                ink(item.colour),
            );
        }
        if let Some(font) = body {
            list.text(font, &item.name, view.x + 54 * s, y + 25 * s, text);
        }
        if let Some(font) = bold {
            let price = item.price.to_string();
            let width = font.width(&price) * s;
            list.text(
                font,
                &price,
                view.right() - 10 * s - width,
                y + 25 * s,
                gold_colour,
            );
        }
        list.rect(
            Rect::new(view.x + 8 * s, y + 43 * s, view.w - 16 * s, 1),
            [0.30, 0.28, 0.22, 0.50],
        );
    }
    list.pop_clip();

    // The chosen item's detail.
    let detail = Rect::new(area.x + 550 * s, area.y + 48 * s, 218 * s, 432 * s);
    dark(theme, list, detail);
    if let Some(item) = shop.items.get(shop.selected) {
        if let Some(font) = bold {
            list.text(
                font,
                &item.name,
                detail.x + 12 * s,
                detail.y + 24 * s,
                gold_colour,
            );
        }
        if let Some(font) = small {
            let price = format!("{} gold", item.price);
            list.text(
                font,
                &price,
                detail.x + 12 * s,
                detail.y + 42 * s,
                gold_colour,
            );
        }
        if let Some(font) = small {
            let mut y = detail.y + 70 * s;
            for stat in &item.stats {
                list.text(font, stat, detail.x + 12 * s, y, good);
                y += 16 * s;
            }
            if let Some(builds) = &item.builds_into {
                y += 8 * s;
                list.text(font, "Builds into", detail.x + 12 * s, y, title);
                list.text(font, builds, detail.x + 12 * s, y + 16 * s, text);
            }
        }
        let buy = Rect::new(
            detail.x + 12 * s,
            detail.bottom() - 48 * s,
            detail.w - 24 * s,
            30 * s,
        );
        match theme.panel("button.stone") {
            Some(panel) => list.panel(panel, buy),
            None => list.rect(buy, [0.12, 0.10, 0.08, 0.95]),
        }
        if let Some(font) = bold {
            let width = font.width("Buy") * s;
            list.text(
                font,
                "Buy",
                buy.x + (buy.w - width) / 2,
                buy.y + (buy.h + font.ascent * s) / 2,
                text,
            );
        }
    } else if let Some(font) = body {
        list.text(
            font,
            "No item selected",
            detail.x + 12 * s,
            detail.y + 24 * s,
            quiet,
        );
    }

    // The foot line: what you own, a hint, and the gold.
    list.rect(
        Rect::new(area.x + 14 * s, area.y + 492 * s, area.w - 28 * s, 1),
        [0.40, 0.35, 0.25, 0.60],
    );
    if let Some(font) = bold {
        list.text(font, "Your items", area.x + 16 * s, area.y + 522 * s, title);
    }
    if let Some(font) = small {
        list.text(font, &shop.hint, area.x + 96 * s, area.y + 522 * s, quiet);
    }
    let count = gold.to_string();
    let count_x = if let Some(font) = bold {
        let width = font.width(&count) * s;
        let x = area.right() - 16 * s - width;
        list.text(font, &count, x, area.y + 522 * s, gold_colour);
        x
    } else {
        area.right() - 16 * s
    };
    if theme.has_sprite("icon.gold") {
        let icon = theme.sprite("icon.gold");
        list.sprite(icon, count_x - 8 * s - icon.w as i32 * s, area.y + 500 * s);
    } else {
        list.rect(
            Rect::new(count_x - 22 * s, area.y + 508 * s, 14 * s, 14 * s),
            gold_colour,
        );
    }

    list.pop_clip();
    list.set_layer(previous);
}

fn dark(theme: &Theme, list: &mut DrawList, area: Rect) {
    match theme.panel("panel.dark") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.02, 0.02, 0.03, 0.90]),
    }
}

fn slot(theme: &Theme, list: &mut DrawList, area: Rect) {
    match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.08, 0.07, 0.06, 0.95]),
    }
}

fn swatch(item: &ShopItem) -> Colour {
    [
        item.colour[0] as f32 / 255.0,
        item.colour[1] as f32 / 255.0,
        item.colour[2] as f32 / 255.0,
        1.0,
    ]
}

fn ink(colour: [u8; 3]) -> Colour {
    let light = 0.299 * colour[0] as f32 + 0.587 * colour[1] as f32 + 0.114 * colour[2] as f32;
    if light > 150.0 {
        [0.05, 0.05, 0.05, 1.0]
    } else {
        [1.0; 4]
    }
}
