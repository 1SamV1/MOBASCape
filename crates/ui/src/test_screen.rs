//! The screen that checks the drawing layer (docs/INTERFACE.md, stage 2):
//! one sprite at scales 1 to 4, a panel at three sizes, every font's test
//! line, see-through rectangles over the scene, a clipped panel, pieces
//! pinned to every corner and edge, and pieces hanging off the window.
//!
//! It asks the theme for roles and nothing else. It is not part of the
//! game's interface and none of it is kept by stage 3.

use crate::draw::{DrawList, Rect};
use crate::font::Font;
use crate::theme::{MISSING, Theme};
use crate::{Colour, scale};

/// What the test screen needs at its own size. The scale steps when the
/// window reaches a whole multiple of this.
pub const NEED: (u32, u32) = (640, 360);

/// The line every font is shown with.
pub const TEST_LINE: &str = "The quick brown fox 0123456789 Oh dear, you are dead!";

/// The roles the test screen asks a theme for.
pub const SPRITE: &str = "test.sprite";
pub const PANEL: &str = "panel";
pub const LABEL_FONT: &str = "font.body";
pub const TEXT: &str = "text";
pub const ANNOUNCEMENT: &str = "announcement";
pub const SHADE: &str = "shade";

const BACK: i32 = 0;
const PANELS: i32 = 1;
const WRITING: i32 = 2;

/// Build the test screen for a window of this size. `forced` overrides the
/// scale the window would pick.
pub fn build(theme: &Theme, window: (u32, u32), forced: Option<u32>) -> DrawList {
    let scale = forced
        .map(|s| s.clamp(1, scale::LARGEST))
        .unwrap_or_else(|| scale::choose(window, NEED));
    let mut list = DrawList::new(window, scale, &theme.atlas);
    let s = list.scale();
    let screen = list.screen();
    let text = theme.colour(TEXT);
    let loud = theme.colour(ANNOUNCEMENT);
    let label = theme.font(LABEL_FONT);
    // a place given at the layout's own size, from the top left
    let at = |x: i32, y: i32| (x * s, y * s);

    // a label, or a magenta bar where the theme has no font for it
    let write =
        |list: &mut DrawList, font: Option<&Font>, words: &str, x: i32, y: i32, c: Colour| {
            let before = list.set_layer(WRITING);
            match font {
                Some(font) => {
                    list.text(font, words, x, y, c);
                }
                None => list.rect(Rect::new(x, y - 8 * s, 60 * s, 8 * s), MISSING),
            }
            list.set_layer(before);
        };

    // 1. what this window is, top left
    let (x, y) = at(28, 16);
    write(
        &mut list,
        label,
        &format!("interface test  {} x {}  scale {}", window.0, window.1, s),
        x,
        y,
        loud,
    );

    // 2. one sprite at one, two, three and four times its size, whatever
    //    the window's scale is
    let sprite = theme.sprite(SPRITE);
    let (mut x, y) = at(8, 26);
    list.set_layer(PANELS);
    for times in 1..=4u32 {
        list.sprite_times(sprite, x, y, times, [1.0; 4]);
        x += sprite.w as i32 * times as i32 + 6 * s;
    }

    // 3. a panel at three sizes
    let (px, py) = at(8, 134);
    let sizes = [(40, 24), (110, 40), (190, 84)];
    let mut x = px;
    for (w, h) in sizes {
        let area = Rect::new(x, py, w * s, h * s);
        match theme.panel(PANEL) {
            Some(panel) => list.panel(panel, area),
            None => list.rect(area, MISSING),
        }
        write(
            &mut list,
            label,
            &format!("{w} x {h}"),
            x + 6 * s,
            py + 15 * s,
            text,
        );
        x += (w + 8) * s;
    }

    // 4. every font the theme has, each on a drawn baseline
    let (fx, mut fy) = at(250, 30);
    let roles: Vec<String> = theme.font_roles().map(str::to_string).collect();
    for role in &roles {
        let Some(font) = theme.font(role) else {
            continue;
        };
        fy += font.line_height.max(font.ascent + 2) * s;
        list.set_layer(PANELS);
        list.rect(
            Rect::new(fx, fy, font.width(TEST_LINE) * s, 1),
            [1.0, 0.25, 0.25, 0.6],
        );
        write(&mut list, Some(font), TEST_LINE, fx, fy, text);
        fy += 4 * s;
    }

    // 5. see-through rectangles over the scene. The red one is asked for
    //    first and put on the higher layer, so it must come out on top.
    let (zx, zy) = at(380, 196);
    list.set_layer(PANELS);
    list.rect(
        Rect::new(zx + 50 * s, zy + 20 * s, 80 * s, 36 * s),
        [0.9, 0.1, 0.1, 0.5],
    );
    list.set_layer(BACK);
    list.rect(Rect::new(zx, zy, 140 * s, 76 * s), theme.colour(SHADE));
    list.rect(
        Rect::new(zx + 10 * s, zy + 6 * s, 80 * s, 36 * s),
        [0.1, 0.2, 0.9, 0.5],
    );
    write(
        &mut list,
        label,
        "red over blue",
        zx + 6 * s,
        zy + 72 * s,
        text,
    );

    // 6. a panel and its writing cut off by a rectangle smaller than they
    //    are; the thin frame is where the cut is
    let (kx, ky) = at(380, 134);
    let cut = Rect::new(kx, ky, 110 * s, 50 * s);
    list.set_layer(PANELS);
    list.push_clip(cut);
    let big = Rect::new(kx - 20 * s, ky + 10 * s, 170 * s, 70 * s);
    match theme.panel(PANEL) {
        Some(panel) => list.panel(panel, big),
        None => list.rect(big, MISSING),
    }
    write(
        &mut list,
        label,
        "clipped at the frame, left and right",
        kx - 14 * s,
        ky + 30 * s,
        text,
    );
    list.pop_clip();
    list.set_layer(WRITING);
    list.outline(cut, 1, loud);

    // 7. a piece pinned to three corners and to the middle of the top and
    //    bottom edges; the writing at the top left is the fourth corner
    let (w, h) = (34 * s, 20 * s);
    let m = 4 * s;
    let pins = [
        ("TR", screen.w - w - m, m),
        ("BL", m, screen.h - h - m),
        ("BR", screen.w - w - m, screen.h - h - m),
        ("T", (screen.w - w) / 2, m),
        ("B", (screen.w - w) / 2, screen.h - h - m),
    ];
    for (name, x, y) in pins {
        let area = Rect::new(x, y, w, h);
        list.set_layer(PANELS);
        match theme.panel(PANEL) {
            Some(panel) => list.panel(panel, area),
            None => list.rect(area, MISSING),
        }
        write(&mut list, label, name, x + 8 * s, y + 14 * s, loud);
    }

    // 8. pieces hanging off the window: half of each is outside it
    list.set_layer(PANELS);
    let off_right = Rect::new(screen.w - 30 * s, (screen.h - 40 * s) / 2, 60 * s, 40 * s);
    let off_bottom = Rect::new(screen.w / 4, screen.h - 12 * s, 70 * s, 24 * s);
    let off_corner = Rect::new(-20 * s, -8 * s, 40 * s, 16 * s);
    for area in [off_right, off_bottom, off_corner] {
        match theme.panel(PANEL) {
            Some(panel) => list.panel(panel, area),
            None => list.rect(area, MISSING),
        }
    }
    list.sprite_times(
        sprite,
        screen.w - sprite.w as i32 * s / 2,
        screen.h / 4,
        s as u32,
        [1.0; 4],
    );

    list
}
