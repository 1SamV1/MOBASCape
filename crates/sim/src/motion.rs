//! Champion locomotion: turning, speeding up, slowing down.
//!
//! Integer-only and tick-based, so the same commands always give the same
//! motion on every machine (AGENTS.md hard rule 3). One `step` is one 30 Hz
//! tick. See docs/MOVEMENT.md for what each value is for.
//!
//! Units:
//! - positions are in 1/256 of a world unit (`FP` per unit), measured from
//!   the grid's corner, so a tile is `128 * FP` across. That is fine enough
//!   for speeds and accelerations to stay accurate at high tick rates;
//! - headings are binary angles: 65536 to a full turn, 0 facing +y, a quarter
//!   turn (16384) facing +x. This matches the client's `yaw = atan2(x, z)`;
//! - speeds are in position units per tick.

use crate::grid::Grid;

/// Position units per world unit.
pub const FP: i32 = 256;
/// Within this distance of the point it is seeking, a champion is on it.
/// An eighth of a world unit: far too small to see.
const ON_POINT: i64 = (FP / 8) as i64;
/// Simulation ticks per second. This is the one place the rate is set. Every
/// value a champion carries is in seconds or in units per second and is
/// converted here, so changing this number changes how often the simulation
/// runs and not how a champion moves (`step_at` and its test hold that).
pub const TICK_HZ: i32 = 30;
/// World units per tile.
pub const TILE: i32 = 128;
/// Below this share of top speed a champion never drops while it has
/// somewhere to go, so a command always produces motion on its first tick.
const FLOOR_Q14: i64 = 3277; // 0.2
const ONE_Q14: i64 = 16384;

/// sin of a quarter turn in 256 steps, scaled by 16384.
const SIN_Q14: [i32; 257] = [
    0, 101, 201, 302, 402, 503, 603, 704, 804, 904, 1005, 1105, 1205, 1306, 1406, 1506, 1606, 1706,
    1806, 1906, 2006, 2105, 2205, 2305, 2404, 2503, 2603, 2702, 2801, 2900, 2999, 3098, 3196, 3295,
    3393, 3492, 3590, 3688, 3786, 3883, 3981, 4078, 4176, 4273, 4370, 4467, 4563, 4660, 4756, 4852,
    4948, 5044, 5139, 5235, 5330, 5425, 5520, 5614, 5708, 5803, 5897, 5990, 6084, 6177, 6270, 6363,
    6455, 6547, 6639, 6731, 6823, 6914, 7005, 7096, 7186, 7276, 7366, 7456, 7545, 7635, 7723, 7812,
    7900, 7988, 8076, 8163, 8250, 8337, 8423, 8509, 8595, 8680, 8765, 8850, 8935, 9019, 9102, 9186,
    9269, 9352, 9434, 9516, 9598, 9679, 9760, 9841, 9921, 10001, 10080, 10159, 10238, 10316, 10394,
    10471, 10549, 10625, 10702, 10778, 10853, 10928, 11003, 11077, 11151, 11224, 11297, 11370,
    11442, 11514, 11585, 11656, 11727, 11797, 11866, 11935, 12004, 12072, 12140, 12207, 12274,
    12340, 12406, 12472, 12537, 12601, 12665, 12729, 12792, 12854, 12916, 12978, 13039, 13100,
    13160, 13219, 13279, 13337, 13395, 13453, 13510, 13567, 13623, 13678, 13733, 13788, 13842,
    13896, 13949, 14001, 14053, 14104, 14155, 14206, 14256, 14305, 14354, 14402, 14449, 14497,
    14543, 14589, 14635, 14680, 14724, 14768, 14811, 14854, 14896, 14937, 14978, 15019, 15059,
    15098, 15137, 15175, 15213, 15250, 15286, 15322, 15357, 15392, 15426, 15460, 15493, 15525,
    15557, 15588, 15619, 15649, 15679, 15707, 15736, 15763, 15791, 15817, 15843, 15868, 15893,
    15917, 15941, 15964, 15986, 16008, 16029, 16049, 16069, 16088, 16107, 16125, 16143, 16160,
    16176, 16192, 16207, 16221, 16235, 16248, 16261, 16273, 16284, 16295, 16305, 16315, 16324,
    16332, 16340, 16347, 16353, 16359, 16364, 16369, 16373, 16376, 16379, 16381, 16383, 16384,
    16384,
];

