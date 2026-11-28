//! Item passives and actives at run time (docs/ITEMS.md). Every hero's
//! items live here as gear pieces: the damage path asks for outgoing and
//! incoming multipliers and calls [`AnimatedScene::gear_after_damage`] after
//! each hit; [`AnimatedScene::gear_tick`] runs timers, regeneration and
//! auras. The player and the bots share all of it.

use glam::Vec3;

use super::{AnimatedScene, Effect, HitsplatKind, Projectile, UnitKind};
use crate::fx;
use crate::item_effects::{ActiveEffect, ItemPassive, NamedActive};
use crate::items::{ItemConfig, ThrallSpec};
use crate::kit::{Gfx, Status};

/// Seconds after an ability in which Spellblade's empowered attack waits.
const SPELLBLADE_WINDOW: f32 = 10.0;
/// Heals smaller than this pop no hitmark (lifesteal would spam them).
const QUIET_HEAL: f32 = 8.0;

/// One owned item that does more than give stats.
#[derive(Debug, Clone)]
pub struct GearPiece {
    pub id: String,
    pub passives: Vec<ItemPassive>,
    pub active: Option<NamedActive>,
    /// Seconds until the active can be used again.
    pub cooldown: f32,
}

/// A special attack waiting for its basic attack.
#[derive(Debug, Clone)]
struct Primed {
    extra: Vec<f32>,
    statuses: Vec<Status>,
    heal: f32,
    splash: f32,
    timer: f32,
}

/// One hero's items and their timers.
#[derive(Debug, Clone, Default)]
pub struct Gear {
    pub pieces: Vec<GearPiece>,
    special: Option<Primed>,
    /// Attack speed from item stats (not frenzy or actives).
    base_attack_speed: f32,
    frenzy: f32,
    frenzy_timer: f32,
    rapid: f32,
    rapid_timer: f32,
    /// Move speed an active added, taken back when `haste_timer` ends.
    haste: f32,
    haste_timer: f32,
    /// Shield an active gave, taken back when `shield_timer` ends.
    shield: f32,
    shield_timer: f32,
    spellblade_ready: f32,
    spellblade_cooldown: f32,
    last_stand_cooldown: f32,
    ability_hit_cooldown: f32,
    attacks: u32,
    last_cooldowns: [f32; 4],
}

impl Gear {
    /// Passives that count: duplicate items give theirs once.
    fn passives(&self) -> Vec<&ItemPassive> {
        let mut seen: Vec<&str> = Vec::new();
        let mut out = Vec::new();
        for piece in &self.pieces {
            if seen.contains(&piece.id.as_str()) {
                continue;
            }
            seen.push(&piece.id);
            out.extend(piece.passives.iter());
        }
        out
    }

    fn sum(&self, pick: impl Fn(&ItemPassive) -> Option<f32>) -> f32 {
        self.passives().into_iter().filter_map(pick).sum()
    }

    fn attack_speed(&self) -> f32 {
        self.base_attack_speed + self.frenzy + self.rapid
    }
}

/// Units an item effect may hit for `team`.
fn hittable(kind: UnitKind) -> bool {
    !matches!(
        kind,
        UnitKind::DeliveryGuard | UnitKind::StorageChest | UnitKind::Ambient
    )
}

impl AnimatedScene {
    /// Put an item on (`on`) or take it off `index`: its stats, and its
    /// passives and active when it has any.
    pub(super) fn equip(&mut self, index: usize, item: &ItemConfig, on: bool) {
        let sign = if on { 1.0 } else { -1.0 };
        {
            let unit = &mut self.units[index];
            unit.attack_damage = (unit.attack_damage + sign * item.damage).max(1.0);
            if item.health != 0.0 {
                unit.max_health = (unit.max_health + sign * item.health).max(100.0);
                unit.health = if on {
                    unit.health + item.health
                } else {
                    unit.health.min(unit.max_health)
                };
            }
            if item.mana != 0.0 {
                unit.max_mana = (unit.max_mana + sign * item.mana).max(50.0);
                unit.mana = if on {
                    unit.mana + item.mana
                } else {
                    unit.mana.min(unit.max_mana)
                };
            }
            unit.speed = (unit.speed + sign * item.speed).max(100.0);
            unit.ability_power = (unit.ability_power + sign * item.ability).max(0.0);
            unit.armor += sign * item.armor;
            unit.attack_range = (unit.attack_range + sign * item.range).max(100.0);
            unit.item_haste = (unit.item_haste + sign * item.haste / 100.0).max(0.0);
        }
        let gear = self.gear.entry(index).or_default();
        gear.base_attack_speed += sign * item.attack_speed / 100.0;
        if item.passives.is_empty() && item.active.is_none() {
            self.units[index].item_attack_speed = gear.attack_speed();
            return;
        }
        if on {
            gear.pieces.push(GearPiece {
                id: item.id.clone(),
                passives: item.passives.iter().map(|p| p.effect.clone()).collect(),
                active: item.active.clone(),
                cooldown: 0.0,
            });
        } else if let Some(at) = gear.pieces.iter().rposition(|p| p.id == item.id) {
            gear.pieces.remove(at);
        }
        self.units[index].item_attack_speed = gear.attack_speed();
    }

