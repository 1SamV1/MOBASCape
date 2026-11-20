//! The drawing layer checked pixel by pixel, through the reference
//! rasteriser: what comes out is compared with what plain arithmetic says
//! each window pixel should be.

use rift_ui::atlas::{AtlasBuilder, Image, Region};
use rift_ui::draw::{DrawList, Rect};
use rift_ui::theme::{GlyphSource, Panel, Theme, ThemeBuilder};
use rift_ui::{raster, scale, test_screen, theme_sheet};

type Px = [u8; 4];
const GROUND: Px = [10, 20, 30, 255];

/// An image whose every pixel is different, so a wrong one is noticed.
fn numbered(width: usize, height: usize) -> Image {
    Image::new(
        width,
        height,
        (0..width * height)
            .map(|i| {
                [
                    (i % width * 9 + 20) as u8,
                    (i / width * 13 + 20) as u8,
                    (i * 7 % 200 + 40) as u8,
                    255,
                ]
            })
            .collect(),
    )
}

fn draw(list: &DrawList, atlas: &rift_ui::Atlas, width: usize, height: usize) -> Vec<Px> {
    let (vertices, indices) = list.finish();
    for vertex in &vertices {
        assert_eq!(vertex.pos[0].fract(), 0.0, "a corner off a whole pixel");
        assert_eq!(vertex.pos[1].fract(), 0.0, "a corner off a whole pixel");
    }
    let mut picture = vec![GROUND; width * height];
    raster::render(&vertices, &indices, atlas, width, height, &mut picture);
    picture
}

fn one_image(image: &Image) -> (rift_ui::Atlas, Region) {
    let mut builder = AtlasBuilder::new();
    let id = builder.add(image.clone());
    let (atlas, regions) = builder.build();
    (atlas, regions[id])
}

#[test]
fn a_sprite_at_each_scale_is_its_own_pixels_made_into_squares() {
    let image = numbered(7, 5);
    let (atlas, region) = one_image(&image);
    for s in 1..=4usize {
        let (w, h) = (60, 40);
        let mut list = DrawList::new((w as u32, h as u32), s as u32, &atlas);
        list.sprite(region, 3, 2);
        let picture = draw(&list, &atlas, w, h);
        for y in 0..h {
            for x in 0..w {
                let inside = x >= 3 && y >= 2 && x < 3 + 7 * s && y < 2 + 5 * s;
                let want = if inside {
                    image.pixels[(y - 2) / s * 7 + (x - 3) / s]
                } else {
                    GROUND
                };
                assert_eq!(picture[y * w + x], want, "scale {s}, pixel {x},{y}");
            }
        }
    }
}

#[test]
fn clipping_cuts_a_sprite_and_changes_nothing_that_is_left() {
    let image = numbered(9, 9);
    let (atlas, region) = one_image(&image);
    let (w, h) = (50usize, 50usize);
    for s in [1u32, 2, 3, 4] {
        let mut whole = DrawList::new((w as u32, h as u32), s, &atlas);
        whole.sprite(region, 5, 6);
        let whole = draw(&whole, &atlas, w, h);
        // a cut through the middle of pixels of the image, on every side
        let clip = Rect::new(8, 9, 17, 13);
        let mut cut = DrawList::new((w as u32, h as u32), s, &atlas);
        cut.push_clip(clip);
        cut.sprite(region, 5, 6);
        cut.pop_clip();
        cut.sprite(region, 40, 40); // after the clip is lifted, drawn whole
        let cut = draw(&cut, &atlas, w, h);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let at = (y as usize) * w + x as usize;
                let kept = x >= clip.x && x < clip.right() && y >= clip.y && y < clip.bottom();
                if kept {
                    assert_eq!(
                        cut[at], whole[at],
                        "scale {s}: changed inside the clip at {x},{y}"
                    );
                } else if x < 40 || y < 40 {
                    assert_eq!(
                        cut[at], GROUND,
                        "scale {s}: drawn outside the clip at {x},{y}"
                    );
                }
            }
        }
        assert_ne!(cut[41 * w + 41], GROUND, "the clip was lifted");
    }
}

