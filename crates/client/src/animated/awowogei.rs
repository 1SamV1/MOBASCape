//! King Awowogei's kit.
//!
//! - **Q Bananarang**: the cache banana (model 2387) thrown at the cursor.
//!   It curves out on the king's right, slows into a hang at full range,
//!   then steers home on his left and is caught in his hand wherever he has
//!   moved. Every enemy is hit once going out and once (for half damage)
//!   coming back.
//! - **W Vine Swing**: a long vine drops from the canopy, the king runs and
//!   jumps onto it and swings along a curved arc that rises over hills and
//!   sets him down softly. Untargetable from the cast until 0.5 s after he
//!   lands.
//! - **E Banana Peel**: a peel lobbed onto a ground circle (it cannot be
//!   aimed at a unit). Landing under an enemy slips it at once; otherwise it
//!   waits as a trap for the first enemy to step on it.
//! - **R Greegree**: the king becomes an Ape Atoll gorilla (cache NPC 5275's
//!   models and animations) with bonus health and heavier, slower slams
//!   that splash around the target.
//!
//! Sounds are named in the champion file (`sounds`) and queued with
//! [`AnimatedScene::play_sound`].

use std::sync::Arc;

use glam::{Mat3, Vec3};
use rift_cache::{AnimationClip, Model};

use super::{
    AnimatedScene, Effect, Indicator, UnitAssets, UnitKind, append_banana_model, append_quad,
    append_ring_width, planar_delta, segment_distance_xz,
};
use crate::scene::{HeightGrid, WorldCollision};

/// Rank-1 cooldowns of Q, W, E and R (the champion file's ranks override).
pub const COOLDOWNS: [f32; 4] = [8.0, 14.0, 12.0, 80.0];
/// Mana for Q, W, E and R.
pub const COSTS: [f32; 4] = [40.0, 50.0, 40.0, 100.0];

const BANANA_MODEL: u32 = 2387;
/// The banana item mesh is tiny; this makes it a comically large boomerang.
const BANANA_SCALE: f32 = 6.25;
const PEEL_SCALE: f32 = 3.4;

/// Bananarang reach at rank 1.
pub const Q_RANGE: f32 = 1_300.0;
/// Top speed on the way home.
const Q_RETURN_SPEED: f32 = 1_900.0;
/// How fast it picks up speed after the hang (units/s^2).
const Q_RETURN_ACCEL: f32 = 2_600.0;
/// Caught when this close to the king's hand.
const Q_CATCH: f32 = 90.0;
/// A banana that has not come home by then drops.
const Q_LIFETIME: f32 = 6.0;
const Q_HIT_RADIUS: f32 = 80.0;
/// Damage of a return-leg hit relative to an outbound hit.
const Q_RETURN_DAMAGE: f32 = 0.5;
const Q_WHIRL_INTERVAL: f32 = 0.36;
/// Height the banana cruises above the ground.
const Q_CRUISE: f32 = 80.0;

/// Shortest and rank-1 longest Vine Swing.
pub const VINE_MIN: f32 = 300.0;
pub const VINE_RANGE: f32 = 900.0;
/// Untargetable for this long after landing.
const VINE_AFTER: f32 = 0.5;
/// How far above the arc's top the vine is tied.
const VINE_HANG: f32 = 760.0;

const PEEL_RANGE: f32 = 850.0;
/// The aimed circle: an enemy inside it when the peel lands slips at once.
pub const PEEL_RADIUS: f32 = 110.0;
/// A waiting trap triggers on an enemy this close (plus its body).
const PEEL_TRIGGER: f32 = 60.0;
const PEEL_FLIGHT: f32 = 0.45;
const PEEL_LIFE: f32 = 25.0;
const PEEL_ARM: f32 = 0.2;
const PEEL_MAX: usize = 3;
const PEEL_STUN: f32 = 1.0;
const SLIP_TIME: f32 = 0.55;
const SLIP_SPEED: f32 = 640.0;

const GREEGREE_DURATION: f32 = 8.0;
const GORILLA_MODELS: [u32; 2] = [4810, 4812];
const GORILLA_IDLE: u32 = 1401;
const GORILLA_WALK: u32 = 1399;
const GORILLA_ATTACK: u32 = 1402;
const GORILLA_SCALE: f32 = 3.2;
const GORILLA_DAMAGE: f32 = 1.7;
const GORILLA_INTERVAL: f32 = 1.35;
pub const SLAM_RADIUS: f32 = 220.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BananaLeg {
    Out,
    Home,
}

