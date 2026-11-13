//! What the map knows about height (docs/MOVEMENT.md, "Getting about on
//! rough ground", stage 1).
//!
//! The walkable map says only whether a cell is blocked. This says, for each
//! cell a shape stands over, **where a champion could stand** on it and
//! **over what stretches of height a body is in the way**. A cell holds as
//! many of each as the shape has there.
//!
//! This only builds and checks the data. Nothing that moves reads it yet:
//! what blocks a unit is still decided by [`Grid::block_solid`], unchanged.
//! [`agreement`] compares the two readings of the same shape.
//!
//! Heights are whole numbers, in sixteenths of a world unit above the ground
//! the shape stands on.

use crate::Grid;
use std::collections::{HashMap, HashSet, VecDeque};

/// Sixteenths of a world unit: the unit heights are kept in.
pub const Q: i32 = 16;

/// Stretches of solid closer together than this (in sixteenths) are joined.
const JOIN: i32 = 4 * Q;
/// Standing surfaces closer together than this are one surface.
const SAME_SURFACE: i32 = 16 * Q;

/// What stands over one cell of the map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Column {
    /// Heights at which a champion could stand here, lowest first. The
    /// ground itself is not listed: it is always a place to stand unless
    /// something solid is on it.
    pub surfaces: Vec<i32>,
    /// Stretches of height in which a body is in the way here, as
    /// (bottom, top), lowest first, never touching or overlapping.
    pub solid: Vec<(i32, i32)>,
}

impl Column {
    /// The tallest solid thing here, or 0.
    pub fn top(&self) -> i32 {
        self.solid.last().map(|s| s.1).unwrap_or(0)
    }

    /// Whether a body `tall` high standing at `level` meets anything solid
    /// here. Solid that ends lower than a `kerb` above that level does not
    /// count: it is low enough to be walked over by anyone.
    ///
    /// `level` is the level the champion is at (see [`Column::levels`]), not
    /// the height its feet are drawn at. A champion walking over a low stone
    /// is still at the level under the stone, and the next stone is measured
    /// from there too: low stones do not add up, in one cell or across
    /// several.
    pub fn in_the_way(&self, level: i32, tall: i32, kerb: i32) -> bool {
        self.solid
            .iter()
            .any(|&(bottom, top)| bottom < level + tall && top >= level + kerb)
    }

    /// The levels a champion can be at here, lowest first:
    ///
    /// - the ground, always;
    /// - each standing surface a `kerb` or more above the ground.
    ///
    /// Nothing lower than a kerb is a level. A low stone is walked over at
    /// the level beneath it; its top is where the feet are drawn, and that is
    /// all it is. Whether a body fits at a level is [`Column::in_the_way`].
    pub fn levels(&self, kerb: i32) -> Vec<i32> {
        let mut levels = vec![0];
        levels.extend(self.surfaces.iter().copied().filter(|&at| at >= kerb));
        levels
    }

    /// Whether a champion walking on the ground could be in this cell: a
    /// level within a `step` of the ground with room above it for a body
    /// `tall` high.
    pub fn open_from_the_ground(&self, step: i32, tall: i32, kerb: i32) -> bool {
        self.levels(kerb)
            .into_iter()
            .any(|at| at <= step && !self.in_the_way(at, tall, kerb))
    }
}

/// How a shape is to be read.
#[derive(Debug, Clone, Copy)]
pub struct Reading {
    /// The game's models are drawn mirrored (their own up is down), which
    /// turns every triangle inside out. True for anything drawn that way.
    pub mirrored: bool,
    /// The steepest a surface may be, in degrees from level, and still be
    /// stood on.
    pub standing_angle: f32,
    /// Nothing above this height is read at all, in world units: leaves do
    /// not stop anyone. `None` reads the whole shape.
    pub head: Option<f32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Facing {
    /// Faces upward and is level enough to stand on.
    Top,
    /// Faces downward.
    Underside,
    /// Too steep either way.
    Side,
}

/// Which way a triangle faces, from its own three corners and nothing else.
fn facing(tri: &[[f32; 3]; 3], reading: &Reading) -> Facing {
    let [a, b, c] = tri;
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let mut n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    if reading.mirrored {
        n = [-n[0], -n[1], -n[2]];
    }
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if len <= f32::EPSILON {
        return Facing::Side;
    }
    let up = n[1] / len;
    let level_enough = reading.standing_angle.to_radians().cos();
    if up >= level_enough {
        Facing::Top
    } else if up <= -level_enough {
        Facing::Underside
    } else {
        Facing::Side
    }
}

/// The height data for a map, or for one shape on a patch of one.
#[derive(Debug, Clone)]
pub struct HeightMap {
    width: i32,
    height: i32,
    cell: i32,
    columns: HashMap<(i32, i32), Column>,
}

impl HeightMap {
    pub fn new(width: usize, height: usize, cell: i32) -> Self {
        assert!(cell > 0, "a cell has a size");
        Self {
            width: width as i32,
            height: height as i32,
            cell,
            columns: HashMap::new(),
        }
    }

