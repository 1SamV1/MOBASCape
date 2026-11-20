//! The theme sheet for stage 3 step A: every role the interface can ask a
//! theme for, shown by name, and the pieces together in one panel.
//!
//! It is evidence for the theme, not part of the game's interface. The
//! assembled stone frame is shown over a bright ground, so a corner that
//! lost its transparency is seen. A missing role is drawn in magenta by the
//! theme itself, which is the point: a hole is seen and not guessed round.

use crate::draw::{DrawList, Rect};
use crate::theme::Theme;
use crate::{Colour, scale};

/// The sheet's own size. The scale steps when the window reaches a whole
/// multiple of it.
pub const NEED: (u32, u32) = (900, 500);

const BACK: i32 = 0;
const PANELS: i32 = 1;
const WRITING: i32 = 2;

const SAMPLE: &str = "The quick brown fox 0123456789";
const OUTLINED: &str = "Giant Mole has appeared!";

/// Build the theme sheet for a window of this size. `forced` overrides the
/// scale the window would pick.
pub fn build(theme: &Theme, window: (u32, u32), forced: Option<u32>) -> DrawList {
    let scale = forced
        .map(|s| s.clamp(1, scale::LARGEST))
        .unwrap_or_else(|| scale::choose(window, NEED));
    let mut list = DrawList::new(window, scale, &theme.atlas);
    let s = list.scale();
    let screen = list.screen();
    // The sheet is laid out at its own size and centred in the window.
    let ox = (screen.w - NEED.0 as i32 * s).max(0) / 2;
    let oy = (screen.h - NEED.1 as i32 * s).max(0) / 2;
    let at = |x: i32, y: i32| (ox + x * s, oy + y * s);

    let text = theme.colour("text");
    let quiet = theme.colour("text.quiet");
    let title = theme.colour("text.title");
    let good = theme.colour("text.good");
    let loud = theme.colour("announcement");
    let outline = theme.colour("text.outline");
    let shade = theme.colour("shade");

    list.set_layer(BACK);
    list.rect(screen, shade);

    // A label, or a magenta bar where the theme has no font for it.
    let write =
        |list: &mut DrawList, role: &str, words: &str, x: i32, baseline: i32, colour: Colour| {
            let before = list.set_layer(WRITING);
            match theme.font(role) {
                Some(font) => {
                    list.text(font, words, x, baseline, colour);
                }
                None => {
                    list.rect(
                        Rect::new(x, baseline - 8 * s, 60 * s, 8 * s),
                        [1.0, 0.0, 1.0, 1.0],
                    );
                }
            }
            list.set_layer(before);
        };
    let width = |role: &str, words: &str| -> i32 {
        theme
            .font(role)
            .map(|font| font.width(words) * s)
            .unwrap_or(60 * s)
    };
    let write_outlined =
        |list: &mut DrawList, role: &str, words: &str, x: i32, baseline: i32, colour: Colour| {
            let before = list.set_layer(WRITING);
            if let Some(font) = theme.font(role) {
                list.text(font, words, x + s, baseline + s, outline);
                list.text(font, words, x, baseline, colour);
            } else {
                list.rect(
                    Rect::new(x, baseline - 8 * s, 60 * s, 8 * s),
                    [1.0, 0.0, 1.0, 1.0],
                );
            }
            list.set_layer(before);
        };
    let panel = |list: &mut DrawList, role: &str, area: Rect| {
        let before = list.set_layer(PANELS);
        match theme.panel(role) {
            Some(panel) => list.panel(panel, area),
            None => list.rect(area, [1.0, 0.0, 1.0, 1.0]),
        }
        list.set_layer(before);
    };

    // 1. The title, and what the sheet is.
    let (x, y) = at(20, 28);
    write(
        &mut list,
        "font.bold",
        "MobaScape interface theme",
        x,
        y,
        title,
    );
    write(
        &mut list,
        "font.small",
        &format!(
            "stage 3 roles, drawn from the cache at {} x {} scale {}",
            window.0, window.1, s
        ),
        x,
        y + 18 * s,
        quiet,
    );

    // 2. Every panel and button, shown by role. The stone frames are over a
    //    bright ground so their clear corners show.
    write(
        &mut list,
        "font.bold",
        "PANELS AND BUTTONS",
        x,
        at(20, 66).1,
        title,
    );
    let panels: Vec<String> = theme.panel_roles().map(str::to_string).collect();
    for (n, role) in panels.iter().enumerate() {
        let col = n % 2;
        let row = n / 2;
        let (px, py) = at(20 + col as i32 * 210, 74 + row as i32 * 45);
        write(&mut list, "font.small", role, px, py + 8 * s, quiet);
        let area = Rect::new(px, py + 12 * s, 190 * s, 28 * s);
        list.set_layer(PANELS);
        list.rect(area, [1.0, 0.0, 1.0, 1.0]);
        panel(&mut list, role, area);
    }

    // 3. Every font, its name and the test line, on its own baseline.
    write(
        &mut list,
        "font.bold",
        "FONTS",
        at(470, 66).0,
        at(470, 66).1,
        title,
    );
    let fonts: Vec<String> = theme.font_roles().map(str::to_string).collect();
    for (n, role) in fonts.iter().enumerate() {
        let (fx, fy) = at(470, 90 + n as i32 * 22);
        write(&mut list, "font.small", role, fx, fy - 8 * s, quiet);
        write(&mut list, role, SAMPLE, fx + 60 * s, fy, text);
    }

    // 4. Every sprite, its name under it.
    write(
        &mut list,
        "font.bold",
        "SPRITES",
        at(470, 174).0,
        at(470, 174).1,
        title,
    );
    write(
        &mut list,
        "font.small",
        "icon. and map.dot. shortened",
        at(542, 174).0,
        at(542, 174).1,
        quiet,
    );
    let sprites: Vec<String> = theme.sprite_roles().map(str::to_string).collect();
    for (n, role) in sprites.iter().enumerate() {
        let col = n % 5;
        let row = n / 5;
        let (sx, sy) = at(470 + col as i32 * 82, 186 + row as i32 * 28);
        list.set_layer(PANELS);
        list.sprite(theme.sprite(role), sx, sy);
        let short = role.replace("icon.", "").replace("map.dot.", "map.");
        write(&mut list, "font.small", &short, sx, sy + 26 * s, quiet);
    }

    // 5. Every colour, as a swatch and its name.
    write(&mut list, "font.bold", "COLOURS", x, at(20, 272).1, title);
    let colours: Vec<String> = theme.colour_roles().map(str::to_string).collect();
    for (n, role) in colours.iter().enumerate() {
        let col = n % 2;
        let row = n / 2;
        let (cx, cy) = at(20 + col as i32 * 210, 284 + row as i32 * 18);
        list.set_layer(PANELS);
        list.rect(Rect::new(cx, cy, 12 * s, 12 * s), theme.colour(role));
        list.outline(Rect::new(cx, cy, 12 * s, 12 * s), 1, quiet);
        write(
            &mut list,
            "font.small",
            role,
            cx + 18 * s,
            cy + 11 * s,
            quiet,
        );
    }

    // 6. The pieces together: one stone panel holding each kind of writing,
    //    a button, a slot with a count, two bars with figures, and yellow
    //    text with its dark outline.
    let combined = at(470, 348);
    write(
        &mut list,
        "font.bold",
        "TOGETHER",
        combined.0,
        combined.1 - 5 * s,
        title,
    );
    let area = Rect::new(combined.0, combined.1, 410 * s, 138 * s);
    panel(&mut list, "panel.stone", area);

    let (bx, by) = at(482, 366);
    write(&mut list, "font.bold", "Objectives", bx, by, title);
    write(
        &mut list,
        "font.body",
        "Might 58s   allies 4/5   gold 1,240",
        bx,
        by + 16 * s,
        text,
    );

    // A button with its label, centred.
    let button = Rect::new(bx, by + 24 * s, 110 * s, 30 * s);
    panel(&mut list, "button.stone", button);
    let label = "Shop";
    let lx = button.x + (button.w - width("font.bold", label)) / 2;
    write(&mut list, "font.bold", label, lx, button.y + 20 * s, text);

    // A slot with a count over it.
    let slot = Rect::new(bx + 122 * s, by + 20 * s, 40 * s, 40 * s);
    panel(&mut list, "panel.slot", slot);
    write(
        &mut list,
        "font.small",
        "3",
        slot.x + 30 * s,
        slot.y + 37 * s,
        loud,
    );

    // Health and resource bars, each with figures on it.
    let bars = Rect::new(bx + 172 * s, by + 23 * s, 218 * s, 11 * s);
    list.set_layer(PANELS);
    list.rect(bars, theme.colour("bar.back"));
    list.rect(
        Rect::new(bars.x, bars.y, bars.w * 70 / 100, bars.h),
        theme.colour("bar.health"),
    );
    write(
        &mut list,
        "font.small",
        "612 / 730",
        bars.x + 66 * s,
        bars.y + 9 * s,
        text,
    );
    let resource = Rect::new(bars.x, bars.y + 15 * s, bars.w, 9 * s);
    list.set_layer(PANELS);
    list.rect(resource, theme.colour("bar.back"));
    list.rect(
        Rect::new(resource.x, resource.y, resource.w * 45 / 100, resource.h),
        theme.colour("bar.resource"),
    );
    write(
        &mut list,
        "font.small",
        "116 / 260",
        resource.x + 66 * s,
        resource.y + 8 * s,
        text,
    );

    // Good text, then the announcement with its outline, over the stone.
    write(&mut list, "font.body", "Alive", bx, by + 70 * s, good);
    write_outlined(
        &mut list,
        "font.title",
        OUTLINED,
        bx + 70 * s,
        by + 72 * s,
        loud,
    );

    // 7. A footer with the roles counted, so a sheet that lost one says so.
    let footer = format!(
        "{} panels   {} fonts   {} sprites   {} colours",
        panels.len(),
        fonts.len(),
        sprites.len(),
        colours.len()
    );
    let (fx, fy) = at(20, 492);
    write(&mut list, "font.small", &footer, fx, fy, quiet);

    list
}