pub struct Bananarang {
    pub pos: Vec3,
    /// Planar heading while flying home.
    pub heading: Vec3,
    pub speed: f32,
    pub leg: BananaLeg,
    pub age: f32,
    /// Seconds on the current leg.
    pub leg_age: f32,
    pub out_time: f32,
    /// Outbound quadratic Bezier (XZ): hand, control, apex.
    pub curve: [Vec3; 3],
    pub spin: f32,
    pub damage: f32,
    pub crit_chance: f32,
    pub stun: f32,
    pub hit_out: Vec<usize>,
    pub hit_home: Vec<usize>,
    pub whirl: f32,
    pub trail: Vec<Vec3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VinePhase {
    /// Running to the vine while it drops.
    Run,
    Swing,
    /// On the ground; the vine flies back up.
    Landed,
}

pub struct VineSwing {
    pub phase: VinePhase,
    pub t: f32,
    pub start: Vec3,
    pub grab: Vec3,
    pub land: Vec3,
    /// XZ Bezier control of the arc.
    pub control: Vec3,
    pub pivot: Vec3,
    pub run_time: f32,
    pub swing_time: f32,
    pub lift: f32,
    pub heights: (f32, f32),
    /// Where the vine's free end is (the king's hand while he holds it).
    pub rope_end: Vec3,
    pub release_end: Vec3,
}

pub struct PeelShot {
    pub from: Vec3,
    pub to: Vec3,
    pub t: f32,
    pub spin: f32,
    pub damage: f32,
}

pub struct PeelTrap {
    pub pos: Vec3,
    pub yaw: f32,
    pub age: f32,
    pub damage: f32,
}

/// An enemy sliding and spinning on a peel.
pub struct Slip {
    pub unit: usize,
    pub vel: Vec3,
    pub t: f32,
}

pub struct GorillaAssets {
    pub model: Arc<Model>,
    pub idle: Arc<AnimationClip>,
    pub walk: Option<Arc<AnimationClip>>,
    pub attack: Option<Arc<AnimationClip>>,
}

/// The king's own look and numbers, parked while he is a gorilla.
pub struct Greegree {
    pub remaining: f32,
    pub bonus_health: f32,
    pub slam: f32,
    model: Arc<Model>,
    idle: Arc<AnimationClip>,
    walk: Option<Arc<AnimationClip>>,
    scale: f32,
    attack_clip: Option<Arc<AnimationClip>>,
    cast_clip: Option<Arc<AnimationClip>>,
}

#[derive(Default)]
pub struct AwowogeiState {
    pub banana_model: Option<Arc<Model>>,
    pub gorilla: Option<GorillaAssets>,
    pub bananas: Vec<Bananarang>,
    pub vine: Option<VineSwing>,
    pub peel_shots: Vec<PeelShot>,
    pub peels: Vec<PeelTrap>,
    pub slips: Vec<Slip>,
    pub greegree: Option<Greegree>,
    /// Seconds the king cannot be targeted or damaged.
    pub untargetable: f32,
}

impl AwowogeiState {
    /// Load the banana mesh and the gorilla form.
    pub(super) fn load(assets: &mut UnitAssets<'_>) -> Self {
        let banana_model = assets
            .model(BANANA_MODEL)
            .inspect_err(|err| tracing::warn!("awowogei: banana model unavailable: {err:#}"))
            .ok();
        let gorilla = (|| -> anyhow::Result<GorillaAssets> {
            let parts = assets.merged_parts(GORILLA_MODELS[0], &GORILLA_MODELS[1..])?;
            Ok(GorillaAssets {
                model: Arc::new(Model::merge(&parts)),
                idle: assets.clip(GORILLA_IDLE)?,
                walk: assets.clip(GORILLA_WALK).ok(),
                attack: assets.clip(GORILLA_ATTACK).ok(),
            })
        })()
        .inspect_err(|err| tracing::warn!("awowogei: gorilla form unavailable: {err:#}"))
        .ok();
        Self {
            banana_model,
            gorilla,
            ..Self::default()
        }
    }

    /// The vine owns the king's movement.
    pub fn controls_player(&self) -> bool {
        self.vine
            .as_ref()
            .is_some_and(|vine| vine.phase != VinePhase::Landed)
    }
}

/// The ground shapes for each slot at rank 1.
pub fn indicators(slot: usize) -> Vec<Indicator> {
    match slot {
        0 => vec![Indicator::Boomerang { length: Q_RANGE }],
        1 => vec![Indicator::Swing { length: VINE_RANGE }],
        2 => vec![Indicator::Target {
            range: PEEL_RANGE,
            radius: PEEL_RADIUS,
        }],
        _ => vec![Indicator::SelfArea {
            radius: SLAM_RADIUS,
        }],
    }
}

fn right_of(dir: Vec3) -> Vec3 {
    Vec3::Y.cross(dir)
}

fn bezier(c: &[Vec3; 3], s: f32) -> Vec3 {
    let u = 1.0 - s;
    c[0] * (u * u) + c[1] * (2.0 * u * s) + c[2] * (s * s)
}

/// Outbound leg: from `origin` out to `length` along `dir`, bowing to the
/// right (`side` +1) or left (-1).
pub fn boomerang_curve(origin: Vec3, dir: Vec3, length: f32, side: f32) -> [Vec3; 3] {
    let flat = Vec3::new(origin.x, 0.0, origin.z);
    [
        flat,
        flat + dir * (length * 0.55) + right_of(dir) * (length * 0.36 * side),
        flat + dir * length,
    ]
}

/// Indicator paths: the outbound leg and the expected way home.
pub fn boomerang_preview(origin: Vec3, dir: Vec3, length: f32) -> (Vec<Vec3>, Vec<Vec3>) {
    let out = boomerang_curve(origin, dir, length, 1.0);
    let home = boomerang_curve(origin, dir, length, -1.0);
    let sample = |c: &[Vec3; 3], from: f32, to: f32| {
        (0..=20)
            .map(|i| bezier(c, from + (to - from) * i as f32 / 20.0))
            .collect::<Vec<_>>()
    };
    (sample(&out, 0.0, 1.0), sample(&home, 1.0, 0.0))
}

/// Outbound progress: fast off the hand, easing to a stop at the apex.
fn out_ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    1.0 - (1.0 - u) * (1.0 - u)
}

fn out_time(length: f32) -> f32 {
    0.55 + length / 2_000.0
}

/// One step of the homing return: turn the heading toward `target` at a
/// rate that grows with time (so it always closes in) and pick up speed.
pub fn steer_home(
    pos: Vec3,
    heading: Vec3,
    leg_age: f32,
    target: Vec3,
    dt: f32,
) -> (Vec3, Vec3, f32) {
    let to = Vec3::new(target.x - pos.x, 0.0, target.z - pos.z);
    let desired = to.normalize_or_zero();
    let mut heading = Vec3::new(heading.x, 0.0, heading.z).normalize_or(desired);
    if desired.length_squared() > 0.0 {
        let max_turn = (3.0 + 6.0 * leg_age) * dt;
        let angle = heading.angle_between(desired);
        if angle <= max_turn {
            heading = desired;
        } else {
            let sign = if heading.cross(desired).y >= 0.0 {
                1.0
            } else {
                -1.0
            };
            heading = Mat3::from_rotation_y(max_turn * sign) * heading;
        }
    }
    let speed = (120.0 + Q_RETURN_ACCEL * leg_age).min(Q_RETURN_SPEED);
    (pos + heading * speed * dt, heading, speed)
}