    /// What stands over a cell, if anything does.
    pub fn column(&self, cx: i32, cy: i32) -> Option<&Column> {
        self.columns.get(&(cx, cy))
    }

    /// How many cells have anything over them.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Every cell with something over it.
    pub fn cells(&self) -> impl Iterator<Item = (&(i32, i32), &Column)> {
        self.columns.iter()
    }

    /// Read a shape into the map. `triangles` are its surface as it is drawn:
    /// each corner is (across, height above the ground, along), in world
    /// units from `at`, the point on this map where it stands.
    ///
    /// The surface is sampled exactly as [`Grid::block_solid`] samples it, so
    /// the two see the same points.
    pub fn add_shape(&mut self, triangles: &[[[f32; 3]; 3]], at: (f32, f32), reading: &Reading) {
        // for each cell, for each triangle over it: which way it faces and
        // the lowest and highest it is there
        let mut seen: HashMap<(i32, i32), Vec<(Facing, i32, i32)>> = HashMap::new();
        let spacing = self.cell as f32 / 3.0;

        for tri in triangles {
            let [a, b, c] = tri;
            let kind = facing(tri, reading);
            let edge = |p: &[f32; 3], q: &[f32; 3]| {
                ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
            };
            let longest = edge(a, b).max(edge(b, c)).max(edge(c, a));
            let steps = ((longest / spacing).ceil() as i32).clamp(1, 400);
            let mut here: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
            for i in 0..=steps {
                for j in 0..=(steps - i) {
                    let (u, v) = (i as f32 / steps as f32, j as f32 / steps as f32);
                    let w = 1.0 - u - v;
                    let height = a[1] * w + b[1] * u + c[1] * v;
                    if height < 0.0 || reading.head.is_some_and(|head| height > head) {
                        continue;
                    }
                    let x = at.0 + a[0] * w + b[0] * u + c[0] * v;
                    let y = at.1 + a[2] * w + b[2] * u + c[2] * v;
                    let cell = (
                        (x.floor() as i32).div_euclid(self.cell),
                        (y.floor() as i32).div_euclid(self.cell),
                    );
                    if cell.0 < 0 || cell.1 < 0 || cell.0 >= self.width || cell.1 >= self.height {
                        continue;
                    }
                    // rounded down, so "at least so high" means the same in
                    // sixteenths as it did in the height it came from
                    let h = (height * Q as f32).floor() as i32;
                    let range = here.entry(cell).or_insert((h, h));
                    range.0 = range.0.min(h);
                    range.1 = range.1.max(h);
                }
            }
            for (cell, (low, high)) in here {
                seen.entry(cell).or_default().push((kind, low, high));
            }
        }

        for (cell, mut parts) in seen {
            // a fixed order, so the result cannot depend on the order the
            // triangles came in
            parts.sort_by_key(|&(kind, low, high)| (low, high, kind as u8));
            let mut column = self.columns.remove(&cell).unwrap_or_default();

            for &(kind, low, high) in &parts {
                match kind {
                    Facing::Side => column.solid.push((low, high)),
                    Facing::Top => {
                        // solid from this top down to the first underside
                        // beneath it, or to the ground
                        let bottom = parts
                            .iter()
                            .filter(|p| p.0 == Facing::Underside && p.2 <= low)
                            .map(|p| p.1)
                            .max()
                            .unwrap_or(0);
                        column.solid.push((bottom, high));
                        column.surfaces.push(high);
                    }
                    Facing::Underside => {
                        // The bottom of something. If its top was read, the
                        // top's stretch already reaches down to it. If not,
                        // all that is known is the underside itself, and that
                        // is all that is recorded: a leaf modelled as one
                        // sheet seen from below is a sheet, not the foot of a
                        // column. (An earlier version guessed that an
                        // underside with no top went on up to head height,
                        // and walled off the ground under every low frond.)
                        column.solid.push((low, high));
                    }
                }
            }

            column.solid.sort_unstable();
            let mut joined: Vec<(i32, i32)> = Vec::new();
            for (bottom, top) in column.solid.drain(..) {
                match joined.last_mut() {
                    Some(last) if bottom <= last.1 + JOIN => last.1 = last.1.max(top),
                    _ => joined.push((bottom, top)),
                }
            }
            column.solid = joined;

            column.surfaces.sort_unstable();
            let mut surfaces: Vec<i32> = Vec::new();
            for level in column.surfaces.drain(..) {
                match surfaces.last_mut() {
                    Some(last) if level <= *last + SAME_SURFACE => *last = level,
                    _ => surfaces.push(level),
                }
            }
            // a surface with something solid standing on it is not somewhere
            // to stand: keep only the tops of stretches and what is clear
            column.surfaces = surfaces
                .into_iter()
                .filter(|&level| !joined_inside(&column.solid, level))
                .collect();

            self.columns.insert(cell, column);
        }
    }

