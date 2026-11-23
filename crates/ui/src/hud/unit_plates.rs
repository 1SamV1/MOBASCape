//! Unit plates (docs/INTERFACE_STAGE3_PLAN.md, section 5.6): a hero's name,
//! level and health bar, a jungle camp's name and buff-coloured bar, a
//! minion's thin bar and a structure's wider one.
//!
//! They are not placed by the window and take no part in the layout, the
//! clash test or the budget. The client's projection decides where each
//! unit's head falls and gives this module a list of plates already placed in
//! window pixels; drawing starts there and goes up. In stage 3 the sample
//! supplies those pixels.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::{PlateKind, UnitPlate};
use crate::theme::Theme;

const LAYER: i32 = 0;
const SKULL_SIZE: i32 = 25;
/// Hero names are drawn at twice the interface font size, matching the
/// legacy HUD's larger champion names.
const HERO_NAME_TIMES: i32 = 2;
/// Baselines measured up from the plate anchor.
const HERO_NAME_BASELINE: i32 = 16;
const TAG_BASELINE: i32 = 52;
const SKULL_TOP: i32 = 78;

/// The room a plate may use, from its anchor upward. A test and the artist
/// can both read it, and the drawing is clipped to it.
pub fn bounds(plate: &UnitPlate, scale: i32) -> Rect {
    let s = scale.max(1);
    let (x, y) = plate.at;
    match plate.kind {
        // Room for the 2x name, the NO SAFE tag and the PK skull.
        PlateKind::Hero => Rect::new(x - 110 * s, y - 90 * s, 220 * s, 90 * s),
        PlateKind::Camp => Rect::new(x - 64 * s, y - 30 * s, 128 * s, 30 * s),
        PlateKind::Minion => Rect::new(x - 20 * s, y - 10 * s, 40 * s, 10 * s),
        PlateKind::Structure => Rect::new(x - 64 * s, y - 16 * s, 128 * s, 16 * s),
    }
}

fn colour(rgb: [u8; 3]) -> Colour {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ]
}

pub fn draw(theme: &Theme, list: &mut DrawList, plates: &[UnitPlate]) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    let back = theme.colour("bar.back");
    let team_blue = theme.colour("text.team.blue");
    let team_red = theme.colour("text.team.red");
    let health = theme.colour("bar.health");
    let enemy = theme.colour("bar.enemy");
    let tag_colour = if theme.has_colour("text.overhead.tag") {
        theme.colour("text.overhead.tag")
    } else {
        theme.colour("announcement")
    };
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);
    let tag_font = theme.font("font.bold").or(small);

    for plate in plates {
        let area = bounds(plate, s);
        list.push_clip(area);
        let (x, y) = plate.at;
        let team = if plate.blue { team_blue } else { team_red };
        let name_colour = plate.name_colour.map_or(team, colour);

        match plate.kind {
            PlateKind::Hero => {
                let left = x - 47 * s;
                let bar = Rect::new(left + 14 * s, y - 12 * s, 80 * s, 8 * s);
                let fill = if plate.blue { health } else { enemy };
                draw_bar(list, bar, plate, fill, back);

                // Level chip rides to the right of the bar.
                let level_box = Rect::new(left + 96 * s, y - 14 * s, 12 * s, 12 * s);
                draw_level(theme, list, level_box, plate.level, s);

                if let Some(font) = body {
                    let times = (HERO_NAME_TIMES * s) as u32;
                    let width = font.width(&plate.name) * times as i32;
                    list.text_times(
                        font,
                        &plate.name,
                        x - width / 2,
                        y - HERO_NAME_BASELINE * s,
                        times,
                        name_colour,
                    );
                }
                if let Some(tag) = plate.tag.as_deref() {
                    if let Some(font) = tag_font {
                        let width = font.width(tag) * s;
                        list.text(font, tag, x - width / 2, y - TAG_BASELINE * s, tag_colour);
                    }
                }
                if plate.pk {
                    draw_skull(theme, list, x, y, s);
                }
            }
            PlateKind::Camp => {
                let bar = Rect::new(x - 48 * s, y - 8 * s, 96 * s, 6 * s);
                let fill = if plate.blue { team_blue } else { enemy };
                draw_bar(list, bar, plate, fill, back);
                let level_box = Rect::new(x + 50 * s, y - 10 * s, 12 * s, 12 * s);
                draw_level(theme, list, level_box, plate.level, s);
                if let Some(font) = body {
                    let width = font.width(&plate.name) * s;
                    list.text(font, &plate.name, x - width / 2, y - 16 * s, name_colour);
                }
            }
            PlateKind::Minion => {
                let bar = Rect::new(x - 14 * s, y - 7 * s, 28 * s, 3 * s);
                let fill = if plate.blue { team_blue } else { enemy };
                draw_bar(list, bar, plate, fill, back);
            }
            PlateKind::Structure => {
                let bar = Rect::new(x - 60 * s, y - 12 * s, 120 * s, 8 * s);
                let fill = if plate.blue { team_blue } else { enemy };
                draw_bar(list, bar, plate, fill, back);
            }
        }
        list.pop_clip();
    }

    list.set_layer(previous);
}

fn draw_level(theme: &Theme, list: &mut DrawList, box_rect: Rect, level: u8, s: i32) {
    match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, box_rect),
        None => list.rect(box_rect, [0.08, 0.07, 0.06, 0.95]),
    }
    let text = if theme.has_colour("text.level") {
        theme.colour("text.level")
    } else {
        theme.colour("text")
    };
    if let Some(font) = theme.font("font.small") {
        let level = level.to_string();
        let width = font.width(&level) * s;
        list.text(
            font,
            &level,
            box_rect.x + (box_rect.w - width) / 2,
            box_rect.y + (box_rect.h + font.ascent * s) / 2,
            text,
        );
    }
}

fn draw_skull(theme: &Theme, list: &mut DrawList, x: i32, y: i32, s: i32) {
    if !theme.has_sprite("icon.skull") {
        return;
    }
    let size = SKULL_SIZE * s;
    list.sprite(theme.sprite("icon.skull"), x - size / 2, y - SKULL_TOP * s);
}

fn draw_bar(list: &mut DrawList, bar: Rect, plate: &UnitPlate, fill: Colour, back: Colour) {
    list.rect(bar, back);
    let fraction = if plate.max_health == 0 {
        0.0
    } else {
        (plate.health as f32 / plate.max_health as f32).clamp(0.0, 1.0)
    };
    list.rect(
        Rect::new(bar.x, bar.y, (bar.w as f32 * fraction) as i32, bar.h),
        fill,
    );
}
