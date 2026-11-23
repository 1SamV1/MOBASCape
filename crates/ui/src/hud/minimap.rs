//! The minimap (docs/INTERFACE_STAGE3_PLAN.md, section 5.3): the stone frame,
//! the sample map picture, unit dots, structure rings, camps and the camera
//! box, with a small round-looking button at its top right.
//!
//! Stage 3 draws a made-up map from `sample.rs`. Stage 4 replaces the picture
//! and marks with what the viewer may see; the frame and placing stay here.

use crate::Colour;
use crate::draw::{DrawList, Rect};
use crate::hud::state::{MapPoint, MinimapState, StructureMark};
use crate::theme::Theme;

/// The component's size at the interface's own scale.
pub const SIZE: (i32, i32) = (266, 264);
const FRAME: i32 = 6;
const PICTURE: (i32, i32) = (254, 252);
const LAYER: i32 = 20;

/// Where a click lands on the map picture, as fractions of its width and
/// height (0 at the top left, 1 at the bottom right). `None` outside the
/// picture, including on the stone frame.
pub fn picture_at(area: Rect, scale: i32, mouse: (i32, i32)) -> Option<(f32, f32)> {
    let s = scale.max(1);
    let x = mouse.0 - (area.x + FRAME * s);
    let y = mouse.1 - (area.y + FRAME * s);
    let (w, h) = (PICTURE.0 * s, PICTURE.1 * s);
    if x < 0 || y < 0 || x >= w || y >= h {
        return None;
    }
    Some((x as f32 / w as f32, y as f32 / h as f32))
}

pub fn draw(theme: &Theme, list: &mut DrawList, area: Rect, state: &MinimapState) {
    let s = list.scale();
    let previous = list.set_layer(LAYER);
    list.push_clip(area);
    match theme.panel("panel.stone") {
        Some(panel) => list.panel(panel, area),
        None => list.rect(area, [0.05, 0.05, 0.07, 0.95]),
    }

    let map = Rect::new(
        area.x + FRAME * s,
        area.y + FRAME * s,
        PICTURE.0 * s,
        PICTURE.1 * s,
    );
    list.push_clip(map);
    list.rect(map, [0.035, 0.075, 0.055, 0.96]);
    // The live theme carries a terrain picture baked from the cache terrain
    // colours. The sample theme has no such role, so the dark map remains.
    if theme.has_sprite("minimap.terrain") {
        list.sprite(theme.sprite("minimap.terrain"), map.x, map.y);
    }

    let at = |point: MapPoint| (map.x + point.x * s, map.y + point.y * s);
    let line = |list: &mut DrawList, points: &[MapPoint], width: i32, colour: Colour| {
        for pair in points.windows(2) {
            let (ax, ay) = at(pair[0]);
            let (bx, by) = at(pair[1]);
            let steps = (bx - ax).abs().max((by - ay).abs()).max(1);
            for step in 0..=steps {
                let x = ax + (bx - ax) * step / steps;
                let y = ay + (by - ay) * step / steps;
                list.rect(
                    Rect::new(x - width * s / 2, y - width * s / 2, width * s, width * s),
                    colour,
                );
            }
        }
    };

    // The river and then the three lanes, so the lanes read over it.
    line(list, &state.river, 9, [0.07, 0.24, 0.36, 0.95]);
    for lane in &state.lanes {
        line(list, lane, 3, [0.52, 0.47, 0.29, 0.92]);
    }

    // Base crystals and structures as rings, in the team's colour.
    let team = |blue: bool| {
        if blue {
            theme.colour("text.team.blue")
        } else {
            theme.colour("text.team.red")
        }
    };
    for StructureMark { at: point, blue } in &state.bases {
        let (x, y) = at(*point);
        let ring = Rect::new(x - 8 * s, y - 8 * s, 16 * s, 16 * s);
        list.rect(ring, [0.02, 0.02, 0.02, 0.9]);
        list.outline(ring, 2 * s.max(1), team(*blue));
    }
    for StructureMark { at: point, blue } in &state.structures {
        let (x, y) = at(*point);
        let ring = Rect::new(x - 4 * s, y - 4 * s, 8 * s, 8 * s);
        list.outline(ring, s.max(1), team(*blue));
    }

    // Camps and units, the unit colours from the theme's map dots.
    let dot = |list: &mut DrawList, role: &str, point: MapPoint, fallback: Colour| {
        let (x, y) = at(point);
        if theme.has_sprite(role) {
            list.sprite(theme.sprite(role), x - 2 * s, y - 2 * s);
        } else {
            list.rect(Rect::new(x - 2 * s, y - 2 * s, 5 * s, 5 * s), fallback);
        }
    };
    for point in &state.camps {
        dot(list, "map.dot.camp", *point, [0.7, 1.0, 0.4, 1.0]);
    }
    for point in &state.allies {
        dot(list, "map.dot.ally", *point, theme.colour("text.team.blue"));
    }
    for point in &state.enemies {
        dot(list, "map.dot.enemy", *point, theme.colour("text.team.red"));
    }

    // The camera's box, a thin pale rectangle.
    let (cx, cy, cw, ch) = state.camera;
    let camera = Rect::new(map.x + cx * s, map.y + cy * s, cw * s, ch * s);
    list.outline(camera, s.max(1), [0.85, 0.9, 1.0, 0.55]);

    // The small button in the top right of the map.
    let button = Rect::new(map.right() - 20 * s, map.y + 4 * s, 16 * s, 16 * s);
    match theme.panel("panel.slot") {
        Some(panel) => list.panel(panel, button),
        None => list.rect(button, [0.03, 0.03, 0.03, 0.9]),
    }
    list.rect(
        Rect::new(button.x + 6 * s, button.y + 6 * s, 4 * s, 4 * s),
        [0.8, 0.8, 0.7, 1.0],
    );

    list.pop_clip();
    list.pop_clip();
    list.set_layer(previous);
}

#[cfg(test)]
mod hit_tests {
    use super::*;

    #[test]
    fn picture_fractions_skip_the_frame() {
        let area = Rect::new(10, 20, SIZE.0, SIZE.1);
        assert_eq!(
            picture_at(area, 1, (10 + FRAME, 20 + FRAME)),
            Some((0.0, 0.0))
        );
        let (u, v) = picture_at(area, 1, (10 + FRAME + 127, 20 + FRAME + 126)).unwrap();
        assert!((u - 0.5).abs() < 0.01 && (v - 0.5).abs() < 0.01);
        assert_eq!(picture_at(area, 1, (10 + 2, 20 + 2)), None, "stone frame");
        assert_eq!(
            picture_at(area, 1, (10 + SIZE.0 - 1, 20 + SIZE.1 - 1)),
            None
        );
    }
}
