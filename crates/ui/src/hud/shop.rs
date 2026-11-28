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
pub const TIERS: [&str; 5] = ["Starter", "Early", "Core", "Late", "Fusion"];

/// Width of a `+` or `>` mark between recipe cells.
const MARK_W: i32 = 14;
/// Stride between grid columns.
const STRIDE: i32 = CELL.0 + 6;

/// The grid as laid out: item cells, section headers, the `+` and `>`
/// marks of recipe rows, and the content height. All unscaled, from the
/// view's top left, before scrolling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub cells: Vec<(usize, Rect)>,
    pub headers: Vec<(&'static str, i32)>,
    /// A mark character and the rectangle it is centred in.
    pub marks: Vec<(char, Rect)>,
    pub height: i32,
}

/// Lay out the grid. Plain items are packed by tier, cheapest first.
/// Upgrade ladders (each step built from the one before) follow as one
/// `a > b > c` row each, wrapping when long; items made of two or more
/// parts get an `a + b > c` row. Parts are only linked when they are in
/// the same list (`ShopItem::recipe`).
pub fn layout(items: &[ShopItem]) -> Grid {
    let n = items.len();
    let key = |i: usize| (items[i].tier.min(4), items[i].price, i);
    let recipe = |i: usize| -> Vec<usize> {
        items[i]
            .recipe
            .iter()
            .copied()
            .filter(|&p| p < n && p != i)
            .collect()
    };
    // Single-part upgrades, by the part they grow from.
    let mut next: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        if let [part] = recipe(i)[..] {
            next[part].push(i);
        }
    }
    for list in &mut next {
        list.sort_by_key(|&i| key(i));
    }
    let mut roots: Vec<usize> = (0..n)
        .filter(|&i| !next[i].is_empty() && recipe(i).len() != 1)
        .collect();
    roots.sort_by_key(|&i| key(i));
    let mut in_ladder = vec![false; n];
    let mut ladders = Vec::new();
    for root in roots {
        let mut chain = vec![root];
        let mut current = root;
        while let Some(&step) = next[current]
            .iter()
            .find(|&&k| !in_ladder[k] && !chain.contains(&k))
        {
            chain.push(step);
            current = step;
        }
        if chain.len() >= 2 {
            for &k in &chain {
                in_ladder[k] = true;
            }
            ladders.push(chain);
        }
    }
    let mut multi: Vec<usize> = (0..n).filter(|&i| recipe(i).len() >= 2).collect();
    multi.sort_by_key(|&i| key(i));
    // Recipe scrolls only appear inside their fusion's row.
    let mut plain: Vec<usize> = (0..n)
        .filter(|&i| !in_ladder[i] && recipe(i).len() < 2 && !items[i].recipe_scroll)
        .collect();
    plain.sort_by_key(|&i| key(i));

    let mut grid = Grid {
        cells: Vec::with_capacity(n),
        headers: Vec::new(),
        marks: Vec::new(),
        height: 0,
    };
    let mut y = 6;
    let mut i = 0;
    while i < plain.len() {
        let tier = items[plain[i]].tier.min(4);
        let run = plain[i..]
            .iter()
            .take_while(|&&k| items[k].tier.min(4) == tier)
            .count();
        grid.headers.push((TIERS[tier as usize], y));
        y += TIER_HEADER;
        for (k, &index) in plain[i..i + run].iter().enumerate() {
            let column = k as i32 % COLUMNS;
            let row = k as i32 / COLUMNS;
            grid.cells.push((
                index,
                Rect::new(10 + column * STRIDE, y + row * CELL.1, CELL.0, CELL.1),
            ));
        }
        y += (run as i32 + COLUMNS - 1) / COLUMNS * CELL.1 + 6;
        i += run;
    }
    let right = VIEW.2 - 10;
    let mark = |grid: &mut Grid, c: char, x: i32, y: i32| {
        grid.marks.push((c, Rect::new(x, y + 2, MARK_W, 38)));
    };
    if !ladders.is_empty() {
        grid.headers.push(("Upgrades", y));
        y += TIER_HEADER;
        for chain in &ladders {
            let mut x = 10;
            for (k, &index) in chain.iter().enumerate() {
                if k > 0 {
                    if x + MARK_W + CELL.0 > right {
                        y += CELL.1;
                        x = 10;
                    }
                    mark(&mut grid, '>', x, y);
                    x += MARK_W;
                }
                grid.cells.push((index, Rect::new(x, y, CELL.0, CELL.1)));
                x += CELL.0;
            }
            y += CELL.1 + 4;
        }
        y += 2;
    }
    if !multi.is_empty() {
        grid.headers.push(("Recipes", y));
        y += TIER_HEADER;
        for &index in &multi {
            let mut x = 10;
            for (k, part) in recipe(index).into_iter().enumerate() {
                if k > 0 {
                    mark(&mut grid, '+', x, y);
                    x += MARK_W;
                }
                if x + CELL.0 > right {
                    y += CELL.1;
                    x = 10;
                }
                grid.cells.push((part, Rect::new(x, y, CELL.0, CELL.1)));
                x += CELL.0;
            }
            if x + MARK_W + CELL.0 > right {
                y += CELL.1;
                x = 10;
            }
            mark(&mut grid, '>', x, y);
            x += MARK_W;
            grid.cells.push((index, Rect::new(x, y, CELL.0, CELL.1)));
            y += CELL.1 + 4;
        }
        y += 2;
    }
    grid.height = y;
    grid
}

