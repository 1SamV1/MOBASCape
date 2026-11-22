//! The inventory checked as rectangles and pixels: six cells, the gold line
//! and the Shop button stay inside 144 by 160 at scales 1 to 4, nothing is
//! drawn outside, and a theme with only fonts still draws the fallback.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{inventory, sample};
use rift_ui::raster;
use rift_ui::theme::{GlyphSource, Theme, ThemeBuilder};

const GROUND: [u8; 4] = [8, 11, 17, 255];

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
    builder
        .panel("panel.stone", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder
        .panel("panel.slot", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder
        .panel("button.stone", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder.sprite("icon.gold", numbered(10, 10));
    builder.font("font.small", boxes(), 8, 10);
    builder.font("font.body", boxes(), 10, 12);
    builder.font("font.bold", boxes(), 10, 12);
    for role in ["text", "text.quiet", "announcement"] {
        builder.colour(role, [1.0; 4]);
    }
    builder.build()
}

#[test]
fn the_inventory_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(
            10,
            10,
            inventory::SIZE.0 * s as i32,
            inventory::SIZE.1 * s as i32,
        );
        let mut list = DrawList::new((800, 800), s, &theme.atlas);
        inventory::draw(&theme, &mut list, area, &state.inventory, state.gold);
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty() && indices.len() % 6 == 0);
        for vertex in vertices {
            assert!(vertex.pos[0] >= area.x as f32 && vertex.pos[0] <= area.right() as f32);
            assert!(vertex.pos[1] >= area.y as f32 && vertex.pos[1] <= area.bottom() as f32);
        }
    }
}

#[test]
fn nothing_is_drawn_outside_the_rectangle() {
    let theme = theme();
    let state = sample::state();
    let area = Rect::new(20, 20, inventory::SIZE.0, inventory::SIZE.1);
    let mut list = DrawList::new((220, 220), 1, &theme.atlas);
    inventory::draw(&theme, &mut list, area, &state.inventory, state.gold);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 220 * 220];
    raster::render(&vertices, &indices, &theme.atlas, 220, 220, &mut picture);
    for y in 0..220 {
        for x in 0..220 {
            let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
            if !inside {
                assert_eq!(
                    picture[y as usize * 220 + x as usize],
                    GROUND,
                    "pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn the_shop_button_sits_inside_the_inventory_at_every_scale() {
    for s in 1..=4i32 {
        let area = Rect::new(10, 20, inventory::SIZE.0 * s, inventory::SIZE.1 * s);
        let button = inventory::shop_button(area, s);
        assert_eq!(button.x, area.x + 8 * s);
        assert_eq!(button.y, area.y + 128 * s);
        assert_eq!((button.w, button.h), (128 * s, 24 * s));
        assert!(button.right() <= area.right());
        assert!(button.bottom() <= area.bottom());
    }
    let area = Rect::new(10, 20, 144, 160);
    assert_eq!(inventory::shop_button(area, 1), Rect::new(18, 148, 128, 24));
    assert!(inventory::shop_button_at(area, 1, (82, 160)));
    assert!(!inventory::shop_button_at(area, 1, (17, 160)));
    assert!(!inventory::shop_button_at(area, 1, (82, 172)));
}

#[test]
fn a_theme_with_only_fonts_still_draws_the_inventory() {
    let mut builder = ThemeBuilder::new();
    builder.font("font.small", boxes(), 8, 10);
    builder.colour("text", [1.0; 4]);
    let theme = builder.build();
    let state = sample::state();
    let area = Rect::new(0, 0, inventory::SIZE.0, inventory::SIZE.1);
    let mut list = DrawList::new(
        (inventory::SIZE.0 as u32, inventory::SIZE.1 as u32),
        1,
        &theme.atlas,
    );
    inventory::draw(&theme, &mut list, area, &state.inventory, state.gold);
    assert!(!list.is_empty());
}
