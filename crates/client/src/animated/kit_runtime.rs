//! Executes data-driven champion kits (`crate::kit`) on the scene.
//!
//! A child of `animated` so it can reach the scene's private helpers. Only
//! the player's champion carries a kit; its abilities hit enemy minions,
//! camps and champions. Structures take damage but ignore crowd control.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Vec2, Vec3};
use rift_cache::{AnimationClip, Model};

use super::{
    AnimatedScene, Effect, Particle, ShotVisual, UnitAssets, UnitKind, advance_trail, build_unit,
    planar_delta, turn_toward,
};
use crate::fx;
use crate::kit::{self, Action, Anchor, Passive, Status};
use crate::scene::{HeightGrid, WorldCollision};

/// A cache NPC's merged model and clips, loaded up front for summons and
/// transforms.
pub struct NpcAssets {
    pub model: Arc<Model>,
    pub idle: Arc<AnimationClip>,
    pub walk: Option<Arc<AnimationClip>>,
}

/// What an action needs to know about its cast.
#[derive(Clone)]
struct CastCtx {
    color: [u8; 3],
    /// Caster position when the action runs.
    origin: Vec3,
    /// Aim point (cursor or landing spot).
    target: Vec3,
    /// Planar aim direction.
    dir: Vec3,
    /// Ability damage multiplier (power, level, passives).
    mult: f32,
}

struct KitShot {
    pos: Vec3,
    dir: Vec3,
    speed: f32,
    remaining: f32,
    width: f32,
    damage: f32,
    pierce: bool,
    explode: f32,
    on_hit: Vec<Status>,
    hit: Vec<usize>,
    color: [u8; 3],
    from: Vec3,
    /// Basic attack: counts for passives and empowers.
    basic: bool,
    /// Projectile look, and a colour that overrides it for shots without a
    /// named look.
    look: usize,
    tint: Option<[u8; 3]>,
    trail: Vec<Vec3>,
    age: f32,
    ember_timer: f32,
    /// Homing basic attack: steers onto this unit and hits only it.
    target: Option<usize>,
}

struct PendingArea {
    pos: Vec3,
    radius: f32,
    delay: f32,
    arc: f32,
    facing: Vec3,
    damage: f32,
    heal_per_hit: f32,
    on_hit: Vec<Status>,
    color: [u8; 3],
    from: Vec3,
}

struct Zone {
    pos: Vec3,
    radius: f32,
    grow: f32,
    remaining: f32,
    tick: f32,
    timer: f32,
    damage: f32,
    heal_caster: f32,
    on_tick: Vec<Status>,
    color: [u8; 3],
}

struct DashState {
    dir: Vec3,
    remaining: f32,
    speed: f32,
    damage: f32,
    on_hit: Vec<Status>,
    hit: Vec<usize>,
    land: Vec<Action>,
    ctx: CastCtx,
}

struct ChannelState {
    remaining: f32,
    interval: f32,
    timer: f32,
    action: Action,
    ctx: CastCtx,
}

struct EmpowerState {
    remaining: f32,
    attacks: u32,
    bonus: f32,
    missing_health: f32,
    reset_on_kill: Option<usize>,
    on_hit: Vec<Status>,
}

struct TransformState {
    remaining: f32,
    model: Arc<Model>,
    idle: Arc<AnimationClip>,
    walk: Option<Arc<AnimationClip>>,
    scale: f32,
    range: f32,
    damage: f32,
    move_speed: f32,
    /// The champion's own attack and cast clips, parked because they
    /// animate a different skeleton from the transformed model.
    attack_clip: Option<Arc<AnimationClip>>,
    cast_clip: Option<Arc<AnimationClip>>,
}

/// Per-match kit state for the player's champion.
#[derive(Default)]
pub struct KitRuntime {
    pub npc_assets: HashMap<u32, NpcAssets>,
    shots: Vec<KitShot>,
    areas: Vec<PendingArea>,
    zones: Vec<Zone>,
    dash: Option<DashState>,
    channel: Option<ChannelState>,
    empower: Option<EmpowerState>,
    transform: Option<TransformState>,
    charges: [u8; 4],
    recharge: [f32; 4],
    attack_count: u32,
    second_form_used: bool,
    combo_stacks: u32,
    combo_timer: f32,
    combo_last: Option<usize>,
    damage_reduction: f32,
    damage_reduction_timer: f32,
    reflect: f32,
    reflect_timer: f32,
    haste: f32,
    haste_timer: f32,
    untargetable_timer: f32,
    rooted_timer: f32,
}

impl KitRuntime {
    /// Load the NPCs a kit summons or turns into.
    pub fn load(cache: &rift_cache::Cache, assets: &mut UnitAssets<'_>, kit: &kit::Kit) -> Self {
        let mut runtime = Self::default();
        for (slot, ability) in kit.abilities.iter().enumerate() {
            runtime.charges[slot] = ability.charges.max(1);
        }
        let wanted = kit.npcs();
        if wanted.is_empty() {
            return runtime;
        }
        let Ok(files) = cache.archive_files_with_ids(2, 9) else {
            tracing::warn!("kit: npc archive unavailable; summons and transforms disabled");
            return runtime;
        };
        for (id, data) in files {
            if !wanted.contains(&id) {
                continue;
            }
            let Ok(def) = rift_cache::NpcDefinition::parse(id, &data) else {
                tracing::warn!("kit: npc {id} does not decode");
                continue;
            };
            let Some((&first, rest)) = def.models.split_first() else {
                continue;
            };
            let loaded = (|| -> anyhow::Result<NpcAssets> {
                let parts = assets.merged_parts(first, rest)?;
                let idle = if def.standing_animation >= 0 {
                    assets.clip(def.standing_animation as u32)?
                } else {
                    Arc::new(AnimationClip::default())
                };
                let walk = if def.walking_animation >= 0 {
                    Some(assets.clip(def.walking_animation as u32)?)
                } else {
                    None
                };
                Ok(NpcAssets {
                    model: Arc::new(Model::merge(&parts)),
                    idle,
                    walk,
                })
            })();
            match loaded {
                Ok(npc) => {
                    tracing::info!("kit: loaded npc {id} \"{}\"", def.name);
                    runtime.npc_assets.insert(id, npc);
                }
                Err(err) => tracing::warn!("kit: npc {id}: {err:#}"),
            }
        }
        runtime
    }

