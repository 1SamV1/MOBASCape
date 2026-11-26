/// A map of where a unit may stand, as square cells.
///
/// A cell is `cell` world units across. The first maps were one cell to a
/// game tile (128 units), which is far coarser than what stands on them: a
/// thin trunk closed a whole tile, and two trees a tile apart made a wall. A
/// finer grid, with each obstacle marked as a disc the size of what is seen,
/// lets a unit pass wherever it would fit.
#[derive(Debug, Clone)]
pub struct Grid {
    width: i32,
    height: i32,
    cell: i32,
    blocked: Vec<bool>,
}

/// The width of a game tile in world units, and the cell size of a grid made
/// with [`Grid::new`].
pub const TILE_CELL: i32 = 128;

impl Grid {
    /// A grid of one cell to a game tile.
    pub fn new(width: usize, height: usize) -> Self {
        Self::with_cell(width, height, TILE_CELL)
    }

    /// A grid whose cells are `cell` world units across.
    pub fn with_cell(width: usize, height: usize, cell: i32) -> Self {
        assert!(cell > 0, "a cell has a size");
        Self {
            width: width as i32,
            height: height as i32,
            cell,
            blocked: vec![false; width * height],
        }
    }

    /// How many world units a cell is across.
    pub fn cell(&self) -> i32 {
        self.cell
    }

    /// The cell that holds a point, given in world units from the grid's
    /// corner.
    pub fn cell_at(&self, x: i32, y: i32) -> (i32, i32) {
        (x.div_euclid(self.cell), y.div_euclid(self.cell))
    }

    /// The middle of a cell, in world units from the grid's corner.
    pub fn centre_of(&self, cx: i32, cy: i32) -> (i32, i32) {
        (
            cx * self.cell + self.cell / 2,
            cy * self.cell + self.cell / 2,
        )
    }

