use rift_sim::motion::{FP, TICK_HZ, TILE, cos_q14, heading_of, sin_q14, turn_between};
use rift_sim::{Grid, Intent, Motion, MotionParams};

const HEAVY: MotionParams = MotionParams {
    speed: 350,
    turn_ms: 450,
    accel_ms: 300,
    stop_ms: 200,
};
const LIGHT: MotionParams = MotionParams {
    speed: 420,
    turn_ms: 150,
    accel_ms: 100,
    stop_ms: 50,
};
const SNAP: MotionParams = MotionParams {
    speed: 400,
    turn_ms: 0,
    accel_ms: 0,
    stop_ms: 0,
};

fn centre(tx: i32, ty: i32) -> (i32, i32) {
    ((tx * TILE + TILE / 2) * FP, (ty * TILE + TILE / 2) * FP)
}

fn top(p: &MotionParams) -> i32 {
    p.speed * FP / TICK_HZ
}

#[test]
fn angles_agree_with_the_clients_yaw() {
    assert_eq!((sin_q14(0), cos_q14(0)), (0, 16384));
    assert_eq!((sin_q14(16384), cos_q14(16384)), (16384, 0));
    assert_eq!((sin_q14(32768), cos_q14(32768)), (0, -16384));
    assert_eq!((sin_q14(49152), cos_q14(49152)), (-16384, 0));
    assert_eq!(heading_of(0, 5), 0);
    assert_eq!(heading_of(5, 0), 16384);
    assert_eq!(heading_of(0, -5), 32768);
    assert_eq!(heading_of(-5, 0), 49152);
    assert_eq!(heading_of(7, 7), 8192);
    assert_eq!(heading_of(0, 0), 0);
    // every heading round-trips through sin/cos to within a quarter of a degree
    for a in (0..65536u32).step_by(97) {
        let a = a as u16;
        let back = heading_of(sin_q14(a) as i64, cos_q14(a) as i64);
        assert!(
            turn_between(a, back).abs() < 48,
            "angle {a} came back as {back}"
        );
    }
    assert_eq!(turn_between(65000, 500), 1036);
    assert_eq!(turn_between(500, 65000), -1036);
}

#[test]
fn it_takes_its_accel_time_to_reach_top_speed_and_its_turn_time_to_turn_round() {
    let open = Grid::new(40, 40);
    for p in [HEAVY, LIGHT] {
        let (x, y) = centre(5, 5);
        let mut m = Motion::at(x, y, 0);
        let mut ticks = 0;
        while m.speed < top(&p) {
            m.step(&p, 1000, &open, Intent::Steer { heading: 0 });
            ticks += 1;
            assert!(ticks < 200);
        }
        let want = p.accel_ms * TICK_HZ / 1000;
        assert!(
            (ticks - want).abs() <= 1,
            "{p:?}: {ticks} ticks to top speed, wanted about {want}"
        );

        // standing still, asked to face the other way
        let mut m = Motion::at(x, y, 0);
        let mut ticks = 0;
        while m.facing != 32768 {
            m.step(&p, 1000, &open, Intent::Face { heading: 32768 });
            ticks += 1;
            assert!(ticks < 200);
        }
        let want = p.turn_ms * TICK_HZ / 1000;
        assert!(
            (ticks - want).abs() <= 1,
            "{p:?}: {ticks} ticks to turn round, wanted about {want}"
        );
        assert_eq!((m.x, m.y), (x, y), "facing does not move it");
    }
}

#[test]
fn a_command_moves_it_on_the_first_tick_even_when_the_target_is_behind() {
    let open = Grid::new(40, 40);
    let (x, y) = centre(20, 20);
    for p in [HEAVY, LIGHT] {
        let mut m = Motion::at(x, y, 0);
        let s = m.step(
            &p,
            1000,
            &open,
            Intent::Seek {
                target: centre(20, 5),
                arrive: true,
            },
        );
        assert!(s.moved > 0, "{p:?} did not move on the first tick");
        assert_ne!(m.facing, 0, "{p:?} did not start turning on the first tick");
    }
}