    /// The cells where a champion walking on the ground, with the given
    /// step, body height and kerb (world units), could not be, by this data
    /// alone: no room for a body is added round them.
    pub fn closed_from_the_ground(&self, step: f32, tall: f32, kerb: f32) -> Vec<(i32, i32)> {
        let q = |v: f32| (v * Q as f32) as i32;
        let mut closed: Vec<(i32, i32)> = self
            .columns
            .iter()
            .filter(|(_, column)| !column.open_from_the_ground(q(step), q(tall), q(kerb)))
            .map(|(&cell, _)| cell)
            .collect();
        closed.sort_unstable();
        closed
    }
}

impl HeightMap {
    /// Where a champion at `level` (above the ground, in sixteenths) would
    /// be if it stepped into the cell `to`, or `None` if it cannot go there.
    ///
    /// Of the levels in that cell ([`Column::levels`]), one is allowed if it
    /// is within `step` of the level stepped from and a body `tall` high at
    /// that level meets nothing solid, in that cell or in any cell with room
    /// to be kept from ([`HeightMap::room`]). Of several, the nearest in
    /// level is taken, and the higher of two equally near.
    ///
    /// Levels are heights above each cell's own ground. A hill is therefore
    /// never a step: the ground carries a champion up and down it freely, as
    /// it does today, and only what stands on the ground is stepped onto.
    ///
    /// Nothing that moves uses this rule yet.
    pub fn step_to(
        &self,
        level: i32,
        to: (i32, i32),
        step: i32,
        tall: i32,
        body: i32,
        kerb: i32,
    ) -> Option<i32> {
        let levels = match self.columns.get(&to) {
            Some(column) => column.levels(kerb),
            None => vec![0],
        };
        let room = self.room(body);
        levels
            .into_iter()
            .filter(|&at| (at - level).abs() <= step && self.fits(to, at, tall, kerb, &room))
            .min_by_key(|&at| ((at - level).abs(), -at))
    }

    /// The cells, as steps from a cell, that a body in that cell has to be
    /// clear of: the cell itself, and for a body with any width every cell
    /// whose middle is within the body's half-width plus half a cell of its
    /// own middle.
    ///
    /// This is the walkable map's rule ([`Grid::block_cells`]) turned round,
    /// so the two agree about how near a thing a body may be. With a body of
    /// 28 and cells of 32 it is the four cells sharing a side, 32 away, and
    /// not the four at the corners, 45 away. Measured from the middle alone,
    /// with no half cell, a body narrower than a cell would be kept from
    /// nothing at all, which is what this function did when it was first
    /// written.
    pub fn room(&self, body: i32) -> Vec<(i32, i32)> {
        let mut room = vec![(0, 0)];
        if body <= 0 {
            return room;
        }
        let reach = body + self.cell / 2;
        let cells = reach / self.cell + 1;
        for dy in -cells..=cells {
            for dx in -cells..=cells {
                let (wx, wy) = (dx * self.cell, dy * self.cell);
                if (dx, dy) != (0, 0) && wx * wx + wy * wy <= reach * reach {
                    room.push((dx, dy));
                }
            }
        }
        room
    }