    /// Block, or with `blocked` false open, every cell whose middle lies
    /// within `radius` of a point. All in world units from the grid's corner.
    ///
    /// To keep a body of some width off an obstacle, mark the obstacle with
    /// its own radius plus the body's: a unit is then a point on this grid.
    pub fn set_disc(&mut self, x: i32, y: i32, radius: i32, blocked: bool) {
        let (lo_x, lo_y) = self.cell_at(x - radius, y - radius);
        let (hi_x, hi_y) = self.cell_at(x + radius, y + radius);
        let reach = radius as i64 * radius as i64;
        for cy in lo_y..=hi_y {
            for cx in lo_x..=hi_x {
                let (mx, my) = self.centre_of(cx, cy);
                let (dx, dy) = ((mx - x) as i64, (my - y) as i64);
                if dx * dx + dy * dy <= reach {
                    self.set_blocked(cx, cy, blocked);
                }
            }
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.blocked[(y * self.width + x) as usize]
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && !self.blocked[(y * self.width + x) as usize]
    }

    pub fn set_blocked(&mut self, x: i32, y: i32, blocked: bool) {
        if self.in_bounds(x, y) {
            self.blocked[(y * self.width + x) as usize] = blocked;
        }
    }

    /// Block the ground a solid thing stands on, worked out from its shape.
    ///
    /// `triangles` are the thing's surface exactly as it is drawn: each
    /// corner is (across, height above the ground, along), in world units
    /// from `at`, the point on this grid where the thing stands. Only the
    /// part of the surface between `low` and `high` above the ground counts:
    /// what is lower can be stepped over, what is higher passes overhead. So
    /// a tree blocks where its trunk is and not under its leaves, and a
    /// leaning rock blocks at its foot and not under its overhang.
    ///
    /// Every cell that part of the surface stands over is blocked, and every
    /// cell within `body` of one, so that a unit of that half-width can be
    /// treated as a point. Returns how many cells the thing itself covers,
    /// before the room for a body is added.
    ///
    /// Because this reads the same shape that is drawn, what blocks and what
    /// is seen cannot drift apart: there is no radius to guess.
    pub fn block_solid(
        &mut self,
        triangles: &[[[f32; 3]; 3]],
        at: (f32, f32),
        low: f32,
        high: f32,
        body: i32,
    ) -> usize {
        let mut covered: Vec<(i32, i32)> = Vec::new();
        let spacing = self.cell as f32 / 3.0;
        for [a, b, c] in triangles {
            let edge = |p: &[f32; 3], q: &[f32; 3]| {
                ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
            };
            let longest = edge(a, b).max(edge(b, c)).max(edge(c, a));
            // sample the face closely enough that no cell under it is missed
            let steps = ((longest / spacing).ceil() as i32).clamp(1, 400);
            for i in 0..=steps {
                for j in 0..=(steps - i) {
                    let (u, v) = (i as f32 / steps as f32, j as f32 / steps as f32);
                    let w = 1.0 - u - v;
                    let height = a[1] * w + b[1] * u + c[1] * v;
                    if height < low || height > high {
                        continue;
                    }
                    let x = at.0 + a[0] * w + b[0] * u + c[0] * v;
                    let y = at.1 + a[2] * w + b[2] * u + c[2] * v;
                    let cell = self.cell_at(x.floor() as i32, y.floor() as i32);
                    if self.in_bounds(cell.0, cell.1) && covered.last() != Some(&cell) {
                        covered.push(cell);
                    }
                }
            }
        }
        covered.sort_unstable();
        covered.dedup();
        self.block_cells(&covered, body)
    }

    /// Block a set of cells a thing covers, whatever they enclose, and the
    /// room a body needs round them. This is the second half of
    /// [`Grid::block_solid`], given the cells it found; it is its own
    /// function so that another way of finding the cells can be finished off
    /// in exactly the same way and compared. Returns how many cells the thing
    /// covers, counting what it encloses and not the room for a body.
    pub fn block_cells(&mut self, cells: &[(i32, i32)], body: i32) -> usize {
        let mut covered: Vec<(i32, i32)> = cells.to_vec();
        covered.sort_unstable();
        covered.dedup();
        if covered.is_empty() {
            return 0;
        }

        // A tall thing is sampled only up to head height, which leaves its
        // walls and not its middle. Whatever its own walls close off is part
        // of it too: flood the cells round it from outside, and what the
        // flood cannot reach is inside. An arch stays open, because the flood
        // gets in through it.
        let (min_x, max_x) = (
            covered.iter().map(|c| c.0).min().unwrap_or(0) - 1,
            covered.iter().map(|c| c.0).max().unwrap_or(0) + 1,
        );
        let (min_y, max_y) = (
            covered.iter().map(|c| c.1).min().unwrap_or(0) - 1,
            covered.iter().map(|c| c.1).max().unwrap_or(0) + 1,
        );
        let (wide, tall) = ((max_x - min_x + 1) as usize, (max_y - min_y + 1) as usize);
        let slot = |cx: i32, cy: i32| (cy - min_y) as usize * wide + (cx - min_x) as usize;
        let mut wall = vec![false; wide * tall];
        for &(cx, cy) in &covered {
            wall[slot(cx, cy)] = true;
        }
        let mut outside = vec![false; wide * tall];
        let mut queue = vec![(min_x, min_y)];
        outside[slot(min_x, min_y)] = true;
        while let Some((cx, cy)) = queue.pop() {
            for (nx, ny) in [(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)] {
                if nx < min_x || nx > max_x || ny < min_y || ny > max_y {
                    continue;
                }
                let k = slot(nx, ny);
                if !wall[k] && !outside[k] {
                    outside[k] = true;
                    queue.push((nx, ny));
                }
            }
        }
        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                let k = slot(cx, cy);
                if !wall[k] && !outside[k] && self.in_bounds(cx, cy) {
                    covered.push((cx, cy));
                }
            }
        }