#[test]
fn it_arrives_exactly_on_the_point_and_stops_there() {
    let open = Grid::new(40, 40);
    for p in [HEAVY, LIGHT, SNAP] {
        let mut m = Motion::at(centre(5, 5).0, centre(5, 5).1, 0);
        let target = centre(17, 11);
        let mut arrived = false;
        for _ in 0..600 {
            let s = m.step(
                &p,
                1000,
                &open,
                Intent::Seek {
                    target,
                    arrive: true,
                },
            );
            if s.arrived {
                arrived = true;
                break;
            }
        }
        assert!(arrived, "{p:?} never arrived");
        assert_eq!((m.x, m.y, m.speed), (target.0, target.1, 0));
        // and stays put
        let s = m.step(
            &p,
            1000,
            &open,
            Intent::Seek {
                target,
                arrive: true,
            },
        );
        assert_eq!((m.x, m.y, s.moved), (target.0, target.1, 0));
    }
}

/// Run north at full speed, then be told to go to a point behind. How far
/// north of the turn-point does it get before coming back?
fn overshoot(p: &MotionParams) -> i32 {
    let open = Grid::new(60, 60);
    let (x, y) = centre(30, 10);
    let mut m = Motion::at(x, y, 0);
    for _ in 0..60 {
        m.step(p, 1000, &open, Intent::Steer { heading: 0 });
    }
    let turn_at = m.y;
    let target = (m.x, m.y - 10 * TILE * FP);
    let mut furthest = 0;
    for _ in 0..300 {
        m.step(
            p,
            1000,
            &open,
            Intent::Seek {
                target,
                arrive: true,
            },
        );
        furthest = furthest.max(m.y - turn_at);
    }
    assert_eq!((m.x, m.y), target, "{p:?} did not get back to the point");
    furthest
}

#[test]
fn a_heavy_champion_carries_further_through_a_reversal_than_a_light_one() {
    let (heavy, light, snap) = (overshoot(&HEAVY), overshoot(&LIGHT), overshoot(&SNAP));
    assert_eq!(snap, 0, "with no turn or stop time it reverses on the spot");
    assert!(light > 0, "the light one still has some momentum");
    assert!(
        heavy > light * 2,
        "heavy carried {heavy}, light {light}: not different enough to feel"
    );
}

#[test]
fn zero_times_reproduce_the_old_movement() {
    let open = Grid::new(40, 40);
    let mut m = Motion::at(centre(5, 5).0, centre(5, 5).1, 0);
    let s = m.step(&SNAP, 1000, &open, Intent::Steer { heading: 16384 });
    assert_eq!(m.facing, 16384, "facing snaps");
    assert_eq!(m.speed, top(&SNAP), "top speed on the first tick");
    assert_eq!(s.moved, top(&SNAP));
    m.step(&SNAP, 1000, &open, Intent::Idle);
    assert_eq!(m.speed, 0, "and it stops dead");
}

#[test]
fn it_never_enters_a_blocked_tile_and_slides_along_a_wall() {
    // a wall along tile row 10
    let mut g = Grid::new(30, 30);
    for x in 0..30 {
        g.set_blocked(x, 10, true);
    }
    for p in [HEAVY, LIGHT, SNAP] {
        let mut m = Motion::at(centre(5, 7).0, centre(5, 7).1, 0);
        let start_x = m.x;
        // steer north-east, into the wall
        for _ in 0..200 {
            m.step(&p, 1000, &g, Intent::Steer { heading: 8192 });
            let (tx, ty) = m.tile();
            assert!(
                g.is_walkable(tx, ty),
                "{p:?} stood on a blocked tile at {tx},{ty}"
            );
        }
        assert!(
            m.tile().1 == 9,
            "{p:?} should be against the wall, is on row {}",
            m.tile().1
        );
        assert!(
            m.x > start_x + 4 * TILE * FP,
            "{p:?} did not slide east along the wall"
        );
    }
    // two blocked tiles touching at a corner leave no way between them
    let mut d = Grid::new(6, 6);
    d.set_blocked(2, 3, true);
    d.set_blocked(3, 2, true);
    let mut m = Motion::at(centre(2, 2).0, centre(2, 2).1, 8192);
    for _ in 0..200 {
        m.step(&SNAP, 1000, &d, Intent::Steer { heading: 8192 });
        assert_ne!(m.tile(), (3, 3), "it squeezed through the corner");
    }
}