    /// The player's own movement is taken over by a dash, a channel or a
    /// rooted buff.
    pub fn controls_player(&self) -> bool {
        self.dash.is_some() || self.channel.is_some() || self.rooted_timer > 0.0
    }
}

impl AnimatedScene {
    fn is_kit_enemy(&self, index: usize) -> bool {
        let unit = &self.units[index];
        unit.health > 0.0
            && unit.spawn_delay <= 0.0
            && (unit.team == 1 || (unit.team == 2 && unit.kind == UnitKind::Camp))
            && !matches!(unit.kind, UnitKind::DeliveryGuard | UnitKind::StorageChest)
            && (unit.kind != UnitKind::Nexus || self.nexus_vulnerable(unit.team))
    }

    fn kit_enemies_in(&self, center: Vec3, radius: f32, facing: Vec3, arc: f32) -> Vec<usize> {
        let facing = Vec2::new(facing.x, facing.z);
        (0..self.units.len())
            .filter(|&i| self.is_kit_enemy(i))
            .filter(|&i| {
                let unit = &self.units[i];
                let offset = Vec2::new(unit.pos.x - center.x, unit.pos.z - center.z);
                offset.length() <= radius + unit.hit_radius.max(40.0) * 0.5
                    && kit::in_arc(offset, facing, arc)
            })
            .collect()
    }

    /// Damage plus statuses on one enemy.
    fn kit_hit(&mut self, index: usize, damage: f32, statuses: &[Status], from: Vec3) {
        let mut pure = 0.0;
        for status in statuses {
            if let Status::CurrentHealth { pct } = status {
                pure += self.units[index].health * pct;
            }
        }
        let pi = self.player_index;
        if damage > 0.0 || pure > 0.0 {
            self.damage_unit_mixed(index, damage, pure, 0, Some(pi));
        }
        if self.units[index].health <= 0.0 {
            return;
        }
        for status in statuses {
            self.apply_status(index, status, from);
        }
    }

    pub(super) fn apply_status(&mut self, index: usize, status: &Status, from: Vec3) {
        let pi = self.player_index;
        let structure = self.units[index].kind.is_structure();
        let unit = &mut self.units[index];
        match *status {
            Status::Venom { damage, duration } | Status::Bleed { damage, duration } => {
                if unit.venom_timer <= 0.0 {
                    unit.venom_tick = 1.0;
                }
                unit.venom_timer = unit.venom_timer.max(duration);
                unit.venom_damage = unit.venom_damage.max(damage);
                unit.venom_team = 0;
            }
            Status::ManaDrain { amount } => unit.mana = (unit.mana - amount).max(0.0),
            Status::ArmorShred { amount, duration } => {
                unit.armor_reduction = unit.armor_reduction.max(amount);
                unit.armor_reduction_timer = unit.armor_reduction_timer.max(duration);
            }
            Status::HealCut { duration } => {
                unit.cc.heal_cut_timer = unit.cc.heal_cut_timer.max(duration)
            }
            Status::CurrentHealth { .. } => {}
            _ if structure => {}
            Status::Slow { pct, duration } => {
                unit.cc.slow_pct = unit.cc.slow_pct.max(pct);
                unit.cc.slow_timer = unit.cc.slow_timer.max(duration);
            }
            Status::Chill { pct, duration } => {
                unit.cc.slow_pct = unit.cc.slow_pct.max(pct);
                unit.cc.slow_timer = unit.cc.slow_timer.max(duration);
                unit.cc.chill_stacks += 1;
                unit.cc.chill_timer = duration.max(2.5);
                if unit.cc.chill_stacks >= 3 {
                    unit.cc.chill_stacks = 0;
                    unit.cc.root_timer = unit.cc.root_timer.max(1.2);
                    let pos = unit.pos;
                    self.emit_burst(pos + Vec3::Y * 40.0, [170, 230, 255], 16, 220.0, 14.0, 0.7);
                    self.add_decal(pos, 110.0, [120, 180, 220], 1.4);
                }
            }
            Status::Root { duration } => unit.cc.root_timer = unit.cc.root_timer.max(duration),
            Status::Stun { duration } => unit.stun_timer = unit.stun_timer.max(duration),
            Status::Knockup { duration } => {
                unit.stun_timer = unit.stun_timer.max(duration);
                unit.cc.lift_timer = unit.cc.lift_timer.max(duration);
            }
            Status::Fear { duration } => {
                unit.cc.fear_timer = unit.cc.fear_timer.max(duration);
                unit.cc.fear_from = from;
            }
            Status::Taunt { duration } => {
                unit.cc.taunt_timer = unit.cc.taunt_timer.max(duration);
                unit.cc.taunt_by = pi;
            }
            Status::Knockback { distance } => {
                let away = planar_delta(from, unit.pos).normalize_or_zero();
                unit.cc.displace = away * (distance / 0.25);
                unit.cc.displace_timer = 0.25;
            }
            Status::Pull { distance } => {
                let toward = planar_delta(unit.pos, from);
                let gap = (toward.length() - 110.0).max(0.0);
                let distance = distance.min(gap);
                unit.cc.displace = toward.normalize_or_zero() * (distance / 0.25);
                unit.cc.displace_timer = 0.25;
            }
        }
    }