#[test]
fn clips_inside_clips_keep_to_what_both_allow() {
    let (atlas, _) = one_image(&numbered(2, 2));
    let mut list = DrawList::new((40, 40), 1, &atlas);
    list.push_clip(Rect::new(5, 5, 20, 20));
    list.push_clip(Rect::new(15, 0, 30, 12));
    list.rect(Rect::new(0, 0, 40, 40), [1.0; 4]);
    list.pop_clip();
    list.pop_clip();
    let picture = draw(&list, &atlas, 40, 40);
    for y in 0..40 {
        for x in 0..40 {
            let inside = (15..25).contains(&x) && (5..12).contains(&y);
            assert_eq!(picture[y * 40 + x] != GROUND, inside, "{x},{y}");
        }
    }
}

#[test]
fn what_hangs_off_the_window_is_cut_at_its_edge_and_right_where_it_shows() {
    let image = numbered(8, 8);
    let (atlas, region) = one_image(&image);
    let s = 3usize;
    let (w, h) = (30usize, 30usize);
    for (x0, y0) in [
        (-12, 4),
        (20, 4),
        (4, -12),
        (4, 20),
        (-12, -12),
        (20, 20),
        (-100, 4),
        (4, 500),
    ] {
        let mut list = DrawList::new((w as u32, h as u32), s as u32, &atlas);
        list.sprite(region, x0, y0);
        let picture = draw(&list, &atlas, w, h);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let (ix, iy) = (x - x0, y - y0);
                let inside = ix >= 0 && iy >= 0 && ix < 8 * s as i32 && iy < 8 * s as i32;
                let want = if inside {
                    image.pixels[(iy as usize / s) * 8 + ix as usize / s]
                } else {
                    GROUND
                };
                assert_eq!(
                    picture[y as usize * w + x as usize],
                    want,
                    "at {x0},{y0}: pixel {x},{y}"
                );
            }
        }
    }
}

#[test]
fn layers_decide_what_is_on_top_and_asking_order_only_within_a_layer() {
    let (atlas, _) = one_image(&numbered(2, 2));
    let red = [1.0, 0.0, 0.0, 1.0];
    let blue = [0.0, 0.0, 1.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];
    let mut list = DrawList::new((10, 10), 1, &atlas);
    list.set_layer(5);
    list.rect(Rect::new(0, 0, 6, 6), red); // asked for first, on the higher layer
    list.set_layer(1);
    list.rect(Rect::new(0, 0, 10, 10), blue);
    list.rect(Rect::new(4, 4, 6, 6), green); // same layer as blue, later: over it
    let picture = draw(&list, &atlas, 10, 10);
    assert_eq!(picture[0], [255, 0, 0, 255], "red is on top of blue");
    assert_eq!(picture[5 * 10 + 5], [255, 0, 0, 255], "and of green");
    assert_eq!(picture[8 * 10 + 8], [0, 255, 0, 255], "green is over blue");
    assert_eq!(picture[9], [0, 0, 255, 255]);
}

#[test]
fn see_through_mixes_with_what_is_under_it() {
    let (atlas, _) = one_image(&numbered(2, 2));
    let mut list = DrawList::new((4, 1), 1, &atlas);
    list.rect(Rect::new(0, 0, 4, 1), [0.0, 0.0, 0.0, 1.0]);
    list.rect(Rect::new(1, 0, 1, 1), [1.0, 1.0, 1.0, 0.5]);
    list.rect(Rect::new(2, 0, 1, 1), [1.0, 1.0, 1.0, 0.0]);
    list.rect(Rect::new(3, 0, 1, 1), [1.0, 1.0, 1.0, 1.0]);
    let picture = draw(&list, &atlas, 4, 1);
    assert_eq!(picture[0], [0, 0, 0, 255]);
    assert_eq!(picture[1], [128, 128, 128, 255], "half white over black");
    assert_eq!(picture[2], [0, 0, 0, 255], "clear changes nothing");
    assert_eq!(picture[3], [255, 255, 255, 255]);
}

#[test]
fn an_image_with_clear_pixels_shows_the_scene_through_them() {
    let mut image = numbered(4, 4);
    image.pixels[5] = [200, 0, 0, 0];
    image.pixels[6] = [255, 255, 255, 128];
    let (atlas, region) = one_image(&image);
    let mut list = DrawList::new((8, 8), 2, &atlas);
    list.sprite(region, 0, 0);
    let picture = draw(&list, &atlas, 8, 8);
    // image pixel 5 is at column 1, row 1: window pixels 2..4, 2..4
    assert_eq!(picture[2 * 8 + 2], GROUND);
    assert_eq!(picture[3 * 8 + 3], GROUND);
    // pixel 6, half white over the ground
    assert_eq!(picture[2 * 8 + 4], [133, 138, 143, 255]);
}

