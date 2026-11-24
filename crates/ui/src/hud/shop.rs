//! The shop overlay (docs/INTERFACE_STAGE3_PLAN.md, section 5.11): a centred
//! 782 by 580 stone frame over a dimmed match, with category rows, a search
//! line, the item rows, the chosen item's detail and a Buy button, and the
//! "Your items" foot line with the gold count.
//!
//! Stage 3 draws the frame and our six sample items; stage 4 fills it from
//! the item system. Categories, rows, Buy and Close are routed. The item list
//! is clipped and scrolled, so a longer list cannot draw outside its panel.

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
/// The item grid: cells of an icon slot with the price under it.
const CELL: (i32, i32) = (52, 58);
const COLUMNS: i32 = 6;
const TIER_HEADER: i32 = 22;
/// The centre view, relative to the overlay's top left (unscaled).
const VIEW: (i32, i32, i32, i32) = (178, 82, 360, 398);

/// Section labels, by [`ShopItem::tier`].
pub const TIERS: [&str; 4] = ["Starter", "Early", "Core", "Late"];

/// Where each item sits in the grid, unscaled and before scrolling, from
/// the view's top left: `(item index, cell)`, plus the section headers
/// `(label, y)` and the content height. Items are grouped by tier and
/// cheapest first inside a tier, the shop's reading order.
#[allow(clippy::type_complexity)]
pub fn grid(items: &[ShopItem]) -> (Vec<(usize, Rect)>, Vec<(&'static str, i32)>, i32) {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| (items[i].tier.min(3), items[i].price, i));
    let mut cells = Vec::with_capacity(items.len());
    let mut headers = Vec::new();
    let mut y = 6;
    let mut i = 0;
    while i < order.len() {
        let tier = items[order[i]].tier.min(3);
        let run = order[i..]
            .iter()
            .take_while(|&&k| items[k].tier.min(3) == tier)
            .count();
        headers.push((TIERS[tier as usize], y));
        y += TIER_HEADER;
        for (k, &index) in order[i..i + run].iter().enumerate() {
            let column = k as i32 % COLUMNS;
            let row = k as i32 / COLUMNS;
            cells.push((
                index,
                Rect::new(10 + column * (CELL.0 + 6), y + row * CELL.1, CELL.0, CELL.1),
            ));
        }
        y += (run as i32 + COLUMNS - 1) / COLUMNS * CELL.1 + 6;
        i += run;
    }
    (cells, headers, y)
}

/// How far the grid can scroll, in interface pixels.
pub fn max_scroll(state: &ShopState) -> i32 {
    (grid(&state.items).2 - VIEW.3).max(0)
}

/// Where the shop sits: centred, and never larger than the window.
pub fn rect(screen: Rect, scale: i32) -> Rect {
    let s = scale.max(1);
    let w = SIZE.0 * s;
    let h = SIZE.1 * s;
    let x = (screen.x + (screen.w - w) / 2).max(screen.x);
    let y = (screen.y + (screen.h - h) / 2).max(screen.y);
    Rect::new(x, y, w.min(screen.w), h.min(screen.h))
}

/// What a click on the shop overlay means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShopHit {
    Close,
    Category(usize),
    Row(usize),
    Buy,
    /// Sell the selected item (only when it is in the bag).
    Sell,
    /// Take back this visit's newest purchase.
    Undo,
    Deliver,
}

/// Buy and Sell share the detail pane's bottom row.
fn trade_rects(area: Rect, scale: i32) -> (Rect, Rect) {
    let s = scale.max(1);
    let detail = Rect::new(area.x + 550 * s, area.y + 48 * s, 218 * s, 432 * s);
    let y = detail.bottom() - 48 * s;
    let w = (detail.w - 30 * s) / 2;
    (
        Rect::new(detail.x + 12 * s, y, w, 30 * s),
        Rect::new(detail.x + 18 * s + w, y, w, 30 * s),
    )
}

/// The Undo button on the foot line, left of the gold.
fn undo_rect(area: Rect, scale: i32) -> Rect {
    let s = scale.max(1);
    Rect::new(area.right() - 200 * s, area.y + 503 * s, 72 * s, 26 * s)
}