/// atan(k / 256) as a binary angle, for k in 0..=256.
const ATAN_BA: [i32; 257] = [
    0, 41, 81, 122, 163, 204, 244, 285, 326, 367, 407, 448, 489, 529, 570, 610, 651, 692, 732, 773,
    813, 854, 894, 935, 975, 1015, 1056, 1096, 1136, 1177, 1217, 1257, 1297, 1337, 1377, 1417,
    1457, 1497, 1537, 1577, 1617, 1656, 1696, 1736, 1775, 1815, 1854, 1894, 1933, 1973, 2012, 2051,
    2090, 2129, 2168, 2207, 2246, 2285, 2324, 2363, 2401, 2440, 2478, 2517, 2555, 2594, 2632, 2670,
    2708, 2746, 2784, 2822, 2860, 2897, 2935, 2973, 3010, 3047, 3085, 3122, 3159, 3196, 3233, 3270,
    3307, 3344, 3380, 3417, 3453, 3490, 3526, 3562, 3599, 3635, 3670, 3706, 3742, 3778, 3813, 3849,
    3884, 3920, 3955, 3990, 4025, 4060, 4095, 4129, 4164, 4199, 4233, 4267, 4302, 4336, 4370, 4404,
    4438, 4471, 4505, 4539, 4572, 4605, 4639, 4672, 4705, 4738, 4771, 4803, 4836, 4869, 4901, 4933,
    4966, 4998, 5030, 5062, 5094, 5125, 5157, 5188, 5220, 5251, 5282, 5313, 5344, 5375, 5406, 5437,
    5467, 5498, 5528, 5559, 5589, 5619, 5649, 5679, 5708, 5738, 5768, 5797, 5826, 5856, 5885, 5914,
    5943, 5972, 6000, 6029, 6058, 6086, 6114, 6142, 6171, 6199, 6227, 6254, 6282, 6310, 6337, 6365,
    6392, 6419, 6446, 6473, 6500, 6527, 6554, 6580, 6607, 6633, 6660, 6686, 6712, 6738, 6764, 6790,
    6815, 6841, 6867, 6892, 6917, 6943, 6968, 6993, 7018, 7043, 7068, 7092, 7117, 7141, 7166, 7190,
    7214, 7238, 7262, 7286, 7310, 7334, 7358, 7381, 7405, 7428, 7451, 7475, 7498, 7521, 7544, 7566,
    7589, 7612, 7635, 7657, 7679, 7702, 7724, 7746, 7768, 7790, 7812, 7834, 7856, 7877, 7899, 7920,
    7942, 7963, 7984, 8005, 8026, 8047, 8068, 8089, 8110, 8131, 8151, 8172, 8192,
];

/// sin of a binary angle, scaled by 16384.
pub fn sin_q14(angle: u16) -> i32 {
    let a = angle as i32;
    let quadrant = a >> 14;
    let within = a & 0x3fff;
    let lookup = |x: i32| -> i32 {
        // x in 0..=16384 across the quarter turn; the table has 256 steps of 64
        let i = (x >> 6) as usize;
        let frac = x & 63;
        if i >= 256 {
            return SIN_Q14[256];
        }
        SIN_Q14[i] + (SIN_Q14[i + 1] - SIN_Q14[i]) * frac / 64
    };
    match quadrant {
        0 => lookup(within),
        1 => lookup(16384 - within),
        2 => -lookup(within),
        _ => -lookup(16384 - within),
    }
}

/// cos of a binary angle, scaled by 16384.
pub fn cos_q14(angle: u16) -> i32 {
    sin_q14(angle.wrapping_add(16384))
}

