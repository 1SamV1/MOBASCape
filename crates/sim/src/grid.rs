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

    /// Bresenham line of sight; returns false if any intermediate tile is
    /// blocked. Used to simplify A* paths and for hold-to-steer checks.
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
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
