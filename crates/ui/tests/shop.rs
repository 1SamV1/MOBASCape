//! The shop checked as rectangles and pixels: the frame is centred at 782 by
//! 580, everything but the dim stays inside it at scales 1 to 4, the dim is
//! the only thing outside, and a theme with only fonts still draws the
//! fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::sample;
use rift_ui::hud::shop::{self, ShopHit};
use rift_ui::raster;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const GROUND: [u8; 4] = [8, 11, 17, 255];
const DIM: [f32; 4] = [0.0, 0.0, 0.0, 0.5];

fn numbered(width: usize, height: usize) -> Image {
    Image::new(
        width,
        height,
        (0..width * height)
            .map(|i| [(i % width) as u8, (i / width) as u8, 100, 255])
            .collect(),
    )
}

fn boxes() -> Vec<GlyphSource> {
    (32u8..127)
        .map(|code| {
            let c = code as char;
            let w = (code % 3 + 2) as usize;
            GlyphSource {
                character: c,
                image: (c != ' ').then(|| Image::filled(w, 6, [0, 0, 0, 255])),
                left: 1,
                top: 2,
                advance: w as i32 + 2,
            }
        })
        .collect()
}

fn theme() -> Theme {
    let mut builder = ThemeBuilder::new();
    for role in ["panel.stone", "panel.dark", "panel.slot", "button.stone"] {
        builder.panel(role, numbered(20, 20), [5, 5, 5, 5]).unwrap();
    }
    builder.sprite("icon.gold", numbered(10, 10));
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    builder.font("font.title", boxes(), 10, 12);
    for role in [
        "text",
        "text.quiet",
        "text.title",
        "announcement",
        "text.good",
    ] {
        builder.colour(role, [1.0; 4]);
    }
    builder.colour("dim", DIM);
    builder.build()
}

#[test]
fn the_shop_is_centred_at_782_by_580() {
    let screen = Rect::new(0, 0, 1920, 1080);
    assert_eq!(shop::rect(screen, 1), Rect::new(569, 250, 782, 580));
    assert_eq!(shop::rect(screen, 2), Rect::new(178, 0, 1564, 1080));
}

#[test]
fn everything_but_the_dim_stays_inside_the_shop_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let (w, h) = (shop::SIZE.0 * s as i32 + 40, shop::SIZE.1 * s as i32 + 40);
        let mut list = DrawList::new((w as u32, h as u32), s, &theme.atlas);
        shop::draw(&theme, &mut list, &state.shop, state.gold);
        let area = shop::rect(Rect::new(0, 0, w, h), s as i32);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        // The first quad is the dim; everything after it is shop content.
        for vertex in vertices.iter().skip(4) {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}

#[test]
fn only_the_dim_is_drawn_outside_the_shop() {
    let theme = theme();
    let state = sample::state();
    let (w, h) = (900usize, 700usize);
    let mut list = DrawList::new((w as u32, h as u32), 1, &theme.atlas);
    shop::draw(&theme, &mut list, &state.shop, state.gold);
    let area = shop::rect(Rect::new(0, 0, w as i32, h as i32), 1);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; w * h];
    raster::render(&vertices, &indices, &theme.atlas, w, h, &mut picture);
    let dimmed = [
        (GROUND[0] as f32 * (1.0 - DIM[3])).round() as u8,
        (GROUND[1] as f32 * (1.0 - DIM[3])).round() as u8,
        (GROUND[2] as f32 * (1.0 - DIM[3])).round() as u8,
    ];
    for y in 0..h {
        for x in 0..w {
            let inside = x as i32 >= area.x
                && (x as i32) < area.right()
                && (y as i32) >= area.y
                && (y as i32) < area.bottom();
            if !inside {
                let pixel = picture[y * w + x];
                assert_eq!([pixel[0], pixel[1], pixel[2]], dimmed, "pixel {x},{y}");
            }
        }
    }
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_shop() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.body", boxes(), 10, 12);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let mut list = DrawList::new((1920, 1080), 1, &theme.atlas);
    shop::draw(&theme, &mut list, &state.shop, state.gold);
    assert!(!list.is_empty());
}