    /// Whether a body `tall` high fits at `level` in `cell`, clear of
    /// everything in `room`.
    fn fits(
        &self,
        cell: (i32, i32),
        level: i32,
        tall: i32,
        kerb: i32,
        room: &[(i32, i32)],
    ) -> bool {
        room.iter().all(|&(dx, dy)| {
            self.columns
                .get(&(cell.0 + dx, cell.1 + dy))
                .is_none_or(|column| !column.in_the_way(level, tall, kerb))
        })
    }

    /// Everywhere a champion with the given step could get to by walking in
    /// from open ground, near anything that stands on the map.
    ///
    /// Open ground is every cell with nothing over it and nothing within two
    /// cells of it. The walk goes in from there a cell at a time, to the
    /// cells sharing a side, by the rule of [`HeightMap::step_to`], taking
    /// every level it is allowed and not only the nearest.
    ///
    /// `across` is how many cells a change of level may pass over. With room
    /// for a body, a champion on the ground cannot be in the cell next to a
    /// block, and one on the block can be at its very edge, so the two are
    /// never in cells that share a side: with `across` at 0 nothing with a
    /// body ever gets onto anything. With `across` at 1 a change of level
    /// may go two cells in a straight line, over one cell it could not stop
    /// in, provided nothing in that cell stands higher than a kerb above the
    /// upper of the two levels. Moves that keep to one level are always to
    /// the next cell.
    pub fn reach(&self, step: i32, tall: i32, body: i32, kerb: i32, across: i32) -> Reach {
        const NEAR: i32 = 2;
        let mut near: HashSet<(i32, i32)> = HashSet::new();
        for &(cx, cy) in self.columns.keys() {
            for dy in -NEAR..=NEAR {
                for dx in -NEAR..=NEAR {
                    near.insert((cx + dx, cy + dy));
                }
            }
        }
        let room = self.room(body);
        let sides = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        let mut levels: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
        let mut queue: VecDeque<((i32, i32), i32)> = VecDeque::new();
        // start on the rim: cells near something that touch open ground
        let mut rim: Vec<(i32, i32)> = near
            .iter()
            .copied()
            .filter(|&(cx, cy)| {
                sides
                    .iter()
                    .any(|&(dx, dy)| !near.contains(&(cx + dx, cy + dy)))
            })
            .collect();
        rim.sort_unstable();
        for cell in rim {
            if self.fits(cell, 0, tall, kerb, &room) {
                levels.insert(cell, vec![0]);
                queue.push_back((cell, 0));
            }
        }
        while let Some(((cx, cy), level)) = queue.pop_front() {
            for (dx, dy) in sides {
                let to = (cx + dx, cy + dy);
                if !near.contains(&to) {
                    continue;
                }
                let there = match self.columns.get(&to) {
                    Some(column) => column.levels(kerb),
                    None => vec![0],
                };
                for at in there {
                    if (at - level).abs() > step || !self.fits(to, at, tall, kerb, &room) {
                        continue;
                    }
                    let known = levels.entry(to).or_default();
                    if !known.contains(&at) {
                        known.push(at);
                        queue.push_back((to, at));
                    }
                }
                // a change of level, over cells it could not stop in
                for far in 2..=across + 1 {
                    let to = (cx + dx * far, cy + dy * far);
                    if !near.contains(&to) {
                        break;
                    }
                    let Some(column) = self.columns.get(&to) else {
                        continue;
                    };
                    for at in column.levels(kerb) {
                        if at == level
                            || (at - level).abs() > step
                            || !self.fits(to, at, tall, kerb, &room)
                        {
                            continue;
                        }
                        let over = (1..far).all(|k| {
                            self.columns
                                .get(&(cx + dx * k, cy + dy * k))
                                .is_none_or(|c| !c.in_the_way(level.max(at), tall, kerb))
                        });
                        if !over {
                            continue;
                        }
                        let known = levels.entry(to).or_default();
                        if !known.contains(&at) {
                            known.push(at);
                            queue.push_back((to, at));
                        }
                    }
                }
            }
        }
        for known in levels.values_mut() {
            known.sort_unstable();
        }
        Reach { near, levels }
    }
}

/// Where a champion can get to: the answer of [`HeightMap::reach`].
#[derive(Debug, Clone, Default)]
pub struct Reach {
    near: HashSet<(i32, i32)>,
    levels: HashMap<(i32, i32), Vec<i32>>,
}

impl Reach {
    /// Whether the champion can be in this cell at all. Open ground, away
    /// from everything, always is.
    pub fn open(&self, cell: (i32, i32)) -> bool {
        !self.near.contains(&cell) || self.levels.contains_key(&cell)
    }

