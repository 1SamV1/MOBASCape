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
    /// The passive's name on hero select ("Three heads"); its description
    /// is written from the numbers by [`Passive::describe`].
    #[serde(default)]
    pub passive_name: Option<String>,
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

/// Cache spot animations an action plays (docs/SPELLS.md). Every field is
/// optional; an action without a block keeps its tinted rings and glows.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Gfx {
    /// Plays on the caster as the action starts.
    #[serde(default)]
    pub cast: Option<u32>,
    /// Plays where an area lands, where a zone is laid, or where a
    /// projectile first strikes.
    #[serde(default)]
    pub land: Option<u32>,
    /// Plays on every unit the action hits.
    #[serde(default)]
    pub hit: Option<u32>,
    /// Stays on a unit while the action's crowd control lasts, in place of
    /// the status's own graphic (an ice block instead of no graphic on a
    /// root).
    #[serde(default)]
    pub hold: Option<u32>,
    /// Size multiplier for every graphic in the block.
    #[serde(default = "one")]
    pub scale: f32,
}

impl Default for Gfx {
    fn default() -> Self {
        Self {
            cast: None,
            land: None,
            hit: None,
            hold: None,
            scale: 1.0,
        }
    }
}

impl Gfx {
    fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        [self.cast, self.land, self.hit, self.hold]
            .into_iter()
            .flatten()
    }
}

/// Spot animations the runtime plays without a kit naming them.
pub mod spot {
    /// Circling stars over a stunned or knocked-up unit.
    pub const STUN: u32 = 80;
    /// A spell striking a shield or an untargetable unit.
    pub const SPLASH: u32 = 85;
    /// The Ice Barrage block around a frozen unit (three chills).
    pub const ICE_BLOCK: u32 = 369;
    /// Purple bars over a tele blocked unit.
    pub const TELE_BLOCK: u32 = 342;
    /// The Chaos Elemental's red spikes over a disarmed unit.
    pub const DISARM: u32 = 559;
    /// The Chaos Elemental's green Confusion splash, where a scrambled unit
    /// leaves and where it lands.
    pub const SCRAMBLE: u32 = 552;
    /// Every graphic above.
    pub const ALL: [u32; 6] = [STUN, SPLASH, ICE_BLOCK, TELE_BLOCK, DISARM, SCRAMBLE];

    /// Spot animations someone has watched play and named in
    /// docs/SPELLS.md. Kits may only use these, so a typo cannot ship a
    /// random cache graphic.
    pub const VERIFIED: &[u32] = &[
        76, 77, 78, 80, 85, 111, 177, 178, 342, 343, 344, 345, 359, 360, 361, 362, 363, 368, 369,
        372, 374, 377, 392, 550, 551, 552, 553, 554, 555, 556, 557, 558, 559, 678, 725, 726, 744,
        745, 747, 750, 1290, 1455, 1456, 1457, 1458, 1459, 1460, 1461, 1462, 1463, 1464, 1465,
        1466, 1526, 1527, 1528, 1568, 1569, 1574, 1576, 1579,
    ];
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
        #[serde(default)]
        gfx: Gfx,
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
        #[serde(default)]
        gfx: Gfx,
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
        /// The zone rides on the caster instead of staying where it was laid
        /// (an aura such as the Bloat's fly swarm). Its `land` graphic is
        /// held on the caster for the duration.
        #[serde(default)]
        follow: bool,
        /// Health the caster loses per tick (never below 1), the price of a
        /// rotting aura.
        #[serde(default)]
        self_damage: f32,
        #[serde(default)]
        on_tick: Vec<Status>,
        #[serde(default)]
        gfx: Gfx,
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
        #[serde(default)]
        gfx: Gfx,
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
        #[serde(default)]
        gfx: Gfx,
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
        #[serde(default)]
        gfx: Gfx,
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
        #[serde(default)]
        gfx: Gfx,
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
        #[serde(default)]
        gfx: Gfx,
    },
}