    /// Fuse the player's parts into `result`: the parts' stats come off and
    /// the fusion's go on, with a burst and a sound. Refused while dead.
    pub fn fuse_items(&mut self, parts: &[ItemConfig], result: &ItemConfig) -> bool {
        let pi = self.player_index;
        if self.units[pi].health <= 0.0 {
            return false;
        }
        for part in parts {
            self.equip(pi, part, false);
        }
        self.equip(pi, result, true);
        let pos = self.units[pi].pos;
        self.emit_burst(pos + Vec3::Y * 90.0, [255, 210, 110], 46, 380.0, 18.0, 1.1);
        self.effects.push(Effect {
            pos,
            radius: 260.0,
            remaining: 1.0,
            duration: 1.0,
            color: [255, 200, 90],
        });
        self.play_sound("shop_buy", pos);
        self.announcement = Some((format!("FUSED: {}", result.name.to_uppercase()), 3.0));
        true
    }

    fn gear_of(&self, unit: Option<usize>) -> Option<&Gear> {
        self.gear.get(&unit?).filter(|gear| !gear.pieces.is_empty())
    }

    /// Damage multiplier from the attacker's items: berserk, execute, giant
    /// slayer, big game and ability amplification.
    pub(super) fn gear_outgoing_mult(
        &self,
        aggressor: Option<usize>,
        target: usize,
        basic: bool,
    ) -> f32 {
        if self.gear_depth > 0 {
            return 1.0;
        }
        let Some(gear) = self.gear_of(aggressor) else {
            return 1.0;
        };
        let me = &self.units[aggressor.unwrap_or_default()];
        let foe = &self.units[target];
        let mut mult = 1.0;
        for passive in gear.passives() {
            mult += match *passive {
                ItemPassive::Berserk { max } => {
                    max * (1.0 - me.health / me.max_health.max(1.0)).clamp(0.0, 1.0)
                }
                ItemPassive::Execute { threshold, pct }
                    if foe.health < foe.max_health * threshold =>
                {
                    pct
                }
                ItemPassive::GiantSlayer { per_100, max } => {
                    ((foe.max_health - me.max_health).max(0.0) / 100.0 * per_100).min(max)
                }
                ItemPassive::BigGame { pct }
                    if foe.kind.is_structure() || foe.kind == UnitKind::Camp =>
                {
                    pct
                }
                ItemPassive::AbilityAmp { pct } if !basic => pct,
                _ => 0.0,
            };
        }
        mult
    }

    /// Damage multiplier from the target's items: damage reduction, the
    /// elysian roll and ranged ward.
    pub(super) fn gear_incoming_mult(&mut self, target: usize, aggressor: Option<usize>) -> f32 {
        let Some(gear) = self.gear_of(Some(target)) else {
            return 1.0;
        };
        let ranged = aggressor.is_some_and(|a| self.units[a].attack_range >= 400.0);
        let mut cut = 0.0;
        let mut elysian = Vec::new();
        for passive in gear.passives() {
            match *passive {
                ItemPassive::DamageReduction { pct } => cut += pct,
                ItemPassive::RangedWard { pct } if ranged => cut += pct,
                ItemPassive::Elysian { chance, pct } => elysian.push((chance, pct)),
                _ => {}
            }
        }
        for (chance, pct) in elysian {
            if self.next_random() < chance {
                cut += pct;
            }
        }
        1.0 - cut.clamp(0.0, 0.75)
    }