#[test]
fn a_speed_modifier_scales_top_speed_and_nothing_else() {
    let open = Grid::new(60, 60);
    let run = |permille: i32| {
        let mut m = Motion::at(centre(5, 5).0, centre(5, 5).1, 0);
        for _ in 0..90 {
            m.step(&HEAVY, permille, &open, Intent::Steer { heading: 0 });
        }
        m
    };
    let (base, fast) = (run(1000), run(1200));
    assert_eq!(base.speed, top(&HEAVY));
    assert_eq!(fast.speed, HEAVY.speed * FP * 1200 / (1000 * TICK_HZ));
    assert_eq!(fast.facing, base.facing);
}

#[test]
fn the_same_intents_give_the_same_motion_every_time() {
    let mut g = Grid::new(40, 40);
    g.block_rect(12, 8, 3, 9);
    let script = |m: &mut Motion| {
        let mut trace = Vec::new();
        for t in 0..400 {
            let intent = match t {
                0..=79 => Intent::Seek {
                    target: centre(25, 12),
                    arrive: false,
                },
                80..=139 => Intent::Steer { heading: 40000 },
                140..=169 => Intent::Face { heading: 1000 },
                170..=320 => Intent::Seek {
                    target: centre(4, 30),
                    arrive: true,
                },
                _ => Intent::Idle,
            };
            m.step(&HEAVY, if t > 200 { 1150 } else { 1000 }, &g, intent);
            trace.push(*m);
        }
        trace
    };
    let mut a = Motion::at(centre(5, 5).0, centre(5, 5).1, 0);
    let mut b = a;
    assert_eq!(script(&mut a), script(&mut b));
}

/// One champion, three simulation rates. What the player would feel, in
/// seconds and world units, measured at each.
fn feel_at(hz: i32, p: &MotionParams) -> (f32, f32, f32, f32) {
    let open = Grid::new(80, 80);
    let (x, y) = centre(40, 10);
    let secs = |ticks: i32| ticks as f32 / hz as f32;
    let world = |units: i32| units as f32 / FP as f32;

    // seconds to top speed
    let mut m = Motion::at(x, y, 0);
    let top = p.speed * FP / hz;
    let mut to_top = 0;
    while m.speed < top {
        m.step_at(hz, p, 1000, &open, Intent::Steer { heading: 0 });
        to_top += 1;
    }
    // seconds to turn round
    let mut m = Motion::at(x, y, 0);
    let mut to_turn = 0;
    while m.facing != 32768 {
        m.step_at(hz, p, 1000, &open, Intent::Face { heading: 32768 });
        to_turn += 1;
    }
    // world units covered in two seconds from standing
    let mut m = Motion::at(x, y, 0);
    for _ in 0..2 * hz {
        m.step_at(hz, p, 1000, &open, Intent::Steer { heading: 0 });
    }
    let covered = world(m.y - y);
    // how far it carries through a reversal
    let turn_at = m.y;
    let target = (m.x, m.y - 10 * TILE * FP);
    let mut carried = 0;
    for _ in 0..10 * hz {
        m.step_at(
            hz,
            p,
            1000,
            &open,
            Intent::Seek {
                target,
                arrive: true,
            },
        );
        carried = carried.max(m.y - turn_at);
    }
    assert_eq!(
        (m.x, m.y),
        target,
        "at {hz} Hz it did not get back to the point"
    );
    (secs(to_top), secs(to_turn), covered, world(carried))
}

#[test]
fn a_champion_moves_the_same_whatever_rate_the_simulation_runs_at() {
    for p in [HEAVY, LIGHT] {
        let base = feel_at(30, &p);
        for hz in [20, 60, 120] {
            let got = feel_at(hz, &p);
            let tick = 1.0 / hz.min(30) as f32;
            assert!(
                (got.0 - base.0).abs() <= tick,
                "{p:?} at {hz} Hz: {} s to top speed, {} s at 30",
                got.0,
                base.0
            );
            assert!(
                (got.1 - base.1).abs() <= tick,
                "{p:?} at {hz} Hz: {} s to turn, {} s at 30",
                got.1,
                base.1
            );
            assert!(
                (got.2 - base.2).abs() <= base.2 * 0.02,
                "{p:?} at {hz} Hz: covered {}, {} at 30",
                got.2,
                base.2
            );
            // the arc of a reversal is the one thing that shifts: at a lower
            // rate the champion comes round in bigger steps. A few world units.
            assert!(
                (got.3 - base.3).abs() <= 6.0,
                "{p:?} at {hz} Hz: carried {}, {} at 30",
                got.3,
                base.3
            );
        }
    }
}