/// Swing timing: slow at the grab and the landing, fastest at the bottom
/// of the arc, like a pendulum.
fn swing_ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u - (std::f32::consts::TAU * u).sin() / std::f32::consts::TAU * 0.6
}

fn smoothstep(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The arc's ground track for a swing of `reach` toward `dir`.
fn swing_track(grab: Vec3, land: Vec3) -> Vec3 {
    let span = planar_delta(grab, land);
    let dir = span.normalize_or_zero();
    (grab + land) * 0.5 + right_of(dir) * (span.length() * 0.22)
}

/// Indicator path of a swing (XZ only).
pub fn swing_preview(origin: Vec3, dir: Vec3, reach: f32) -> Vec<Vec3> {
    let run = (reach * 0.15).min(150.0);
    let grab = origin + dir * run;
    let land = origin + dir * reach;
    let control = swing_track(grab, land);
    let curve = [grab, control, land];
    let mut points = vec![origin];
    points.extend((0..=20).map(|i| bezier(&curve, i as f32 / 20.0)));
    points
}

/// Position on the swing at progress `s` (0 grab .. 1 land).
fn swing_point(vine: &VineSwing, s: f32) -> Vec3 {
    let mut p = bezier(&[vine.grab, vine.control, vine.land], s);
    let (h0, h1) = vine.heights;
    p.y = h0 + (h1 - h0) * smoothstep(s) + vine.lift * 4.0 * s * (1.0 - s);
    p
}

/// How high the arc has to rise to clear the ground along the track.
fn swing_lift(curve: &[Vec3; 3], heights: (f32, f32), height: &HeightGrid) -> f32 {
    let span = planar_delta(curve[0], curve[2]).length();
    let mut lift = 140.0 + span * 0.16;
    for i in 1..16 {
        let s = i as f32 / 16.0;
        let p = bezier(curve, s);
        let base = heights.0 + (heights.1 - heights.0) * smoothstep(s);
        let ground = height.height_at(p.x, p.z);
        let needed = (ground - base + 90.0) / (4.0 * s * (1.0 - s));
        lift = lift.max(needed);
    }
    lift.min(900.0)
}

/// The peel's lob at progress `t`.
fn peel_point(shot: &PeelShot, t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0);
    shot.from.lerp(shot.to, t) + Vec3::Y * (240.0 * 4.0 * t * (1.0 - t))
}

impl AnimatedScene {
    /// The king cannot be targeted (Vine Swing, or a kit's untargetable
    /// dash).
    pub(super) fn player_untargetable(&self) -> bool {
        self.awowogei.untargetable > 0.0 || self.kit_untargetable()
    }

    /// The slot's authored damage at its rank, times ability power and
    /// level.
    fn ability_damage(&self, slot: usize) -> f32 {
        self.rank_entry(slot, self.ability_rank(slot))
            .and_then(|rank| rank.damage)
            .unwrap_or(0.0)
            * self.ability_power_mult()
    }

    /// Mana, cooldown and state checks shared by the four casts.
    fn awowogei_ready(&self, slot: usize) -> bool {
        let player = &self.units[self.player_index];
        player.health > 0.0
            && player.stun_timer <= 0.0
            && player.cooldowns[slot] <= 0.0
            && player.mana >= COSTS[slot]
            && !self.awowogei.controls_player()
    }

    fn awowogei_spend(&mut self, slot: usize, fallback: f32) {
        let cooldown = self.rank_cooldown(slot, fallback);
        let player = &mut self.units[self.player_index];
        player.mana -= COSTS[slot];
        player.cooldowns[slot] = cooldown;
    }

    fn aim_dir(&self, target: Vec3) -> Vec3 {
        let player = &self.units[self.player_index];
        planar_delta(player.pos, target)
            .try_normalize()
            .unwrap_or(Vec3::new(player.yaw.sin(), 0.0, player.yaw.cos()))
    }

    /// The king's throwing hand.
    fn awowogei_hand(&self) -> Vec3 {
        let player = &self.units[self.player_index];
        let forward = Vec3::new(player.yaw.sin(), 0.0, player.yaw.cos());
        player.pos + Vec3::Y * (player.scale * 55.0) + right_of(forward) * 28.0
    }

    /// Q: throw the Bananarang toward `target`.
    pub(super) fn cast_bananarang(&mut self, target: Vec3) -> bool {
        if !self.awowogei_ready(0) {
            return false;
        }
        let dir = self.aim_dir(target);
        let length = Q_RANGE * self.rank_reach_mult(0);
        let damage = self.ability_damage(0);
        let rank = self.ability_rank(0).saturating_sub(1) as f32;
        self.awowogei_spend(0, COOLDOWNS[0]);
        let pi = self.player_index;
        self.units[pi].yaw = dir.x.atan2(dir.z);
        let hand = self.awowogei_hand();
        let curve = boomerang_curve(hand, dir, length, 1.0);
        self.awowogei.bananas.push(Bananarang {
            pos: hand,
            heading: dir,
            speed: 0.0,
            leg: BananaLeg::Out,
            age: 0.0,
            leg_age: 0.0,
            out_time: out_time(length),
            curve,
            spin: 0.0,
            damage,
            crit_chance: 0.10 + 0.04 * rank,
            stun: 0.10 + 0.05 * rank,
            hit_out: Vec::new(),
            hit_home: Vec::new(),
            whirl: Q_WHIRL_INTERVAL,
            trail: Vec::new(),
        });
        self.effects.push(Effect {
            pos: hand,
            radius: 90.0,
            remaining: 0.18,
            duration: 0.18,
            color: [255, 220, 60],
        });
        self.emit_burst(hand, [255, 220, 60], 10, 220.0, 16.0, 0.4);
        self.play_sound("q_throw", hand);
        true
    }