impl Action {
    /// The action's graphics block; a channel uses its repeated action's.
    pub fn gfx(&self) -> &Gfx {
        match self {
            Action::Projectile { gfx, .. }
            | Action::Area { gfx, .. }
            | Action::Zone { gfx, .. }
            | Action::Dash { gfx, .. }
            | Action::Buff { gfx, .. }
            | Action::Summon { gfx, .. }
            | Action::Empower { gfx, .. }
            | Action::Transform { gfx, .. } => gfx,
            Action::Channel { action, .. } => action.gfx(),
        }
    }

    /// The shared sound (`data/sounds.json`) this action makes when it
    /// starts. Buffs are scored by their strongest effect: a heal chimes
    /// like Rapid Heal, a shield like Steel Skin, a haste like Incredible
    /// Reflexes. Areas sound when they land, not here.
    pub fn sound_name(&self) -> Option<&'static str> {
        Some(match self {
            Action::Projectile { .. } => "kit_projectile",
            Action::Area { .. } => return None,
            Action::Zone { .. } => "kit_zone",
            Action::Dash { .. } => "kit_dash",
            Action::Buff {
                heal,
                heal_missing,
                shield,
                damage_reduction,
                reflect,
                untargetable,
                attack_speed,
                move_speed,
                ..
            } => {
                if heal + heal_missing > 0.0 {
                    "kit_heal"
                } else if shield + damage_reduction + reflect + untargetable > 0.0 {
                    "kit_shield"
                } else if attack_speed + move_speed > 0.0 {
                    "kit_haste"
                } else {
                    "kit_buff"
                }
            }
            Action::Summon { .. } => "kit_summon",
            Action::Channel { .. } => "kit_channel",
            Action::Empower { .. } => "kit_empower",
            Action::Transform { .. } => "kit_transform",
        })
    }
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
    /// Tele Block: no dashes, blinks or teleport scroll for `duration`, and
    /// a scroll being read breaks.
    NoMobility {
        duration: f32,
    },
    /// No basic attacks for `duration` (the Chaos Elemental's Madness).
    Disarm {
        duration: f32,
    },
    /// Thrown `distance` in a random direction (its Confusion).
    Scramble {
        distance: f32,
    },
    /// One of these, picked at random each time it lands.
    OneOf {
        of: Vec<Status>,
    },
}