    /// Share of the target's armor the attacker's items ignore.
    pub(super) fn gear_armor_pen(&self, aggressor: Option<usize>) -> f32 {
        self.gear_of(aggressor)
            .map(|gear| {
                gear.sum(|p| match *p {
                    ItemPassive::ArmorPen { pct } => Some(pct),
                    _ => None,
                })
            })
            .unwrap_or(0.0)
            .clamp(0.0, 0.7)
    }

    /// Kill gold multiplier from `unit`'s items.
    pub(super) fn gear_bounty(&self, unit: usize) -> f32 {
        1.0 + self
            .gear_of(Some(unit))
            .map(|gear| {
                gear.sum(|p| match *p {
                    ItemPassive::Bounty { pct } => Some(pct),
                    _ => None,
                })
            })
            .unwrap_or(0.0)
    }

    /// A thrall scroll's numbers after the hero's Thrall Master items.
    pub(super) fn gear_thrall(&self, unit: usize, spec: &ThrallSpec) -> ThrallSpec {
        let mut spec = spec.clone();
        if let Some(gear) = self.gear_of(Some(unit)) {
            for passive in gear.passives() {
                if let ItemPassive::ThrallMaster { pct, duration } = *passive {
                    spec.health *= 1.0 + pct;
                    spec.damage *= 1.0 + pct;
                    spec.duration += duration;
                }
            }
        }
        spec
    }

    /// Heal `index` by `amount` (halved under heal cut).
    fn gear_heal(&mut self, index: usize, amount: f32) {
        let unit = &mut self.units[index];
        if unit.health <= 0.0 || amount <= 0.0 {
            return;
        }
        let cut = if unit.cc.heal_cut_timer > 0.0 {
            0.5
        } else {
            1.0
        };
        let before = unit.health;
        unit.health = (unit.health + amount * cut).min(unit.max_health);
        let healed = unit.health - before;
        if healed >= QUIET_HEAL {
            self.push_hitsplat(index, healed, HitsplatKind::Heal, None);
        }
    }

    /// An item's own pure damage, which never triggers item effects.
    fn gear_strike(&mut self, attacker: usize, target: usize, amount: f32, kind: HitsplatKind) {
        if amount <= 0.0 || self.units[target].health <= 0.0 {
            return;
        }
        let team = self.units[attacker].team;
        self.gear_depth += 1;
        self.damage_unit_mixed_kind(target, 0.0, amount, team, Some(attacker), kind);
        self.gear_depth -= 1;
    }

    fn gear_statuses(&mut self, attacker: usize, target: usize, statuses: &[Status]) {
        if statuses.is_empty() || self.units[target].health <= 0.0 {
            return;
        }
        let from = self.units[attacker].pos;
        self.gear_depth += 1;
        for status in statuses {
            self.apply_status_from(target, status, from, attacker);
        }
        self.gear_depth -= 1;
    }

    /// Living enemies of `team` within `radius` of `at`.
    fn gear_enemies_near(&self, team: u8, at: Vec3, radius: f32) -> Vec<usize> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                u.health > 0.0
                    && u.team != team
                    && hittable(u.kind)
                    && !u.kind.is_structure()
                    && super::planar_delta(at, u.pos).length() <= radius + u.body_radius
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// After `amount` landed on `target` from `aggressor`: on-hit effects,
    /// lifesteal, spell vamp and specials for the attacker; reflect and
    /// last stand for the target.
    pub(super) fn gear_after_damage(
        &mut self,
        aggressor: Option<usize>,
        target: usize,
        amount: f32,
        basic: bool,
    ) {
        if self.gear_depth > 0 {
            return;
        }
        if let Some(attacker) = aggressor.filter(|&a| a != target)
            && self.gear_of(Some(attacker)).is_some()
            && self.units[attacker].health > 0.0
        {
            if basic {
                self.gear_on_attack(attacker, target, amount);
            } else {
                self.gear_on_ability(attacker, target, amount);
            }
        }
        self.gear_on_hurt(aggressor, target, amount, basic);
    }