    /// The levels it can be at in this cell, lowest first; none if it cannot
    /// be there, and the ground alone on open ground.
    pub fn levels(&self, cell: (i32, i32)) -> &[i32] {
        match self.levels.get(&cell) {
            Some(levels) => levels,
            None if self.near.contains(&cell) => &[],
            None => &[0],
        }
    }

    /// What this champion reaches that `other` does not: every cell where it
    /// can be at a level the other cannot, gathered into patches of cells
    /// that touch, biggest first. Each patch is its cells and the highest
    /// level reached in it.
    pub fn beyond(&self, other: &Reach) -> Vec<(Vec<(i32, i32)>, i32)> {
        let mut gained: HashMap<(i32, i32), i32> = HashMap::new();
        for (&cell, levels) in &self.levels {
            let theirs = other.levels(cell);
            if let Some(&top) = levels.iter().filter(|at| !theirs.contains(at)).max() {
                gained.insert(cell, top);
            }
        }
        let mut cells: Vec<(i32, i32)> = gained.keys().copied().collect();
        cells.sort_unstable();
        let mut seen: HashSet<(i32, i32)> = HashSet::new();
        let mut patches = Vec::new();
        for start in cells {
            if !seen.insert(start) {
                continue;
            }
            let (mut patch, mut top, mut queue) = (Vec::new(), 0, vec![start]);
            while let Some((cx, cy)) = queue.pop() {
                patch.push((cx, cy));
                top = top.max(gained[&(cx, cy)]);
                for next in [(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)] {
                    if gained.contains_key(&next) && seen.insert(next) {
                        queue.push(next);
                    }
                }
            }
            patch.sort_unstable();
            patches.push((patch, top));
        }
        patches.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(&b.0)));
        patches
    }
}

/// The height of the ground itself, from the map's own tile heights.
///
/// It holds one whole number for each corner where tiles meet, and reads the
/// height anywhere between them off the same two triangles a tile is drawn
/// with (split from its (1,0) corner to its (0,1) corner). Every machine
/// holds the same numbers and does the same whole-number arithmetic, so no
/// two can disagree about how high the ground is.
#[derive(Debug, Clone)]
pub struct Ground {
    /// Corners across; one more than the tiles across.
    across: i32,
    along: i32,
    /// World units from one corner to the next.
    tile: i32,
    /// The height of each corner, in sixteenths of a world unit.
    corners: Vec<i32>,
}

impl Ground {
    /// `corners` are heights in sixteenths of a world unit, row by row,
    /// `across` to a row.
    pub fn new(across: usize, tile: i32, corners: Vec<i32>) -> Self {
        assert!(across >= 2 && tile > 0 && corners.len() % across == 0);
        Self {
            across: across as i32,
            along: (corners.len() / across) as i32,
            tile,
            corners,
        }
    }

    fn corner(&self, gx: i32, gy: i32) -> i32 {
        let (gx, gy) = (gx.clamp(0, self.across - 1), gy.clamp(0, self.along - 1));
        self.corners[(gy * self.across + gx) as usize]
    }