    /// Crowd control that takes over a non-player unit for this frame:
    /// forced movement and fear. Returns true when the unit should skip its
    /// own AI.
    pub(super) fn kit_afflicted(&mut self, index: usize, dt: f32, height: &HeightGrid) -> bool {
        let unit = &mut self.units[index];
        if unit.cc.lift_timer > 0.0 {
            let ground = height.height_at(unit.pos.x, unit.pos.z);
            let t = (unit.cc.lift_timer * 2.0).min(1.0);
            unit.pos.y = ground + (t * std::f32::consts::PI).sin().abs() * 110.0;
        }
        if unit.cc.displace_timer > 0.0 {
            unit.pos += unit.cc.displace * dt;
            unit.pos.y = height.height_at(unit.pos.x, unit.pos.z);
            unit.moving = false;
            return true;
        }
        if unit.cc.fear_timer > 0.0 && !unit.kind.is_structure() {
            let away = planar_delta(unit.cc.fear_from, unit.pos).normalize_or_zero();
            let step = unit.speed * 0.8 * dt;
            unit.pos += away * step;
            unit.pos.y = height.height_at(unit.pos.x, unit.pos.z);
            if away.length_squared() > 0.0 {
                turn_toward(unit, away.x.atan2(away.z), dt);
            }
            unit.moving = true;
            unit.attack_target = None;
            return true;
        }
        false
    }

    /// Damage multiplier for the player's abilities right now.
    fn kit_ability_mult(&self) -> f32 {
        let player = &self.units[self.player_index];
        let combo = match self.kit.as_ref().and_then(|kit| kit.passive.as_ref()) {
            Some(Passive::SpellCombo { per_stack, .. }) => {
                1.0 + per_stack * self.kit_rt.combo_stacks as f32
            }
            _ => 1.0,
        };
        kit::scaled_damage(1.0, player.ability_power, player.level) * combo
    }

    /// Cast slot 0..4 from the champion's kit. False when it cannot be cast.
    pub(super) fn cast_kit(&mut self, slot: usize, target: Vec3) -> bool {
        let Some(kit) = self.kit.clone() else {
            return false;
        };
        let ability = &kit.abilities[slot];
        let pi = self.player_index;
        {
            let player = &self.units[pi];
            if player.health <= 0.0
                || player.stun_timer > 0.0
                || player.mana < ability.cost
                || self.kit_rt.dash.is_some()
                || self.kit_rt.channel.is_some()
            {
                return false;
            }
        }
        if ability.charges > 1 {
            if self.kit_rt.charges[slot] == 0 {
                return false;
            }
            if self.kit_rt.charges[slot] == ability.charges {
                self.kit_rt.recharge[slot] = ability.cooldown;
            }
            self.kit_rt.charges[slot] -= 1;
        } else if self.units[pi].cooldowns[slot] > 0.0 {
            return false;
        } else {
            self.units[pi].cooldowns[slot] = ability.cooldown;
        }
        self.units[pi].mana -= ability.cost;

        if let Some(Passive::SpellCombo { max, window, .. }) = kit.passive.as_ref() {
            if self.kit_rt.combo_last != Some(slot) && self.kit_rt.combo_timer > 0.0 {
                self.kit_rt.combo_stacks = (self.kit_rt.combo_stacks + 1).min(*max);
            } else if self.kit_rt.combo_timer <= 0.0 {
                self.kit_rt.combo_stacks = 0;
            }
            self.kit_rt.combo_last = Some(slot);
            self.kit_rt.combo_timer = *window;
        }

        let origin = self.units[pi].pos;
        let flat = planar_delta(origin, target);
        let dir = if flat.length_squared() > 1.0 {
            flat.normalize()
        } else {
            let yaw = self.units[pi].yaw;
            Vec3::new(yaw.sin(), 0.0, yaw.cos())
        };
        let player = &mut self.units[pi];
        player.yaw = dir.x.atan2(dir.z);
        let ctx = CastCtx {
            color: ability.color,
            origin,
            target,
            dir,
            mult: self.kit_ability_mult(),
        };
        self.effects.push(Effect {
            pos: origin,
            radius: 120.0,
            remaining: 0.3,
            duration: 0.3,
            color: ability.color,
        });
        for action in &ability.actions {
            self.run_action(action, &ctx);
        }
        tracing::info!("kit: cast slot {slot}");
        true
    }

    fn anchor_point(&self, at: Anchor, range: f32, ctx: &CastCtx) -> Vec3 {
        match at {
            Anchor::Caster => ctx.origin,
            Anchor::Front => ctx.origin + ctx.dir * 160.0,
            Anchor::Point => {
                let offset = planar_delta(ctx.origin, ctx.target);
                if offset.length() <= range {
                    ctx.target
                } else {
                    ctx.origin + offset.normalize_or_zero() * range
                }
            }
        }
    }

