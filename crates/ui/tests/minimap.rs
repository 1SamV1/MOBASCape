//! The minimap checked as rectangles and as pixels: it stays inside the
//! place layout.rs gave it, draws its frame, lanes, marks and camera box, and
//! touches nothing outside.

use rift_ui::atlas::Image;
use rift_ui::draw::{DrawList, Rect};
use rift_ui::hud::{minimap, sample};
use rift_ui::raster;
use rift_ui::theme::{Theme, ThemeBuilder};

const GROUND: [u8; 4] = [8, 10, 16, 255];

fn numbered(width: usize, height: usize) -> Image {
    Image::new(
        width,
        height,
        (0..width * height)
            .map(|i| [(i % width) as u8, (i / width) as u8, 90, 255])
            .collect(),
    )
}

fn theme() -> Theme {
    let mut builder = ThemeBuilder::new();
    builder
        .panel("panel.stone", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    builder
        .panel("panel.slot", numbered(20, 20), [5, 5, 5, 5])
        .unwrap();
    for role in ["map.dot.ally", "map.dot.enemy", "map.dot.camp"] {
        builder.sprite(role, Image::filled(4, 5, [20, 200, 30, 255]));
    }
    builder.colour("text.team.blue", [0.4, 0.7, 1.0, 1.0]);
    builder.colour("text.team.red", [1.0, 0.4, 0.4, 1.0]);
    builder.build()
}

#[test]
fn the_minimap_stays_inside_its_rectangle_at_every_scale() {
    let theme = theme();
    let state = sample::state();
    for s in 1..=4u32 {
        let area = Rect::new(
            10,
            10,
            minimap::SIZE.0 * s as i32,
            minimap::SIZE.1 * s as i32,
        );
        let mut list = DrawList::new((1200, 1200), s, &theme.atlas);
        minimap::draw(&theme, &mut list, area, &state.minimap);
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
    let area = Rect::new(30, 20, minimap::SIZE.0, minimap::SIZE.1);
    let mut list = DrawList::new((400, 340), 1, &theme.atlas);
    minimap::draw(&theme, &mut list, area, &state.minimap);
    let (vertices, indices) = list.finish();
    let mut picture = vec![GROUND; 400 * 340];
    raster::render(&vertices, &indices, &theme.atlas, 400, 340, &mut picture);
    for y in 0..340 {
        for x in 0..400 {
            let inside = x >= area.x && x < area.right() && y >= area.y && y < area.bottom();
            if !inside {
                assert_eq!(
                    picture[y as usize * 400 + x as usize],
                    GROUND,
                    "pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn a_theme_with_no_panels_or_sprites_still_draws_the_map_fallback() {
    let mut builder = ThemeBuilder::new();
    builder.colour("text.team.blue", [0.4, 0.7, 1.0, 1.0]);
    builder.colour("text.team.red", [1.0, 0.4, 0.4, 1.0]);
    let theme = builder.build();
    let state = sample::state();
    let area = Rect::new(0, 0, minimap::SIZE.0, minimap::SIZE.1);
    let mut list = DrawList::new(
        (minimap::SIZE.0 as u32, minimap::SIZE.1 as u32),
        1,
        &theme.atlas,
    );
    minimap::draw(&theme, &mut list, area, &state.minimap);
    assert!(!list.is_empty());
}