        // Room for a body: block every cell whose middle is within the
        // body's half-width of a covered cell's edge (half a cell out from
        // its middle). Measured from the middle alone, a body narrower than
        // a cell would add nothing at all.
        let reach = if body > 0 { body + self.cell / 2 } else { 0 };
        for &(cx, cy) in &covered {
            let (mx, my) = self.centre_of(cx, cy);
            self.set_disc(mx, my, reach, true);
            self.set_blocked(cx, cy, true);
        }
        covered.len()
    }

    pub fn block_rect(&mut self, x: i32, y: i32, width: i32, height: i32) {
        for yy in y..y + height {
            for xx in x..x + width {
                self.set_blocked(xx, yy, true);
            }
        }
    }

    pub fn nearest_walkable(&self, x: i32, y: i32, max_radius: i32) -> Option<(i32, i32)> {
        if self.is_walkable(x, y) {
            return Some((x, y));
        }
        for radius in 1..=max_radius {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() != radius && dy.abs() != radius {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if self.is_walkable(nx, ny) {
                        return Some((nx, ny));
                    }
                }
            }
        }
        None
    }

    /// Whether a straight line between the middles of two cells is clear:
    /// false if it crosses a blocked cell or squeezes past the corner of one.
    /// Used to straighten paths and to decide when a walker may turn for its
    /// next waypoint.
    pub fn line_of_sight(&self, a: (i32, i32), b: (i32, i32)) -> bool {
        let (mut x0, mut y0) = a;
        let (x1, y1) = b;
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if !self.is_walkable(x0, y0) {
                return false;
            }
            if x0 == x1 && y0 == y1 {
                return true;
            }
            let e2 = 2 * err;
            let (step_x, step_y) = (e2 >= dy, e2 <= dx);
            // a step that goes across and along at once passes the corner
            // where four cells meet; the line is clear there only if the two
            // cells beside it are, or a body following the line would clip one
            if step_x && step_y && !(self.is_walkable(x0 + sx, y0) && self.is_walkable(x0, y0 + sy))
            {
                return false;
            }
            if step_x {
                err += dy;
                x0 += sx;
            }
            if step_y {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Every cell a walker standing on `start` can reach, in row order
    /// (`y * width + x`). Steps go to the four side neighbours, which joins
    /// the same cells as [`crate::find_path`]'s eight-way steps that may not
    /// cut a blocked corner. A blocked or outside `start` reaches nothing.
    pub fn reachable_from(&self, start: (i32, i32)) -> Vec<bool> {
        let mut seen = vec![false; self.blocked.len()];
        if !self.is_walkable(start.0, start.1) {
            return seen;
        }
        let mut queue = std::collections::VecDeque::from([start]);
        seen[(start.1 * self.width + start.0) as usize] = true;
        while let Some((x, y)) = queue.pop_front() {
            for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if self.is_walkable(nx, ny) {
                    let index = (ny * self.width + nx) as usize;
                    if !seen[index] {
                        seen[index] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reachable_cells_stop_at_a_wall_and_its_corners() {
        // A wall down column 3 splits a 7x5 grid.
        let mut grid = Grid::new(7, 5);
        for y in 0..5 {
            grid.set_blocked(3, y, true);
        }
        let left = grid.reachable_from((0, 0));
        let at = |x: i32, y: i32| left[(y * 7 + x) as usize];
        assert!(at(2, 4) && at(0, 2), "its own side");
        assert!(!at(4, 0) && !at(6, 4), "not past the wall");
        assert!(!at(3, 2), "not the wall itself");
        grid.set_blocked(3, 2, false);
        let open = grid.reachable_from((0, 0));
        assert!(open[4 * 7 + 6], "a door in the wall lets it through");
        assert!(
            grid.reachable_from((3, 0)).iter().all(|seen| !seen),
            "a blocked start"
        );
        assert!(
            grid.reachable_from((-1, 0)).iter().all(|seen| !seen),
            "outside"
        );
        // Two blocked cells touching at a corner seal it, as for A*.
        let mut corner = Grid::new(3, 3);
        corner.set_blocked(1, 0, true);
        corner.set_blocked(0, 1, true);
        assert!(
            !corner.reachable_from((0, 0))[4],
            "no squeezing past a corner"
        );
        assert!(
            crate::find_path(&corner, (0, 0), (1, 1)).is_none(),
            "A* agrees"
        );
    }

    /// The four walls of an upright box, as triangles: `half` wide each way
    /// about (cx, cz), from `bottom` to `top`.
    fn pillar(cx: f32, cz: f32, half: f32, bottom: f32, top: f32) -> Vec<[[f32; 3]; 3]> {
        let corners = [
            (cx - half, cz - half),
            (cx + half, cz - half),
            (cx + half, cz + half),
            (cx - half, cz + half),
        ];
        let mut out = Vec::new();
        for k in 0..4 {
            let (p, q) = (corners[k], corners[(k + 1) % 4]);
            out.push([[p.0, bottom, p.1], [q.0, bottom, q.1], [q.0, top, q.1]]);
            out.push([[p.0, bottom, p.1], [q.0, top, q.1], [p.0, top, p.1]]);
        }
        // a lid, so the inside is covered too
        out.push([
            [corners[0].0, top, corners[0].1],
            [corners[1].0, top, corners[1].1],
            [corners[2].0, top, corners[2].1],
        ]);
        out.push([
            [corners[0].0, top, corners[0].1],
            [corners[2].0, top, corners[2].1],
            [corners[3].0, top, corners[3].1],
        ]);
        out
    }

    fn blocked_cells(grid: &Grid) -> Vec<(i32, i32)> {
        (0..grid.height())
            .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.is_blocked(x, y))
            .collect()
    }

    #[test]
    fn a_solid_blocks_under_itself_and_a_bodys_width_round_and_no_further() {
        let mut grid = Grid::with_cell(32, 32, 32);
        // a pillar 64 across standing at (512, 512), waist-high and more
        let covered = grid.block_solid(
            &pillar(0.0, 0.0, 32.0, 0.0, 120.0),
            (512.0, 512.0),
            28.0,
            160.0,
            28,
        );
        assert!(
            covered >= 4,
            "it stands on the four cells round its middle: {covered}"
        );
        for (cx, cy) in blocked_cells(&grid) {
            let (mx, my) = grid.centre_of(cx, cy);
            let (dx, dy) = ((mx - 512).abs(), (my - 512).abs());
            // its own half-width, a body's, and a cell of slack for the grid
            assert!(
                dx <= 32 + 28 + 32 && dy <= 32 + 28 + 32,
                "blocked too far out: {cx},{cy}"
            );
        }
        assert!(
            grid.is_blocked(15, 15) && grid.is_blocked(16, 16),
            "under the pillar"
        );
        let mut bare = Grid::with_cell(32, 32, 32);
        bare.block_solid(
            &pillar(0.0, 0.0, 32.0, 0.0, 120.0),
            (512.0, 512.0),
            28.0,
            160.0,
            0,
        );
        assert!(
            blocked_cells(&grid).len() > blocked_cells(&bare).len(),
            "a body's width adds ground round the thing, even a body narrower than a cell"
        );
        assert!(grid.is_blocked(14, 16), "and where a body would overlap it");
        assert!(
            grid.is_walkable(12, 16) && grid.is_walkable(16, 20),
            "open ground beside it"
        );
    }

    #[test]
    fn it_blocks_where_the_thing_stands_and_not_where_it_was_placed() {
        // a rock built well to one side of the point it is placed by
        let mut grid = Grid::with_cell(32, 32, 32);
        grid.block_solid(
            &pillar(150.0, -90.0, 30.0, 0.0, 200.0),
            (512.0, 512.0),
            28.0,
            160.0,
            28,
        );
        let (cx, cy) = grid.cell_at(512 + 150, 512 - 90);
        assert!(grid.is_blocked(cx, cy), "under the rock");
        let (px, py) = grid.cell_at(512, 512);
        assert!(
            grid.is_walkable(px, py),
            "the point it was placed by is open ground"
        );
    }

    #[test]
    fn what_is_low_is_stepped_over_and_what_is_high_passes_overhead() {
        let mut grid = Grid::with_cell(32, 32, 32);
        let mut shape = Vec::new();
        // a trunk, thin and tall
        shape.extend(pillar(0.0, 0.0, 10.0, 0.0, 400.0));
        // leaves, wide, far above head height
        shape.extend(pillar(0.0, 0.0, 200.0, 300.0, 400.0));
        // a stone, off to one side, ankle-high
        shape.extend(pillar(200.0, 0.0, 20.0, 0.0, 18.0));
        grid.block_solid(&shape, (512.0, 512.0), 28.0, 160.0, 28);
        let (tx, ty) = grid.cell_at(512, 512);
        assert!(grid.is_blocked(tx, ty), "the trunk blocks");
        let (lx, ly) = grid.cell_at(512 + 150, 512 + 150);
        assert!(grid.is_walkable(lx, ly), "under the leaves is open");
        let (sx, sy) = grid.cell_at(512 + 200, 512);
        assert!(grid.is_walkable(sx, sy), "the low stone is stepped over");
    }

    #[test]
    fn a_leaning_rock_blocks_at_its_foot_and_not_under_its_overhang() {
        let mut grid = Grid::with_cell(32, 32, 32);
        // one slanted slab: foot at the origin, top 300 up and 300 across
        let slab = vec![
            [
                [-30.0, 0.0, -30.0],
                [30.0, 0.0, -30.0],
                [330.0, 300.0, -30.0],
            ],
            [[-30.0, 0.0, 30.0], [30.0, 0.0, 30.0], [330.0, 300.0, 30.0]],
            [
                [-30.0, 0.0, -30.0],
                [-30.0, 0.0, 30.0],
                [330.0, 300.0, 30.0],
            ],
        ];
        grid.block_solid(&slab, (256.0, 512.0), 28.0, 160.0, 28);
        let (fx, fy) = grid.cell_at(256 + 60, 512);
        assert!(
            grid.is_blocked(fx, fy),
            "near the foot, where it is below head height"
        );
        let (ox, oy) = grid.cell_at(256 + 300, 512);
        assert!(
            grid.is_walkable(ox, oy),
            "under the overhang, which is above head height"
        );
    }

    #[test]
    fn a_hollow_thing_is_solid_inside_but_an_arch_can_be_walked_through() {
        // a wide tower, taller than head height: only its walls are sampled
        let mut grid = Grid::with_cell(32, 32, 32);
        grid.block_solid(
            &pillar(0.0, 0.0, 110.0, 0.0, 500.0),
            (512.0, 512.0),
            28.0,
            160.0,
            0,
        );
        let (mx, my) = grid.cell_at(512, 512);
        assert!(
            grid.is_blocked(mx, my),
            "the middle of the tower is not somewhere to stand"
        );

        // two legs of an arch with a gap between them
        let mut grid = Grid::with_cell(32, 32, 32);
        let mut arch = pillar(-150.0, 0.0, 30.0, 0.0, 400.0);
        arch.extend(pillar(150.0, 0.0, 30.0, 0.0, 400.0));
        arch.extend(pillar(0.0, 0.0, 180.0, 300.0, 400.0));
        grid.block_solid(&arch, (512.0, 512.0), 28.0, 160.0, 0);
        let (gx, gy) = grid.cell_at(512, 512);
        assert!(grid.is_walkable(gx, gy), "the way through the arch is open");
    }

    #[test]
    fn a_shape_off_the_edge_of_the_map_does_no_harm() {
        let mut grid = Grid::with_cell(8, 8, 32);
        let covered = grid.block_solid(
            &pillar(0.0, 0.0, 40.0, 0.0, 100.0),
            (-500.0, -500.0),
            28.0,
            160.0,
            28,
        );
        assert_eq!(covered, 0);
        assert!(blocked_cells(&grid).is_empty());
    }

    #[test]
    fn a_plain_grid_is_one_cell_to_a_tile() {
        let grid = Grid::new(4, 4);
        assert_eq!(grid.cell(), 128);
        assert_eq!(grid.cell_at(127, 128), (0, 1));
        assert_eq!(grid.centre_of(1, 0), (192, 64));
    }

    #[test]
    fn a_disc_blocks_what_it_covers_and_no_more() {
        let mut grid = Grid::with_cell(16, 16, 32);
        // a thin trunk in the middle of the second tile
        grid.set_disc(192, 192, 40, true);
        assert!(
            grid.is_blocked(5, 5) && grid.is_blocked(6, 6),
            "the cells round the trunk"
        );
        assert!(
            grid.is_walkable(4, 4),
            "the corner of the same tile is open"
        );
        assert!(grid.is_walkable(8, 6), "the next tile is untouched");
        let blocked = (0..16)
            .flat_map(|y| (0..16).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.is_blocked(x, y))
            .count();
        assert!(
            blocked < 16,
            "far less than the whole tile it stands on: {blocked}"
        );
        grid.set_disc(192, 192, 40, false);
        assert!(grid.is_walkable(5, 5), "and a disc can be opened again");
    }

    #[test]
    fn two_trunks_a_tile_apart_leave_a_way_between() {
        let mut grid = Grid::with_cell(16, 8, 32);
        // trunk 18 across the middle plus a body of 28: 46 kept clear
        grid.set_disc(64, 128, 46, true);
        grid.set_disc(192, 128, 46, true);
        let open: Vec<i32> = (0..8)
            .filter(|&x| grid.is_walkable(x, 3) && grid.is_walkable(x, 4))
            .collect();
        assert!(
            open.contains(&3) && open.contains(&4),
            "the gap between them: {open:?}"
        );
        // and a disc reaching past the edge of the grid does no harm
        grid.set_disc(0, 0, 100, true);
        assert!(grid.is_blocked(0, 0));
    }

    #[test]
    fn bounds_and_blocking() {
        let mut grid = Grid::new(8, 8);
        assert!(grid.is_walkable(0, 0));
        grid.set_blocked(0, 0, true);
        assert!(!grid.is_walkable(0, 0));
        assert!(!grid.is_walkable(-1, 0));
        assert!(!grid.is_walkable(8, 0));
        assert_eq!(grid.nearest_walkable(0, 0, 3), Some((1, 0)));
    }

    #[test]
    fn line_of_sight_detects_walls() {
        let mut grid = Grid::new(8, 8);
        assert!(grid.line_of_sight((0, 0), (7, 7)));
        for y in 0..8 {
            grid.set_blocked(4, y, true);
        }
        assert!(!grid.line_of_sight((0, 0), (7, 7)));
    }
}