    fn run_action(&mut self, action: &Action, ctx: &CastCtx) {
        let pi = self.player_index;
        match action {
            Action::Projectile {
                speed,
                range,
                damage,
                width,
                pierce,
                count,
                spread,
                explode,
                visual,
                on_hit,
            } => {
                let count = (*count).max(1);
                let (look, tint) = match visual {
                    Some(name) => (fx::look_by_name(name), None),
                    None => (fx::DEFAULT_LOOK, Some(ctx.color)),
                };
                let base = ctx.dir.z.atan2(ctx.dir.x);
                let height = self.units[pi].scale * 80.0;
                for i in 0..count {
                    let offset = if count == 1 {
                        0.0
                    } else {
                        spread.to_radians() * (i as f32 / (count - 1) as f32 - 0.5)
                    };
                    let angle = base + offset;
                    let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                    self.kit_rt.shots.push(KitShot {
                        pos: ctx.origin + Vec3::Y * height + dir * 40.0,
                        dir,
                        speed: *speed,
                        remaining: *range,
                        width: *width,
                        damage: damage * ctx.mult,
                        pierce: *pierce,
                        explode: *explode,
                        on_hit: on_hit.clone(),
                        hit: Vec::new(),
                        color: ctx.color,
                        from: ctx.origin,
                        basic: false,
                        look,
                        tint,
                        trail: Vec::new(),
                        age: 0.0,
                        ember_timer: 0.0,
                        target: None,
                    });
                }
            }
            Action::Area {
                at,
                radius,
                damage,
                delay,
                arc,
                range,
                repeat,
                interval,
                scatter,
                heal_per_hit,
                on_hit,
            } => {
                let center = self.anchor_point(*at, *range, ctx);
                for r in 0..(*repeat).max(1) {
                    let jitter = if *scatter > 0.0 {
                        let a = self.next_random() * std::f32::consts::TAU;
                        let d = self.next_random().sqrt() * scatter;
                        Vec3::new(a.cos() * d, 0.0, a.sin() * d)
                    } else {
                        Vec3::ZERO
                    };
                    let wait = delay + r as f32 * interval;
                    let pos = center + jitter;
                    if wait > 0.05 {
                        self.add_decal(pos, *radius * 0.9, dim(ctx.color), wait + 0.1);
                    }
                    self.kit_rt.areas.push(PendingArea {
                        pos,
                        radius: *radius,
                        delay: wait,
                        arc: *arc,
                        facing: ctx.dir,
                        damage: damage * ctx.mult,
                        heal_per_hit: *heal_per_hit,
                        on_hit: on_hit.clone(),
                        color: ctx.color,
                        from: ctx.origin,
                    });
                }
            }
            Action::Zone {
                at,
                radius,
                duration,
                damage,
                tick,
                range,
                count,
                spacing,
                grow,
                heal_caster,
                on_tick,
            } => {
                let center = self.anchor_point(*at, *range, ctx);
                for i in 0..(*count).max(1) {
                    let pos = center + ctx.dir * (spacing * i as f32);
                    self.add_decal(pos, *radius, dim(ctx.color), *duration);
                    self.kit_rt.zones.push(Zone {
                        pos,
                        radius: *radius,
                        grow: *grow,
                        remaining: *duration,
                        tick: tick.max(0.1),
                        timer: 0.0,
                        damage: damage * ctx.mult,
                        heal_caster: *heal_caster,
                        on_tick: on_tick.clone(),
                        color: ctx.color,
                    });
                }
            }
            Action::Dash {
                distance,
                speed,
                to_cursor,
                backwards,
                damage,
                untargetable,
                on_hit,
                land,
            } => {
                let dir = if *backwards { -ctx.dir } else { ctx.dir };
                let mut distance = *distance;
                if *to_cursor {
                    distance = distance.min(planar_delta(ctx.origin, ctx.target).length());
                }
                if *untargetable {
                    self.kit_rt.untargetable_timer = distance / speed.max(1.0) + 0.15;
                }
                let player = &mut self.units[pi];
                player.path.clear();
                player.path_index = 0;
                player.attack_target = None;
                player.attack_pending = false;
                self.kit_rt.dash = Some(DashState {
                    dir,
                    remaining: distance,
                    speed: *speed,
                    damage: damage * ctx.mult,
                    on_hit: on_hit.clone(),
                    hit: Vec::new(),
                    land: land.clone(),
                    ctx: ctx.clone(),
                });
                if let Some(Passive::Swift { dash_bonus, .. }) =
                    self.kit.as_ref().and_then(|kit| kit.passive.as_ref())
                {
                    let bonus = *dash_bonus;
                    self.kit_rt.empower = Some(EmpowerState {
                        remaining: 4.0,
                        attacks: 1,
                        bonus,
                        missing_health: 0.0,
                        reset_on_kill: None,
                        on_hit: Vec::new(),
                    });
                }
            }
            Action::Buff {
                heal,
                heal_missing,
                shield,
                duration,
                damage_reduction,
                reflect,
                attack_speed,
                move_speed,
                untargetable,
                rooted,
            } => {
                let mult = ctx.mult;
                let player = &mut self.units[pi];
                let missing = player.max_health - player.health;
                player.health =
                    (player.health + heal * mult + missing * heal_missing).min(player.max_health);
                if *shield > 0.0 {
                    player.shield = player.shield.max(shield * mult);
                    player.shield_timer = duration.max(3.0);
                }
                if *attack_speed > 0.0 {
                    player.attack_speed_mult = 1.0 + attack_speed;
                    player.attack_speed_timer = *duration;
                }
                let rt = &mut self.kit_rt;
                if *damage_reduction > 0.0 {
                    rt.damage_reduction = *damage_reduction;
                    rt.damage_reduction_timer = *duration;
                }
                if *reflect > 0.0 {
                    rt.reflect = *reflect;
                    rt.reflect_timer = *duration;
                }
                if *move_speed > 0.0 {
                    rt.haste = *move_speed;
                    rt.haste_timer = *duration;
                }
                if *untargetable > 0.0 {
                    rt.untargetable_timer = rt.untargetable_timer.max(*untargetable);
                }
                if *rooted {
                    rt.rooted_timer = *duration;
                }
                if heal + heal_missing > 0.0 {
                    self.emit_burst(ctx.origin, [120, 255, 160], 18, 160.0, 14.0, 0.8);
                }
            }
            Action::Summon {
                npc,
                count,
                duration,
                health,
                damage,
                range,
                interval,
                speed,
                scale,
                at,
                npcs,
            } => {
                let center = self.anchor_point(*at, 700.0, ctx);
                for i in 0..(*count).max(1) {
                    let id = npcs.get(i as usize).copied().unwrap_or(*npc);
                    let Some(asset) = self.kit_rt.npc_assets.get(&id) else {
                        tracing::warn!("kit: summon npc {id} not loaded");
                        continue;
                    };
                    let angle = i as f32 / count.max(&1).to_owned() as f32 * std::f32::consts::TAU;
                    let pos = center + Vec3::new(angle.cos(), 0.0, angle.sin()) * 140.0;
                    let mut unit = build_unit(
                        asset.model.clone(),
                        asset.idle.clone(),
                        asset.walk.clone(),
                        pos,
                        *scale,
                        UnitKind::MeleeMinion,
                        0,
                        *health,
                        *range,
                        damage * ctx.mult,
                        *interval,
                        *speed,
                        70.0,
                        180.0 * scale.max(0.6),
                        Vec::new(),
                    );
                    unit.yaw = ctx.dir.x.atan2(ctx.dir.z);
                    unit.cc.summoned = true;
                    unit.cc.life_timer = *duration;
                    self.units.push(unit);
                    self.emit_burst(pos, ctx.color, 14, 220.0, 14.0, 0.6);
                }
            }
            Action::Channel {
                duration,
                interval,
                action,
            } => {
                let player = &mut self.units[pi];
                player.path.clear();
                player.path_index = 0;
                player.moving = false;
                self.kit_rt.channel = Some(ChannelState {
                    remaining: *duration,
                    interval: interval.max(0.05),
                    timer: 0.0,
                    action: (**action).clone(),
                    ctx: ctx.clone(),
                });
            }
            Action::Empower {
                duration,
                attacks,
                bonus,
                missing_health,
                reset_on_kill,
                on_hit,
            } => {
                self.kit_rt.empower = Some(EmpowerState {
                    remaining: *duration,
                    attacks: (*attacks).max(1),
                    bonus: bonus * ctx.mult,
                    missing_health: *missing_health,
                    reset_on_kill: *reset_on_kill,
                    on_hit: on_hit.clone(),
                });
                self.emit_burst(ctx.origin + Vec3::Y * 90.0, ctx.color, 12, 160.0, 12.0, 0.5);
            }
            Action::Transform {
                npc,
                duration,
                scale,
                damage,
                range,
                move_speed,
                heal,
            } => {
                self.kit_transform(*npc, *duration, *scale, *damage, *range, *move_speed);
                let player = &mut self.units[pi];
                player.health = (player.health + heal * ctx.mult).min(player.max_health);
                self.emit_burst(ctx.origin, ctx.color, 30, 320.0, 18.0, 0.9);
            }
        }
    }