/// What a panel should show at a window pixel, by arithmetic alone.
fn panel_pixel(
    image: &Image,
    insets: [usize; 4],
    area: (usize, usize),
    s: usize,
    x: usize,
    y: usize,
) -> Px {
    let [l, t, r, b] = insets;
    let along = |p: usize, size: usize, near: usize, far: usize, whole: usize| -> usize {
        // which pixel of the image, across or down
        let (near_px, far_px) = ((near * s).min(size - size / 2), (far * s).min(size / 2));
        if p < near_px {
            p / s
        } else if p >= size - far_px {
            // counted back from the far edge
            whole - 1 - (size - 1 - p) / s
        } else {
            let middle = whole - near - far;
            near + ((p - near_px) / s) % middle
        }
    };
    let ix = along(x, area.0, l, r, image.width);
    let iy = along(y, area.1, t, b, image.height);
    image.pixels[iy * image.width + ix]
}

#[test]
fn a_panel_keeps_its_corners_and_repeats_its_edges_and_middle_at_every_size() {
    let image = numbered(11, 9);
    let insets = [3usize, 2, 4, 3];
    let mut builder = ThemeBuilder::new();
    builder.panel("p", image.clone(), [3, 2, 4, 3]).unwrap();
    let theme = builder.build();
    let panel: Panel = *theme.panel("p").unwrap();
    for s in 1..=4usize {
        // sizes that are and are not whole multiples of the middle, one
        // barely bigger than its corners, and long thin ones
        for (w, h) in [
            (40, 30),
            (41, 31),
            (7 * s + 1, 5 * s + 1),
            (120, 6 * s),
            (8 * s, 90),
            (77, 53),
        ] {
            let (sw, sh) = (140usize, 110usize);
            let mut list = DrawList::new((sw as u32, sh as u32), s as u32, &theme.atlas);
            list.panel(&panel, Rect::new(6, 5, w as i32, h as i32));
            let picture = draw(&list, &theme.atlas, sw, sh);
            for y in 0..sh {
                for x in 0..sw {
                    let inside = x >= 6 && y >= 5 && x < 6 + w && y < 5 + h;
                    let want = if inside {
                        panel_pixel(&image, insets, (w, h), s, x - 6, y - 5)
                    } else {
                        GROUND
                    };
                    assert_eq!(
                        picture[y * sw + x],
                        want,
                        "scale {s}, panel {w}x{h}, pixel {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_panel_too_small_for_its_corners_still_draws_inside_its_area_only() {
    let mut builder = ThemeBuilder::new();
    builder.panel("p", numbered(12, 12), [5, 5, 5, 5]).unwrap();
    let theme = builder.build();
    let panel = *theme.panel("p").unwrap();
    for (w, h) in [(1, 1), (3, 9), (9, 3), (5, 5), (0, 7)] {
        let mut list = DrawList::new((30, 30), 2, &theme.atlas);
        list.panel(&panel, Rect::new(4, 4, w, h));
        let picture = draw(&list, &theme.atlas, 30, 30);
        for y in 0..30i32 {
            for x in 0..30i32 {
                let inside = x >= 4 && y >= 4 && x < 4 + w && y < 4 + h;
                assert_eq!(
                    picture[(y * 30 + x) as usize] != GROUND,
                    inside,
                    "{w}x{h} at {x},{y}"
                );
            }
        }
    }
}

#[test]
fn a_panel_whose_insets_leave_nothing_to_repeat_is_refused() {
    let mut builder = ThemeBuilder::new();
    assert!(builder.panel("p", numbered(8, 8), [4, 2, 4, 2]).is_err());
    assert!(builder.panel("p", numbered(8, 8), [2, 4, 2, 4]).is_err());
    assert!(builder.panel("p", numbered(8, 8), [3, 3, 4, 4]).is_ok());
}

/// A font of solid boxes: each letter `code % 3 + 2` wide and 6 tall, one in
/// from the pen and two down from the top of the line; a g hangs lower.
fn boxes() -> Vec<GlyphSource> {
    (32u8..127)
        .map(|code| {
            let c = code as char;
            let w = (code % 3 + 2) as usize;
            GlyphSource {
                character: c,
                image: (c != ' ').then(|| Image::filled(w, 6, [0, 0, 0, 255])),
                left: 1,
                top: if c == 'g' { 5 } else { 2 },
                advance: w as i32 + 2,
            }
        })
        .collect()
}

#[test]
fn text_stands_on_its_baseline_takes_its_colour_and_moves_on_by_its_advances() {
    let mut builder = ThemeBuilder::new();
    builder.font("f", boxes(), 8, 12);
    let theme = builder.build();
    let font = theme.font("f").unwrap();
    for s in 1..=4i32 {
        let (w, h) = (120usize, 60usize);
        let mut list = DrawList::new((w as u32, h as u32), s as u32, &theme.atlas);
        let (x0, baseline) = (5, 40);
        let wide = list.text(font, "Ag i", x0, baseline, [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(wide, font.width("Ag i") * s);
        let picture = draw(&list, &theme.atlas, w, h);
        let mut want = vec![GROUND; w * h];
        let mut pen = x0;
        for c in "Ag i".chars() {
            let code = c as u8;
            let gw = (code % 3 + 2) as i32;
            if c != ' ' {
                let top = baseline - 8 * s + if c == 'g' { 5 } else { 2 } * s;
                for y in top..top + 6 * s {
                    for x in pen + s..pen + s + gw * s {
                        want[y as usize * w + x as usize] = [255, 255, 0, 255];
                    }
                }
            }
            pen += (gw + 2) * s;
        }
        assert!(picture == want, "scale {s}");
        // the capital stands exactly on the baseline: lit above it, not on it
        assert_eq!(
            picture[(baseline - 1) as usize * w + (x0 + s) as usize],
            [255, 255, 0, 255]
        );
        assert_eq!(picture[baseline as usize * w + (x0 + s) as usize], GROUND);
    }
}

fn test_theme(with_everything: bool) -> Theme {
    let mut builder = ThemeBuilder::new();
    if with_everything {
        builder.sprite(test_screen::SPRITE, numbered(20, 16));
        builder
            .panel(test_screen::PANEL, numbered(12, 12), [4, 4, 4, 4])
            .unwrap();
        builder.font(test_screen::LABEL_FONT, boxes(), 8, 12);
        builder.font("font.title", boxes(), 8, 14);
        builder.colour(test_screen::TEXT, [1.0, 1.0, 1.0, 1.0]);
        builder.colour(test_screen::ANNOUNCEMENT, [1.0, 1.0, 0.0, 1.0]);
        builder.colour(test_screen::SHADE, [0.0, 0.0, 0.0, 0.5]);
    }
    builder.build()
}

#[test]
fn the_test_screen_builds_at_any_window_size_on_whole_pixels_inside_the_window() {
    for theme in [test_theme(true), test_theme(false)] {
        for window in [
            (640, 360),
            (800, 600),
            (1280, 720),
            (1281, 721),
            (1366, 768),
            (1920, 1080),
            (2560, 1440),
            (3840, 2160),
            (333, 211),
            (1, 1),
        ] {
            let list = test_screen::build(&theme, window, None);
            assert_eq!(
                list.scale() as u32,
                scale::choose(window, test_screen::NEED)
            );
            let (vertices, indices) = list.finish();
            assert_eq!(indices.len() % 6, 0);
            for vertex in &vertices {
                assert_eq!(vertex.pos[0].fract(), 0.0);
                assert_eq!(vertex.pos[1].fract(), 0.0);
                assert!(
                    vertex.pos[0] >= 0.0 && vertex.pos[0] <= window.0 as f32,
                    "{window:?}"
                );
                assert!(
                    vertex.pos[1] >= 0.0 && vertex.pos[1] <= window.1 as f32,
                    "{window:?}"
                );
            }
        }
    }
}

#[test]
fn the_test_screens_pins_stay_the_same_distance_from_their_edges_as_the_window_grows() {
    let theme = test_theme(true);
    // at one scale, growing the window moves the bottom right pin by exactly
    // what the window grew, and leaves the top left writing where it was
    let seen = |window: (u32, u32)| {
        let (vertices, _) = test_screen::build(&theme, window, Some(2)).finish();
        let right = vertices
            .iter()
            .map(|v| v.pos[0] as i32)
            .filter(|&x| x < window.0 as i32)
            .max()
            .unwrap();
        // the row of sprites near the top left, and nothing else, is here
        let sprites: Vec<[i32; 2]> = vertices
            .iter()
            .map(|v| [v.pos[0] as i32, v.pos[1] as i32])
            .filter(|p| p[0] < 400 && (52..=116).contains(&p[1]))
            .collect();
        (right, sprites)
    };
    let (right_a, sprites_a) = seen((1300, 760));
    let (right_b, sprites_b) = seen((1437, 911));
    assert_eq!(right_b - right_a, 137);
    assert_eq!(sprites_a.len(), 16, "four sprites, four corners each");
    assert_eq!(sprites_a, sprites_b);
}

#[test]
fn a_theme_without_a_role_draws_a_loud_stand_in_and_does_not_fail() {
    let theme = test_theme(false);
    assert!(!theme.has_sprite("anything"));
    assert_eq!(theme.sprite("anything"), theme.missing());
    assert!(theme.panel("anything").is_none() && theme.font("anything").is_none());
    assert_eq!(theme.colour("anything"), [1.0, 0.0, 1.0, 1.0]);
    let list = test_screen::build(&theme, (1280, 720), None);
    assert!(!list.is_empty());
}

#[test]
fn two_themes_with_the_same_roles_draw_the_same_layout() {
    // the same roles made of different pictures: every quad lands in the
    // same place on the window; only what it reads from the atlas differs
    let a = test_theme(true);
    let mut builder = ThemeBuilder::new();
    builder.sprite(test_screen::SPRITE, Image::filled(20, 16, [1, 2, 3, 255]));
    builder
        .panel(
            test_screen::PANEL,
            Image::filled(12, 12, [9, 9, 9, 255]),
            [4, 4, 4, 4],
        )
        .unwrap();
    builder.font(test_screen::LABEL_FONT, boxes(), 8, 12);
    builder.font("font.title", boxes(), 8, 14);
    builder.sprite("something.else", Image::filled(50, 50, [0, 0, 0, 255]));
    let b = builder.build();
    let places = |theme: &Theme| -> Vec<[i32; 2]> {
        test_screen::build(theme, (1280, 720), None)
            .finish()
            .0
            .iter()
            .map(|v| [v.pos[0] as i32, v.pos[1] as i32])
            .collect()
    };
    assert_eq!(places(&a), places(&b));
}

#[test]
fn the_theme_sheet_builds_on_whole_pixels_inside_the_window() {
    let theme = test_theme(true);
    for window in [
        (640, 360),
        (1280, 720),
        (1366, 768),
        (1600, 900),
        (1920, 1080),
        (2560, 1440),
        (3840, 2160),
        (333, 211),
        (1, 1),
    ] {
        let list = theme_sheet::build(&theme, window, None);
        assert_eq!(
            list.scale() as u32,
            scale::choose(window, theme_sheet::NEED)
        );
        let (vertices, indices) = list.finish();
        assert!(!vertices.is_empty(), "{window:?}");
        assert_eq!(indices.len() % 6, 0);
        for vertex in &vertices {
            assert_eq!(vertex.pos[0].fract(), 0.0, "{window:?}");
            assert_eq!(vertex.pos[1].fract(), 0.0, "{window:?}");
            assert!(
                vertex.pos[0] >= 0.0 && vertex.pos[0] <= window.0 as f32,
                "{window:?}"
            );
            assert!(
                vertex.pos[1] >= 0.0 && vertex.pos[1] <= window.1 as f32,
                "{window:?}"
            );
        }
    }
}

#[test]
fn the_theme_sheet_shows_every_role_the_theme_has() {
    let theme = test_theme(true);
    let expected = theme.sprite_roles().count()
        + theme.panel_roles().count()
        + theme.font_roles().count()
        + theme.colour_roles().count();
    let list = theme_sheet::build(&theme, (1920, 1080), Some(2));
    // A name, a picture or swatch and a sample for each role is more than
    // one quad each; the count is a floor, not an exact shape.
    assert!(
        list.len() > expected,
        "{} roles, {} quads",
        expected,
        list.len()
    );
}