/// The grid's cells, headers and height (see [`layout`]).
#[allow(clippy::type_complexity)]
pub fn grid(items: &[ShopItem]) -> (Vec<(usize, Rect)>, Vec<(&'static str, i32)>, i32) {
    let grid = layout(items);
    (grid.cells, grid.headers, grid.height)
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
    /// The search box: start typing.
    Search,
    Buy,
    /// Sell the selected item (only when it is in the bag).
    Sell,
    /// Take back this visit's newest purchase.
    Undo,
    Deliver,
    /// A bag slot on the foot line (its inventory slot): show that item.
    Bag(usize),
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

/// The foot line's recessed panel: your items and the gold.
fn foot_rect(area: Rect, scale: i32) -> Rect {
    let s = scale.max(1);
    Rect::new(area.x + 14 * s, area.y + 488 * s, area.w - 28 * s, 80 * s)
}

/// The bag slots on the foot line, in bag order: an icon slot with the
/// sell price under it.
pub fn bag_rects(area: Rect, scale: i32, count: usize) -> Vec<Rect> {
    let s = scale.max(1);
    let foot = foot_rect(area, s);
    (0..count.min(6) as i32)
        .map(|k| Rect::new(foot.x + (150 + k * 46) * s, foot.y + 8 * s, 40 * s, 62 * s))
        .collect()
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
    if point(area.x + 178 * s, area.y + 48 * s, 360 * s, 26 * s) {
        return Some(ShopHit::Search);
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
    for (bag, rect) in state.bag.iter().zip(bag_rects(area, s, state.bag.len())) {
        if point(rect.x, rect.y, rect.w, rect.h) {
            return Some(ShopHit::Bag(bag.slot));
        }
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
    // The OSRS window: stone inside the riveted border.
    match theme
        .panel("panel.frame")
        .or_else(|| theme.panel("panel.stone"))
    {
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

    // Title centred in the top border band, as the Grand Exchange does.
    if let Some(font) = bold {
        let label = "Grand Exchange";
        let width = font.width(label) * s;
        list.text(
            font,
            label,
            area.x + (area.w - width) / 2,
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

    // Categories down the left: orange rows, the chosen one lit.
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
        let chosen = index == shop.category;
        if chosen {
            list.rect(row, [1.0, 1.0, 1.0, 0.10]);
        }
        if let Some(font) = body {
            list.text(
                font,
                &sentence(label),
                left.x + 12 * s,
                row.y + 18 * s,
                if chosen { gold_colour } else { title },
            );
        }
    }

    // The search line.
    let search = Rect::new(area.x + 178 * s, area.y + 48 * s, 360 * s, 26 * s);
    dark(theme, list, search);
    if shop.searching {
        // A thin gold frame while the box has the keyboard.
        let edge = [gold_colour[0], gold_colour[1], gold_colour[2], 0.8];
        list.outline(search, s.max(1), edge);
    }
    if let Some(font) = small {
        let typed = shop.searching || shop.search != "Search items";
        let shown = if shop.searching {
            format!("{}|", shop.search)
        } else if typed {
            shop.search.clone()
        } else {
            "Search items...".to_string()
        };
        list.text(
            font,
            &shown,
            search.x + 8 * s,
            search.y + 18 * s,
            if typed { text } else { quiet },
        );
    }

    // The item grid: tier sections, upgrade ladders and recipes, clipped
    // and scrolled.
    let view = Rect::new(
        area.x + VIEW.0 * s,
        area.y + VIEW.1 * s,
        VIEW.2 * s,
        VIEW.3 * s,
    );
    dark(theme, list, view);
    if shop.items.is_empty()
        && let Some(font) = small
    {
        list.text(
            font,
            "No items match the search.",
            view.x + 12 * s,
            view.y + 24 * s,
            quiet,
        );
    }
    let grid = layout(&shop.items);
    let scroll = Scroll::new(view, grid.height * s, shop.scroll * s);
    list.push_clip(view);
    for (label, y) in &grid.headers {
        let y = scroll.place(y * s);
        if let Some(font) = bold {
            list.text(font, label, view.x + 10 * s, y + 15 * s, title);
            let width = font.width(label) * s;
            list.rect(
                Rect::new(
                    view.x + 18 * s + width,
                    y + 10 * s,
                    view.w - 30 * s - width,
                    s.max(1),
                ),
                [0.62, 0.50, 0.28, 0.55],
            );
        }
    }
    for (mark, at) in &grid.marks {
        let y = scroll.place(at.y * s);
        if let Some(font) = bold {
            let label = mark.to_string();
            let width = font.width(&label) * s;
            list.text(
                font,
                &label,
                view.x + at.x * s + (at.w * s - width) / 2,
                y + (at.h * s + font.ascent * s) / 2,
                if *mark == '>' { title } else { quiet },
            );
        }
    }
    for &(n, cell) in &grid.cells {
        let Some(item) = shop.items.get(n) else {
            continue;
        };
        let x = view.x + cell.x * s;
        let y = scroll.place(cell.y * s);
        if y + cell.h * s < view.y || y > view.bottom() {
            continue;
        }
        let frame = Rect::new(x + 4 * s, y + 2 * s, (cell.w - 8) * s, 38 * s);
        item_slot(theme, list, frame, item.icon.as_deref(), item, s);
        if n == shop.selected {
            list.outline(frame, s.max(1), [1.0, 1.0, 0.0, 1.0]);
        } else if item.owned {
            list.outline(frame, s.max(1), [0.10, 0.85, 0.10, 1.0]);
        }
        // The price under the icon: green for an upgrade, gold when
        // affordable, red when not.
        if let Some(font) = small {
            let price = item.price.to_string();
            let width = font.width(&price) * s;
            list.text(
                font,
                &price,
                x + (cell.w * s - width) / 2,
                y + 54 * s,
                if item.price > gold {
                    [0.85, 0.20, 0.15, 1.0]
                } else if item.price < item.full_price {
                    good
                } else {
                    gold_colour
                },
            );
        }
    }
    list.pop_clip();

    // The chosen item's detail: icon, name and price; stats; what it is
    // made from and what it builds into, with icons; then the trade row.
    let detail = Rect::new(area.x + 550 * s, area.y + 48 * s, 218 * s, 432 * s);
    dark(theme, list, detail);
    list.push_clip(detail);
    if let Some(item) = shop.items.get(shop.selected) {
        let icon_slot = Rect::new(detail.x + 10 * s, detail.y + 10 * s, 36 * s, 34 * s);
        item_slot(theme, list, icon_slot, item.icon.as_deref(), item, s);
        if let Some(font) = bold {
            list.text(
                font,
                &item.name,
                detail.x + 52 * s,
                detail.y + 24 * s,
                title,
            );
        }
        if let Some(font) = small {
            let price = if item.price < item.full_price {
                format!("{} coins (full {})", item.price, item.full_price)
            } else {
                format!("{} coins", item.price)
            };
            list.text(
                font,
                &price,
                detail.x + 52 * s,
                detail.y + 40 * s,
                gold_colour,
            );
        }
        let mut y = detail.y + 66 * s;
        if let Some(font) = small {
            for stat in &item.stats {
                list.text(font, stat, detail.x + 12 * s, y, good);
                y += 15 * s;
            }
        }
        let trade_top = detail.bottom() - 82 * s;
        // Passives and the active: an orange heading, the sentence wrapped.
        if let Some(font) = small {
            let width = detail.w / s - 24;
            for (heading, sentence) in &item.effects {
                if y + 15 * s > trade_top {
                    break;
                }
                y += 4 * s;
                list.text(font, heading, detail.x + 12 * s, y, title);
                y += 14 * s;
                for line in crate::wrap::wrap(font, sentence, width) {
                    if y + 4 * s > trade_top {
                        break;
                    }
                    list.text(font, &line, detail.x + 12 * s, y, text);
                    y += 13 * s;
                }
            }
        }
        for (heading, links) in [("Builds from", &item.from), ("Builds into", &item.into)] {
            if links.is_empty() {
                continue;
            }
            y += 6 * s;
            if let Some(font) = small {
                list.text(font, heading, detail.x + 12 * s, y, gold_colour);
            }
            y += 6 * s;
            for link in links.iter() {
                if y + 26 * s > trade_top {
                    break;
                }
                let at = Rect::new(detail.x + 12 * s, y, 26 * s, 24 * s);
                if let Some(role) = link.icon.as_deref().filter(|role| theme.has_sprite(role)) {
                    list.sprite_fit(theme.sprite(role), at);
                }
                if let Some(font) = small {
                    list.text(font, &link.name, at.right() + 6 * s, y + 16 * s, text);
                    if link.owned {
                        let width = font.width("owned") * s;
                        list.text(
                            font,
                            "owned",
                            detail.right() - 10 * s - width,
                            y + 16 * s,
                            good,
                        );
                    }
                }
                y += 26 * s;
            }
        }
        // Ownership and the sell-back price, above the buttons.
        if let Some(font) = small {
            let line = match shop.sell_price {
                Some(price) => format!("In your bag: {}  Sells for {price}", shop.owned),
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
            if item.price <= gold { title } else { quiet },
        );
        button(
            theme,
            list,
            sell,
            "Sell",
            bold,
            if shop.sell_price.is_some() {
                title
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
    list.pop_clip();
    if let Some(label) = &shop.delivery {
        let delivery = delivery_rect(area, s);
        button(theme, list, delivery, label, bold, text);
    }

    // The foot line: your items (click one to sell it back), Undo and the
    // gold.
    let foot = foot_rect(area, s);
    dark(theme, list, foot);
    if let Some(font) = bold {
        list.text(font, "Your items", foot.x + 12 * s, foot.y + 24 * s, title);
    }
    if let Some(font) = small {
        let hint = if shop.bag.is_empty() {
            "Nothing yet."
        } else {
            "Click one to sell it."
        };
        list.text(font, hint, foot.x + 12 * s, foot.y + 42 * s, quiet);
        list.text(font, &shop.hint, foot.x + 12 * s, foot.y + 70 * s, quiet);
    }
    for (bag, rect) in shop.bag.iter().zip(bag_rects(area, s, shop.bag.len())) {
        let frame = Rect::new(rect.x, rect.y, rect.w, 38 * s);
        match bag.icon.as_deref().filter(|role| theme.has_sprite(role)) {
            Some(role) => {
                slot(theme, list, frame);
                let icon = theme.sprite(role);
                let (w, h) = list.size_of(icon);
                list.sprite(
                    icon,
                    frame.x + (frame.w - w) / 2,
                    frame.y + (frame.h - h) / 2,
                );
            }
            None => slot(theme, list, frame),
        }
        if bag.selected {
            list.outline(frame, s.max(1), [1.0, 1.0, 0.0, 1.0]);
        }
        if let Some(font) = small {
            let price = bag.sell.to_string();
            let width = font.width(&price) * s;
            list.text(
                font,
                &price,
                rect.x + (rect.w - width) / 2,
                rect.y + 52 * s,
                good,
            );
        }
    }
    let undo = undo_rect(area, s);
    button(
        theme,
        list,
        undo,
        "Undo",
        bold,
        if shop.undo.is_some() { title } else { quiet },
    );
    let count = format!("{gold} coins");
    let count_x = if let Some(font) = bold {
        let width = font.width(&count) * s;
        let x = area.right() - 24 * s - width;
        list.text(font, &count, x, area.y + 522 * s, gold_colour);
        x
    } else {
        area.right() - 24 * s
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

/// "BOOTS" -> "Boots" for the category rows.
fn sentence(label: &str) -> String {
    let lower = label.to_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// An item slot with the item's icon, or its colour swatch and initial.
fn item_slot(
    theme: &Theme,
    list: &mut DrawList,
    frame: Rect,
    icon: Option<&str>,
    item: &ShopItem,
    s: i32,
) {
    slot(theme, list, frame);
    match icon.filter(|role| theme.has_sprite(role)) {
        Some(role) => {
            let icon = theme.sprite(role);
            let (w, h) = list.size_of(icon);
            list.sprite(
                icon,
                frame.x + (frame.w - w) / 2,
                frame.y + (frame.h - h) / 2,
            );
        }
        None => {
            list.rect(
                Rect::new(
                    frame.x + 4 * s,
                    frame.y + 4 * s,
                    frame.w - 8 * s,
                    frame.h - 8 * s,
                ),
                swatch(item),
            );
            if let Some(font) = theme.font("font.bold").or(theme.font("font.body")) {
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
    }
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
    match theme
        .panel("button.osrs")
        .or_else(|| theme.panel("button.stone"))
    {
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
    match theme
        .panel("panel.inset")
        .or_else(|| theme.panel("panel.dark"))
    {
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