impl Status {
    /// Crowd control that can carry a held graphic: the status's own
    /// graphic, if it has one, and how long it holds. Chill's freeze plays
    /// its ice block when the third stack lands, not here.
    pub fn hold(&self) -> Option<(Option<u32>, f32)> {
        match *self {
            Status::Stun { duration } | Status::Knockup { duration } => {
                Some((Some(spot::STUN), duration))
            }
            Status::Root { duration } | Status::Fear { duration } | Status::Taunt { duration } => {
                Some((None, duration))
            }
            Status::NoMobility { duration } => Some((Some(spot::TELE_BLOCK), duration)),
            Status::Disarm { duration } => Some((Some(spot::DISARM), duration)),
            _ => None,
        }
    }
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
    /// Every `every`th ability hit also lands one of `statuses`, at random.
    Chaos { every: u32, statuses: Vec<Status> },
    /// Each enemy that dies within `radius` grows the caster's maximum (and
    /// current) health for good: `minion` for anything, `hero` for a hero.
    FleshHeap { radius: f32, minion: f32, hero: f32 },
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
    /// Tele Block: no dashes, blinks or scroll.
    pub no_mobility_timer: f32,
    /// Disarm: no basic attacks.
    pub disarm_timer: f32,
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
        self.no_mobility_timer = (self.no_mobility_timer - dt).max(0.0);
        self.disarm_timer = (self.disarm_timer - dt).max(0.0);
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

impl Kit {
    /// Every spot animation the kit may play, plus the runtime's own status
    /// graphics, so the scene can load them with the match.
    pub fn spotanims(&self) -> Vec<u32> {
        let mut out = spot::ALL.to_vec();
        for ability in &self.abilities {
            for action in &ability.actions {
                collect_spotanims(action, &mut out);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

fn collect_spotanims(action: &Action, out: &mut Vec<u32>) {
    out.extend(action.gfx().ids());
    if let Action::Dash { land, .. } = action {
        land.iter().for_each(|a| collect_spotanims(a, out));
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

/// A share as a whole percentage: 0.15 -> "15".
fn pct(share: f32) -> String {
    format!("{:.0}", share * 100.0)
}

/// Status phrases joined as alternatives: "a, b or c".
fn either(statuses: &[Status]) -> String {
    let parts: Vec<String> = statuses.iter().map(Status::describe).collect();
    match parts.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
    }
}

/// Seconds without trailing zeros: 2.5 -> "2.5", 6.0 -> "6", 1.25 ->
/// "1.25".
fn secs(seconds: f32) -> String {
    let text = format!("{seconds:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn ordinal(n: u32) -> String {
    match n {
        2 => "second".to_string(),
        3 => "third".to_string(),
        4 => "fourth".to_string(),
        5 => "fifth".to_string(),
        n => {
            let suffix = match (n % 10, n % 100) {
                (1, x) if x != 11 => "st",
                (2, x) if x != 12 => "nd",
                (3, x) if x != 13 => "rd",
                _ => "th",
            };
            format!("{n}{suffix}")
        }
    }
}

impl Status {
    /// What the status does to whoever it lands on, as a verb phrase
    /// ("slow by 15% for 1 s").
    pub fn describe(&self) -> String {
        match self {
            Status::Slow { pct: p, duration } => {
                format!("slow by {}% for {} s", pct(*p), secs(*duration))
            }
            Status::Root { duration } => format!("root for {} s", secs(*duration)),
            Status::Stun { duration } => format!("stun for {} s", secs(*duration)),
            Status::Knockup { duration } => format!("knock up for {} s", secs(*duration)),
            Status::Fear { duration } => format!("fear for {} s", secs(*duration)),
            Status::Taunt { duration } => format!("taunt for {} s", secs(*duration)),
            Status::Knockback { distance } => format!("knock back {distance:.0}"),
            Status::Pull { distance } => format!("pull in {distance:.0}"),
            Status::Venom { damage, duration } => {
                format!(
                    "envenom for {damage:.0} damage a second over {} s",
                    secs(*duration)
                )
            }
            Status::Bleed { damage, duration } => {
                format!(
                    "bleed for {damage:.0} damage a second over {} s",
                    secs(*duration)
                )
            }
            Status::HealCut { duration } => format!("halve healing for {} s", secs(*duration)),
            Status::ManaDrain { amount } => format!("drain {amount:.0} mana"),
            Status::ArmorShred { amount, duration } => {
                format!("shred {amount:.0} armour for {} s", secs(*duration))
            }
            Status::CurrentHealth { pct: p } => format!("deal {}% of current health", pct(*p)),
            Status::Chill { pct: p, duration } => {
                format!("chill, slowing by {}% for {} s", pct(*p), secs(*duration))
            }
            Status::NoMobility { duration } => {
                format!(
                    "block dashes, blinks and teleports for {} s",
                    secs(*duration)
                )
            }
            Status::Disarm { duration } => format!("disarm for {} s", secs(*duration)),
            Status::Scramble { distance } => {
                format!("throw {distance:.0} in a random direction")
            }
            Status::OneOf { of } => format!("one of: {}", either(of)),
        }
    }
}

impl Passive {
    /// The passive in a sentence, written from its numbers so hero select
    /// never disagrees with the match.
    pub fn describe(&self) -> String {
        match self {
            Passive::MinionAura { radius, damage } => format!(
                "Allied minions within {radius:.0} range deal {}% more damage.",
                pct(*damage)
            ),
            Passive::NthAttack { n, radius, damage } => format!(
                "Every {} basic attack also hits enemies within {radius:.0} of the target for {}% damage.",
                ordinal(*n),
                pct(*damage)
            ),
            Passive::SecondForm {
                threshold,
                heal,
                duration,
                range,
                damage,
                ..
            } => {
                let mut extras = Vec::new();
                if *damage > 0.0 {
                    extras.push(format!("{}% more attack damage", pct(*damage)));
                }
                if *range > 0.0 {
                    extras.push(format!("{range:.0} more attack range"));
                }
                let with = if extras.is_empty() {
                    String::new()
                } else {
                    format!(", with {}", extras.join(" and "))
                };
                format!(
                    "Once per life, falling below {}% health heals {}% of max health and transforms for {} s{with}.",
                    pct(*threshold),
                    pct(*heal),
                    secs(*duration)
                )
            }
            Passive::OnHit { statuses } => {
                let effects: Vec<String> = statuses.iter().map(Status::describe).collect();
                format!("Basic attacks {}.", effects.join(" and "))
            }
            Passive::ThickHide { reduction } => {
                format!("Takes {}% less damage from projectiles.", pct(*reduction))
            }
            Passive::Swift {
                move_speed,
                dash_bonus,
            } => format!(
                "Moves {}% faster. A dash empowers the next basic attack with {dash_bonus:.0} bonus damage.",
                pct(*move_speed)
            ),
            Passive::SpellCombo {
                per_stack,
                max,
                window,
            } => format!(
                "Each ability unlike the last adds {}% ability damage, stacking {max} times; the stacks fade after {} s.",
                pct(*per_stack),
                secs(*window)
            ),
            Passive::Splash { radius, pct: p } => format!(
                "Basic attacks splash {}% damage onto enemies within {radius:.0} of the target.",
                pct(*p)
            ),
            Passive::Scholar { xp } => {
                format!(
                    "Earns {}% more experience from kills and assists.",
                    pct(*xp)
                )
            }
            Passive::FrostBite { pct: p, duration } => format!(
                "Basic attacks chill, slowing by {}% for {} s; three chills freeze.",
                pct(*p),
                secs(*duration)
            ),
            Passive::FleshHeap {
                radius,
                minion,
                hero,
            } => format!(
                "Every enemy that dies within {radius:.0} adds {minion:.0} maximum health \
                 for good ({hero:.0} for a hero)."
            ),
            Passive::Chaos { every, statuses } => format!(
                "Every {} ability hit also lands one of: {}.",
                ordinal(*every),
                either(statuses)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn collect_sounds(action: &Action, out: &mut Vec<&'static str>) {
        out.extend(action.sound_name());
        match action {
            Action::Area { .. } => out.push("kit_area"),
            Action::Dash { land, .. } => land.iter().for_each(|a| collect_sounds(a, out)),
            Action::Channel { action, .. } => collect_sounds(action, out),
            _ => {}
        }
    }

    #[test]
    fn passives_describe_themselves_from_their_numbers() {
        let passive = |json: &str| -> Passive { serde_json::from_str(json).unwrap() };
        assert_eq!(
            passive(r#"{"type": "nth_attack", "n": 3, "radius": 260, "damage": 0.7}"#).describe(),
            "Every third basic attack also hits enemies within 260 of the target for 70% damage."
        );
        assert_eq!(
            passive(r#"{"type": "on_hit", "statuses": [{"type": "slow", "pct": 0.15, "duration": 1.0}]}"#)
                .describe(),
            "Basic attacks slow by 15% for 1 s."
        );
        assert_eq!(
            passive(
                r#"{"type": "second_form", "threshold": 0.3, "heal": 0.25, "npc": 965,
                    "duration": 8, "range": 250, "damage": 0.3}"#
            )
            .describe(),
            "Once per life, falling below 30% health heals 25% of max health and transforms \
             for 8 s, with 30% more attack damage and 250 more attack range."
        );
        assert_eq!(
            passive(r#"{"type": "frost_bite", "pct": 0.15, "duration": 2.5}"#).describe(),
            "Basic attacks chill, slowing by 15% for 2.5 s; three chills freeze."
        );
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(12), "12th");
    }

    #[test]
    fn a_gfx_block_parses_with_defaults_and_its_ids_are_collected() {
        let kit: Kit = serde_json::from_str(
            r#"{"abilities": [
                {"cost": 0, "cooldown": 1, "actions": [
                    {"type": "area", "radius": 100, "gfx": {"land": 77, "hit": 77}}]},
                {"cost": 0, "cooldown": 1, "actions": [
                    {"type": "dash", "distance": 300, "land": [
                        {"type": "area", "radius": 100, "gfx": {"hit": 363, "scale": 1.5}}]}]},
                {"cost": 0, "cooldown": 1, "actions": [
                    {"type": "channel", "duration": 1, "interval": 0.5, "action":
                        {"type": "projectile", "speed": 1, "range": 1, "damage": 1,
                         "gfx": {"cast": 744}}}]},
                {"cost": 0, "cooldown": 1, "actions": [{"type": "buff", "heal": 10}]}
            ]}"#,
        )
        .unwrap();
        let plain = kit.abilities[3].actions[0].gfx();
        assert_eq!(plain, &Gfx::default());
        // An action with no block and one with an empty block agree.
        assert_eq!(plain.scale, 1.0);
        let area = kit.abilities[0].actions[0].gfx();
        assert_eq!((area.land, area.hit, area.scale), (Some(77), Some(77), 1.0));
        // A channel answers with its repeated action's block.
        assert_eq!(kit.abilities[2].actions[0].gfx().cast, Some(744));
        let mut want = vec![77, 363, 744];
        want.extend(spot::ALL);
        want.sort_unstable();
        assert_eq!(kit.spotanims(), want);
    }

    #[test]
    fn flesh_heap_and_following_zones_parse() {
        let passive: Passive = serde_json::from_str(
            r#"{ "type": "flesh_heap", "radius": 450, "minion": 2, "hero": 25 }"#,
        )
        .unwrap();
        assert_eq!(
            passive.describe(),
            "Every enemy that dies within 450 adds 2 maximum health for good (25 for a hero)."
        );
        let zone: Action = serde_json::from_str(
            r#"{ "type": "zone", "at": "caster", "follow": true, "radius": 260,
                 "duration": 5, "damage": 12, "self_damage": 4 }"#,
        )
        .unwrap();
        let Action::Zone {
            follow,
            self_damage,
            ..
        } = zone
        else {
            panic!("a zone");
        };
        assert!(follow);
        assert_eq!(self_damage, 4.0);
    }

    #[test]
    fn chaos_statuses_parse_describe_and_hold() {
        let statuses: Vec<Status> = serde_json::from_str(
            r#"[
                { "type": "disarm", "duration": 2 },
                { "type": "scramble", "distance": 350 },
                { "type": "one_of", "of": [
                    { "type": "mana_drain", "amount": 40 },
                    { "type": "scramble", "distance": 250 },
                    { "type": "disarm", "duration": 1.5 }
                ] }
            ]"#,
        )
        .unwrap();
        let text: Vec<String> = statuses.iter().map(Status::describe).collect();
        assert_eq!(
            text,
            [
                "disarm for 2 s",
                "throw 350 in a random direction",
                "one of: drain 40 mana, throw 250 in a random direction or disarm for 1.5 s",
            ]
        );
        assert_eq!(statuses[0].hold(), Some((Some(spot::DISARM), 2.0)));
        assert_eq!(statuses[1].hold(), None);
        let passive: Passive = serde_json::from_str(
            r#"{ "type": "chaos", "every": 3, "statuses": [
                { "type": "slow", "pct": 0.3, "duration": 1.5 },
                { "type": "mana_drain", "amount": 25 }
            ] }"#,
        )
        .unwrap();
        assert_eq!(
            passive.describe(),
            "Every third ability hit also lands one of: slow by 30% for 1.5 s or drain 25 mana."
        );
        let mut cc = Afflictions {
            disarm_timer: 1.0,
            ..Default::default()
        };
        cc.tick(0.6);
        assert!(cc.disarm_timer > 0.0);
        cc.tick(0.6);
        assert_eq!(cc.disarm_timer, 0.0);
    }

    #[test]
    fn crowd_control_holds_its_own_graphic_or_leaves_room_for_one() {
        let stun = Status::Stun { duration: 1.5 };
        assert_eq!(stun.hold(), Some((Some(spot::STUN), 1.5)));
        assert_eq!(
            Status::Knockup { duration: 0.75 }.hold(),
            Some((Some(spot::STUN), 0.75))
        );
        // A root has no graphic of its own: ice and vines both root.
        assert_eq!(Status::Root { duration: 1.25 }.hold(), Some((None, 1.25)));
        assert_eq!(Status::ManaDrain { amount: 10.0 }.hold(), None);
        assert_eq!(
            Status::Chill {
                pct: 0.3,
                duration: 2.0
            }
            .hold(),
            None,
            "the freeze plays its ice block on the third stack"
        );
    }

    #[test]
    fn shipped_kits_only_use_verified_graphics() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/champions");
        let mut used = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let config: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let Some(kit) = config.get("kit") else {
                continue;
            };
            let kit: Kit = serde_json::from_value(kit.clone()).unwrap();
            for id in kit.spotanims() {
                assert!(
                    spot::VERIFIED.contains(&id),
                    "{} uses spot animation {id}, which docs/SPELLS.md has not verified",
                    path.display()
                );
                used += 1;
            }
        }
        assert!(used > 0);
    }

    #[test]
    fn every_shipped_passive_has_a_name_and_a_sentence() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/champions");
        let mut passives = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let config: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let Some(kit) = config.get("kit") else {
                continue;
            };
            let kit: Kit = serde_json::from_value(kit.clone()).unwrap();
            let Some(passive) = &kit.passive else {
                continue;
            };
            passives += 1;
            let name = kit.passive_name.as_deref().unwrap_or_default();
            assert!(
                !name.trim().is_empty(),
                "{} names its passive",
                path.display()
            );
            let text = passive.describe();
            assert!(
                text.ends_with('.') && !text.contains("NaN") && !text.contains("inf"),
                "{}: {text}",
                path.display()
            );
        }
        assert!(passives >= 12, "the twelve kit passives, found {passives}");
    }

