//! Data-driven champion kits.
//!
//! A champion file can carry a `kit`: one optional passive and four
//! abilities. Each ability is a list of actions (projectile, area, zone,
//! dash, buff, summon, channel, empower, transform) and every action that
//! touches enemies carries a list of statuses (slow, root, stun, fear,
//! knockback, venom, ...). The runtime in `animated/kit_runtime.rs` executes
//! them; this module is plain serde data plus small pure helpers, so the same
//! description can later drive the deterministic `rift-sim`.

use serde::Deserialize;

/// A champion's passive and its Q, W, E and R.
#[derive(Debug, Clone, Deserialize)]
pub struct Kit {
    #[serde(default)]
    pub passive: Option<Passive>,
    pub abilities: [Ability; 4],
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ability {
    /// Mana cost.
    pub cost: f32,
    /// Seconds before the ability (or its next charge) is ready again.
    pub cooldown: f32,
    /// Charges held at once; each recharges over `cooldown`.
    #[serde(default = "one_u8")]
    pub charges: u8,
    /// Accent colour for this ability's effects.
    #[serde(default = "default_colour")]
    pub color: [u8; 3],
    /// Executed in order when the ability is cast.
    pub actions: Vec<Action>,
}

/// Where an area, zone or summon is placed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    /// The cursor, clamped to the action's range.
    #[default]
    Point,
    /// The caster's feet.
    Caster,
    /// Just in front of the caster, toward the cursor.
    Front,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// A straight skillshot. `count` > 1 fans the shots over `spread`
    /// degrees. `explode` splashes the damage around the first hit.
    Projectile {
        speed: f32,
        range: f32,
        damage: f32,
        #[serde(default = "default_width")]
        width: f32,
        #[serde(default)]
        pierce: bool,
        #[serde(default = "one_u32")]
        count: u32,
        #[serde(default)]
        spread: f32,
        #[serde(default)]
        explode: f32,
        /// Projectile look from `fx::LOOKS` (for example `fire_wave`).
        /// Without one the shot is an orb tinted by the ability colour.
        #[serde(default)]
        visual: Option<String>,
        #[serde(default)]
        on_hit: Vec<Status>,
    },
    /// A circle or cone that lands after `delay`, optionally repeating.
    Area {
        #[serde(default)]
        at: Anchor,
        radius: f32,
        #[serde(default)]
        damage: f32,
        #[serde(default)]
        delay: f32,
        /// Cone width in degrees around the aim direction; 360 is a circle.
        #[serde(default = "full_circle")]
        arc: f32,
        #[serde(default = "default_range")]
        range: f32,
        #[serde(default = "one_u32")]
        repeat: u32,
        #[serde(default)]
        interval: f32,
        /// Scatter each repeat this far around the anchor.
        #[serde(default)]
        scatter: f32,
        /// Heal the caster this much per enemy hit.
        #[serde(default)]
        heal_per_hit: f32,
        #[serde(default)]
        on_hit: Vec<Status>,
    },
    /// A lingering ground effect that ticks on enemies inside it.
    Zone {
        #[serde(default)]
        at: Anchor,
        radius: f32,
        duration: f32,
        /// Damage per tick.
        #[serde(default)]
        damage: f32,
        #[serde(default = "half")]
        tick: f32,
        #[serde(default = "default_range")]
        range: f32,
        #[serde(default = "one_u32")]
        count: u32,
        /// Spacing of extra zones along the aim direction (a line of pools).
        #[serde(default)]
        spacing: f32,
        /// Radius growth per second.
        #[serde(default)]
        grow: f32,
        /// Heal the caster per tick while standing inside.
        #[serde(default)]
        heal_caster: f32,
        #[serde(default)]
        on_tick: Vec<Status>,
    },
    /// Move the caster quickly along the ground, hitting what it passes.
    /// `to_cursor` stops at the cursor (up to `distance`). `land` runs at
    /// the end, which makes leaps and slams.
    Dash {
        distance: f32,
        #[serde(default = "default_dash_speed")]
        speed: f32,
        #[serde(default)]
        to_cursor: bool,
        #[serde(default)]
        backwards: bool,
        #[serde(default)]
        damage: f32,
        #[serde(default)]
        untargetable: bool,
        #[serde(default)]
        on_hit: Vec<Status>,
        #[serde(default)]
        land: Vec<Action>,
    },
    /// Self heal, shield and timed buffs.
    Buff {
        #[serde(default)]
        heal: f32,
        /// Fraction of missing health restored.
        #[serde(default)]
        heal_missing: f32,
        #[serde(default)]
        shield: f32,
        #[serde(default)]
        duration: f32,
        /// Fraction of incoming damage ignored.
        #[serde(default)]
        damage_reduction: f32,
        /// Fraction of melee damage sent back to the attacker.
        #[serde(default)]
        reflect: f32,
        /// Attack speed multiplier bonus (0.4 = +40%).
        #[serde(default)]
        attack_speed: f32,
        #[serde(default)]
        move_speed: f32,
        /// Seconds the caster cannot be damaged.
        #[serde(default)]
        untargetable: f32,
        /// Gold/heal over time is not modelled; `rooted` holds the caster
        /// still for `duration` (a channelled heal).
        #[serde(default)]
        rooted: bool,
    },
    /// Allied creatures that hunt nearby enemies for `duration`.
    Summon {
        npc: u32,
        #[serde(default = "one_u32")]
        count: u32,
        duration: f32,
        health: f32,
        damage: f32,
        #[serde(default = "default_summon_range")]
        range: f32,
        #[serde(default = "one")]
        interval: f32,
        #[serde(default = "default_summon_speed")]
        speed: f32,
        #[serde(default = "one")]
        scale: f32,
        #[serde(default)]
        at: Anchor,
        /// Optional variant NPCs; summon `i` uses `npcs[i]` when present.
        #[serde(default)]
        npcs: Vec<u32>,
    },
    /// Repeat an action every `interval` while the caster stands still.
    Channel {
        duration: f32,
        interval: f32,
        action: Box<Action>,
    },
    /// Strengthen the next basic attacks.
    Empower {
        #[serde(default = "default_empower_window")]
        duration: f32,
        #[serde(default = "one_u32")]
        attacks: u32,
        #[serde(default)]
        bonus: f32,
        /// Extra damage as a fraction of the target's missing health.
        #[serde(default)]
        missing_health: f32,
        /// Reset this ability slot's cooldown when the empowered hit kills.
        #[serde(default)]
        reset_on_kill: Option<usize>,
        #[serde(default)]
        on_hit: Vec<Status>,
    },
    /// Take another NPC's model for a while, with stat bonuses.
    Transform {
        npc: u32,
        duration: f32,
        #[serde(default = "one")]
        scale: f32,
        /// Basic attack damage multiplier bonus (0.3 = +30%).
        #[serde(default)]
        damage: f32,
        #[serde(default)]
        range: f32,
        #[serde(default)]
        move_speed: f32,
        #[serde(default)]
        heal: f32,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Status {
    Slow {
        pct: f32,
        duration: f32,
    },
    Root {
        duration: f32,
    },
    Stun {
        duration: f32,
    },
    /// Airborne: a stun with a lift.
    Knockup {
        duration: f32,
    },
    /// Run away from the caster.
    Fear {
        duration: f32,
    },
    /// Attack the caster.
    Taunt {
        duration: f32,
    },
    Knockback {
        distance: f32,
    },
    Pull {
        distance: f32,
    },
    /// Damage per tick (every second) for `duration`.
    Venom {
        damage: f32,
        duration: f32,
    },
    Bleed {
        damage: f32,
        duration: f32,
    },
    /// Healing received is halved.
    HealCut {
        duration: f32,
    },
    ManaDrain {
        amount: f32,
    },
    ArmorShred {
        amount: f32,
        duration: f32,
    },
    /// Extra damage equal to a fraction of current health.
    CurrentHealth {
        pct: f32,
    },
    /// A stacking slow; the third stack freezes (roots) the target.
    Chill {
        pct: f32,
        duration: f32,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Passive {
    /// Allied minions near the champion deal more damage.
    MinionAura { radius: f32, damage: f32 },
    /// Every `n`th basic attack also hits enemies around the target.
    NthAttack { n: u32, radius: f32, damage: f32 },
    /// The first drop below `threshold` per life heals and transforms.
    SecondForm {
        threshold: f32,
        heal: f32,
        npc: u32,
        duration: f32,
        #[serde(default = "one")]
        scale: f32,
        #[serde(default)]
        range: f32,
        #[serde(default)]
        damage: f32,
    },
    /// Basic attacks apply these statuses.
    OnHit { statuses: Vec<Status> },
    /// Incoming projectile damage is reduced.
    ThickHide { reduction: f32 },
    /// Permanent move speed; dashing empowers the next basic attack.
    Swift { move_speed: f32, dash_bonus: f32 },
    /// Casting a different ability than the last one stacks bonus damage.
    SpellCombo {
        per_stack: f32,
        max: u32,
        window: f32,
    },
    /// Basic attacks splash around the target.
    Splash { radius: f32, pct: f32 },
    /// More experience from every kill and assist.
    Scholar { xp: f32 },
    /// Chill statuses stack to a freeze (handled by `Status::Chill`); this
    /// passive also chills on basic attacks.
    FrostBite { pct: f32, duration: f32 },
}

/// Crowd control and summon state carried by every unit.
#[derive(Debug, Clone, Default)]
pub struct Afflictions {
    pub slow_pct: f32,
    pub slow_timer: f32,
    pub root_timer: f32,
    pub fear_timer: f32,
    pub fear_from: glam::Vec3,
    pub taunt_timer: f32,
    pub taunt_by: usize,
    pub chill_stacks: u32,
    pub chill_timer: f32,
    pub heal_cut_timer: f32,
    /// Forced movement (knockback or pull), world units per second.
    pub displace: glam::Vec3,
    pub displace_timer: f32,
    /// Knock-up height animation, seconds left.
    pub lift_timer: f32,
    /// A kit summon: seconds left before it expires.
    pub summoned: bool,
    pub life_timer: f32,
}

impl Afflictions {
    /// Movement multiplier from slows and roots.
    pub fn move_mult(&self) -> f32 {
        if self.root_timer > 0.0 || self.displace_timer > 0.0 {
            0.0
        } else {
            (1.0 - self.slow_pct).clamp(0.1, 1.0)
        }
    }

    /// Count every timer down. Returns true when a summon has just expired.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.slow_timer = (self.slow_timer - dt).max(0.0);
        if self.slow_timer <= 0.0 {
            self.slow_pct = 0.0;
        }
        self.root_timer = (self.root_timer - dt).max(0.0);
        self.fear_timer = (self.fear_timer - dt).max(0.0);
        self.taunt_timer = (self.taunt_timer - dt).max(0.0);
        self.heal_cut_timer = (self.heal_cut_timer - dt).max(0.0);
        self.lift_timer = (self.lift_timer - dt).max(0.0);
        self.chill_timer = (self.chill_timer - dt).max(0.0);
        if self.chill_timer <= 0.0 {
            self.chill_stacks = 0;
        }
        self.displace_timer = (self.displace_timer - dt).max(0.0);
        if self.summoned && self.life_timer > 0.0 {
            self.life_timer -= dt;
            return self.life_timer <= 0.0;
        }
        false
    }
}

impl Kit {
    /// Cooldowns in slot order, for the HUD's sweep maxima.
    pub fn cooldowns(&self) -> [f32; 4] {
        std::array::from_fn(|slot| self.abilities[slot].cooldown)
    }

    /// Every NPC the kit may need at runtime (summons and transforms), so
    /// the scene can load their models up front.
    pub fn npcs(&self) -> Vec<u32> {
        let mut out = Vec::new();
        if let Some(Passive::SecondForm { npc, .. }) = &self.passive {
            out.push(*npc);
        }
        for ability in &self.abilities {
            for action in &ability.actions {
                collect_npcs(action, &mut out);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

fn collect_npcs(action: &Action, out: &mut Vec<u32>) {
    match action {
        Action::Summon { npc, npcs, .. } => {
            out.push(*npc);
            out.extend(npcs.iter().copied());
        }
        Action::Transform { npc, .. } => out.push(*npc),
        Action::Channel { action, .. } => collect_npcs(action, out),
        Action::Dash { land, .. } => land.iter().for_each(|a| collect_npcs(a, out)),
        _ => {}
    }
}

/// Ability damage after ability power and level.
pub fn scaled_damage(base: f32, ability_power: f32, level: u32) -> f32 {
    base * (1.0 + ability_power / 100.0) * (1.0 + 0.05 * level.saturating_sub(1) as f32)
}

/// True when `offset` (planar, from the area's centre) lies inside a cone of
/// `arc` degrees around `facing`. A 360 degree arc is a full circle.
pub fn in_arc(offset: glam::Vec2, facing: glam::Vec2, arc: f32) -> bool {
    if arc >= 359.0 || offset.length_squared() < 1.0 {
        return true;
    }
    let half = (arc * 0.5).to_radians();
    offset.normalize().dot(facing.normalize_or_zero()) >= half.cos()
}

fn one() -> f32 {
    1.0
}
fn one_u8() -> u8 {
    1
}
fn one_u32() -> u32 {
    1
}
fn half() -> f32 {
    0.5
}
fn full_circle() -> f32 {
    360.0
}
fn default_width() -> f32 {
    70.0
}
fn default_range() -> f32 {
    1100.0
}
fn default_dash_speed() -> f32 {
    1900.0
}
fn default_summon_range() -> f32 {
    130.0
}
fn default_summon_speed() -> f32 {
    400.0
}
fn default_empower_window() -> f32 {
    5.0
}
fn default_colour() -> [u8; 3] {
    [255, 200, 120]
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn every_action_and_status_parses() {
        let json = r#"{
            "passive": {"type": "nth_attack", "n": 3, "radius": 250, "damage": 0.6},
            "abilities": [
                {"cost": 30, "cooldown": 6, "actions": [
                    {"type": "projectile", "speed": 1200, "range": 900, "damage": 40,
                     "count": 3, "spread": 30, "on_hit": [{"type": "slow", "pct": 0.3, "duration": 1.5}]},
                    {"type": "area", "radius": 200, "arc": 90, "damage": 20,
                     "on_hit": [{"type": "knockback", "distance": 200}]}
                ]},
                {"cost": 40, "cooldown": 9, "charges": 2, "actions": [
                    {"type": "dash", "distance": 500, "to_cursor": true, "untargetable": true,
                     "land": [{"type": "area", "at": "caster", "radius": 250,
                               "on_hit": [{"type": "knockup", "duration": 1}]}]}
                ]},
                {"cost": 50, "cooldown": 12, "actions": [
                    {"type": "buff", "shield": 80, "duration": 3, "damage_reduction": 0.3},
                    {"type": "summon", "npc": 6631, "count": 3, "duration": 8, "health": 200, "damage": 12},
                    {"type": "zone", "radius": 160, "duration": 4, "damage": 8, "count": 3, "spacing": 200,
                     "on_tick": [{"type": "venom", "damage": 6, "duration": 4}]}
                ]},
                {"cost": 100, "cooldown": 60, "actions": [
                    {"type": "channel", "duration": 2.5, "interval": 0.25,
                     "action": {"type": "projectile", "speed": 1400, "range": 1200, "damage": 30}},
                    {"type": "empower", "bonus": 50, "reset_on_kill": 1},
                    {"type": "transform", "npc": 965, "duration": 8, "damage": 0.3},
                    {"type": "area", "radius": 300,
                     "on_hit": [{"type": "fear", "duration": 1.5}, {"type": "taunt", "duration": 1},
                                {"type": "root", "duration": 1}, {"type": "stun", "duration": 1},
                                {"type": "pull", "distance": 300}, {"type": "bleed", "damage": 5, "duration": 3},
                                {"type": "heal_cut", "duration": 4}, {"type": "mana_drain", "amount": 40},
                                {"type": "armor_shred", "amount": 10, "duration": 4},
                                {"type": "current_health", "pct": 0.3},
                                {"type": "chill", "pct": 0.2, "duration": 2}]}
                ]}
            ]
        }"#;
        let kit: Kit = serde_json::from_str(json).expect("kit parses");
        assert_eq!(kit.abilities[1].charges, 2);
        assert_eq!(kit.cooldowns(), [6.0, 9.0, 12.0, 60.0]);
        assert_eq!(kit.npcs(), vec![965, 6631]);
    }

    #[test]
    fn afflictions_slow_root_and_expire() {
        let mut cc = Afflictions {
            slow_pct: 0.4,
            slow_timer: 1.0,
            ..Default::default()
        };
        assert!((cc.move_mult() - 0.6).abs() < 1e-6);
        cc.root_timer = 0.5;
        assert_eq!(cc.move_mult(), 0.0);
        assert!(!cc.tick(0.6));
        assert!(
            (cc.move_mult() - 0.6).abs() < 1e-6,
            "root ended, slow remains"
        );
        assert!(!cc.tick(0.5));
        assert_eq!(cc.move_mult(), 1.0);
        cc.summoned = true;
        cc.life_timer = 0.2;
        assert!(cc.tick(0.3), "summon expires");
    }

    #[test]
    fn damage_scales_with_power_and_level() {
        assert_eq!(scaled_damage(100.0, 0.0, 1), 100.0);
        assert!((scaled_damage(100.0, 50.0, 1) - 150.0).abs() < 1e-3);
        assert!((scaled_damage(100.0, 0.0, 11) - 150.0).abs() < 1e-3);
    }

    #[test]
    fn cones_include_only_the_facing_side() {
        let facing = Vec2::new(1.0, 0.0);
        assert!(in_arc(Vec2::new(100.0, 10.0), facing, 90.0));
        assert!(!in_arc(Vec2::new(-100.0, 0.0), facing, 90.0));
        assert!(!in_arc(Vec2::new(10.0, 100.0), facing, 90.0));
        assert!(in_arc(Vec2::new(-100.0, 0.0), facing, 360.0));
    }
}