/// Hit-test the live shop. `area` is [`rect`]'s result and `scale` is the
/// interface scale the overlay was drawn at.
pub fn hit(area: Rect, scale: i32, state: &ShopState, mouse: (i32, i32)) -> Option<ShopHit> {
    let s = scale.max(1);
    let point = |x: i32, y: i32, w: i32, h: i32| {
        mouse.0 >= x && mouse.0 < x + w && mouse.1 >= y && mouse.1 < y + h
    };
    if point(area.right() - 40 * s, area.y + 10 * s, 24 * s, 24 * s) {
        return Some(ShopHit::Close);
    }
    let left = Rect::new(area.x + 14 * s, area.y + 48 * s, 150 * s, 432 * s);
    for (index, _) in state.categories.iter().enumerate() {
        let row = Rect::new(
            left.x + 6 * s,
            left.y + (26 + index as i32 * 27) * s,
            left.w - 12 * s,
            24 * s,
        );
        if point(row.x, row.y, row.w, row.h) {
            return Some(ShopHit::Category(index));
        }
    }
    let view = Rect::new(
        area.x + VIEW.0 * s,
        area.y + VIEW.1 * s,
        VIEW.2 * s,
        VIEW.3 * s,
    );
    if mouse.1 >= view.y && mouse.1 < view.bottom() {
        for (index, cell) in grid(&state.items).0 {
            let x = view.x + cell.x * s;
            let y = view.y + (cell.y - state.scroll) * s;
            if point(x, y, cell.w * s, cell.h * s) {
                return Some(ShopHit::Row(index));
            }
        }
    }
    let (buy, sell) = trade_rects(area, s);
    if point(buy.x, buy.y, buy.w, buy.h) {
        return Some(ShopHit::Buy);
    }
    if state.sell_price.is_some() && point(sell.x, sell.y, sell.w, sell.h) {
        return Some(ShopHit::Sell);
    }
    let undo = undo_rect(area, s);
    if state.undo.is_some() && point(undo.x, undo.y, undo.w, undo.h) {
        return Some(ShopHit::Undo);
    }
    if state.delivery.is_some() {
        let delivery = delivery_rect(area, s);
        if point(delivery.x, delivery.y, delivery.w, delivery.h) {
            return Some(ShopHit::Deliver);
        }
    }
    None
}