#[test]
fn clicks_hit_categories_rows_buy_and_close() {
    let mut state = sample::state();
    state.shop.categories = [
        "ALL",
        "MELEE",
        "RANGED",
        "MAGIC",
        "ARMOUR",
        "JEWELLERY",
        "BOOTS",
    ]
    .iter()
    .map(|label| (*label).to_string())
    .collect();
    let area = shop::rect(Rect::new(0, 0, 1920, 1080), 1);
    for index in 0..7 {
        let y = area.y + 48 + 26 + index as i32 * 27 + 5;
        assert_eq!(
            shop::hit(area, 1, &state.shop, (600, y)),
            Some(ShopHit::Category(index))
        );
    }
    // Every grid cell hits its own item, and scrolling moves the cells.
    let (cells, _, _) = shop::grid(&state.shop.items);
    let centre = |cell: Rect, scroll: i32| {
        (
            area.x + 178 + cell.x + cell.w / 2,
            area.y + 82 + cell.y - scroll + cell.h / 2,
        )
    };
    for &(index, cell) in &cells {
        assert_eq!(
            shop::hit(area, 1, &state.shop, centre(cell, 0)),
            Some(ShopHit::Row(index))
        );
    }
    let (index, cell) = cells[cells.len() - 1];
    state.shop.scroll = 20;
    assert_eq!(
        shop::hit(area, 1, &state.shop, centre(cell, 20)),
        Some(ShopHit::Row(index))
    );
    state.shop.scroll = 0;
    // Buy and Sell share the bottom row; Sell only answers when the item
    // is in the bag, and Undo only when there is a purchase to undo.
    assert_eq!(
        shop::hit(area, 1, &state.shop, (1170, 697)),
        Some(ShopHit::Buy)
    );
    assert_eq!(shop::hit(area, 1, &state.shop, (1280, 697)), None);
    state.shop.sell_price = Some(210);
    assert_eq!(
        shop::hit(area, 1, &state.shop, (1280, 697)),
        Some(ShopHit::Sell)
    );
    assert_eq!(
        shop::hit(area, 1, &state.shop, (area.x + 200, area.y + 60)),
        Some(ShopHit::Search)
    );
    let undo = (area.right() - 200 + 30, area.y + 503 + 12);
    assert_eq!(shop::hit(area, 1, &state.shop, undo), None);
    state.shop.undo = Some("Bronze sword".to_string());
    assert_eq!(shop::hit(area, 1, &state.shop, undo), Some(ShopHit::Undo));
    state.shop.delivery = Some("DELIVER (2)".to_string());
    assert_eq!(
        shop::hit(area, 1, &state.shop, (1200, 660)),
        Some(ShopHit::Deliver)
    );
    assert_eq!(
        shop::hit(area, 1, &state.shop, (1323, 272)),
        Some(ShopHit::Close)
    );
    assert_eq!(shop::hit(area, 1, &state.shop, (10, 10)), None);
}

#[test]
fn the_grid_groups_items_by_tier_cheapest_first() {
    let state = sample::state();
    let items = &state.shop.items;
    let (cells, headers, height) = shop::grid(items);
    assert_eq!(cells.len(), items.len());
    // Reading order never goes back a tier, and within a tier never down
    // in price.
    for pair in cells.windows(2) {
        let (a, b) = (&items[pair[0].0], &items[pair[1].0]);
        assert!((a.tier, a.price) <= (b.tier, b.price));
    }
    assert!(!headers.is_empty());
    assert!(height > cells.iter().map(|(_, c)| c.bottom()).max().unwrap());
    // Cells never overlap.
    for (i, (_, a)) in cells.iter().enumerate() {
        for (_, b) in &cells[i + 1..] {
            assert!(a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y);
        }
    }
}

#[test]
fn scrolling_stops_at_the_end_of_the_grid() {
    let mut state = sample::state();
    assert_eq!(
        shop::max_scroll(&state.shop),
        (shop::grid(&state.shop.items).2 - 398).max(0)
    );
    state.shop.items.clear();
    assert_eq!(shop::max_scroll(&state.shop), 0);
}

#[test]
fn upgrade_ladders_and_recipes_get_their_own_rows() {
    let mut state = sample::state();
    let items = &mut state.shop.items;
    // 0 > 1 > 2 is a ladder; 3 + 4 > 5 is a two-part recipe.
    items.truncate(6);
    items[1].recipe = vec![0];
    items[2].recipe = vec![1];
    items[5].recipe = vec![3, 4];
    let grid = shop::layout(items);
    let labels: Vec<&str> = grid.headers.iter().map(|(label, _)| *label).collect();
    assert!(labels.contains(&"Upgrades") && labels.contains(&"Recipes"));
    let row_of = |index: usize| {
        grid.cells
            .iter()
            .filter(|(i, _)| *i == index)
            .map(|(_, cell)| cell.y)
            .collect::<Vec<_>>()
    };
    // The ladder sits on one row, left to right.
    let ladder: Vec<_> = [0, 1, 2]
        .iter()
        .map(|&i| {
            grid.cells
                .iter()
                .find(|(k, _)| *k == i)
                .map(|(_, cell)| *cell)
                .unwrap()
        })
        .collect();
    assert!(
        ladder
            .windows(2)
            .all(|pair| pair[0].y == pair[1].y && pair[0].x < pair[1].x)
    );
    // The recipe's parts are drawn again beside the result, with marks.
    assert_eq!(row_of(5).len(), 1);
    assert!(grid.marks.iter().any(|(mark, _)| *mark == '+'));
    assert!(grid.marks.iter().filter(|(mark, _)| *mark == '>').count() >= 3);
    // Every cell stays inside the view's width.
    for (_, cell) in &grid.cells {
        assert!(cell.x >= 0 && cell.right() <= 360, "{cell:?}");
    }
}

#[test]
fn a_click_on_a_bag_slot_names_its_inventory_slot() {
    let mut state = sample::state();
    state.shop.bag = vec![rift_ui::hud::state::BagItem {
        slot: 3,
        name: "Bronze sword".to_string(),
        icon: None,
        sell: 210,
        selected: false,
    }];
    let area = shop::rect(Rect::new(0, 0, 1920, 1080), 1);
    let slot = shop::bag_rects(area, 1, 1)[0];
    assert_eq!(
        shop::hit(area, 1, &state.shop, (slot.x + 5, slot.y + 5)),
        Some(ShopHit::Bag(3))
    );
}