    fn kit_transform(
        &mut self,
        npc: u32,
        duration: f32,
        scale: f32,
        damage: f32,
        range: f32,
        move_speed: f32,
    ) {
        let Some(asset) = self.kit_rt.npc_assets.get(&npc) else {
            tracing::warn!("kit: transform npc {npc} not loaded");
            return;
        };
        let (model, idle, walk) = (asset.model.clone(), asset.idle.clone(), asset.walk.clone());
        self.kit_revert_transform();
        let pi = self.player_index;
        let player = &mut self.units[pi];
        let old_model = std::mem::replace(&mut player.model, model);
        let old_idle = std::mem::replace(&mut player.idle, idle);
        let old_walk = std::mem::replace(&mut player.walk, walk);
        let old_scale = std::mem::replace(&mut player.scale, scale);
        let attack_clip = player.attack_clip.take();
        let cast_clip = player.cast_clip.take();
        player.action = None;
        player.attack_range += range;
        player.attack_damage *= 1.0 + damage;
        self.kit_rt.transform = Some(TransformState {
            remaining: duration,
            model: old_model,
            idle: old_idle,
            walk: old_walk,
            scale: old_scale,
            range,
            damage,
            move_speed,
            attack_clip,
            cast_clip,
        });
    }

    fn kit_revert_transform(&mut self) {
        let Some(state) = self.kit_rt.transform.take() else {
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
        player.attack_range -= state.range;
        player.attack_damage /= 1.0 + state.damage;
    }

    /// Movement multiplier from kit buffs and passives.
    pub(super) fn kit_move_mult(&self) -> f32 {
        let swift = match self.kit.as_ref().and_then(|kit| kit.passive.as_ref()) {
            Some(Passive::Swift { move_speed, .. }) => *move_speed,
            _ => 0.0,
        };
        let haste = if self.kit_rt.haste_timer > 0.0 {
            self.kit_rt.haste
        } else {
            0.0
        };
        let transform = self.kit_rt.transform.as_ref().map_or(0.0, |t| t.move_speed);
        1.0 + swift + haste + transform
    }

    /// Fraction of incoming damage the player takes. Zero while untargetable.
    pub(super) fn kit_incoming_mult(&self) -> f32 {
        if self.kit_rt.untargetable_timer > 0.0 {
            return 0.0;
        }
        if self.kit_rt.damage_reduction_timer > 0.0 {
            1.0 - self.kit_rt.damage_reduction.clamp(0.0, 0.9)
        } else {
            1.0
        }
    }

    /// Extra multiplier for projectiles that hit the player.
    pub(super) fn kit_projectile_taken(&self) -> f32 {
        match self.kit.as_ref().and_then(|kit| kit.passive.as_ref()) {
            Some(Passive::ThickHide { reduction }) => 1.0 - reduction.clamp(0.0, 0.9),
            _ => 1.0,
        }
    }

    /// Reflect and second-form hooks after the player took damage.
    pub(super) fn kit_after_player_hit(&mut self, amount: f32, aggressor: Option<usize>) {
        if self.kit.is_none() {
            return;
        }
        if self.kit_rt.reflect_timer > 0.0
            && let Some(attacker) = aggressor
            && attacker != self.player_index
            && self.units[attacker].health > 0.0
            && self.units[attacker].attack_range < 300.0
        {
            let back = amount * self.kit_rt.reflect;
            self.damage_unit_mixed(attacker, 0.0, back, 0, Some(self.player_index));
        }
        let Some(Passive::SecondForm {
            threshold,
            heal,
            npc,
            duration,
            scale,
            range,
            damage,
        }) = self.kit.as_ref().and_then(|kit| kit.passive.clone())
        else {
            return;
        };
        let pi = self.player_index;
        let player = &self.units[pi];
        if self.kit_rt.second_form_used
            || player.health <= 0.0
            || player.health > player.max_health * threshold
        {
            return;
        }
        self.kit_rt.second_form_used = true;
        let player = &mut self.units[pi];
        player.health = (player.health + player.max_health * heal).min(player.max_health);
        let pos = player.pos;
        self.kit_transform(npc, duration, scale, damage, range, 0.0);
        self.emit_burst(pos, [255, 220, 120], 40, 360.0, 18.0, 1.0);
        self.announcement = Some(("SECOND FORM".to_string(), 2.0));
        tracing::info!("kit: second form");
    }

    /// Allied minion damage multiplier from the player's aura passive.
    pub(super) fn kit_minion_aura(&self, minion: usize) -> f32 {
        let Some(Passive::MinionAura { radius, damage }) =
            self.kit.as_ref().and_then(|kit| kit.passive.as_ref())
        else {
            return 1.0;
        };
        let player = &self.units[self.player_index];
        let unit = &self.units[minion];
        if unit.team == player.team
            && player.health > 0.0
            && unit.pos.distance(player.pos) <= *radius
        {
            1.0 + damage
        } else {
            1.0
        }
    }

    /// Experience multiplier from the scholar passive.
    pub(super) fn kit_xp_mult(&self) -> f32 {
        match self.kit.as_ref().and_then(|kit| kit.passive.as_ref()) {
            Some(Passive::Scholar { xp }) => 1.0 + xp,
            _ => 1.0,
        }
    }

    /// The player's basic attack for kit champions: passives, empowers,
    /// splash and frost all live here.
    pub(super) fn kit_land_attack(&mut self, target: usize) {
        let pi = self.player_index;
        let passive = self.kit.as_ref().and_then(|kit| kit.passive.clone());
        let (mut damage, ranged, from, target_pos) = {
            let unit = &mut self.units[pi];
            unit.attack_cooldown = unit.attack_interval / unit.attack_speed_mult.max(0.1);
            let mut damage = unit.attack_damage + unit.min_base_damage_bonus;
            if unit.ranged_auto {
                damage *= unit.ranged_scale();
            }
            (
                damage,
                unit.ranged_auto,
                unit.pos + Vec3::Y * (unit.scale * 80.0),
                self.units[target].pos,
            )
        };
        let mut statuses: Vec<Status> = Vec::new();
        match &passive {
            Some(Passive::OnHit { statuses: s }) => statuses.extend(s.iter().cloned()),
            Some(Passive::FrostBite { pct, duration }) => statuses.push(Status::Chill {
                pct: *pct,
                duration: *duration,
            }),
            _ => {}
        }
        let mut reset_slot = None;
        if let Some(empower) = self.kit_rt.empower.as_mut() {
            damage += empower.bonus;
            let target_unit = &self.units[target];
            damage += (target_unit.max_health - target_unit.health) * empower.missing_health;
            statuses.extend(empower.on_hit.iter().cloned());
            reset_slot = empower.reset_on_kill;
            empower.attacks = empower.attacks.saturating_sub(1);
            if empower.attacks == 0 {
                self.kit_rt.empower = None;
            }
            self.emit_burst(
                target_pos + Vec3::Y * 60.0,
                [255, 240, 160],
                14,
                240.0,
                14.0,
                0.5,
            );
        }
        self.kit_rt.attack_count += 1;
        let splash = match &passive {
            Some(Passive::Splash { radius, pct }) => Some((*radius, *pct)),
            _ => None,
        };
        if ranged {
            let aim = self.unit_chest(target);
            let dir = (aim - from).normalize_or_zero();
            let look = self.units[pi].look;
            self.kit_rt.shots.push(KitShot {
                pos: from,
                dir,
                speed: 1400.0,
                remaining: (aim - from).length().max(1.0),
                width: 80.0,
                damage,
                pierce: false,
                explode: splash.map_or(0.0, |(radius, _)| radius),
                on_hit: statuses,
                hit: Vec::new(),
                color: fx::look(look).glow,
                from: self.units[pi].pos,
                basic: true,
                look,
                tint: None,
                trail: Vec::new(),
                age: 0.0,
                ember_timer: 0.0,
                target: Some(target),
            });
            self.muzzle_flash(from, look);
            return;
        }
        self.melee_slash(pi, target, [255, 235, 200]);
        self.kit_hit(target, damage, &statuses, self.units[pi].pos);
        let killed = self.units[target].health <= 0.0;
        if killed {
            self.units[pi].attack_target = None;
            if let Some(slot) = reset_slot {
                self.units[pi].cooldowns[slot] = 0.0;
                self.kit_rt.charges[slot] =
                    self.kit.as_ref().map_or(1, |k| k.abilities[slot].charges);
            }
        }
        let mut aoe: Option<(f32, f32)> = splash;
        if let Some(Passive::NthAttack {
            n,
            radius,
            damage: pct,
        }) = &passive
            && self.kit_rt.attack_count % n.max(&1) == 0
        {
            aoe = Some((*radius, *pct));
            self.effects.push(Effect {
                pos: target_pos,
                radius: *radius,
                remaining: 0.4,
                duration: 0.4,
                color: [255, 120, 60],
            });
        }
        if let Some((radius, pct)) = aoe {
            for other in self.kit_enemies_in(target_pos, radius, Vec3::Z, 360.0) {
                if other != target {
                    self.kit_hit(other, damage * pct, &[], target_pos);
                }
            }
        }
    }

    /// Advance every running kit effect. Called once per frame after the
    /// player moves.
    pub(super) fn update_kit(&mut self, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        let Some(kit) = self.kit.as_ref() else {
            return;
        };
        let pi = self.player_index;
        let charges: [(u8, f32); 4] =
            std::array::from_fn(|slot| (kit.abilities[slot].charges, kit.abilities[slot].cooldown));

        // Recharge multi-charge abilities; show the recharge only when empty.
        for (slot, (max, cooldown)) in charges.into_iter().enumerate() {
            if max <= 1 {
                continue;
            }
            let rt = &mut self.kit_rt;
            if rt.charges[slot] < max {
                rt.recharge[slot] -= dt;
                if rt.recharge[slot] <= 0.0 {
                    rt.charges[slot] += 1;
                    rt.recharge[slot] = if rt.charges[slot] < max {
                        cooldown
                    } else {
                        0.0
                    };
                }
            }
            self.units[pi].cooldowns[slot] = if rt.charges[slot] == 0 {
                rt.recharge[slot].max(0.0)
            } else {
                0.0
            };
        }

        let rt = &mut self.kit_rt;
        for timer in [
            &mut rt.combo_timer,
            &mut rt.damage_reduction_timer,
            &mut rt.reflect_timer,
            &mut rt.haste_timer,
            &mut rt.untargetable_timer,
            &mut rt.rooted_timer,
        ] {
            *timer = (*timer - dt).max(0.0);
        }
        if let Some(empower) = rt.empower.as_mut() {
            empower.remaining -= dt;
            if empower.remaining <= 0.0 {
                rt.empower = None;
            }
        }
        if let Some(transform) = rt.transform.as_mut() {
            transform.remaining -= dt;
            if transform.remaining <= 0.0 {
                self.kit_revert_transform();
            }
        }

        // Death clears everything the champion was doing; respawn re-arms
        // the second form.
        if self.units[pi].health <= 0.0 {
            self.kit_rt.dash = None;
            self.kit_rt.channel = None;
            self.kit_rt.empower = None;
            self.kit_rt.rooted_timer = 0.0;
            self.kit_revert_transform();
            self.kit_rt.second_form_used = false;
        }

        self.update_kit_dash(dt, height, collision);
        self.update_kit_channel(dt);
        self.update_kit_shots(dt, height);
        self.update_kit_areas(dt);
        self.update_kit_zones(dt);
    }

    fn update_kit_dash(&mut self, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        let pi = self.player_index;
        let Some(mut dash) = self.kit_rt.dash.take() else {
            return;
        };
        let step = (dash.speed * dt).min(dash.remaining);
        let next = self.units[pi].pos + dash.dir * step;
        let blocked = !collision.is_walkable_world(next.x, next.z);
        if !blocked {
            let player = &mut self.units[pi];
            player.pos = Vec3::new(next.x, height.height_at(next.x, next.z), next.z);
            player.yaw = dash.dir.x.atan2(dash.dir.z);
            player.moving = true;
            dash.remaining -= step;
        }
        let pos = self.units[pi].pos;
        if self.particles.len() < 300 {
            self.particles.push(Particle {
                pos: pos + Vec3::Y * 40.0,
                vel: -dash.dir * 60.0 + Vec3::Y * 30.0,
                color: dash.ctx.color,
                size: 12.0,
                remaining: 0.3,
                duration: 0.3,
                gravity: 0.0,
                drag: 2.0,
            });
        }
        if dash.damage > 0.0 || !dash.on_hit.is_empty() {
            for index in self.kit_enemies_in(pos, 130.0, Vec3::Z, 360.0) {
                if !dash.hit.contains(&index) {
                    dash.hit.push(index);
                    self.kit_hit(index, dash.damage, &dash.on_hit, pos);
                }
            }
        }
        if dash.remaining <= 0.5 || blocked {
            let mut ctx = dash.ctx.clone();
            ctx.origin = pos;
            ctx.target = pos;
            for action in &dash.land {
                self.run_action(action, &ctx);
            }
            if let Some(motion) = self.units[pi].motion.as_mut() {
                motion.reset();
            }
            self.units[pi].moving = false;
        } else {
            self.kit_rt.dash = Some(dash);
        }
    }

    fn update_kit_channel(&mut self, dt: f32) {
        let pi = self.player_index;
        let Some(mut channel) = self.kit_rt.channel.take() else {
            return;
        };
        if self.units[pi].stun_timer > 0.0 {
            return;
        }
        channel.remaining -= dt;
        channel.timer -= dt;
        while channel.timer <= 0.0 && channel.remaining > -dt {
            channel.timer += channel.interval;
            let mut ctx = channel.ctx.clone();
            ctx.origin = self.units[pi].pos;
            self.run_action(&channel.action, &ctx);
        }
        if channel.remaining > 0.0 {
            self.units[pi].moving = false;
            self.kit_rt.channel = Some(channel);
        }
    }

    fn update_kit_shots(&mut self, dt: f32, height: &HeightGrid) {
        let shots = std::mem::take(&mut self.kit_rt.shots);
        let mut survivors = Vec::with_capacity(shots.len());
        for mut shot in shots {
            let look = fx::look(shot.look).tinted(shot.tint);
            shot.age += dt;
            let step = shot.speed * dt;
            let mut hits = Vec::new();
            if let Some(ti) = shot.target {
                // Homing basic attack: chase the target and land on arrival.
                if self.units.get(ti).is_none_or(|unit| unit.health <= 0.0) {
                    continue;
                }
                let aim = self.unit_chest(ti);
                let to = aim - shot.pos;
                let distance = to.length();
                if distance > 1.0 {
                    shot.dir = to / distance;
                }
                if distance <= step + 6.0 {
                    shot.pos = aim;
                    hits.push(ti);
                } else {
                    shot.pos += shot.dir * step;
                }
            } else {
                shot.pos += shot.dir * step;
                shot.remaining -= step;
                let ground = height.height_at(shot.pos.x, shot.pos.z);
                if shot.pos.y < ground + 30.0 {
                    shot.pos.y = ground + 30.0;
                }
                for index in 0..self.units.len() {
                    if shot.hit.contains(&index) || !self.is_kit_enemy(index) {
                        continue;
                    }
                    let unit = &self.units[index];
                    let flat = Vec2::new(unit.pos.x - shot.pos.x, unit.pos.z - shot.pos.z);
                    if flat.length() <= shot.width * 0.5 + unit.hit_radius.max(50.0) * 0.6 {
                        hits.push(index);
                        if !shot.pierce {
                            break;
                        }
                    }
                }
            }
            advance_trail(&mut shot.trail, shot.pos, look.trail_points, shot.speed);
            shot.ember_timer -= dt;
            if look.embers > 0.0 && shot.ember_timer <= 0.0 {
                shot.ember_timer = 1.0 / look.embers;
                self.shed_ember(shot.pos, shot.dir, &look);
            }
            for &index in &hits {
                shot.hit.push(index);
                self.kit_hit(index, shot.damage, &shot.on_hit, shot.from);
                self.impact_burst(shot.pos, &look);
                if shot.explode > 0.0 {
                    let center = shot.pos;
                    self.effects.push(Effect {
                        pos: center,
                        radius: shot.explode,
                        remaining: 0.35,
                        duration: 0.35,
                        color: shot.color,
                    });
                    let splash = if shot.basic {
                        match self.kit.as_ref().and_then(|kit| kit.passive.as_ref()) {
                            Some(Passive::Splash { pct, .. }) => *pct,
                            _ => 0.5,
                        }
                    } else {
                        0.6
                    };
                    for other in self.kit_enemies_in(center, shot.explode, Vec3::Z, 360.0) {
                        if other != index {
                            self.kit_hit(other, shot.damage * splash, &shot.on_hit, center);
                        }
                    }
                }
            }
            let spent = (!hits.is_empty() && (!shot.pierce || shot.target.is_some()))
                || (shot.target.is_none() && shot.remaining <= 0.0);
            if shot.target.is_none() && shot.remaining <= 0.0 && hits.is_empty() {
                self.effects.push(Effect {
                    pos: shot.pos,
                    radius: look.glow_size * 0.8,
                    remaining: 0.22,
                    duration: 0.22,
                    color: look.glow,
                });
            }
            if !spent {
                survivors.push(shot);
            }
        }
        self.kit_rt.shots.extend(survivors);
    }

    /// Drawing data for every kit shot in flight.
    pub(super) fn kit_shot_visuals(&self) -> Vec<ShotVisual> {
        self.kit_rt
            .shots
            .iter()
            .map(|shot| ShotVisual {
                pos: shot.pos,
                dir: shot.dir,
                look: shot.look,
                trail: shot.trail.clone(),
                age: shot.age,
                tint: shot.tint,
            })
            .collect()
    }

    /// The look a kit shot should be drawn and lit with.
    pub(super) fn kit_shot_lights(&self) -> Vec<(Vec3, fx::ProjectileLook)> {
        self.kit_rt
            .shots
            .iter()
            .map(|shot| (shot.pos, fx::look(shot.look).tinted(shot.tint)))
            .collect()
    }

    fn update_kit_areas(&mut self, dt: f32) {
        let areas = std::mem::take(&mut self.kit_rt.areas);
        let mut waiting = Vec::with_capacity(areas.len());
        for mut area in areas {
            area.delay -= dt;
            if area.delay > 0.0 {
                waiting.push(area);
                continue;
            }
            let hits = self.kit_enemies_in(area.pos, area.radius, area.facing, area.arc);
            let count = hits.len();
            for index in hits {
                self.kit_hit(index, area.damage, &area.on_hit, area.from);
            }
            if area.heal_per_hit > 0.0 && count > 0 {
                let pi = self.player_index;
                let player = &mut self.units[pi];
                player.health =
                    (player.health + area.heal_per_hit * count as f32).min(player.max_health);
            }
            self.effects.push(Effect {
                pos: area.pos,
                radius: area.radius,
                remaining: 0.45,
                duration: 0.45,
                color: area.color,
            });
            let sparks = (area.radius / 12.0).clamp(10.0, 36.0) as usize;
            self.emit_burst(area.pos, area.color, sparks, area.radius * 1.4, 16.0, 0.7);
        }
        self.kit_rt.areas.extend(waiting);
    }

    fn update_kit_zones(&mut self, dt: f32) {
        let zones = std::mem::take(&mut self.kit_rt.zones);
        let mut alive = Vec::with_capacity(zones.len());
        let pi = self.player_index;
        for mut zone in zones {
            zone.remaining -= dt;
            zone.radius += zone.grow * dt;
            zone.timer -= dt;
            if zone.timer <= 0.0 {
                zone.timer += zone.tick;
                for index in self.kit_enemies_in(zone.pos, zone.radius, Vec3::Z, 360.0) {
                    self.kit_hit(index, zone.damage, &zone.on_tick, zone.pos);
                }
                if zone.heal_caster > 0.0 {
                    let player = &mut self.units[pi];
                    if player.health > 0.0 && player.pos.distance(zone.pos) <= zone.radius {
                        player.health = (player.health + zone.heal_caster).min(player.max_health);
                    }
                }
                let sparks = (zone.radius / 25.0).clamp(4.0, 16.0) as usize;
                self.emit_burst(zone.pos, zone.color, sparks, zone.radius * 0.8, 12.0, 0.6);
            }
            if zone.remaining > 0.0 {
                alive.push(zone);
            }
        }
        self.kit_rt.zones.extend(alive);
    }
}

/// A muted shade of an ability colour for ground decals. Dark colours are
/// lifted so a red or black zone reads as a stain, not a hole in the map.
fn dim(color: [u8; 3]) -> [u8; 3] {
    color.map(|c| (c as f32 * 0.55 + 38.0).min(255.0) as u8)
}