/// The delivery control in the detail pane, above Buy.
fn delivery_rect(area: Rect, scale: i32) -> Rect {
    let s = scale.max(1);
    let detail = Rect::new(area.x + 550 * s, area.y + 48 * s, 218 * s, 432 * s);
    Rect::new(
        detail.x + 12 * s,
        detail.bottom() - 80 * s,
        detail.w - 24 * s,
        26 * s,
    )
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

    // Categories down the left, under their heading.
    let left = Rect::new(area.x + 14 * s, area.y + 48 * s, 150 * s, 432 * s);
    dark(theme, list, left);
    if let Some(font) = small {
        list.text(font, "Categories", left.x + 12 * s, left.y + 18 * s, quiet);
    }
    for (index, label) in shop.categories.iter().enumerate() {
        let row = Rect::new(
            left.x + 6 * s,
            left.y + (26 + index as i32 * 27) * s,
            left.w - 12 * s,
            24 * s,
        );
        if index == shop.category {
            list.rect(row, [0.26, 0.21, 0.10, 0.90]);
        }
        if let Some(font) = body {
            list.text(
                font,
                label,
                left.x + 12 * s,
                row.y + 18 * s,
                if index == shop.category { title } else { text },
            );
        }
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

    // The item grid, in tier sections, clipped and scrolled.
    let view = Rect::new(
        area.x + VIEW.0 * s,
        area.y + VIEW.1 * s,
        VIEW.2 * s,
        VIEW.3 * s,
    );
    dark(theme, list, view);
    let (cells, headers, content) = grid(&shop.items);
    let scroll = Scroll::new(view, content * s, shop.scroll * s);
    list.push_clip(view);
    for (label, y) in headers {
        let y = scroll.place(y * s);
        if let Some(font) = bold {
            list.text(font, label, view.x + 10 * s, y + 15 * s, title);
            let width = font.width(label) * s;
            list.rect(
                Rect::new(
                    view.x + 18 * s + width,
                    y + 10 * s,
                    view.w - 30 * s - width,
                    1,
                ),
                [0.62, 0.50, 0.28, 0.55],
            );
        }
    }
    for (n, cell) in cells {
        let Some(item) = shop.items.get(n) else {
            continue;
        };
        let x = view.x + cell.x * s;
        let y = scroll.place(cell.y * s);
        if y + cell.h * s < view.y || y > view.bottom() {
            continue;
        }
        let frame = Rect::new(x + 4 * s, y + 2 * s, (cell.w - 8) * s, 38 * s);
        if n == shop.selected {
            list.rect(
                Rect::new(x, y, cell.w * s, cell.h * s),
                [0.30, 0.24, 0.10, 0.85],
            );
        }
        slot(theme, list, frame);
        if n == shop.selected {
            list.outline(frame, s.max(1), [0.95, 0.80, 0.40, 1.0]);
        }
        let icon = item
            .icon
            .as_deref()
            .filter(|role| theme.has_sprite(role))
            .map(|role| theme.sprite(role));
        if let Some(icon) = icon {
            let (w, h) = list.size_of(icon);
            list.sprite(
                icon,
                frame.x + (frame.w - w) / 2,
                frame.y + (frame.h - h) / 2,
            );
        } else {
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
        }
        // The price under the icon: green for an upgrade, gold when
        // affordable, greyed when not.
        if let Some(font) = small {
            let price = item.price.to_string();
            let width = font.width(&price) * s;
            list.text(
                font,
                &price,
                x + (cell.w * s - width) / 2,
                y + 54 * s,
                if item.price > gold {
                    quiet
                } else if item.price < item.full_price {
                    good
                } else {
                    gold_colour
                },
            );
        }
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
            let price = if item.price < item.full_price {
                format!("Upgrade {} gold  (full {})", item.price, item.full_price)
            } else {
                format!("{} gold", item.price)
            };
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
            if !item.builds_from.is_empty() {
                y += 8 * s;
                list.text(font, "Builds from", detail.x + 12 * s, y, title);
                y += 16 * s;
                for part in &item.builds_from {
                    list.text(font, part, detail.x + 12 * s, y, text);
                    y += 16 * s;
                }
            }
            if let Some(builds) = &item.builds_into {
                y += 8 * s;
                list.text(font, "Builds into", detail.x + 12 * s, y, title);
                list.text(font, builds, detail.x + 12 * s, y + 16 * s, text);
            }
        }
        // Ownership and the sell-back price, above the buttons.
        if let Some(font) = small {
            let line = match shop.sell_price {
                Some(price) => format!("In your bag: {}   Sells for {price}", shop.owned),
                None => "Not in your bag".to_string(),
            };
            list.text(
                font,
                &line,
                detail.x + 12 * s,
                detail.bottom() - 58 * s,
                quiet,
            );
        }
        let (buy, sell) = trade_rects(area, s);
        button(
            theme,
            list,
            buy,
            "Buy",
            bold,
            if item.price <= gold { text } else { quiet },
        );
        button(
            theme,
            list,
            sell,
            "Sell",
            bold,
            if shop.sell_price.is_some() {
                text
            } else {
                quiet
            },
        );
    } else if let Some(font) = body {
        list.text(
            font,
            "No item selected",
            detail.x + 12 * s,
            detail.y + 24 * s,
            quiet,
        );
    }
    if let Some(label) = &shop.delivery {
        let delivery = delivery_rect(area, s);
        match theme.panel("button.stone") {
            Some(panel) => list.panel(panel, delivery),
            None => list.rect(delivery, [0.12, 0.10, 0.08, 0.95]),
        }
        if let Some(font) = bold {
            let width = font.width(label) * s;
            list.text(
                font,
                label,
                delivery.x + (delivery.w - width) / 2,
                delivery.y + (delivery.h + font.ascent * s) / 2,
                text,
            );
        }
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
    let undo = undo_rect(area, s);
    button(
        theme,
        list,
        undo,
        "Undo",
        bold,
        if shop.undo.is_some() { text } else { quiet },
    );
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

/// A stone button with a centred label.
fn button(
    theme: &Theme,
    list: &mut DrawList,
    area: Rect,
    label: &str,
    font: Option<&crate::font::Font>,
    colour: Colour,
) {
    let s = list.scale();
    match theme.panel("button.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.12, 0.10, 0.08, 0.95]),
    }
    if let Some(font) = font {
        let width = font.width(label) * s;
        list.text(
            font,
            label,
            area.x + (area.w - width) / 2,
            area.y + (area.h + font.ascent * s) / 2,
            colour,
        );
    }
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