/// The heading of the vector (dx, dy): 0 for +y, a quarter turn for +x.
/// (0, 0) answers 0.
pub fn heading_of(dx: i64, dy: i64) -> u16 {
    if dx == 0 && dy == 0 {
        return 0;
    }
    let (ax, ay) = (dx.abs(), dy.abs());
    // angle from the +y axis toward +x, within the first quadrant
    let first = if ax <= ay {
        ATAN_BA[(ax * 256 / ay) as usize]
    } else {
        16384 - ATAN_BA[(ay * 256 / ax) as usize]
    };
    let angle = match (dx >= 0, dy >= 0) {
        (true, true) => first,
        (true, false) => 32768 - first,
        (false, false) => 32768 + first,
        (false, true) => 65536 - first,
    };
    (angle & 0xffff) as u16
}

/// The signed shortest turn from `from` to `to`, in binary-angle units.
pub fn turn_between(from: u16, to: u16) -> i32 {
    to.wrapping_sub(from) as i16 as i32
}

fn isqrt(n: i64) -> i64 {
    if n < 2 {
        return n.max(0);
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// How one champion moves. Times are in milliseconds; zero means instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionParams {
    /// Top speed, in world units per second.
    pub speed: i32,
    /// Time to turn half way round.
    pub turn_ms: i32,
    /// Time from standing to top speed.
    pub accel_ms: i32,
    /// Time from top speed to standing.
    pub stop_ms: i32,
}

impl MotionParams {
    /// Top speed in position units per tick, after a modifier in thousandths
    /// (1000 = unchanged, 1200 = hasted by a fifth).
    fn top(&self, permille: i32, hz: i32) -> i64 {
        self.speed as i64 * FP as i64 * permille as i64 / (1000 * hz as i64)
    }

    fn per_tick(total: i64, ms: i32, hz: i32) -> i64 {
        if ms <= 0 {
            return i64::MAX / 4;
        }
        (total * 1000 / (ms as i64 * hz as i64)).max(1)
    }
}

/// What the champion is trying to do this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    /// Come to rest.
    Idle,
    /// Come to rest, turning to face a heading (an attacker facing its target).
    Face { heading: u16 },
    /// Head for a point. `arrive` brakes to stop on it; without it the
    /// champion passes through at speed (a waypoint in the middle of a path).
    Seek { target: (i32, i32), arrive: bool },
    /// Run in a direction until told otherwise (hold-to-steer).
    Steer { heading: u16 },
}

/// A champion's motion state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Motion {
    pub x: i32,
    pub y: i32,
    pub facing: u16,
    /// Current speed, in position units per tick.
    pub speed: i32,
}

/// What one tick did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stepped {
    /// Distance covered, in position units.
    pub moved: i32,
    /// A `Seek` with `arrive` reached its point and stopped on it.
    pub arrived: bool,
    /// The way was blocked and no slide was possible.
    pub blocked: bool,
}

impl Motion {
    pub fn at(x: i32, y: i32, facing: u16) -> Self {
        Self {
            x,
            y,
            facing,
            speed: 0,
        }
    }

    /// The tile this position stands on.
    pub fn tile(&self) -> (i32, i32) {
        (self.x.div_euclid(TILE * FP), self.y.div_euclid(TILE * FP))
    }

    /// The cell of `grid` this position stands in. A grid's cells may be
    /// finer than a tile.
    fn cell(&self, grid: &Grid) -> (i32, i32) {
        let size = grid.cell() * FP;
        (self.x.div_euclid(size), self.y.div_euclid(size))
    }

    fn open(grid: &Grid, from: (i32, i32), x: i32, y: i32) -> bool {
        let size = grid.cell() * FP;
        let to = (x.div_euclid(size), y.div_euclid(size));
        if !grid.is_walkable(to.0, to.1) {
            return false;
        }
        // stepping diagonally into the next cell must not squeeze between two
        // blocked ones
        if to.0 != from.0 && to.1 != from.1 {
            return grid.is_walkable(to.0, from.1) && grid.is_walkable(from.0, to.1);
        }
        true
    }

    /// Advance one tick at the game's rate.
    pub fn step(
        &mut self,
        params: &MotionParams,
        permille: i32,
        grid: &Grid,
        intent: Intent,
    ) -> Stepped {
        self.step_at(TICK_HZ, params, permille, grid, intent)
    }

