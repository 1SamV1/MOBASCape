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
/// The veil over ground the team cannot see.
/// Dark green, not navy: over the brown jungle floor a blue veil read
/// grey, while the reference's unseen jungle stays a deep green.
const VEIL: Colour = [0.03, 0.05, 0.04, 0.66];
/// The lighter veil just outside the team's sight.
const VEIL_EDGE: Colour = [0.03, 0.05, 0.04, 0.32];

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

    // The river as a soft band, then the three lanes as worn beige
    // roads over it, as the reference map paints them.
    line(list, &state.river, 7, [0.30, 0.52, 0.66, 0.42]);
    for lane in &state.lanes {
        line(list, lane, 6, [0.70, 0.64, 0.45, 0.92]);
    }

    // The team's live vision: unseen ground sits under a dark veil, so the
    // areas allies can see (and the shadows walls cast in their sight)
    // show in real time. Hidden cells touching seen ones take a lighter
    // veil, which softens the edge as the reference's sight circles are.
    // Runs of cells with the same veil merge into one rectangle.
    if let Some(vision) = &state.vision
        && vision.cols > 0
        && vision.rows > 0
        && vision.seen.len() == vision.cols * vision.rows
    {
        let (cols, rows) = (vision.cols as i32, vision.rows as i32);
        let seen = |c: i32, r: i32| {
            c >= 0 && r >= 0 && c < cols && r < rows && vision.seen[(r * cols + c) as usize]
        };
        // 0 seen, 1 hidden at the edge of sight, 2 hidden.
        let veil = |c: i32, r: i32| {
            if seen(c, r) {
                0
            } else if (-1..=1).any(|dr| (-1..=1).any(|dc| seen(c + dc, r + dr))) {
                1
            } else {
                2
            }
        };
        let cell = |i: i32, total: i32, span: i32| (i * span) / total;
        for row in 0..rows {
            let y0 = cell(row, rows, PICTURE.1);
            let y1 = cell(row + 1, rows, PICTURE.1);
            let mut col = 0;
            while col < cols {
                let kind = veil(col, row);
                let start = col;
                while col < cols && veil(col, row) == kind {
                    col += 1;
                }
                if kind == 0 {
                    continue;
                }
                let x0 = cell(start, cols, PICTURE.0);
                let x1 = cell(col, cols, PICTURE.0);
                list.rect(
                    Rect::new(map.x + x0 * s, map.y + y0 * s, (x1 - x0) * s, (y1 - y0) * s),
                    if kind == 1 { VEIL_EDGE } else { VEIL },
                );
            }
        }
    }

    // Base crystals and structures as rings, in the team's colour.
    let team = |blue: bool| {
        if blue {
            theme.colour("text.team.blue")
        } else {
            theme.colour("text.team.red")
        }
    };
    // Bases and towers as solid team squares with a white core, the way
    // the reference minimap marks buildings.
    let building = |list: &mut DrawList, x: i32, y: i32, half: i32, core: i32, colour: Colour| {
        let outer = Rect::new(
            x - half * s - s,
            y - half * s - s,
            (half * 2 + 2) * s,
            (half * 2 + 2) * s,
        );
        list.rect(outer, [0.02, 0.02, 0.02, 0.95]);
        list.rect(
            Rect::new(x - half * s, y - half * s, half * 2 * s, half * 2 * s),
            colour,
        );
        list.outline(
            Rect::new(x - core * s, y - core * s, core * 2 * s, core * 2 * s),
            s.max(1),
            [1.0, 1.0, 1.0, 0.95],
        );
    };
    for StructureMark { at: point, blue } in &state.bases {
        let (x, y) = at(*point);
        building(list, x, y, 7, 3, team(*blue));
    }
    for StructureMark { at: point, blue } in &state.structures {
        let (x, y) = at(*point);
        building(list, x, y, 4, 1, team(*blue));
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

    // Heroes: a dark disc inside a ring in the team colour (gold for the
    // player), drawn last so they sit over minions and camps.
    for hero in &state.heroes {
        // Keep the whole ring on the map: the fountains sit on its edge.
        let inset = 11;
        let (x, y) = at(MapPoint {
            x: hero.at.x.clamp(inset, PICTURE.0 - inset),
            y: hero.at.y.clamp(inset, PICTURE.1 - inset),
        });
        let colour = if hero.player {
            [1.0, 0.82, 0.25, 1.0]
        } else {
            team(hero.blue)
        };
        match hero.portrait.filter(|role| theme.has_sprite(role)) {
            Some(role) => {
                // The hero's face in a slightly larger ring.
                disc(list, x, y, 9 * s, [0.03, 0.03, 0.04, 0.95]);
                list.sprite_fit(
                    theme.sprite(role),
                    Rect::new(x - 8 * s, y - 8 * s, 16 * s, 16 * s),
                );
                ring(list, x, y, 10 * s, 2 * s.max(1), colour);
            }
            None => {
                disc(list, x, y, 6 * s, [0.03, 0.03, 0.04, 0.92]);
                ring(list, x, y, 7 * s, 2 * s.max(1), colour);
                disc(list, x, y, 3 * s, colour);
            }
        }
    }

    // Teleport destinations: two rings in the team colour, over the heroes
    // so a scroll toward a fight is never hidden.
    for ping in &state.pings {
        let (x, y) = at(ping.at);
        let colour = team(ping.blue);
        ring(list, x, y, 9 * s, s.max(1), colour);
        ring(list, x, y, 5 * s, s.max(1), colour);
    }

    // What the screen shows: its true ground footprint (a trapezoid under
    // the tilted camera) when the client gives one, else a rectangle.
    let view_colour = [0.92, 0.94, 1.0, 0.75];
    if let Some(corners) = state.view {
        let mut ring = corners.to_vec();
        ring.push(corners[0]);
        line(list, &ring, 1, view_colour);
    } else {
        let (cx, cy, cw, ch) = state.camera;
        let camera = Rect::new(map.x + cx * s, map.y + cy * s, cw * s, ch * s);
        list.outline(camera, s.max(1), view_colour);
    }

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

/// A filled disc as one rectangle per scanline.
fn disc(list: &mut DrawList, cx: i32, cy: i32, r: i32, colour: Colour) {
    for dy in -r..=r {
        let half = ((r * r - dy * dy) as f32).sqrt().round() as i32;
        list.rect(Rect::new(cx - half, cy + dy, half * 2 + 1, 1), colour);
    }
}

/// A ring `thick` pixels wide, outer radius `r`, as scanline spans.
fn ring(list: &mut DrawList, cx: i32, cy: i32, r: i32, thick: i32, colour: Colour) {
    let inner = (r - thick).max(0);
    for dy in -r..=r {
        let outer = ((r * r - dy * dy) as f32).sqrt().round() as i32;
        if dy.abs() > inner {
            list.rect(Rect::new(cx - outer, cy + dy, outer * 2 + 1, 1), colour);
            continue;
        }
        let hole = ((inner * inner - dy * dy) as f32).sqrt().round() as i32;
        list.rect(Rect::new(cx - outer, cy + dy, outer - hole, 1), colour);
        list.rect(Rect::new(cx + hole + 1, cy + dy, outer - hole, 1), colour);
    }
}

#[cfg(test)]
mod hit_tests {
    use super::*;
    use crate::hud::state::{HeroMark, VisionMask};

    fn empty_state() -> MinimapState {
        MinimapState {
            lanes: Vec::new(),
            river: Vec::new(),
            bases: Vec::new(),
            allies: Vec::new(),
            enemies: Vec::new(),
            camps: Vec::new(),
            structures: Vec::new(),
            heroes: Vec::new(),
            vision: None,
            camera: (0, 0, 10, 10),
            view: None,
            pings: Vec::new(),
        }
    }

    fn quads(state: &MinimapState) -> usize {
        let theme = crate::theme::ThemeBuilder::new().build();
        let mut list = DrawList::new((400, 400), 1, &theme.atlas);
        draw(&theme, &mut list, Rect::new(0, 0, SIZE.0, SIZE.1), state);
        list.len()
    }

    #[test]
    fn unseen_ground_is_veiled_in_merged_runs_with_a_soft_edge() {
        let lit = quads(&empty_state());
        let mut state = empty_state();
        // Two rows: the first fully hidden (one run), the second with a
        // seen cell in the middle (two runs).
        state.vision = Some(VisionMask {
            cols: 3,
            rows: 2,
            seen: vec![false, false, false, false, true, false],
        });
        // Every hidden cell touches the seen one, so all take the edge
        // veil: one run on the first row, two on the second.
        assert_eq!(quads(&state), lit + 3);
        // Far from any seen cell the full veil takes over as its own run.
        state.vision = Some(VisionMask {
            cols: 4,
            rows: 1,
            seen: vec![true, false, false, false],
        });
        assert_eq!(quads(&state), lit + 2);
        state.vision = Some(VisionMask {
            cols: 3,
            rows: 2,
            seen: vec![true; 6],
        });
        assert_eq!(quads(&state), lit, "fully seen draws no veil");
    }

    #[test]
    fn the_view_trapezoid_replaces_the_camera_box() {
        let boxed = quads(&empty_state());
        let mut state = empty_state();
        state.view = Some([
            MapPoint { x: 80, y: 60 },
            MapPoint { x: 180, y: 60 },
            MapPoint { x: 150, y: 120 },
            MapPoint { x: 110, y: 120 },
        ]);
        // Four stepped edges: many more one-pixel quads than the box's four.
        assert!(quads(&state) > boxed + 100);
    }

    #[test]
    fn heroes_draw_as_rings_over_the_map() {
        let before = quads(&empty_state());
        let mut state = empty_state();
        state.heroes.push(HeroMark {
            at: MapPoint { x: 120, y: 120 },
            blue: true,
            player: true,
            portrait: None,
        });
        assert!(quads(&state) > before + 10);
    }

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