    fn gear_on_attack(&mut self, attacker: usize, target: usize, amount: f32) {
        let Some(gear) = self.gear.get_mut(&attacker) else {
            return;
        };
        gear.attacks += 1;
        let attacks = gear.attacks;
        let special = gear.special.take();
        let spellblade = (gear.spellblade_ready > 0.0).then_some(());
        if spellblade.is_some() {
            gear.spellblade_ready = 0.0;
        }
        let passives: Vec<ItemPassive> = gear.passives().into_iter().cloned().collect();
        let melee = self.units[attacker].attack_range < 400.0;
        let team = self.units[attacker].team;
        let target_pos = self.units[target].pos;
        let mut dealt = amount;
        for passive in &passives {
            match passive {
                ItemPassive::Crit { chance, mult } => {
                    if self.next_random() < *chance {
                        let bonus = amount * (mult - 1.0);
                        self.gear_strike(attacker, target, bonus, HitsplatKind::MaxHit);
                        dealt += bonus;
                    }
                }
                ItemPassive::OnHit {
                    statuses,
                    every,
                    chance,
                    flat,
                    max_health,
                    current_health,
                    cap,
                } => {
                    if attacks % (*every).max(1) != 0 {
                        continue;
                    }
                    if *chance < 1.0 && self.next_random() >= *chance {
                        continue;
                    }
                    let foe = &self.units[target];
                    let mut bonus =
                        flat + foe.max_health * max_health + foe.health * current_health;
                    if *cap > 0.0 && !foe.kind.is_champion() {
                        bonus = bonus.min(*cap);
                    }
                    self.gear_strike(attacker, target, bonus, HitsplatKind::Damage);
                    dealt += bonus;
                    self.gear_statuses(attacker, target, statuses);
                }
                ItemPassive::Cleave { radius, pct } if melee => {
                    let others: Vec<usize> = self
                        .gear_enemies_near(team, target_pos, *radius)
                        .into_iter()
                        .filter(|&i| i != target)
                        .collect();
                    for other in others {
                        self.gear_strike(attacker, other, amount * pct, HitsplatKind::Damage);
                    }
                }
                ItemPassive::Ricochet {
                    bounces,
                    pct,
                    range,
                } if !melee => {
                    self.gear_ricochet(attacker, target, amount * pct, *bounces, *range);
                }
                ItemPassive::Frenzy {
                    per_hit,
                    max,
                    window,
                } => {
                    if let Some(gear) = self.gear.get_mut(&attacker) {
                        gear.frenzy = (gear.frenzy + per_hit).min(*max);
                        gear.frenzy_timer = *window;
                        self.units[attacker].item_attack_speed = gear.attack_speed();
                    }
                }
                ItemPassive::Spellblade {
                    damage,
                    ability,
                    cooldown,
                } if spellblade.is_some() => {
                    let unit = &self.units[attacker];
                    let bonus = unit.attack_damage * damage + unit.ability_power * ability;
                    self.gear_strike(attacker, target, bonus, HitsplatKind::MaxHit);
                    dealt += bonus;
                    if let Some(gear) = self.gear.get_mut(&attacker) {
                        gear.spellblade_cooldown = *cooldown;
                    }
                    let at = self.unit_chest(target);
                    self.emit_burst(at, [150, 120, 255], 12, 220.0, 12.0, 0.4);
                }
                _ => {}
            }
        }
        if let Some(special) = special {
            let at = self.unit_chest(target);
            self.emit_burst(at, [255, 220, 120], 18, 300.0, 14.0, 0.5);
            let mut extra_total = 0.0;
            for share in &special.extra {
                let hit = amount * share;
                self.gear_strike(attacker, target, hit, HitsplatKind::MaxHit);
                extra_total += hit;
            }
            dealt += extra_total;
            self.gear_statuses(attacker, target, &special.statuses);
            if special.splash > 0.0 {
                self.effects.push(Effect {
                    pos: target_pos,
                    radius: special.splash,
                    remaining: 0.4,
                    duration: 0.4,
                    color: [255, 150, 70],
                });
                let splash = (amount + extra_total) * 0.6;
                for other in self.gear_enemies_near(team, target_pos, special.splash) {
                    if other != target {
                        self.gear_strike(attacker, other, splash, HitsplatKind::Damage);
                    }
                }
            }
            if special.heal > 0.0 {
                self.gear_heal(attacker, dealt * special.heal);
            }
        }
        let lifesteal: f32 = passives
            .iter()
            .filter_map(|p| match *p {
                ItemPassive::Lifesteal { pct } => Some(pct),
                _ => None,
            })
            .sum();
        if lifesteal > 0.0 {
            self.gear_heal(attacker, dealt * lifesteal);
        }
    }