    /// The ground's height at a point, in sixteenths. `x` and `y` are in
    /// sixteenths of a world unit from the map's corner.
    pub fn at(&self, x: i32, y: i32) -> i32 {
        let span = self.tile * Q;
        let (gx, gy) = (x.div_euclid(span), y.div_euclid(span));
        let (fx, fy) = (x.rem_euclid(span) as i64, y.rem_euclid(span) as i64);
        let (h00, h10) = (self.corner(gx, gy) as i64, self.corner(gx + 1, gy) as i64);
        let (h01, h11) = (
            self.corner(gx, gy + 1) as i64,
            self.corner(gx + 1, gy + 1) as i64,
        );
        let span = span as i64;
        let height = if fx + fy <= span {
            h00 * span + (h10 - h00) * fx + (h01 - h00) * fy
        } else {
            h11 * span + (h01 - h11) * (span - fx) + (h10 - h11) * (span - fy)
        };
        height.div_euclid(span) as i32
    }
}

/// Whether `level` lies strictly inside one of the stretches (not at its top).
fn joined_inside(solid: &[(i32, i32)], level: i32) -> bool {
    solid
        .iter()
        .any(|&(bottom, top)| level > bottom && level < top)
}

/// How the height data and the walkable map read the same shape.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Agreement {
    /// Cells both say are blocked.
    pub both: usize,
    /// Cells only the walkable map blocks.
    pub only_map: usize,
    /// Cells only the height data blocks.
    pub only_heights: usize,
}

impl Agreement {
    pub fn agrees(&self) -> bool {
        self.only_map == 0 && self.only_heights == 0
    }

    pub fn add(&mut self, other: Agreement) {
        self.both += other.both;
        self.only_map += other.only_map;
        self.only_heights += other.only_heights;
    }
}

/// Compare two maps of the same size cell by cell. Two maps that block the
/// same number of cells in different places do not agree: every cell blocked
/// in one and open in the other is counted, on whichever side it is.
pub fn compare(map: &Grid, from_heights: &Grid) -> Agreement {
    let mut out = Agreement::default();
    let width = map.width().max(from_heights.width());
    let height = map.height().max(from_heights.height());
    for cy in 0..height {
        for cx in 0..width {
            match (map.is_blocked(cx, cy), from_heights.is_blocked(cx, cy)) {
                (true, true) => out.both += 1,
                (true, false) => out.only_map += 1,
                (false, true) => out.only_heights += 1,
                (false, false) => {}
            }
        }
    }
    out
}

/// Read one shape both ways, on a patch of map of its own, and count the
/// cells on which the two disagree about whether it blocks.
///
/// `at` is where the shape stands on the real map. The patch keeps the part
/// of that position that falls within a cell, so the shape lies across the
/// patch's cells exactly as it lies across the map's, and the cells compared
/// here are the cells the map gets.
///
/// The walkable map's reading is [`Grid::block_solid`] with `low` and `high`.
/// The height data's is: read the shape up to `high`, close every cell a
/// champion with a step and a kerb of `low` and a body `high` tall could not
/// be in from the ground, then close what those cells enclose and add room
/// for a body, exactly as the walkable map does.
pub fn agreement(
    triangles: &[[[f32; 3]; 3]],
    at: (f32, f32),
    mirrored: bool,
    standing_angle: f32,
    low: f32,
    high: f32,
    body: i32,
    cell: i32,
) -> Agreement {
    // a patch big enough for the shape and the room round it
    let reach = triangles
        .iter()
        .flatten()
        .map(|p| p[0].abs().max(p[2].abs()))
        .fold(0.0f32, f32::max);
    let half = ((reach as i32 + body) / cell + 4).min(256);
    let size = (half * 2) as usize;
    let within = |v: f32| v.rem_euclid(cell as f32);
    let at = (
        (half * cell) as f32 + within(at.0),
        (half * cell) as f32 + within(at.1),
    );

    let mut map = Grid::with_cell(size, size, cell);
    map.block_solid(triangles, at, low, high, body);

    let mut heights = HeightMap::new(size, size, cell);
    let reading = Reading {
        mirrored,
        standing_angle,
        head: Some(high),
    };
    heights.add_shape(triangles, at, &reading);
    // The walkable map lets a walker over what is lower than `low` and stops
    // it at what is `low` or more. A step reaches what is no more than a
    // step up, so the step that matches the map is the smallest amount the
    // height data can tell apart under `low`.
    let step = low - 1.0 / Q as f32;
    let mut from_heights = Grid::with_cell(size, size, cell);
    from_heights.block_cells(&heights.closed_from_the_ground(step, high, low), body);

    compare(&map, &from_heights)
}
