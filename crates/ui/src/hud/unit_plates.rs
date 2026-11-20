//! Unit plates (docs/INTERFACE_STAGE3_PLAN.md, section 5.6): a hero's name,
//! level and health bar, a minion's thin bar and a structure's wider one.
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
const HERO_NAME_HEIGHT: i32 = 20;

/// The room a plate may use, from its anchor upward. A test and the artist
/// can both read it, and the drawing is clipped to it.
pub fn bounds(plate: &UnitPlate, scale: i32) -> Rect {
    let s = scale.max(1);
    let (x, y) = plate.at;
    match plate.kind {
        PlateKind::Hero => Rect::new(x - 60 * s, y - 34 * s, 120 * s, 34 * s),
        PlateKind::Minion => Rect::new(x - 20 * s, y - 10 * s, 40 * s, 10 * s),
        PlateKind::Structure => Rect::new(x - 64 * s, y - 16 * s, 128 * s, 16 * s),
    }
}

pub fn draw(theme: &Theme, list: &mut DrawList, plates: &[UnitPlate]) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    let text = theme.colour("text");
    let back = theme.colour("bar.back");
    let team_blue = theme.colour("text.team.blue");
    let team_red = theme.colour("text.team.red");
    let health = theme.colour("bar.health");
    let enemy = theme.colour("bar.enemy");
    let small = theme.font("font.small");
    let body = theme.font("font.body").or(small);

    for plate in plates {
        let area = bounds(plate, s);
        list.push_clip(area);
        let (x, y) = plate.at;
        let team = if plate.blue { team_blue } else { team_red };

        match plate.kind {
            PlateKind::Hero => {
                // A level box and an 80 by 8 bar, 94 wide together and
                // centred on the unit; the name above them.
                let left = x - 47 * s;
                let level_box = Rect::new(left, y - 14 * s, 12 * s, 12 * s);
                match theme.panel("panel.slot") {
                    Some(panel) => list.panel(panel, level_box),
                    None => list.rect(level_box, [0.08, 0.07, 0.06, 0.95]),
                }
                if let Some(font) = small {
                    let level = plate.level.to_string();
                    let width = font.width(&level) * s;
                    list.text(
                        font,
                        &level,
                        level_box.x + (level_box.w - width) / 2,
                        level_box.y + (level_box.h + font.ascent * s) / 2,
                        text,
                    );
                }
                let bar = Rect::new(left + 14 * s, y - 12 * s, 80 * s, 8 * s);
                let fill = if plate.blue { health } else { enemy };
                draw_bar(list, bar, plate, fill, back);
                if let Some(font) = body {
                    let width = font.width(&plate.name) * s;
                    list.text(
                        font,
                        &plate.name,
                        x - width / 2,
                        y - HERO_NAME_HEIGHT * s,
                        team,
                    );
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