    /// Bounce a basic attack's share on from `from` to the nearest enemies
    /// it has not hit yet.
    fn gear_ricochet(
        &mut self,
        attacker: usize,
        from: usize,
        damage: f32,
        bounces: u32,
        range: f32,
    ) {
        let team = self.units[attacker].team;
        let look = self.units[attacker].look;
        let mut hit = vec![from];
        let mut at = from;
        for _ in 0..bounces {
            let origin = self.units[at].pos;
            let next = self
                .gear_enemies_near(team, origin, range)
                .into_iter()
                .filter(|i| !hit.contains(i))
                .min_by(|&a, &b| {
                    let da = super::planar_delta(origin, self.units[a].pos).length();
                    let db = super::planar_delta(origin, self.units[b].pos).length();
                    da.total_cmp(&db)
                });
            let Some(next) = next else {
                break;
            };
            let start = self.unit_chest(at);
            let goal = self.unit_chest(next);
            let mut shot = Projectile::homing(
                start,
                next,
                goal,
                1500.0,
                damage,
                team,
                Some(attacker),
                look,
            );
            shot.proc_hit = true;
            self.projectiles.push(shot);
            hit.push(next);
            at = next;
        }
    }

    fn gear_on_ability(&mut self, attacker: usize, target: usize, amount: f32) {
        let Some(gear) = self.gear.get(&attacker) else {
            return;
        };
        let ready = gear.ability_hit_cooldown <= 0.0;
        let mut vamp = 0.0;
        let mut statuses: Vec<Status> = Vec::new();
        let mut gap: f32 = 0.0;
        for passive in gear.passives() {
            match passive {
                ItemPassive::SpellVamp { pct } => vamp += pct,
                ItemPassive::AbilityHit {
                    statuses: s,
                    gap: g,
                } if ready => {
                    statuses.extend(s.iter().cloned());
                    gap = gap.max(*g);
                }
                _ => {}
            }
        }
        if vamp > 0.0 {
            self.gear_heal(attacker, amount * vamp);
        }
        if !statuses.is_empty() {
            if let Some(gear) = self.gear.get_mut(&attacker) {
                gear.ability_hit_cooldown = gap;
            }
            self.gear_statuses(attacker, target, &statuses);
        }
    }

    fn gear_on_hurt(&mut self, aggressor: Option<usize>, target: usize, amount: f32, basic: bool) {
        let Some(gear) = self.gear_of(Some(target)) else {
            return;
        };
        let mut reflect = 0.0;
        let mut last_stand = None;
        for passive in gear.passives() {
            match *passive {
                ItemPassive::Reflect { pct } => reflect += pct,
                ItemPassive::LastStand {
                    threshold,
                    heal,
                    cooldown,
                } if gear.last_stand_cooldown <= 0.0 => {
                    last_stand = Some((threshold, heal, cooldown));
                }
                _ => {}
            }
        }
        if basic
            && reflect > 0.0
            && let Some(attacker) = aggressor.filter(|&a| a != target)
            && self.units[attacker].health > 0.0
        {
            self.gear_strike(target, attacker, amount * reflect, HitsplatKind::Damage);
        }
        let unit = &self.units[target];
        if let Some((threshold, heal, cooldown)) = last_stand
            && unit.health > 0.0
            && unit.health < unit.max_health * threshold
        {
            let amount = unit.max_health * heal;
            let pos = unit.pos;
            if let Some(gear) = self.gear.get_mut(&target) {
                gear.last_stand_cooldown = cooldown;
            }
            self.gear_heal(target, amount);
            self.emit_burst(pos + Vec3::Y * 80.0, [255, 240, 170], 30, 300.0, 14.0, 0.9);
            if target == self.player_index {
                self.announcement = Some(("LAST STAND".to_string(), 2.0));
            }
        }
    }