    #[test]
    fn every_kit_action_has_a_shared_sound() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let table: std::collections::BTreeMap<String, serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(root.join("sounds.json")).unwrap())
                .unwrap();
        let mut names = Vec::new();
        for entry in std::fs::read_dir(root.join("champions")).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            let config: serde_json::Value = serde_json::from_str(&text).unwrap();
            let Some(kit) = config.get("kit") else {
                continue;
            };
            let kit: Kit = serde_json::from_value(kit.clone()).unwrap();
            for ability in &kit.abilities {
                ability
                    .actions
                    .iter()
                    .for_each(|a| collect_sounds(a, &mut names));
            }
        }
        names.sort_unstable();
        names.dedup();
        assert!(names.len() >= 8, "kits use most action types: {names:?}");
        for name in names {
            assert!(table.contains_key(name), "data/sounds.json lacks {name}");
        }
    }

    #[test]
    fn buffs_sound_like_their_strongest_effect() {
        let buff = |json: &str| -> Action { serde_json::from_str(json).unwrap() };
        let heal = buff(r#"{"type": "buff", "heal": 80, "shield": 50}"#);
        let shield = buff(r#"{"type": "buff", "damage_reduction": 0.3, "move_speed": 0.2}"#);
        let haste = buff(r#"{"type": "buff", "attack_speed": 0.4}"#);
        let plain = buff(r#"{"type": "buff", "rooted": true}"#);
        assert_eq!(heal.sound_name(), Some("kit_heal"));
        assert_eq!(shield.sound_name(), Some("kit_shield"));
        assert_eq!(haste.sound_name(), Some("kit_haste"));
        assert_eq!(plain.sound_name(), Some("kit_buff"));
        let area = buff(r#"{"type": "area", "radius": 200}"#);
        assert_eq!(area.sound_name(), None, "areas thud when they land");
    }

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
