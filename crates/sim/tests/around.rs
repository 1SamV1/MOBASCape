use rift_sim::motion::{FP, TICK_HZ};
use rift_sim::{Grid, Intent, Motion, MotionParams, find_path, simplify_path};

/// Follow a path the way the client does: head for a waypoint, turn for the
/// next one once it is near and in plain sight, and ask for a new path when
/// stopped. Returns the ticks taken to arrive, or `None` if it never did.
fn walk(grid: &Grid, start: (i32, i32), goal: (i32, i32), params: &MotionParams) -> Option<i32> {
    const REACH: i64 = 56;
    let mut m = Motion::at(start.0 * FP, start.1 * FP, 0);
    let mut path: Vec<(i32, i32)> = Vec::new();
    let mut index = 0;
    let mut stuck = 0;
    let mut repath = true;
    for tick in 0..TICK_HZ * 20 {
        if repath {
            let p = find_path(
                grid,
                grid.cell_at(m.x / FP, m.y / FP),
                grid.cell_at(goal.0, goal.1),
            )?;
            path = simplify_path(grid, &p)
                .into_iter()
                .skip(1)
                .map(|(cx, cy)| grid.centre_of(cx, cy))
                .collect();
            index = 0;
            repath = false;
            if path.is_empty() {
                return Some(tick);
            }
        }
        while index + 1 < path.len() {
            let (dx, dy) = (
                (path[index].0 - m.x / FP) as i64,
                (path[index].1 - m.y / FP) as i64,
            );
            let d2 = dx * dx + dy * dy;
            if d2 > REACH * REACH {
                break;
            }
            let here = grid.cell_at(m.x / FP, m.y / FP);
            let next = grid.cell_at(path[index + 1].0, path[index + 1].1);
            if d2 > 6 * 6 && !grid.line_of_sight(here, next) {
                break;
            }
            index += 1;
        }
        let last = index + 1 >= path.len();
        let target = (path[index].0 * FP, path[index].1 * FP);
        let out = m.step(
            params,
            1000,
            grid,
            Intent::Seek {
                target,
                arrive: last,
            },
        );
        if out.arrived {
            return Some(tick);
        }
        if out.moved == 0 {
            stuck += 1;
        } else {
            stuck = 0;
        }
        if stuck >= 15 {
            stuck = 0;
            repath = true;
        }
    }
    None
}

/// Walking from any side of an obstacle to any other side arrives. Before
/// this was tested, about one walk in fifty round a tower-sized disc ended
/// with the champion pressed against it or running rings round its goal.
#[test]
fn it_gets_round_a_disc_from_every_side_to_every_other() {
    let champions = [
        (
            "monkey",
            MotionParams {
                speed: 540,
                turn_ms: 152,
                accel_ms: 102,
                stop_ms: 51,
            },
        ),
        (
            "kree",
            MotionParams {
                speed: 450,
                turn_ms: 253,
                accel_ms: 170,
                stop_ms: 250,
            },
        ),
        (
            "jad",
            MotionParams {
                speed: 380,
                turn_ms: 451,
                accel_ms: 304,
                stop_ms: 201,
            },
        ),
    ];
    let mut failures = Vec::new();
    let mut total = 0;
    for (name, params) in &champions {
        for radius in [46, 60, 98, 126, 160] {
            let mut grid = Grid::with_cell(64, 64, 32);
            grid.set_disc(1024, 1024, radius, true);
            // a second obstacle close by, so there is a gap to thread as well
            grid.set_disc(1024 + radius + 150, 1024, 46, true);
            let at = |k: i32, r: f64| {
                let t = k as f64 / 24.0 * std::f64::consts::TAU;
                (1024 + (t.cos() * r) as i32, 1024 + (t.sin() * r) as i32)
            };
            for a in 0..24 {
                for b in 0..24 {
                    if a == b {
                        continue;
                    }
                    let start = at(a, radius as f64 + 40.0);
                    let goal = at(b, radius as f64 + 60.0);
                    let (sc, gc) = (grid.cell_at(start.0, start.1), grid.cell_at(goal.0, goal.1));
                    if !grid.is_walkable(sc.0, sc.1) || !grid.is_walkable(gc.0, gc.1) {
                        continue;
                    }
                    total += 1;
                    match walk(&grid, start, goal, params) {
                        Some(ticks) if ticks <= TICK_HZ * 8 => {}
                        other => failures.push((*name, radius, a, b, other)),
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} walks failed: {:?}",
        failures.len(),
        total,
        &failures[..failures.len().min(12)]
    );
}

/// The same, round a thing with an awkward shape: a rock with a second lump
/// set against it, so its outline has an inside corner to get caught in.
#[test]
fn it_gets_round_an_awkward_shape_from_every_side() {
    fn slab(cx: f32, cz: f32, half_x: f32, half_z: f32) -> Vec<[[f32; 3]; 3]> {
        let c = [
            [cx - half_x, cz - half_z],
            [cx + half_x, cz - half_z],
            [cx + half_x, cz + half_z],
            [cx - half_x, cz + half_z],
        ];
        vec![
            [
                [c[0][0], 80.0, c[0][1]],
                [c[1][0], 80.0, c[1][1]],
                [c[2][0], 80.0, c[2][1]],
            ],
            [
                [c[0][0], 80.0, c[0][1]],
                [c[2][0], 80.0, c[2][1]],
                [c[3][0], 80.0, c[3][1]],
            ],
        ]
    }
    let mut shape = slab(0.0, 0.0, 70.0, 40.0);
    shape.extend(slab(50.0, 90.0, 30.0, 60.0));
    let mut grid = Grid::with_cell(64, 64, 32);
    grid.block_solid(&shape, (1024.0, 1024.0), 28.0, 160.0, 28);

    let champions = [
        (
            "monkey",
            MotionParams {
                speed: 540,
                turn_ms: 152,
                accel_ms: 102,
                stop_ms: 51,
            },
        ),
        (
            "jad",
            MotionParams {
                speed: 380,
                turn_ms: 451,
                accel_ms: 304,
                stop_ms: 201,
            },
        ),
    ];
    let at = |k: i32, r: f64| {
        let t = k as f64 / 32.0 * std::f64::consts::TAU;
        (1024 + (t.cos() * r) as i32, 1024 + (t.sin() * r) as i32)
    };
    let mut failures = Vec::new();
    let mut total = 0;
    for (name, params) in &champions {
        for ring in [150.0, 200.0, 260.0] {
            for a in 0..32 {
                for b in 0..32 {
                    if a == b {
                        continue;
                    }
                    let (start, goal) = (at(a, ring), at(b, ring));
                    let (sc, gc) = (grid.cell_at(start.0, start.1), grid.cell_at(goal.0, goal.1));
                    if !grid.is_walkable(sc.0, sc.1) || !grid.is_walkable(gc.0, gc.1) {
                        continue;
                    }
                    total += 1;
                    match walk(&grid, start, goal, params) {
                        Some(ticks) if ticks <= TICK_HZ * 8 => {}
                        other => failures.push((*name, ring as i32, a, b, other)),
                    }
                }
            }
        }
    }
    assert!(total > 2000, "enough walks were tried: {total}");
    assert!(
        failures.is_empty(),
        "{} of {} walks failed: {:?}",
        failures.len(),
        total,
        &failures[..failures.len().min(12)]
    );
}