    /// Timers, Spellblade arming, regeneration and auras, every frame.
    pub(super) fn gear_tick(&mut self, dt: f32) {
        let owners: Vec<usize> = self
            .gear
            .iter()
            .filter(|(_, gear)| !gear.pieces.is_empty())
            .map(|(&i, _)| i)
            .collect();
        for index in owners {
            let alive = self.units[index].health > 0.0;
            let cooldowns = self.units[index].cooldowns;
            let Some(gear) = self.gear.get_mut(&index) else {
                continue;
            };
            for piece in &mut gear.pieces {
                piece.cooldown = (piece.cooldown - dt).max(0.0);
            }
            gear.spellblade_cooldown = (gear.spellblade_cooldown - dt).max(0.0);
            gear.spellblade_ready = (gear.spellblade_ready - dt).max(0.0);
            gear.last_stand_cooldown = (gear.last_stand_cooldown - dt).max(0.0);
            gear.ability_hit_cooldown = (gear.ability_hit_cooldown - dt).max(0.0);
            if let Some(special) = gear.special.as_mut() {
                special.timer -= dt;
                if special.timer <= 0.0 {
                    gear.special = None;
                }
            }
            // A cooldown that jumped up means an ability was just cast.
            let cast = cooldowns
                .iter()
                .zip(gear.last_cooldowns.iter())
                .any(|(now, before)| *now > before + 0.25);
            gear.last_cooldowns = cooldowns;
            let has_spellblade = gear
                .passives()
                .iter()
                .any(|p| matches!(p, ItemPassive::Spellblade { .. }));
            if cast && has_spellblade && gear.spellblade_cooldown <= 0.0 {
                gear.spellblade_ready = SPELLBLADE_WINDOW;
            }
            let mut speed_dirty = false;
            if gear.frenzy_timer > 0.0 {
                gear.frenzy_timer -= dt;
                if gear.frenzy_timer <= 0.0 {
                    gear.frenzy = 0.0;
                    speed_dirty = true;
                }
            }
            if gear.rapid_timer > 0.0 {
                gear.rapid_timer -= dt;
                if gear.rapid_timer <= 0.0 {
                    gear.rapid = 0.0;
                    speed_dirty = true;
                }
            }
            let mut haste_back = 0.0;
            if gear.haste_timer > 0.0 {
                gear.haste_timer -= dt;
                if gear.haste_timer <= 0.0 {
                    haste_back = std::mem::take(&mut gear.haste);
                }
            }
            let mut shield_back = 0.0;
            if gear.shield_timer > 0.0 {
                gear.shield_timer -= dt;
                if gear.shield_timer <= 0.0 {
                    shield_back = std::mem::take(&mut gear.shield);
                }
            }
            let (mut regen_hp, mut regen_mp) = (0.0, 0.0);
            let mut auras: Vec<(f32, f32)> = Vec::new();
            for passive in gear.passives() {
                match *passive {
                    ItemPassive::Regen { health, mana } => {
                        regen_hp += health;
                        regen_mp += mana;
                    }
                    ItemPassive::Aura { radius, health } => auras.push((radius, health)),
                    _ => {}
                }
            }
            let attack_speed = gear.attack_speed();
            let unit = &mut self.units[index];
            if speed_dirty {
                unit.item_attack_speed = attack_speed;
            }
            unit.speed = (unit.speed - haste_back).max(100.0);
            unit.shield = (unit.shield - shield_back).max(0.0);
            if !alive {
                continue;
            }
            unit.health = (unit.health + regen_hp * dt).min(unit.max_health);
            unit.mana = (unit.mana + regen_mp * dt).min(unit.max_mana);
            let (team, pos) = (unit.team, unit.pos);
            for (radius, health) in auras {
                for ally in self.units.iter_mut().filter(|u| {
                    u.is_champion
                        && u.team == team
                        && u.health > 0.0
                        && super::planar_delta(pos, u.pos).length() <= radius
                }) {
                    ally.health = (ally.health + health * dt).min(ally.max_health);
                }
            }
        }
    }

    /// Seconds left and the full cooldown of `unit`'s active on item `id`
    /// (the readiest copy), when it has one.
    pub fn gear_cooldown(&self, unit: usize, id: &str) -> Option<(f32, f32)> {
        self.gear
            .get(&unit)?
            .pieces
            .iter()
            .filter(|p| p.id == id)
            .filter_map(|p| p.active.as_ref().map(|a| (p.cooldown, a.cooldown)))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// Use `unit`'s active on item `id`. Returns the active's name.
    pub fn use_gear_active(&mut self, unit: usize, id: &str) -> Result<String, &'static str> {
        if self.units[unit].health <= 0.0 {
            return Err("dead");
        }
        let Some(gear) = self.gear.get(&unit) else {
            return Err("no active");
        };
        let Some(at) = gear
            .pieces
            .iter()
            .position(|p| p.id == id && p.active.is_some() && p.cooldown <= 0.0)
        else {
            return Err(
                if gear.pieces.iter().any(|p| p.id == id && p.active.is_some()) {
                    "on cooldown"
                } else {
                    "no active"
                },
            );
        };
        let active = gear.pieces[at].active.clone().expect("checked");
        self.run_active(unit, &active.effect)?;
        if let Some(gear) = self.gear.get_mut(&unit) {
            gear.pieces[at].cooldown = active.cooldown;
        }
        tracing::info!("gear: unit {unit} used {} ({id})", active.name);
        Ok(active.name)
    }

