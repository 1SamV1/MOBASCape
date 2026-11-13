//! The player's champion, moved by the simulation.
//!
//! `rift_sim::motion` does the moving: integer, 30 ticks a second. This file
//! is the join between that and the render loop. It feeds the simulation whole
//! ticks out of the frame time, draws between the last two ticks so motion is
//! smooth at any frame rate, and converts between world floats and the
//! simulation's units. See docs/MOVEMENT.md.
//!
//! It also gives the champion its spring: a hop in each step, a small bounce
//! as it stops and a slow float at rest, all scaled by how light it is. That
//! is drawing only. It is a height to draw the body at and nothing else: the
//! simulation never sees it, so it cannot move a champion, a hitbox or a blow.

use std::f32::consts::{PI, TAU};

use glam::Vec3;
use rift_sim::motion::{FP, TICK_HZ, TILE, heading_of, turn_between};
use rift_sim::{Intent, Motion, MotionParams};

use crate::champion::ChampionConfig;
use crate::scene::WorldCollision;

const TICK_SECONDS: f32 = 1.0 / TICK_HZ as f32;
/// Ticks of making no headway toward a point before asking for a new path.
const STUCK_TICKS: u32 = 15;
const FULL_TURN: f32 = 65536.0;
/// The most a step lifts the lightest champion, in world units.
const STEP_LIFT: f32 = 14.0;
/// The first bounce of the lightest champion as it comes to rest.
const SETTLE_LIFT: f32 = 10.0;
/// How high the lightest champion floats while standing.
const HOVER_LIFT: f32 = 7.0;
/// Seconds for one rise and fall of that float.
const HOVER_PERIOD: f32 = 2.6;

/// What the player's champion should be doing this frame, in world terms.
#[derive(Clone, Copy, Debug)]
pub enum Wish {
    /// Come to rest.
    Rest,
    /// Come to rest facing this yaw.
    Face(f32),
    /// Head for a point; `arrive` stops on it.
    Seek { target: Vec3, arrive: bool },
    /// Run in this direction.
    Steer(Vec3),
}

/// Where the champion is drawn this frame, and what happened getting there.
#[derive(Clone, Copy, Debug)]
pub struct Driven {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    /// World units covered since the last frame.
    pub moved: f32,
    pub moving: bool,
    /// A `Seek` with `arrive` reached its point.
    pub arrived: bool,
    /// It has been unable to make headway toward its point.
    pub stuck: bool,
    /// For `Face`: how far it still is from that yaw, in radians.
    pub facing_error: f32,
    /// How far above the ground to draw the body. Drawing only.
    pub lift: f32,
}

pub struct ChampionMotion {
    turn_ms: i32,
    accel_ms: i32,
    stop_ms: i32,
    stride: f32,
    walk_loop_ms: f32,
    motion: Motion,
    previous: Motion,
    accumulator: f32,
    last: (f32, f32),
    stuck_ticks: u32,
    /// Lightness squared times the champion's bounce: 0 gives no spring.
    spring: f32,
    travelled: f32,
    resting: f32,
    was_moving: bool,
    lift: f32,
}

fn to_units(world: f32, origin_tile: i32) -> i32 {
    ((world - (origin_tile * TILE) as f32) * FP as f32).round() as i32
}

fn to_world(units: f32, origin_tile: i32) -> f32 {
    units / FP as f32 + (origin_tile * TILE) as f32
}

fn heading_from_yaw(yaw: f32) -> u16 {
    ((yaw / TAU * FULL_TURN).round() as i64).rem_euclid(65536) as u16
}

impl ChampionMotion {
    /// `walk_loop_ms` is the length of one pass through the walk animation.
    pub fn new(
        champion: &ChampionConfig,
        walk_loop_ms: f32,
        pos: Vec3,
        collision: &WorldCollision,
    ) -> Self {
        let (ox, oy) = collision.origin_tile();
        let motion = Motion::at(to_units(pos.x, ox), to_units(pos.z, oy), 0);
        let handling = champion.handling();
        Self {
            turn_ms: (handling.turn_time * 1000.0).round() as i32,
            accel_ms: (handling.accel_time * 1000.0).round() as i32,
            stop_ms: (handling.stop_time * 1000.0).round() as i32,
            // with no stride given, one that plays the walk at its authored
            // rate at top speed, and slower in step with the champion
            stride: if champion.stride > 0.0 {
                champion.stride
            } else {
                champion.speed * walk_loop_ms / 1000.0
            },
            walk_loop_ms,
            motion,
            previous: motion,
            accumulator: 0.0,
            last: (pos.x, pos.z),
            stuck_ticks: 0,
            spring: handling.lightness * handling.lightness * champion.bounce.max(0.0),
            travelled: 0.0,
            resting: 0.0,
            was_moving: false,
            lift: 0.0,
        }
    }