    /// Advance one tick of a simulation running `hz` ticks a second.
    pub fn step_at(
        &mut self,
        hz: i32,
        params: &MotionParams,
        permille: i32,
        grid: &Grid,
        intent: Intent,
    ) -> Stepped {
        let top = params.top(permille, hz);
        let turn_rate = MotionParams::per_tick(32768, params.turn_ms, hz);
        let accel = MotionParams::per_tick(top, params.accel_ms, hz);
        let decel = MotionParams::per_tick(top, params.stop_ms, hz);

        // where it wants to face, and how fast it wants to go
        let (want_heading, mut want_speed, seek) = match intent {
            Intent::Idle => (None, 0, None),
            Intent::Face { heading } => (Some(heading), 0, None),
            Intent::Steer { heading } => (Some(heading), top, None),
            Intent::Seek { target, arrive } => {
                let (dx, dy) = ((target.0 - self.x) as i64, (target.1 - self.y) as i64);
                let dist = isqrt(dx * dx + dy * dy);
                if dist == 0 {
                    (None, 0, Some((0, arrive, target)))
                } else {
                    let mut want = top;
                    if arrive && params.stop_ms > 0 {
                        // the fastest it may go and still stop on the point
                        want = want.min(isqrt(2 * decel * dist).max(1));
                    }
                    (Some(heading_of(dx, dy)), want, Some((dist, arrive, target)))
                }
            }
        };

        // turn
        let mut error = 0;
        if let Some(heading) = want_heading {
            let turn = turn_between(self.facing, heading) as i64;
            let step = turn.clamp(-turn_rate, turn_rate);
            self.facing = self.facing.wrapping_add(step as i32 as u16);
            error = turn_between(self.facing, heading);
        }

        // the further it is from facing its way, the slower it goes
        if want_speed > 0 {
            let agree = (cos_q14((error & 0xffff) as u16) as i64).max(0);
            let factor = FLOOR_Q14 + (ONE_Q14 - FLOOR_Q14) * agree / ONE_Q14;
            want_speed = (want_speed * factor / ONE_Q14).max(1);
        }

        // speed up or slow down toward that
        let speed = self.speed as i64;
        let speed = if speed < want_speed {
            (speed + accel).min(want_speed)
        } else {
            (speed - decel).max(want_speed)
        };
        self.speed = speed as i32;

        let mut out = Stepped {
            moved: 0,
            arrived: false,
            blocked: false,
        };
        if speed == 0 {
            if let Some((dist, true, _)) = seek {
                out.arrived = dist == 0;
            }
            return out;
        }

        // close enough to land on the point this tick
        if let Some((dist, arrive, target)) = seek {
            if dist <= ON_POINT || (dist <= speed && error.abs() < 8192) {
                let from = self.cell(grid);
                if Self::open(grid, from, target.0, target.1) {
                    self.x = target.0;
                    self.y = target.1;
                    out.moved = dist as i32;
                    if arrive {
                        self.speed = 0;
                        out.arrived = true;
                    }
                    return out;
                }
            }
        }

        // move the way it is facing; if that is blocked, straight at the
        // target; then along one axis; then not at all
        let from = self.cell(grid);
        let along = |heading: u16| -> (i32, i32) {
            (
                (sin_q14(heading) as i64 * speed / ONE_Q14) as i32,
                (cos_q14(heading) as i64 * speed / ONE_Q14) as i32,
            )
        };
        let (fx, fy) = along(self.facing);
        let mut tries = vec![(fx, fy)];
        if let Some(heading) = want_heading {
            if heading != self.facing {
                tries.push(along(heading));
            }
        }
        tries.push((fx, 0));
        tries.push((0, fy));
        // at a crawl the step can round to nothing; that is not a wall
        if tries.iter().all(|&(dx, dy)| dx == 0 && dy == 0) {
            return out;
        }
        for (dx, dy) in tries {
            if dx == 0 && dy == 0 {
                continue;
            }
            if Self::open(grid, from, self.x + dx, self.y + dy) {
                self.x += dx;
                self.y += dy;
                out.moved = isqrt(dx as i64 * dx as i64 + dy as i64 * dy as i64) as i32;
                return out;
            }
        }
        self.speed = 0;
        out.blocked = true;
        out
    }
}