    fn run_active(&mut self, unit: usize, effect: &ActiveEffect) -> Result<(), &'static str> {
        let team = self.units[unit].team;
        let pos = self.units[unit].pos;
        let power = 1.0 + self.units[unit].ability_power / 100.0;
        match effect {
            ActiveEffect::Special {
                extra,
                statuses,
                heal,
                splash,
                window,
            } => {
                if let Some(gear) = self.gear.get_mut(&unit) {
                    gear.special = Some(Primed {
                        extra: extra.clone(),
                        statuses: statuses.clone(),
                        heal: *heal,
                        splash: *splash,
                        timer: *window,
                    });
                }
                self.emit_burst(pos + Vec3::Y * 100.0, [255, 210, 90], 20, 200.0, 12.0, 0.6);
            }
            ActiveEffect::Nova {
                radius,
                damage,
                statuses,
                heal,
            } => {
                self.effects.push(Effect {
                    pos,
                    radius: *radius,
                    remaining: 0.5,
                    duration: 0.5,
                    color: [255, 170, 90],
                });
                self.emit_burst(pos + Vec3::Y * 40.0, [255, 190, 110], 36, 420.0, 16.0, 0.7);
                let mut dealt = 0.0;
                let mut foes = self.gear_enemies_near(team, pos, *radius);
                foes.retain(|&i| i != unit);
                for foe in foes {
                    let before = self.units[foe].health;
                    self.gear_strike(unit, foe, damage * power, HitsplatKind::Damage);
                    dealt += (before - self.units[foe].health).max(0.0);
                    self.gear_statuses(unit, foe, statuses);
                }
                if *heal > 0.0 {
                    self.gear_heal(unit, dealt * heal);
                }
            }
            ActiveEffect::Bolt {
                range,
                damage,
                statuses,
                mana,
                look,
            } => {
                let preferred = self.units[unit].attack_target.filter(|&t| {
                    let u = &self.units[t];
                    u.is_champion
                        && u.team != team
                        && u.health > 0.0
                        && super::planar_delta(pos, u.pos).length() <= *range
                });
                let target = preferred.or_else(|| {
                    self.gear_enemies_near(team, pos, *range)
                        .into_iter()
                        .filter(|&i| self.units[i].is_champion && self.units[i].visible)
                        .min_by(|&a, &b| {
                            let da = super::planar_delta(pos, self.units[a].pos).length();
                            let db = super::planar_delta(pos, self.units[b].pos).length();
                            da.total_cmp(&db)
                        })
                });
                let Some(target) = target else {
                    return Err("no enemy hero in range");
                };
                let look = fx::look_by_name(look.as_deref().unwrap_or("trident"));
                let from = self.unit_muzzle(unit);
                let goal = self.unit_chest(target);
                let mut shot = Projectile::homing(
                    from,
                    target,
                    goal,
                    1300.0,
                    damage * power,
                    team,
                    Some(unit),
                    look,
                );
                shot.proc_hit = true;
                if !statuses.is_empty() {
                    shot.spell = Some(Box::new(super::battle::SpellHit {
                        statuses: statuses.clone(),
                        gfx: Gfx::default(),
                        from: pos,
                    }));
                }
                self.projectiles.push(shot);
                self.muzzle_flash(from, look);
                let me = &mut self.units[unit];
                me.mana = (me.mana + mana).min(me.max_mana);
            }
            ActiveEffect::Shield {
                amount,
                max_health,
                duration,
            } => {
                let me = &mut self.units[unit];
                let shield = amount + me.max_health * max_health;
                me.shield += shield;
                if let Some(gear) = self.gear.get_mut(&unit) {
                    gear.shield += shield;
                    gear.shield_timer = gear.shield_timer.max(*duration);
                }
                self.emit_burst(pos + Vec3::Y * 80.0, [200, 225, 255], 24, 240.0, 14.0, 0.7);
            }
            ActiveEffect::Haste { pct, duration } => {
                let me = &mut self.units[unit];
                let add = me.speed * pct;
                me.speed += add;
                if let Some(gear) = self.gear.get_mut(&unit) {
                    gear.haste += add;
                    gear.haste_timer = gear.haste_timer.max(*duration);
                }
                self.emit_burst(pos + Vec3::Y * 20.0, [230, 240, 255], 16, 260.0, 10.0, 0.5);
            }
            ActiveEffect::Rapid { pct, duration } => {
                if let Some(gear) = self.gear.get_mut(&unit) {
                    gear.rapid = *pct;
                    gear.rapid_timer = *duration;
                    self.units[unit].item_attack_speed = gear.attack_speed();
                }
                self.emit_burst(pos + Vec3::Y * 90.0, [160, 255, 160], 18, 220.0, 10.0, 0.5);
            }
            ActiveEffect::Restore { health, mana } => {
                let (hp, mp) = {
                    let me = &self.units[unit];
                    (me.max_health * health, me.max_mana * mana)
                };
                self.gear_heal(unit, hp);
                let me = &mut self.units[unit];
                me.mana = (me.mana + mp).min(me.max_mana);
                self.emit_burst(pos + Vec3::Y * 80.0, [140, 200, 255], 24, 220.0, 12.0, 0.7);
            }
            ActiveEffect::Refresh { seconds } => {
                for cd in &mut self.units[unit].cooldowns {
                    *cd = (*cd - seconds).max(0.0);
                }
                if let Some(gear) = self.gear.get_mut(&unit) {
                    gear.last_cooldowns = self.units[unit].cooldowns;
                }
                self.emit_burst(pos + Vec3::Y * 100.0, [180, 150, 255], 24, 240.0, 12.0, 0.6);
            }
        }
        Ok(())
    }

    /// A bot's item actives: attacks and bursts when it is fighting a hero,
    /// shields, heals and haste when it is losing.
    pub(super) fn gear_bot_actives(&mut self, unit: usize) {
        let Some(gear) = self.gear.get(&unit) else {
            return;
        };
        let ready: Vec<(String, ActiveEffect)> = gear
            .pieces
            .iter()
            .filter(|p| p.cooldown <= 0.0)
            .filter_map(|p| p.active.as_ref().map(|a| (p.id.clone(), a.effect.clone())))
            .collect();
        if ready.is_empty() || self.units[unit].health <= 0.0 {
            return;
        }
        let me = &self.units[unit];
        let (team, pos) = (me.team, me.pos);
        let low = me.health < me.max_health * 0.4;
        let foe = me.attack_target.filter(|&t| {
            let u = &self.units[t];
            u.is_champion && u.team != team && u.health > 0.0
        });
        let foe_distance = foe.map(|t| super::planar_delta(pos, self.units[t].pos).length());
        let threatened = self
            .gear_enemies_near(team, pos, 900.0)
            .into_iter()
            .any(|i| self.units[i].is_champion);
        for (id, effect) in ready {
            let wanted = if effect.defensive() {
                low && threatened
            } else {
                match (&effect, foe_distance) {
                    (ActiveEffect::Nova { radius, .. }, Some(d)) => d <= *radius,
                    (ActiveEffect::Bolt { range, .. }, Some(d)) => d <= *range,
                    (ActiveEffect::Refresh { .. }, _) => {
                        threatened && self.units[unit].cooldowns.iter().any(|&c| c > 4.0)
                    }
                    (_, Some(d)) => d <= self.units[unit].attack_range + 150.0,
                    _ => false,
                }
            };
            if wanted {
                let _ = self.use_gear_active(unit, &id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(id: &str, pct: f32) -> GearPiece {
        GearPiece {
            id: id.to_string(),
            passives: vec![ItemPassive::Lifesteal { pct }],
            active: None,
            cooldown: 0.0,
        }
    }

    #[test]
    fn two_copies_of_an_item_give_its_passive_once() {
        let gear = Gear {
            pieces: vec![
                piece("torture", 0.1),
                piece("torture", 0.1),
                piece("guthan", 0.2),
            ],
            ..Gear::default()
        };
        let lifesteal = gear.sum(|p| match *p {
            ItemPassive::Lifesteal { pct } => Some(pct),
            _ => None,
        });
        assert!((lifesteal - 0.3).abs() < 1e-6, "{lifesteal}");
    }

    #[test]
    fn attack_speed_adds_items_frenzy_and_actives() {
        let gear = Gear {
            base_attack_speed: 0.25,
            frenzy: 0.12,
            rapid: 0.6,
            ..Gear::default()
        };
        assert!((gear.attack_speed() - 0.97).abs() < 1e-6);
    }
}