    /// The height to draw the body at this frame: a hop on each footfall
    /// while moving, a dying bounce as it stops, then a slow float.
    fn lift_for(&mut self, dt: f32, moved: f32, moving: bool) -> f32 {
        if self.spring <= 0.0 {
            return 0.0;
        }
        let want = if moving {
            self.was_moving = true;
            self.resting = 0.0;
            self.travelled += moved;
            // two footfalls to a stride
            let phase = if self.stride > 0.0 {
                self.travelled / self.stride * 2.0
            } else {
                0.0
            };
            STEP_LIFT * (phase * PI).sin().abs()
        } else {
            if self.was_moving {
                self.was_moving = false;
                self.resting = 0.0;
                self.travelled = 0.0;
            }
            self.resting += dt;
            let settle =
                SETTLE_LIFT * (-self.resting / 0.12).exp() * (self.resting * PI / 0.11).sin().abs();
            let eased = (self.resting / 0.6).min(1.0);
            let hover =
                HOVER_LIFT * eased * (0.5 - 0.5 * (self.resting * TAU / HOVER_PERIOD).cos());
            settle + hover
        };
        // ease toward it, so changing between the three never pops
        let blend = 1.0 - (-dt / 0.04).exp();
        self.lift += (want * self.spring - self.lift) * blend;
        self.lift.max(0.0)
    }

    /// How far to advance the walk animation for a distance covered. `None`
    /// when there is no walk cycle to measure against, which leaves the
    /// animation on the clock.
    pub fn walk_ms_for(&self, moved: f32) -> Option<f32> {
        (self.stride > 0.0 && self.walk_loop_ms > 0.0)
            .then(|| moved / self.stride * self.walk_loop_ms)
    }

    /// Advance by one frame. `pos` and `yaw` are where the champion is now;
    /// if something else has moved it (a respawn, a blink) motion restarts
    /// from there. `speed` is its current top speed in world units a second.
    #[allow(clippy::too_many_arguments)]
    pub fn drive(
        &mut self,
        dt: f32,
        wish: Wish,
        speed: f32,
        speed_mult: f32,
        pos: Vec3,
        yaw: f32,
        collision: &WorldCollision,
    ) -> Driven {
        let (ox, oy) = collision.origin_tile();
        if (pos.x - self.last.0).abs() > 1.0 || (pos.z - self.last.1).abs() > 1.0 {
            self.motion = Motion::at(
                to_units(pos.x, ox),
                to_units(pos.z, oy),
                heading_from_yaw(yaw),
            );
            self.previous = self.motion;
            self.accumulator = 0.0;
            self.stuck_ticks = 0;
            // the jump itself is not distance walked
            self.last = (pos.x, pos.z);
        }

        let params = MotionParams {
            speed: speed.round() as i32,
            turn_ms: self.turn_ms,
            accel_ms: self.accel_ms,
            stop_ms: self.stop_ms,
        };
        let permille = (speed_mult * 1000.0).round() as i32;
        let intent = match wish {
            Wish::Rest => Intent::Idle,
            Wish::Face(yaw) => Intent::Face {
                heading: heading_from_yaw(yaw),
            },
            Wish::Seek { target, arrive } => Intent::Seek {
                target: (to_units(target.x, ox), to_units(target.z, oy)),
                arrive,
            },
            Wish::Steer(direction) => {
                let (dx, dz) = (direction.x as i64, direction.z as i64);
                if dx == 0 && dz == 0 {
                    Intent::Idle
                } else {
                    Intent::Steer {
                        heading: heading_of(dx, dz),
                    }
                }
            }
        };
        let seeking = matches!(intent, Intent::Seek { .. });

        let mut arrived = false;
        let mut stuck = false;
        self.accumulator += dt.clamp(0.0, 0.25);
        while self.accumulator >= TICK_SECONDS {
            self.accumulator -= TICK_SECONDS;
            self.previous = self.motion;
            let stepped = self
                .motion
                .step(&params, permille, collision.grid(), intent);
            arrived |= stepped.arrived;
            if seeking && stepped.moved == 0 && !stepped.arrived {
                self.stuck_ticks += 1;
            } else {
                self.stuck_ticks = 0;
            }
            if self.stuck_ticks >= STUCK_TICKS {
                stuck = true;
                self.stuck_ticks = 0;
            }
        }

        // draw between the last two ticks
        let alpha = (self.accumulator / TICK_SECONDS).clamp(0.0, 1.0);
        let between = |a: i32, b: i32| a as f32 + (b - a) as f32 * alpha;
        let x = to_world(between(self.previous.x, self.motion.x), ox);
        let z = to_world(between(self.previous.y, self.motion.y), oy);
        let facing = self.previous.facing as f32
            + turn_between(self.previous.facing, self.motion.facing) as f32 * alpha;
        let moved = ((x - self.last.0).powi(2) + (z - self.last.1).powi(2)).sqrt();
        self.last = (x, z);

        let facing_error = match intent {
            Intent::Face { heading } => {
                turn_between(self.motion.facing, heading).abs() as f32 / FULL_TURN * TAU
            }
            _ => 0.0,
        };
        let moving = self.motion.speed > 0 || moved > 0.05;
        Driven {
            x,
            z,
            yaw: facing / FULL_TURN * TAU,
            moved,
            moving,
            arrived,
            stuck,
            facing_error,
            lift: self.lift_for(dt, moved, moving),
        }
    }
}