    /// W: drop a vine and swing toward `target`.
    pub(super) fn cast_vine_swing(
        &mut self,
        target: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> bool {
        if !self.awowogei_ready(1) {
            return false;
        }
        let dir = self.aim_dir(target);
        let pi = self.player_index;
        let start = self.units[pi].pos;
        let longest = VINE_RANGE * self.rank_reach_mult(1);
        let wanted = planar_delta(start, target)
            .length()
            .clamp(VINE_MIN, longest);
        // Land on the furthest walkable spot along the aim.
        let mut reach = wanted;
        let land = loop {
            let p = start + dir * reach;
            if collision.is_walkable_world(p.x, p.z) {
                break Some(Vec3::new(p.x, height.height_at(p.x, p.z), p.z));
            }
            reach -= 32.0;
            if reach < VINE_MIN * 0.5 {
                break None;
            }
        };
        let Some(land) = land else {
            return false;
        };
        let mut run = (reach * 0.15).min(150.0);
        let mut grab = start + dir * run;
        if !collision.is_walkable_world(grab.x, grab.z) {
            run = 0.0;
            grab = start;
        }
        grab.y = height.height_at(grab.x, grab.z);
        let control = swing_track(grab, land);
        let curve = [grab, control, land];
        let heights = (grab.y, land.y);
        let lift = swing_lift(&curve, heights, height);
        let mid = bezier(&curve, 0.5);
        let top = heights.0.max(heights.1) + lift;
        let pivot = Vec3::new(mid.x, top + VINE_HANG, mid.z);
        let speed = self.units[pi].speed.max(1.0);
        let run_time = (run / (speed * 1.8)).clamp(0.14, 0.3);
        let span = planar_delta(grab, land).length();
        let swing_time = 0.6 + span / 2_200.0;
        self.awowogei_spend(1, COOLDOWNS[1]);
        self.awowogei.untargetable = run_time + swing_time + VINE_AFTER;
        {
            let player = &mut self.units[pi];
            player.path.clear();
            player.path_index = 0;
            player.target = None;
            player.attack_target = None;
            player.attack_pending = false;
            player.yaw = dir.x.atan2(dir.z);
        }
        self.awowogei.vine = Some(VineSwing {
            phase: VinePhase::Run,
            t: 0.0,
            start,
            grab,
            land,
            control,
            pivot,
            run_time,
            swing_time,
            lift,
            heights,
            rope_end: pivot,
            release_end: pivot,
        });
        self.emit_burst(start, [90, 170, 60], 10, 160.0, 14.0, 0.5);
        self.play_sound("w_swing", start);
        true
    }

    /// E: lob a banana peel onto the ground at `target`.
    pub(super) fn cast_banana_peel(&mut self, target: Vec3, height: &HeightGrid) -> bool {
        if !self.awowogei_ready(2) {
            return false;
        }
        let dir = self.aim_dir(target);
        let pi = self.player_index;
        let start = self.units[pi].pos;
        let range = PEEL_RANGE * self.rank_reach_mult(2);
        let reach = planar_delta(start, target).length().min(range);
        let mut to = start + dir * reach;
        to.y = height.height_at(to.x, to.z);
        let damage = self.ability_damage(2);
        self.awowogei_spend(2, COOLDOWNS[2]);
        self.units[pi].yaw = dir.x.atan2(dir.z);
        let from = self.awowogei_hand();
        self.awowogei.peel_shots.push(PeelShot {
            from,
            to,
            t: 0.0,
            spin: 0.0,
            damage,
        });
        self.play_sound("e_throw", from);
        true
    }

    /// R: become a gorilla.
    pub(super) fn cast_greegree(&mut self) -> bool {
        if !self.awowogei_ready(3) || self.awowogei.greegree.is_some() {
            return false;
        }
        let Some(gorilla) = self.awowogei.gorilla.as_ref() else {
            tracing::warn!("greegree: gorilla form not loaded");
            return false;
        };
        let (model, idle, walk, attack) = (
            gorilla.model.clone(),
            gorilla.idle.clone(),
            gorilla.walk.clone(),
            gorilla.attack.clone(),
        );
        let rank = self.ability_rank(3).saturating_sub(1) as f32;
        let bonus_health = 250.0 + 150.0 * rank;
        let slam = self.ability_damage(3);
        self.awowogei_spend(3, COOLDOWNS[3]);
        let pi = self.player_index;
        let player = &mut self.units[pi];
        let greegree = Greegree {
            remaining: GREEGREE_DURATION,
            bonus_health,
            slam,
            model: std::mem::replace(&mut player.model, model),
            idle: std::mem::replace(&mut player.idle, idle),
            walk: std::mem::replace(&mut player.walk, walk),
            scale: std::mem::replace(&mut player.scale, GORILLA_SCALE),
            attack_clip: std::mem::replace(&mut player.attack_clip, attack),
            cast_clip: player.cast_clip.take(),
        };
        player.action = None;
        player.max_health += bonus_health;
        player.health += bonus_health;
        player.attack_damage *= GORILLA_DAMAGE;
        player.attack_interval *= GORILLA_INTERVAL;
        let pos = player.pos;
        self.awowogei.greegree = Some(greegree);
        self.effects.push(Effect {
            pos,
            radius: 220.0,
            remaining: 0.6,
            duration: 0.6,
            color: [120, 200, 90],
        });
        self.emit_burst(pos, [110, 190, 70], 36, 340.0, 20.0, 0.9);
        self.emit_burst(pos, [90, 60, 30], 18, 260.0, 16.0, 0.8);
        self.play_sound("r_transform", pos);
        true
    }

    fn revert_greegree(&mut self) {
        let Some(state) = self.awowogei.greegree.take() else {
            return;
        };
        let pi = self.player_index;
        let player = &mut self.units[pi];
        player.model = state.model;
        player.idle = state.idle;
        player.walk = state.walk;
        player.scale = state.scale;
        player.attack_clip = state.attack_clip;
        player.cast_clip = state.cast_clip;
        player.action = None;
        player.attack_damage /= GORILLA_DAMAGE;
        player.attack_interval /= GORILLA_INTERVAL;
        player.max_health -= state.bonus_health;
        if player.health > 0.0 {
            player.health = player.health.min(player.max_health).max(1.0);
        }
        let pos = player.pos;
        self.emit_burst(pos, [150, 210, 120], 24, 260.0, 16.0, 0.7);
        self.play_sound("r_revert", pos);
    }

    /// A gorilla basic attack: the slam splashes around the target.
    pub(super) fn greegree_slam(&mut self, target: usize) {
        let Some(slam) = self.awowogei.greegree.as_ref().map(|g| g.slam) else {
            return;
        };
        let pi = self.player_index;
        let at = self.units[target].pos;
        let hits: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                self.is_awowogei_enemy(i)
                    && planar_delta(at, self.units[i].pos).length()
                        <= SLAM_RADIUS + self.units[i].body_radius
            })
            .collect();
        for index in hits {
            self.damage_unit_mixed(index, slam, 0.0, 0, Some(pi));
        }
        self.effects.push(Effect {
            pos: at,
            radius: SLAM_RADIUS,
            remaining: 0.35,
            duration: 0.35,
            color: [200, 150, 80],
        });
        self.emit_burst(at, [130, 100, 60], 18, 320.0, 18.0, 0.6);
        self.add_decal(at, SLAM_RADIUS * 0.6, [60, 45, 25], 3.0);
        self.play_sound("r_slam", at);
    }

    /// Enemies the kit can hit: hostile, alive, not a building or prop.
    fn is_awowogei_enemy(&self, index: usize) -> bool {
        let unit = &self.units[index];
        unit.health > 0.0
            && unit.spawn_delay <= 0.0
            && (unit.team == 1 || (unit.team == 2 && unit.kind == UnitKind::Camp))
            && !unit.kind.is_structure()
            && !matches!(
                unit.kind,
                UnitKind::DeliveryGuard | UnitKind::StorageChest | UnitKind::Ambient
            )
    }

    /// Advance every piece of the kit.
    pub(super) fn update_awowogei(
        &mut self,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) {
        self.awowogei.untargetable = (self.awowogei.untargetable - dt).max(0.0);
        let alive = self.units[self.player_index].health > 0.0;
        if !alive {
            self.awowogei.vine = None;
            self.awowogei.untargetable = 0.0;
            self.revert_greegree();
        }
        if let Some(greegree) = self.awowogei.greegree.as_mut() {
            greegree.remaining -= dt;
            if greegree.remaining <= 0.0 {
                self.revert_greegree();
            }
        }
        self.update_vine(dt, height);
        self.update_bananas(dt, height);
        self.update_peels(dt, height);
        self.update_slips(dt, height, collision);
    }

    fn update_bananas(&mut self, dt: f32, height: &HeightGrid) {
        let mut bananas = std::mem::take(&mut self.awowogei.bananas);
        let hand = self.awowogei_hand();
        let alive = self.units[self.player_index].health > 0.0;
        bananas.retain_mut(|banana| {
            let previous = banana.pos;
            banana.age += dt;
            banana.leg_age += dt;
            banana.spin += dt * 19.0;
            let mut flat = match banana.leg {
                BananaLeg::Out => {
                    let s = out_ease(banana.leg_age / banana.out_time);
                    let p = bezier(&banana.curve, s);
                    if banana.leg_age >= banana.out_time {
                        banana.leg = BananaLeg::Home;
                        banana.leg_age = 0.0;
                        banana.heading = (banana.curve[2] - banana.curve[1]).normalize_or_zero();
                    }
                    p
                }
                BananaLeg::Home => {
                    let (p, heading, speed) =
                        steer_home(banana.pos, banana.heading, banana.leg_age, hand, dt);
                    banana.heading = heading;
                    banana.speed = speed;
                    p
                }
            };
            // Ride smoothly over the ground, with a small rise on the way out.
            let cruise = height.height_at(flat.x, flat.z) + Q_CRUISE;
            let bump = match banana.leg {
                BananaLeg::Out => {
                    let s = out_ease(banana.leg_age / banana.out_time);
                    50.0 * (std::f32::consts::PI * s).sin()
                }
                BananaLeg::Home => 0.0,
            };
            let ease = 1.0 - (-9.0 * dt).exp();
            flat.y = previous.y + (cruise + bump - previous.y) * ease;
            if banana.leg == BananaLeg::Home {
                // Settle into the hand height on the final approach.
                let near = (1.0 - planar_delta(flat, hand).length() / 400.0).clamp(0.0, 1.0);
                flat.y += (hand.y - flat.y) * near * ease;
            }
            banana.pos = flat;
            super::advance_trail(&mut banana.trail, flat, 10, banana.speed.max(600.0));

            // Hits, once per enemy per leg.
            let leg = banana.leg;
            for index in 0..self.units.len() {
                let already = match leg {
                    BananaLeg::Out => banana.hit_out.contains(&index),
                    BananaLeg::Home => banana.hit_home.contains(&index),
                };
                if already || !self.is_awowogei_enemy(index) {
                    continue;
                }
                let unit = &self.units[index];
                if segment_distance_xz(unit.pos, previous, flat) > Q_HIT_RADIUS + unit.hit_radius {
                    continue;
                }
                match leg {
                    BananaLeg::Out => banana.hit_out.push(index),
                    BananaLeg::Home => banana.hit_home.push(index),
                }
                let crit = self.next_random() < banana.crit_chance;
                let leg_mult = if leg == BananaLeg::Out {
                    1.0
                } else {
                    Q_RETURN_DAMAGE
                };
                let amount = banana.damage * leg_mult * if crit { 1.5 } else { 1.0 };
                self.damage_unit_mixed(
                    index,
                    amount * 0.6,
                    amount * 0.4,
                    0,
                    Some(self.player_index),
                );
                if crit {
                    let stun = banana.stun;
                    let unit = &mut self.units[index];
                    unit.stun_timer = unit.stun_timer.max(stun);
                }
                let color = if crit { [255, 90, 40] } else { [255, 220, 60] };
                self.effects.push(Effect {
                    pos: flat,
                    radius: if crit { 150.0 } else { 90.0 },
                    remaining: 0.25,
                    duration: 0.25,
                    color,
                });
                self.emit_burst(flat, color, if crit { 14 } else { 8 }, 260.0, 15.0, 0.45);
                self.play_sound("q_hit", flat);
            }

            banana.whirl -= dt;
            if banana.whirl <= 0.0 {
                banana.whirl += Q_WHIRL_INTERVAL;
                self.play_sound("q_whirl", flat);
            }

            if banana.leg == BananaLeg::Home {
                let reach = planar_delta(flat, hand).length();
                if alive && reach <= Q_CATCH + banana.speed * dt * 0.5 {
                    self.emit_burst(hand, [255, 230, 120], 8, 160.0, 12.0, 0.35);
                    self.play_sound("q_catch", hand);
                    return false;
                }
            }
            if banana.age >= Q_LIFETIME || (!alive && banana.leg == BananaLeg::Home) {
                self.emit_burst(flat, [255, 214, 60], 10, 180.0, 14.0, 0.5);
                return false;
            }
            true
        });
        self.awowogei.bananas = bananas;
    }

    fn update_vine(&mut self, dt: f32, height: &HeightGrid) {
        let Some(mut vine) = self.awowogei.vine.take() else {
            return;
        };
        let pi = self.player_index;
        vine.t += dt;
        // Arms raised to the vine: a little above the head bar's base.
        let hand_lift = self.units[pi].scale * 62.0;
        match vine.phase {
            VinePhase::Run => {
                let u = (vine.t / vine.run_time).clamp(0.0, 1.0);
                let mut pos = vine.start.lerp(vine.grab, u);
                pos.y = height.height_at(pos.x, pos.z);
                let player = &mut self.units[pi];
                player.pos = pos;
                player.moving = true;
                // The vine drops from the canopy to meet the jump.
                let drop = smoothstep((vine.t / (vine.run_time + 0.05)).min(1.0));
                vine.rope_end = vine.pivot.lerp(vine.grab + Vec3::Y * hand_lift, drop);
                if u >= 1.0 {
                    vine.phase = VinePhase::Swing;
                    vine.t = 0.0;
                    self.emit_burst(vine.grab, [120, 100, 60], 8, 140.0, 12.0, 0.4);
                }
            }
            VinePhase::Swing => {
                let u = vine.t / vine.swing_time;
                let s = swing_ease(u);
                let pos = swing_point(&vine, s);
                let ahead = swing_point(&vine, (s + 0.02).min(1.0));
                let player = &mut self.units[pi];
                let heading = planar_delta(pos, ahead);
                if heading.length_squared() > 1.0 {
                    player.yaw = heading.x.atan2(heading.z);
                }
                player.pos = pos;
                player.moving = false;
                let hand = pos + Vec3::Y * hand_lift;
                if s < 0.9 {
                    vine.rope_end = hand;
                    vine.release_end = hand;
                } else {
                    // Let go: the free end swings back under the pivot.
                    let k = ((s - 0.9) / 0.1).clamp(0.0, 1.0);
                    let rest = vine.pivot - Vec3::Y * (vine.pivot.y - hand.y) * 0.8;
                    vine.rope_end = vine.release_end.lerp(rest, smoothstep(k) * 0.5);
                }
                if u >= 1.0 {
                    let land = Vec3::new(
                        vine.land.x,
                        height.height_at(vine.land.x, vine.land.z),
                        vine.land.z,
                    );
                    let player = &mut self.units[pi];
                    player.pos = land;
                    player.path.clear();
                    player.path_index = 0;
                    player.target = None;
                    if let Some(motion) = player.motion.as_mut() {
                        motion.reset();
                    }
                    vine.phase = VinePhase::Landed;
                    vine.t = 0.0;
                    vine.release_end = vine.rope_end;
                    self.emit_burst(land, [140, 120, 80], 16, 240.0, 16.0, 0.6);
                    self.add_decal(land, 90.0, [70, 60, 40], 1.6);
                    self.play_sound("w_land", land);
                }
            }
            VinePhase::Landed => {
                // The vine is hauled back up into the canopy.
                let k = smoothstep(vine.t / 0.6);
                vine.rope_end = vine.release_end.lerp(vine.pivot, k);
                if vine.t >= 0.6 {
                    return;
                }
            }
        }
        self.awowogei.vine = Some(vine);
    }

    fn update_peels(&mut self, dt: f32, height: &HeightGrid) {
        let pi = self.player_index;
        let shots = std::mem::take(&mut self.awowogei.peel_shots);
        let mut flying = Vec::with_capacity(shots.len());
        for mut shot in shots {
            shot.t += dt / PEEL_FLIGHT;
            shot.spin += dt * 14.0;
            if shot.t < 1.0 {
                flying.push(shot);
                continue;
            }
            let at = Vec3::new(shot.to.x, height.height_at(shot.to.x, shot.to.z), shot.to.z);
            self.play_sound("e_land", at);
            self.emit_burst(at, [230, 200, 70], 6, 120.0, 10.0, 0.35);
            // Thrown right under an enemy: it slips straight away.
            let victim = (0..self.units.len())
                .filter(|&i| self.is_awowogei_enemy(i))
                .map(|i| (i, planar_delta(at, self.units[i].pos).length()))
                .filter(|&(i, d)| d <= PEEL_RADIUS + self.units[i].body_radius * 0.5)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i);
            match victim {
                Some(index) => self.slip(index, shot.damage, at),
                None => {
                    if self.awowogei.peels.len() >= PEEL_MAX {
                        self.awowogei.peels.remove(0);
                    }
                    let yaw = shot.spin;
                    self.awowogei.peels.push(PeelTrap {
                        pos: at,
                        yaw,
                        age: 0.0,
                        damage: shot.damage,
                    });
                }
            }
        }
        self.awowogei.peel_shots = flying;

        let traps = std::mem::take(&mut self.awowogei.peels);
        let mut waiting = Vec::with_capacity(traps.len());
        for mut trap in traps {
            trap.age += dt;
            if trap.age >= PEEL_LIFE {
                continue;
            }
            let victim = (trap.age >= PEEL_ARM)
                .then(|| {
                    (0..self.units.len()).find(|&i| {
                        self.is_awowogei_enemy(i)
                            && planar_delta(trap.pos, self.units[i].pos).length()
                                <= PEEL_TRIGGER + self.units[i].body_radius
                    })
                })
                .flatten();
            match victim {
                Some(index) => self.slip(index, trap.damage, trap.pos),
                None => waiting.push(trap),
            }
        }
        self.awowogei.peels = waiting;
        let _ = pi;
    }

    /// An enemy steps on a peel: damage, a stun, and a spinning slide in
    /// the direction it was heading.
    fn slip(&mut self, index: usize, damage: f32, at: Vec3) {
        let pi = self.player_index;
        let unit = &self.units[index];
        let facing = Vec3::new(unit.yaw.sin(), 0.0, unit.yaw.cos());
        let away = planar_delta(at, unit.pos).normalize_or(facing);
        let dir = if unit.moving { facing } else { away };
        self.damage_unit_mixed(index, damage, 0.0, 0, Some(pi));
        let unit = &mut self.units[index];
        unit.stun_timer = unit.stun_timer.max(SLIP_TIME + PEEL_STUN);
        unit.moving = false;
        unit.attack_target = None;
        let pos = unit.pos;
        self.awowogei.slips.retain(|slip| slip.unit != index);
        self.awowogei.slips.push(Slip {
            unit: index,
            vel: dir * SLIP_SPEED,
            t: 0.0,
        });
        self.emit_burst(pos, [255, 220, 70], 12, 220.0, 14.0, 0.5);
        self.play_sound("e_slip", pos);
        tracing::info!("banana peel: unit {index} slipped for {damage:.0}");
    }

    fn update_slips(&mut self, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        let mut slips = std::mem::take(&mut self.awowogei.slips);
        slips.retain_mut(|slip| {
            slip.t += dt;
            let Some(unit) = self.units.get_mut(slip.unit) else {
                return false;
            };
            if unit.health <= 0.0 {
                return false;
            }
            let k = (slip.t / SLIP_TIME).clamp(0.0, 1.0);
            let step = slip.vel * (1.0 - k) * dt;
            let next = unit.pos + step;
            if collision.is_walkable_world(next.x, next.z) {
                unit.pos.x = next.x;
                unit.pos.z = next.z;
            }
            let ground = height.height_at(unit.pos.x, unit.pos.z);
            unit.pos.y = ground + 70.0 * (std::f32::consts::PI * k).sin();
            unit.yaw += dt * 16.0 * (1.0 - k);
            if k >= 1.0 {
                unit.pos.y = ground;
                return false;
            }
            true
        });
        self.awowogei.slips = slips;
    }

    /// Draw the bananas, peels and vine.
    pub(super) fn draw_awowogei(&mut self, height: &HeightGrid, right: Vec3, up: Vec3) {
        let forward = right.cross(up);
        let model = self.awowogei.banana_model.take();
        let bananas: Vec<(Vec3, Mat3, Vec<Vec3>)> = self
            .awowogei
            .bananas
            .iter()
            .map(|banana| {
                let heading = match banana.leg {
                    BananaLeg::Home => banana.heading,
                    BananaLeg::Out => {
                        let s = out_ease(banana.leg_age / banana.out_time);
                        let c = &banana.curve;
                        (2.0 * (1.0 - s) * (c[1] - c[0]) + 2.0 * s * (c[2] - c[1]))
                            .normalize_or_zero()
                    }
                };
                let axis = heading.try_normalize().unwrap_or(Vec3::Z);
                let rot = Mat3::from_axis_angle(axis, 0.35) * Mat3::from_rotation_y(banana.spin);
                (banana.pos, rot, banana.trail.clone())
            })
            .collect();
        for (pos, rot, trail) in bananas {
            match model.as_ref() {
                Some(model) => append_banana_model(self, model, pos, rot, BANANA_SCALE),
                None => self.fx.glow(pos, 40.0, [255, 214, 45], 1.0, right, up),
            }
            self.fx.ribbon(&trail, 22.0, [255, 225, 90], 0.55, forward);
        }
        let shots: Vec<(Vec3, f32)> = self
            .awowogei
            .peel_shots
            .iter()
            .map(|shot| (peel_point(shot, shot.t), shot.spin))
            .collect();
        for (pos, spin) in shots {
            match model.as_ref() {
                Some(model) => append_banana_model(
                    self,
                    model,
                    pos,
                    Mat3::from_rotation_x(spin) * Mat3::from_rotation_y(spin * 0.5),
                    PEEL_SCALE,
                ),
                None => self.fx.glow(pos, 24.0, [255, 214, 45], 1.0, right, up),
            }
        }
        let traps: Vec<(Vec3, f32, f32)> = self
            .awowogei
            .peels
            .iter()
            .map(|trap| (trap.pos, trap.yaw, trap.age))
            .collect();
        for (pos, yaw, age) in traps {
            let lie = Vec3::new(pos.x, height.height_at(pos.x, pos.z) + 10.0, pos.z);
            if let Some(model) = model.as_ref() {
                append_banana_model(
                    self,
                    model,
                    lie,
                    Mat3::from_rotation_y(yaw) * Mat3::from_rotation_z(0.25),
                    PEEL_SCALE,
                );
            }
            let pulse = 0.6 + 0.4 * (age * 3.0).sin().abs();
            let color = [(200.0 * pulse) as u8, (170.0 * pulse) as u8, 50];
            append_ring_width(self, height, pos, PEEL_TRIGGER + 20.0, color, 0.88);
        }
        self.awowogei.banana_model = model;

        if let Some(vine) = self.awowogei.vine.as_ref() {
            let (top, end) = (vine.pivot, vine.rope_end);
            draw_vine(self, top, end, forward);
        }
    }
}

/// A long vine from `top` down to `end`: a twisted brown-green rope with
/// leaves.
fn draw_vine(scene: &mut AnimatedScene, top: Vec3, end: Vec3, forward: Vec3) {
    let segments = 24;
    let along = end - top;
    let length = along.length();
    if length < 1.0 {
        return;
    }
    let side = along.cross(forward).normalize_or(Vec3::X);
    let sag = side.cross(along).normalize_or(Vec3::Y) * (length * 0.03);
    let point = |i: usize| {
        let s = i as f32 / segments as f32;
        top + along * s + sag * (4.0 * s * (1.0 - s))
    };
    for i in 0..segments {
        let (a, b) = (point(i), point(i + 1));
        let half = (b - a) * 0.5;
        let width = side * 9.0;
        let color = if i % 2 == 0 {
            [86, 116, 46]
        } else {
            [104, 122, 54]
        };
        append_quad(scene, a + half, half, width, color);
        if i % 4 == 2 {
            let leaf = a + half + side * 14.0;
            append_quad(
                scene,
                leaf,
                side * 10.0,
                half.normalize_or(Vec3::Y) * 7.0,
                [60, 130, 44],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outbound_leg_bows_right_and_reaches_full_range() {
        let curve = boomerang_curve(Vec3::ZERO, Vec3::Z, 1300.0, 1.0);
        assert!((bezier(&curve, 1.0) - Vec3::new(0.0, 0.0, 1300.0)).length() < 0.01);
        let mid = bezier(&curve, 0.5);
        assert!(mid.x > 200.0, "bows to the right (+X): {mid:?}");
        let (out, home) = boomerang_preview(Vec3::ZERO, Vec3::Z, 1300.0);
        assert!(home[home.len() / 2].x < -200.0, "comes home on the left");
        assert_eq!(out.last(), home.first(), "the legs meet at the apex");
    }

    #[test]
    fn it_leaves_fast_and_hangs_at_the_apex() {
        let start = out_ease(0.05) - out_ease(0.0);
        let end = out_ease(1.0) - out_ease(0.95);
        assert!(start > end * 10.0, "{start} vs {end}");
        assert_eq!(out_ease(1.0), 1.0);
    }

    #[test]
    fn the_return_always_comes_home_even_to_a_moving_king() {
        let curve = boomerang_curve(Vec3::ZERO, Vec3::Z, 1300.0, 1.0);
        let mut pos = curve[2];
        let mut heading = (curve[2] - curve[1]).normalize();
        let mut king = Vec3::ZERO;
        let dt = 1.0 / 60.0;
        let mut caught = None;
        for frame in 0..600 {
            let age = frame as f32 * dt;
            king += Vec3::new(300.0, 0.0, -150.0) * dt; // walking away
            let (p, h, speed) = steer_home(pos, heading, age, king, dt);
            pos = p;
            heading = h;
            if planar_delta(pos, king).length() <= Q_CATCH + speed * dt * 0.5 {
                caught = Some(age);
                break;
            }
        }
        let age = caught.expect("caught");
        assert!(age < 3.0, "took {age}s");
    }

    #[test]
    fn the_return_turns_through_the_left() {
        let curve = boomerang_curve(Vec3::ZERO, Vec3::Z, 1300.0, 1.0);
        let mut pos = curve[2];
        let mut heading = (curve[2] - curve[1]).normalize();
        let mut min_x: f32 = 0.0;
        for frame in 0..240 {
            let (p, h, _) = steer_home(pos, heading, frame as f32 / 60.0, Vec3::ZERO, 1.0 / 60.0);
            pos = p;
            heading = h;
            min_x = min_x.min(pos.x);
        }
        assert!(min_x < -50.0, "loops out to the left: {min_x}");
    }

    #[test]
    fn the_swing_is_slow_at_the_ends_and_fast_in_the_middle() {
        let speed = |u: f32| (swing_ease(u + 0.01) - swing_ease(u)) / 0.01;
        assert!(speed(0.5) > speed(0.0) * 3.0);
        assert!(speed(0.5) > speed(0.99) * 3.0);
        assert!(swing_ease(0.0).abs() < 1e-6 && (swing_ease(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_swing_lands_on_the_target_height() {
        let vine = VineSwing {
            phase: VinePhase::Swing,
            t: 0.0,
            start: Vec3::ZERO,
            grab: Vec3::ZERO,
            land: Vec3::new(0.0, 0.0, 900.0),
            control: swing_track(Vec3::ZERO, Vec3::new(0.0, 0.0, 900.0)),
            pivot: Vec3::ZERO,
            run_time: 0.2,
            swing_time: 1.0,
            lift: 300.0,
            heights: (10.0, 240.0),
            rope_end: Vec3::ZERO,
            release_end: Vec3::ZERO,
        };
        assert!((swing_point(&vine, 0.0).y - 10.0).abs() < 0.01);
        let end = swing_point(&vine, 1.0);
        assert!((end.y - 240.0).abs() < 0.01, "lands up on the hill");
        assert!((end.z - 900.0).abs() < 0.01);
        assert!(swing_point(&vine, 0.5).y > 240.0, "rises above both ends");
        let preview = swing_preview(Vec3::ZERO, Vec3::Z, 900.0);
        assert!(preview[10].x > 50.0, "curves to the side");
    }

    #[test]
    fn the_peel_lobs_and_lands_on_its_circle() {
        let shot = PeelShot {
            from: Vec3::new(0.0, 100.0, 0.0),
            to: Vec3::new(0.0, 0.0, 800.0),
            t: 0.0,
            spin: 0.0,
            damage: 50.0,
        };
        assert_eq!(peel_point(&shot, 1.0), shot.to);
        assert!(peel_point(&shot, 0.5).y > 200.0);
    }

    #[test]
    fn every_slot_has_an_indicator() {
        for slot in 0..4 {
            assert_eq!(indicators(slot).len(), 1);
        }
    }
}
