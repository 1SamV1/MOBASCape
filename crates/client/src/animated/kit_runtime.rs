//! Executes data-driven champion kits (`crate::kit`) on the scene.
//!
//! A child of `animated` so it can reach the scene's private helpers. Every
//! champion with a kit casts it, the player and bots alike: each caster has
//! its own [`KitRuntime`] (cooldown charges, dashes, channels, buffs), and
//! the effects in flight (shots, areas, zones) live in the shared
//! [`KitWorld`], tagged with their caster and team. Abilities hit the
//! caster's enemies: the other team's minions, champions and structures,
//! and neutral camps. Structures take damage but ignore crowd control.

use std::collections::HashMap;
use std::sync::Arc;

use glam::{Vec2, Vec3};
use rift_cache::{AnimationClip, Model};

use super::{
    AnimatedScene, Effect, Particle, ShotVisual, UnitAssets, UnitKind, advance_trail, build_unit,
    planar_delta, turn_toward,
};
use crate::champion::{AbilityRank, ChampionAbility};
use crate::fx;
use crate::kit::{self, Action, Anchor, Gfx, Passive, Status, Targeting};
use crate::scene::{HeightGrid, WorldCollision};

/// A cache NPC's merged model and clips, loaded up front for summons and
/// transforms.
pub struct NpcAssets {
    pub model: Arc<Model>,
    pub idle: Arc<AnimationClip>,
    pub walk: Option<Arc<AnimationClip>>,
}

/// Seconds a Scramble's throw takes.
const SCRAMBLE_SECONDS: f32 = 0.2;
/// A roped shot's head reels home at this speed (units/s)...
const REEL_SPEED: f32 = 2_600.0;
/// ...after staying stuck in its catch this long, while the pull drags it.
const REEL_LATCH: f32 = 0.3;
/// The Discord swirl's landing ring, shown when the Chaos passive procs.
const CHAOS_PROC: u32 = 558;

/// How near the cursor a unit-targeted cast looks for its unit.
const UNIT_PICK: f32 = 260.0;
/// A vector shorter than this is a tap, aimed from the hero.
const VECTOR_MIN: f32 = 60.0;
/// A unit-targeted dash stops this far from its target.
const DASH_STOP: f32 = 110.0;
/// Seconds a tossed unit is in the air.
const TOSS_SECONDS: f32 = 0.35;
/// How near a healer must be to mend.
const HEALER_REACH: f32 = 700.0;
/// Width of a Jad attack's fire line, in degrees.
const JAD_FIRE_ARC: f32 = 14.0;

/// What an action needs to know about its cast.
#[derive(Clone)]
struct CastCtx {
    /// The casting unit and its team.
    caster: usize,
    team: u8,
    color: [u8; 3],
    /// Caster position when the action runs.
    origin: Vec3,
    /// Aim point (cursor or landing spot).
    target: Vec3,
    /// Planar aim direction.
    dir: Vec3,
    /// Ability damage multiplier (power, level, passives, rank).
    mult: f32,
    /// Reach multiplier from the ability's rank: shot length, cast range
    /// and dash length.
    reach: f32,
    /// False when the champion file scores this slot itself (`q_*`, ...):
    /// the shared kit action sounds then stay quiet.
    shared_sounds: bool,
    /// The unit a unit- or ally-targeted ability was cast on.
    unit: Option<usize>,
    /// The ability slot, so toggles can find what they started.
    slot: usize,
    /// Damage ignores armour.
    pure: bool,
}

struct KitShot {
    caster: usize,
    team: u8,
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
    pure: bool,
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
    gfx: Gfx,
    /// The `land` graphic has played (at the first strike).
    landed: bool,
}

/// A roped shot after it lands: the head stays in its catch while the pull
/// drags it in, then the rope reels the head back to the caster.
struct KitReel {
    caster: usize,
    pos: Vec3,
    look: usize,
    /// The unit the head is stuck in, for `latch` more seconds.
    catch: Option<usize>,
    latch: f32,
    age: f32,
}

struct PendingArea {
    caster: usize,
    team: u8,
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
    pure: bool,
    /// Thud with the shared `kit_area` sound on landing.
    sound: bool,
    gfx: Gfx,
}

struct Zone {
    caster: usize,
    team: u8,
    pos: Vec3,
    radius: f32,
    grow: f32,
    remaining: f32,
    tick: f32,
    timer: f32,
    damage: f32,
    heal_caster: f32,
    follow: bool,
    self_damage: f32,
    on_tick: Vec<Status>,
    color: [u8; 3],
    gfx: Gfx,
    pure: bool,
    /// The toggle slot that laid it; turning the toggle off removes it.
    slot: Option<usize>,
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
    gfx: Gfx,
    /// Run at this unit and stop beside it.
    chase: Option<usize>,
    /// Hitting a hero below this share of health resets the cooldown.
    reset_below: f32,
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
    heal_pct: f32,
    mana_per_hit: f32,
    max_health: f32,
    on_hit: Vec<Status>,
    gfx: Gfx,
    /// The toggle slot that started it.
    slot: Option<usize>,
}

/// A held vector or charge press, waiting for the key to come up.
#[derive(Debug, Clone, Copy)]
struct PendingInput {
    slot: usize,
    start: Vec3,
    held: f32,
}

/// A hexed unit's own body, back when the hex ends.
pub struct HexState {
    remaining: f32,
    model: Arc<Model>,
    idle: Arc<AnimationClip>,
    walk: Option<Arc<AnimationClip>>,
    scale: f32,
    attack_clip: Option<Arc<AnimationClip>>,
    cast_clip: Option<Arc<AnimationClip>>,
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
    armor: f32,
    bonus_health: f32,
    on_hit: Vec<Status>,
    splash: f32,
    evasion: bool,
}

/// Effects in flight from every caster, and the NPCs kits summon.
#[derive(Default)]
pub struct KitWorld {
    pub npc_assets: HashMap<u32, NpcAssets>,
    shots: Vec<KitShot>,
    reels: Vec<KitReel>,
    areas: Vec<PendingArea>,
    zones: Vec<Zone>,
    /// Hexed units' own bodies, by unit index.
    hexed: HashMap<usize, HexState>,
}

/// One caster's kit and its per-match state.
pub struct KitRuntime {
    pub kit: kit::Kit,
    /// Current rank of each slot (0 = not learned). The player's mirror
    /// the skill points spent; bots learn theirs with level.
    pub ranks: [u8; 4],
    /// The champion file's numbers per rank, Q to R.
    pub rank_table: [Vec<AbilityRank>; 4],
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
    /// Ability hits counted for the Chaos passive.
    chaos_hits: u32,
    /// Bonus damage the Flesh heap passive has banked for the next hit.
    pub(super) flesh_heap: f32,
    /// A recast window per slot, and the unit the first cast held.
    recast: [Option<(f32, Option<usize>)>; 4],
    /// Toggle abilities that are on.
    toggled: [bool; 4],
    pending: Option<PendingInput>,
    /// The next form a cycling transform takes.
    form_index: usize,
    fury_target: Option<usize>,
    fury_stacks: u32,
    fury_timer: f32,
    reflect_stun: f32,
    reflect_stun_timer: f32,
    reflected: Vec<usize>,
    /// Jad attacks: the attack count, and which style comes next.
    jad_count: u32,
    jad_boulder_next: bool,
    /// Inferno Unleashed: every `jad_every`th attack with a shorter wind-up.
    jad_every: u32,
    jad_windup: f32,
    jad_timer: f32,
    slam_stun: f32,
    slam_timer: f32,
    bonus_health: f32,
    bonus_health_timer: f32,
    attack_bonus: f32,
    attack_bonus_timer: f32,
    lifesteal: f32,
    lifesteal_timer: f32,
    /// Tailwind attacks left after a dash.
    tailwind: u32,
}

impl KitRuntime {
    /// A fresh runtime for `kit`, with the champion's rank tables.
    pub fn new(kit: &kit::Kit, abilities: &[ChampionAbility]) -> Self {
        let mut charges = [1u8; 4];
        for (slot, ability) in kit.abilities.iter().enumerate().take(4) {
            charges[slot] = ability.charges.max(1);
        }
        Self {
            kit: kit.clone(),
            ranks: [1; 4],
            rank_table: std::array::from_fn(|slot| {
                abilities
                    .get(slot)
                    .map(|a| a.ranks.clone())
                    .unwrap_or_default()
            }),
            dash: None,
            channel: None,
            empower: None,
            transform: None,
            charges,
            recharge: [0.0; 4],
            attack_count: 0,
            second_form_used: false,
            combo_stacks: 0,
            combo_timer: 0.0,
            combo_last: None,
            damage_reduction: 0.0,
            damage_reduction_timer: 0.0,
            reflect: 0.0,
            reflect_timer: 0.0,
            haste: 0.0,
            haste_timer: 0.0,
            untargetable_timer: 0.0,
            rooted_timer: 0.0,
            chaos_hits: 0,
            flesh_heap: 0.0,
            recast: [None; 4],
            toggled: [false; 4],
            pending: None,
            form_index: 0,
            fury_target: None,
            fury_stacks: 0,
            fury_timer: 0.0,
            reflect_stun: 0.0,
            reflect_stun_timer: 0.0,
            reflected: Vec::new(),
            jad_count: 0,
            jad_boulder_next: false,
            jad_every: 0,
            jad_windup: 0.0,
            jad_timer: 0.0,
            slam_stun: 0.0,
            slam_timer: 0.0,
            bonus_health: 0.0,
            bonus_health_timer: 0.0,
            attack_bonus: 0.0,
            attack_bonus_timer: 0.0,
            lifesteal: 0.0,
            lifesteal_timer: 0.0,
            tailwind: 0,
        }
    }

    /// The slot's last rank (Q/W/E 4, R 3, or the rank table's length).
    pub fn max_rank(&self, slot: usize) -> u8 {
        match self.rank_table.get(slot).map(Vec::len) {
            Some(n) if n > 0 => n as u8,
            _ => {
                if slot == 3 {
                    3
                } else {
                    4
                }
            }
        }
    }

    /// The slot is at its last rank, so its mastery is on.
    pub fn mastered(&self, slot: usize) -> bool {
        self.ranks[slot] >= self.max_rank(slot)
    }

    /// Charges the slot holds at its current rank.
    pub fn max_charges(&self, slot: usize) -> u8 {
        let ability = &self.kit.abilities[slot];
        match ability.mastery_charges {
            Some(n) if self.mastered(slot) => n.max(1),
            _ => ability.charges.max(1),
        }
    }

    /// The actions a cast of `slot` runs now: the mastery's at the last
    /// rank, with durations and summon counts grown by rank.
    pub fn actions_for(&self, slot: usize) -> Vec<Action> {
        let ability = &self.kit.abilities[slot];
        let list = if self.mastered(slot) && !ability.mastery.is_empty() {
            &ability.mastery
        } else {
            &ability.actions
        };
        let steps = self.ranks[slot].saturating_sub(1) as f32;
        let f = 1.0 + ability.duration_growth * steps;
        let extra = (ability.count_growth * steps).round() as u32;
        list.iter().map(|a| a.scaled(f, extra)).collect()
    }

    /// A passive an ability of this kit brings, once that ability is learned.
    fn slot_passive(&self, pick: impl Fn(&Passive) -> bool) -> Option<(usize, &Passive)> {
        self.kit
            .ability_passives
            .iter()
            .find(|sp| self.ranks.get(sp.slot).is_some_and(|&r| r > 0) && pick(&sp.passive))
            .map(|sp| (sp.slot, &sp.passive))
    }

    /// The caster's own movement is taken over by a dash, a channel or a
    /// rooted buff.
    pub fn controls_movement(&self) -> bool {
        self.dash.is_some() || self.channel.is_some() || self.rooted_timer > 0.0
    }

    fn rank_entry(&self, slot: usize, rank: u8) -> Option<AbilityRank> {
        self.rank_table
            .get(slot)?
            .get(rank.saturating_sub(1) as usize)
            .copied()
    }

    fn current_and_first(&self, slot: usize) -> Option<(AbilityRank, AbilityRank)> {
        Some((
            self.rank_entry(slot, self.ranks[slot].max(1))?,
            self.rank_entry(slot, 1)?,
        ))
    }

    /// Damage, heal and shield multiplier from the slot's rank.
    pub fn damage_mult(&self, slot: usize) -> f32 {
        self.current_and_first(slot)
            .map(|(now, first)| super::ranks::ratio(now.damage, first.damage))
            .unwrap_or(1.0)
    }

    /// Reach multiplier (shot length, cast range, dash length).
    pub fn reach_mult(&self, slot: usize) -> f32 {
        self.current_and_first(slot)
            .map(|(now, first)| super::ranks::ratio(Some(now.range), Some(first.range)))
            .unwrap_or(1.0)
    }

    /// The slot's cooldown at its rank, or `fallback` without authored ranks.
    pub fn cooldown(&self, slot: usize, fallback: f32) -> f32 {
        self.rank_entry(slot, self.ranks[slot].max(1))
            .map(|entry| entry.cooldown)
            .filter(|cooldown| *cooldown > 0.0)
            .unwrap_or(fallback)
    }
}

/// Load NPC models and idle/walk clips for summons, transforms and thralls.
/// `who` names the caller in the log.
pub fn load_npcs(
    cache: &rift_cache::Cache,
    assets: &mut UnitAssets<'_>,
    wanted: &[u32],
    who: &str,
) -> HashMap<u32, NpcAssets> {
    let mut out = HashMap::new();
    if wanted.is_empty() {
        return out;
    }
    let Ok(files) = cache.archive_files_with_ids(2, 9) else {
        tracing::warn!("{who}: npc archive unavailable");
        return out;
    };
    for (id, data) in files {
        if !wanted.contains(&id) {
            continue;
        }
        let Ok(def) = rift_cache::NpcDefinition::parse(id, &data) else {
            tracing::warn!("{who}: npc {id} does not decode");
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
            let clips: Vec<Arc<AnimationClip>> = std::iter::once(Arc::clone(&idle))
                .chain(walk.iter().cloned())
                .collect();
            Ok(NpcAssets {
                model: Arc::new(super::bind_parts(&parts, &clips, "summon")),
                idle,
                walk,
            })
        })();
        match loaded {
            Ok(npc) => {
                tracing::info!("{who}: loaded npc {id} \"{}\"", def.name);
                out.insert(id, npc);
            }
            Err(err) => tracing::warn!("{who}: npc {id}: {err:#}"),
        }
    }
    out
}

impl AnimatedScene {
    /// Register `unit` as a kit caster, loading the NPCs its kit summons or
    /// turns into.
    pub(super) fn add_kit_caster(
        &mut self,
        cache: &rift_cache::Cache,
        assets: &mut UnitAssets<'_>,
        unit: usize,
        kit: &kit::Kit,
        abilities: &[ChampionAbility],
    ) {
        let wanted: Vec<u32> = kit
            .npcs()
            .into_iter()
            .filter(|id| !self.kit_world.npc_assets.contains_key(id))
            .collect();
        let loaded = load_npcs(cache, assets, &wanted, "kit");
        self.kit_world.npc_assets.extend(loaded);
        self.casters.insert(unit, KitRuntime::new(kit, abilities));
    }

    /// Kit casters in a fixed order, so a frame resolves the same way every
    /// run.
    fn caster_ids(&self) -> Vec<usize> {
        let mut ids: Vec<usize> = self.casters.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// The kit passive of `caster`, if it has one.
    fn passive_of(&self, caster: usize) -> Option<&Passive> {
        self.casters.get(&caster)?.kit.passive.as_ref()
    }

    /// True when `index` is a live target for kit hits from `team`: the other
    /// team's units and neutral camps, never the delivery props, scenery
    /// creatures or anyone untargetable.
    fn is_kit_enemy(&self, team: u8, index: usize) -> bool {
        let unit = &self.units[index];
        let hostile = if unit.team == 2 {
            unit.kind == UnitKind::Camp
        } else {
            unit.team != team
        };
        unit.health > 0.0
            && unit.spawn_delay <= 0.0
            && hostile
            && !matches!(
                unit.kind,
                UnitKind::DeliveryGuard | UnitKind::StorageChest | UnitKind::Ambient
            )
            && !self.unit_untargetable(index)
    }

    /// True when kit dashes, buffs or Vine Swing make `index` untargetable.
    pub(super) fn unit_untargetable(&self, index: usize) -> bool {
        if index == self.player_index && self.awowogei.untargetable > 0.0 {
            return true;
        }
        self.kit_untargetable(index)
    }

    fn kit_enemies_in(
        &self,
        team: u8,
        center: Vec3,
        radius: f32,
        facing: Vec3,
        arc: f32,
    ) -> Vec<usize> {
        let facing = Vec2::new(facing.x, facing.z);
        (0..self.units.len())
            .filter(|&i| self.is_kit_enemy(team, i))
            .filter(|&i| {
                let unit = &self.units[i];
                let offset = Vec2::new(unit.pos.x - center.x, unit.pos.z - center.z);
                offset.length() <= radius + unit.hit_radius.max(40.0) * 0.5
                    && kit::in_arc(offset, facing, arc)
            })
            .collect()
    }

    /// Damage plus statuses on one enemy, with the action's graphics: its
    /// `hit` on the unit (a splash instead when a shield soaks the spell)
    /// and a held graphic for as long as its crowd control lasts.
    #[allow(clippy::too_many_arguments)]
    fn kit_hit(
        &mut self,
        caster: usize,
        team: u8,
        index: usize,
        damage: f32,
        statuses: &[Status],
        from: Vec3,
        gfx: &Gfx,
        pure_hit: bool,
    ) {
        let splashed = damage > 0.0 && self.units[index].shield > 0.0;
        let (damage, mut pure) = if pure_hit {
            (0.0, damage)
        } else {
            (damage, 0.0)
        };
        for status in statuses {
            if let Status::CurrentHealth { pct } = status {
                pure += self.units[index].health * pct;
            }
        }
        // Fury stacks: hits on the same enemy grow the next ones.
        let fury = self.kit_fury(caster, index);
        let damage = if damage + pure > 0.0 {
            damage + fury
        } else {
            damage
        };
        if damage > 0.0 || pure > 0.0 {
            self.damage_unit_mixed(index, damage, pure, team, Some(caster));
        }
        if self.units[index].health <= 0.0 {
            return;
        }
        self.land_statuses(index, statuses, from, gfx, splashed, caster);
    }

    /// After a hit's damage: its `hit` graphic (a splash instead when a
    /// shield soaked it), its statuses from `caster`, and a held graphic for
    /// as long as its crowd control lasts. Kits and battle spells share it.
    pub(super) fn land_statuses(
        &mut self,
        index: usize,
        statuses: &[Status],
        from: Vec3,
        gfx: &Gfx,
        splashed: bool,
        caster: usize,
    ) {
        if self.units[index].health <= 0.0 {
            return;
        }
        // A "one of" status picks its effect now, so its held graphic is
        // the one that landed.
        let statuses: Vec<Status> = statuses.iter().map(|s| self.resolve_status(s)).collect();
        let statuses = statuses.as_slice();
        let size = gfx.scale * self.gfx_size(index);
        if splashed {
            self.gfx_on(kit::spot::SPLASH, index, size);
        } else if let Some(id) = gfx.hit {
            self.gfx_on(id, index, size);
        }
        for status in statuses {
            self.apply_status_from(index, status, from, caster);
        }
        if self.units[index].kind.is_structure() {
            return;
        }
        let hold = statuses
            .iter()
            .filter_map(Status::hold)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((own, seconds)) = hold
            && let Some(id) = gfx.hold.or(own)
        {
            self.gfx_hold(id, index, seconds, size);
        }
    }

    /// Spell graphics are authored for a player-sized body; bigger units
    /// (camps, bosses) get somewhat bigger ones, short of hiding them.
    pub(super) fn gfx_size(&self, index: usize) -> f32 {
        (self.units[index].body_radius / 65.0)
            .sqrt()
            .clamp(0.8, 1.6)
    }

    /// One of `of`, at random.
    fn pick_status(&mut self, of: &[Status]) -> Option<Status> {
        if of.is_empty() {
            return None;
        }
        let n = ((self.next_random() * of.len() as f32) as usize).min(of.len() - 1);
        Some(of[n].clone())
    }

    /// The concrete status a "one of" lands as (others as they are).
    fn resolve_status(&mut self, status: &Status) -> Status {
        match status {
            Status::OneOf { of } => match self.pick_status(of) {
                Some(pick) => self.resolve_status(&pick),
                None => status.clone(),
            },
            _ => status.clone(),
        }
    }

    /// The Chaos passive: every few ability hits on `index` also land one
    /// of its statuses, with the Discord swirl.
    fn chaos_passive(&mut self, caster: usize, index: usize, from: Vec3) {
        let Some(Passive::Chaos { every, statuses }) = self.passive_of(caster).cloned() else {
            return;
        };
        if self.units[index].health <= 0.0 {
            return;
        }
        let Some(rt) = self.casters.get_mut(&caster) else {
            return;
        };
        rt.chaos_hits += 1;
        if rt.chaos_hits % every.max(1) != 0 {
            return;
        }
        if let Some(pick) = self.pick_status(&statuses) {
            let gfx = Gfx {
                hit: Some(CHAOS_PROC),
                ..Gfx::default()
            };
            self.land_statuses(index, &[pick], from, &gfx, false, caster);
        }
    }

    /// The Flesh heap passive: `index` died; every caster with the passive
    /// near it grows.
    pub(super) fn kit_flesh_heap(&mut self, index: usize) {
        for caster in self.caster_ids() {
            self.kit_flesh_heap_for(caster, index);
        }
    }

    fn kit_flesh_heap_for(&mut self, pi: usize, index: usize) {
        let Some(Passive::FleshHeap {
            radius,
            minion,
            hero,
        }) = self.passive_of(pi).cloned()
        else {
            return;
        };
        if index == pi || self.units[pi].health <= 0.0 {
            return;
        }
        let victim = &self.units[index];
        let enemy = victim.team != self.units[pi].team
            && !victim.kind.is_structure()
            && victim.kind != UnitKind::Ambient;
        if !enemy || victim.pos.distance(self.units[pi].pos) > radius {
            return;
        }
        let gain = if victim.kind.is_champion() {
            hero
        } else {
            minion
        };
        if let Some(rt) = self.casters.get_mut(&pi) {
            rt.flesh_heap += gain;
        }
        let chest = self.unit_chest(pi);
        self.emit_burst(chest, [170, 40, 30], 6, 120.0, 9.0, 0.5);
    }

    /// A status from `caster` (who a taunt makes the target attack).
    pub(super) fn apply_status_from(
        &mut self,
        index: usize,
        status: &Status,
        from: Vec3,
        caster: usize,
    ) {
        match status {
            Status::OneOf { of } => {
                if let Some(pick) = self.pick_status(of) {
                    self.apply_status_from(index, &pick, from, caster);
                }
                return;
            }
            Status::Scramble { distance } if !self.units[index].kind.is_structure() => {
                let angle = self.next_random() * std::f32::consts::TAU;
                let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                let start = self.units[index].pos;
                let size = self.gfx_size(index);
                self.gfx_at(kit::spot::SCRAMBLE, start, size);
                let unit = &mut self.units[index];
                unit.cc.displace = dir * (distance / SCRAMBLE_SECONDS);
                unit.cc.displace_timer = SCRAMBLE_SECONDS;
                unit.attack_target = None;
                self.gfx_on(kit::spot::SCRAMBLE, index, size);
                return;
            }
            Status::Hex { duration } if !self.units[index].kind.is_structure() => {
                let unit = &mut self.units[index];
                unit.cc.hex_timer = unit.cc.hex_timer.max(*duration);
                unit.strike = None;
                unit.attack_target = None;
                self.hex_unit(index, *duration);
                return;
            }
            Status::Duel { duration, reward } if !self.units[index].kind.is_structure() => {
                let unit = &mut self.units[index];
                unit.cc.duel_with = caster;
                unit.cc.duel_timer = *duration;
                unit.cc.duel_reward = 0.0;
                unit.cc.taunt_timer = unit.cc.taunt_timer.max(*duration);
                unit.cc.taunt_by = caster;
                let hero = &mut self.units[caster];
                hero.cc.duel_with = index;
                hero.cc.duel_timer = *duration;
                hero.cc.duel_reward = *reward;
                hero.attack_target = Some(index);
                return;
            }
            _ => {}
        }
        let pi = caster;
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
                    let size = self.gfx_size(index);
                    self.gfx_hold(kit::spot::ICE_BLOCK, index, 1.2, size);
                }
            }
            Status::Root { duration } => unit.cc.root_timer = unit.cc.root_timer.max(duration),
            Status::NoMobility { duration } => {
                unit.cc.no_mobility_timer = unit.cc.no_mobility_timer.max(duration);
            }
            Status::Disarm { duration } => {
                unit.cc.disarm_timer = unit.cc.disarm_timer.max(duration);
                unit.strike = None;
            }
            Status::Scramble { .. }
            | Status::OneOf { .. }
            | Status::Hex { .. }
            | Status::Duel { .. } => {}
            Status::Silence { duration } => {
                unit.cc.silence_timer = unit.cc.silence_timer.max(duration)
            }
            Status::Mark { pct, duration } => {
                unit.cc.mark_pct = unit.cc.mark_pct.max(pct);
                unit.cc.mark_timer = unit.cc.mark_timer.max(duration);
            }
            Status::Freeze { duration } => {
                unit.cc.root_timer = unit.cc.root_timer.max(duration);
                unit.cc.disarm_timer = unit.cc.disarm_timer.max(duration);
                unit.strike = None;
            }
            Status::Stun { duration } => unit.stun_timer = unit.stun_timer.max(duration),
            Status::Knockup { duration } => {
                unit.stun_timer = unit.stun_timer.max(duration);
                if duration > unit.cc.lift_timer {
                    unit.cc.lift_timer = duration;
                    unit.cc.lift_total = duration;
                }
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
            // Rise over a quarter second, hang, and fall over the last half.
            let ground = height.height_at(unit.pos.x, unit.pos.z);
            let up = ((unit.cc.lift_total - unit.cc.lift_timer) * 4.0).min(1.0);
            let down = (unit.cc.lift_timer * 2.0).min(1.0);
            unit.pos.y = ground + up.min(down) * 110.0;
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

    /// Fury stacks: bonus damage `caster` adds to a hit on `index`, then
    /// one more stack (a new target starts over).
    fn kit_fury(&mut self, caster: usize, index: usize) -> f32 {
        let Some(rt) = self.casters.get_mut(&caster) else {
            return 0.0;
        };
        let Some((
            slot,
            &Passive::Fury {
                per_stack,
                max,
                duration,
            },
        )) = rt.slot_passive(|p| matches!(p, Passive::Fury { .. }))
        else {
            return 0.0;
        };
        let mult = rt.damage_mult(slot);
        if rt.fury_target != Some(index) || rt.fury_timer <= 0.0 {
            rt.fury_target = Some(index);
            rt.fury_stacks = 0;
        }
        let bonus = per_stack * mult * rt.fury_stacks as f32;
        rt.fury_stacks = (rt.fury_stacks + 1).min(max);
        // Mastery: the stacks hold while Callisto keeps hitting.
        rt.fury_timer = if rt.mastered(slot) {
            duration * 3.0
        } else {
            duration
        };
        bonus
    }

    /// Turn `index` into a frog for `seconds` (or keep it one longer).
    fn hex_unit(&mut self, index: usize, seconds: f32) {
        if let Some(hex) = self.kit_world.hexed.get_mut(&index) {
            hex.remaining = hex.remaining.max(seconds);
            return;
        }
        let Some(frog) = self.kit_world.npc_assets.get(&kit::HEX_NPC) else {
            return;
        };
        let (model, idle, walk) = (frog.model.clone(), frog.idle.clone(), frog.walk.clone());
        let unit = &mut self.units[index];
        let hex = HexState {
            remaining: seconds,
            model: std::mem::replace(&mut unit.model, model),
            idle: std::mem::replace(&mut unit.idle, idle),
            walk: std::mem::replace(&mut unit.walk, walk),
            scale: std::mem::replace(&mut unit.scale, 1.6),
            attack_clip: unit.attack_clip.take(),
            cast_clip: unit.cast_clip.take(),
        };
        unit.action = None;
        let pos = unit.pos;
        self.emit_burst(pos + Vec3::Y * 40.0, [120, 220, 110], 20, 220.0, 14.0, 0.6);
        self.kit_world.hexed.insert(index, hex);
    }

    /// Give a hexed unit its own body back.
    fn end_hex(&mut self, index: usize) {
        let Some(hex) = self.kit_world.hexed.remove(&index) else {
            return;
        };
        let unit = &mut self.units[index];
        unit.model = hex.model;
        unit.idle = hex.idle;
        unit.walk = hex.walk;
        unit.scale = hex.scale;
        unit.attack_clip = hex.attack_clip;
        unit.cast_clip = hex.cast_clip;
        unit.action = None;
        unit.cc.hex_timer = 0.0;
    }

    /// Count hexes down and end those that ran out (or whose unit died).
    fn update_hexes(&mut self, dt: f32) {
        let mut done = Vec::new();
        for (&index, hex) in self.kit_world.hexed.iter_mut() {
            hex.remaining -= dt;
            if hex.remaining <= 0.0 || self.units[index].health <= 0.0 {
                done.push(index);
            }
        }
        done.sort_unstable();
        for index in done {
            self.end_hex(index);
        }
    }

    /// Damage multiplier for `caster`'s abilities right now.
    fn kit_ability_mult(&self, caster: usize) -> f32 {
        let unit = &self.units[caster];
        let combo = match (self.passive_of(caster), self.casters.get(&caster)) {
            (Some(Passive::SpellCombo { per_stack, .. }), Some(rt)) => {
                1.0 + per_stack * rt.combo_stacks as f32
            }
            _ => 1.0,
        };
        kit::scaled_damage(1.0, unit.ability_power, unit.level) * combo
    }

    /// A bot's turn to use its kit on `target`, an enemy it is fighting.
    /// Tries R down to Q and casts the first ability that fits the moment:
    /// heals and shields when hurt, gap closers on heroes out of reach,
    /// damage when the target is inside the ability's reach. Ultimates are
    /// kept for heroes and boss camps, and basic abilities spend mana on
    /// minions only when there is plenty. True when something was cast.
    pub(super) fn bot_try_kit_cast(&mut self, unit: usize, target: usize) -> bool {
        let Some(rt) = self.casters.get(&unit) else {
            return false;
        };
        let (pos, mana, max_mana, hp_frac, range) = {
            let u = &self.units[unit];
            (
                u.pos,
                u.mana,
                u.max_mana.max(1.0),
                u.health / u.max_health.max(1.0),
                u.attack_range,
            )
        };
        let foe = &self.units[target];
        let hero = foe.kind.is_champion();
        let big = hero || (foe.kind == UnitKind::Camp && foe.camp_boss);
        let distance = (planar_delta(pos, foe.pos).length() - foe.hit_radius * 0.5).max(0.0);
        let aim = foe.pos;
        let mut choice = None;
        for slot in (0..4).rev() {
            let Some(ability) = rt.kit.abilities.get(slot) else {
                continue;
            };
            let charged = ability.charges <= 1 || rt.charges[slot] > 0;
            let ready = rt.ranks[slot] > 0
                && mana >= ability.cost
                && (self.units[unit].cooldowns[slot] <= 0.0 || (ability.charges > 1 && charged));
            if !ready {
                continue;
            }
            let worth = hero || (slot < 3 && (big || mana > max_mana * 0.6));
            let reach = rt.reach_mult(slot);
            let wanted = match bot_plan(&ability.actions, reach) {
                BotPlan::Care => hp_frac < 0.6 && distance < 900.0,
                BotPlan::Power => big && distance <= range + 250.0,
                BotPlan::Gap(far) => hero && distance > range + 150.0 && distance <= far,
                BotPlan::Hit(reach) => worth && distance <= reach,
            };
            if wanted {
                choice = Some(slot);
                break;
            }
        }
        match choice {
            Some(slot) => self.cast_kit_for(unit, slot, aim),
            None => false,
        }
    }

    /// Cast slot 0..4 from the player's kit. False when it cannot be cast.
    pub(super) fn cast_kit(&mut self, slot: usize, target: Vec3) -> bool {
        self.kit_press(slot, target)
    }

    /// The player let go of `slot`'s key: a held vector or charge fires.
    pub fn release_slot(&mut self, slot: usize, target: Vec3) -> bool {
        let ok = self.kit_release(slot, target);
        if ok {
            self.after_cast(slot, true);
        }
        ok
    }

    /// Cast slot 0..4 of `caster`'s kit at `target`, a tap: full charge,
    /// and a vector aimed from the hero through `target`. False when it
    /// cannot be cast (dead, stunned, silenced, no mana, on cooldown,
    /// unlearned, busy, or no unit to target).
    pub(super) fn cast_kit_for(&mut self, pi: usize, slot: usize, target: Vec3) -> bool {
        self.cast_kit_with(pi, slot, target, None, 1.0)
    }

    /// The player pressed a slot's key. Vector and charge abilities wait
    /// for the release (`kit_release`); everything else casts now.
    pub(super) fn kit_press(&mut self, slot: usize, target: Vec3) -> bool {
        let pi = self.player_index;
        let held = self.casters.get(&pi).is_some_and(|rt| {
            let ability = &rt.kit.abilities[slot];
            (ability.vector || ability.charge > 0.0)
                && rt.recast[slot].is_none()
                && rt.ranks[slot] > 0
                && self.units[pi].mana >= ability.cost
                && (self.units[pi].cooldowns[slot] <= 0.0 || rt.charges[slot] > 0)
        });
        if !held {
            return self.cast_kit_for(pi, slot, target);
        }
        if let Some(rt) = self.casters.get_mut(&pi) {
            rt.pending = Some(PendingInput {
                slot,
                start: target,
                held: 0.0,
            });
        }
        false
    }

    /// The player let go of a slot's key: fire a held vector or charge.
    pub fn kit_release(&mut self, slot: usize, target: Vec3) -> bool {
        let pi = self.player_index;
        let Some(rt) = self.casters.get_mut(&pi) else {
            return false;
        };
        let Some(pending) = rt.pending.filter(|p| p.slot == slot) else {
            return false;
        };
        rt.pending = None;
        let ability = &rt.kit.abilities[slot];
        let charge = if ability.charge > 0.0 {
            (pending.held / ability.charge).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let (aim, end) = if ability.vector {
            (pending.start, Some(target))
        } else {
            (target, None)
        };
        self.cast_kit_with(pi, slot, aim, end, charge)
    }

    /// Seconds the player has held a charge ability, and its full charge.
    #[allow(dead_code)] // read by the charge-bar HUD, next step
    pub fn kit_charging(&self) -> Option<(usize, f32, f32)> {
        let rt = self.casters.get(&self.player_index)?;
        let pending = rt.pending?;
        let full = rt.kit.abilities[pending.slot].charge;
        (full > 0.0).then_some((pending.slot, pending.held, full))
    }

    /// The enemy nearest `point` that `pi` can target within `range`.
    fn kit_pick_enemy(&self, pi: usize, point: Vec3, range: f32) -> Option<usize> {
        let team = self.units[pi].team;
        let origin = self.units[pi].pos;
        (0..self.units.len())
            .filter(|&i| self.is_kit_enemy(team, i) && !self.units[i].kind.is_structure())
            .filter(|&i| {
                let unit = &self.units[i];
                planar_delta(point, unit.pos).length() <= UNIT_PICK + unit.hit_radius * 0.5
                    && planar_delta(origin, unit.pos).length()
                        <= range + unit.hit_radius * 0.5 + 60.0
            })
            .min_by(|&a, &b| {
                let da = planar_delta(point, self.units[a].pos).length();
                let db = planar_delta(point, self.units[b].pos).length();
                da.total_cmp(&db)
            })
    }

    /// The allied hero or unit nearest `point` within `range`, else `pi`.
    fn kit_pick_ally(&self, pi: usize, point: Vec3, range: f32) -> usize {
        let team = self.units[pi].team;
        let origin = self.units[pi].pos;
        (0..self.units.len())
            .filter(|&i| {
                let unit = &self.units[i];
                i != pi
                    && unit.team == team
                    && unit.health > 0.0
                    && unit.kind.is_champion()
                    && planar_delta(point, unit.pos).length() <= UNIT_PICK
                    && planar_delta(origin, unit.pos).length() <= range + 60.0
            })
            .min_by(|&a, &b| {
                let da = planar_delta(point, self.units[a].pos).length();
                let db = planar_delta(point, self.units[b].pos).length();
                da.total_cmp(&db)
            })
            .unwrap_or(pi)
    }

    /// The context a cast of `slot` by `pi` runs its actions with.
    fn kit_ctx(&self, pi: usize, slot: usize, target: Vec3, unit: Option<usize>) -> CastCtx {
        let rt = &self.casters[&pi];
        let ability = &rt.kit.abilities[slot];
        let talent = self.units[pi].talent;
        let origin = self.units[pi].pos;
        let flat = planar_delta(origin, target);
        let dir = if flat.length_squared() > 1.0 {
            flat.normalize()
        } else {
            let yaw = self.units[pi].yaw;
            Vec3::new(yaw.sin(), 0.0, yaw.cos())
        };
        CastCtx {
            caster: pi,
            team: self.units[pi].team,
            shared_sounds: pi != self.player_index || !self.slot_has_own_sounds(slot),
            color: ability.color,
            origin,
            target,
            dir,
            mult: self.kit_ability_mult(pi) * rt.damage_mult(slot) * talent.damage_mult(slot),
            reach: rt.reach_mult(slot) * talent.range_mult(slot),
            unit,
            slot,
            pure: ability.pure,
        }
    }

    /// The second press of a recast ability: its follow-up, free. Waking
    /// from a rooted buff (Hibernate) ends the buff.
    fn kit_recast(&mut self, pi: usize, slot: usize, target: Vec3, held: Option<usize>) -> bool {
        let Some(rt) = self.casters.get_mut(&pi) else {
            return false;
        };
        rt.recast[slot] = None;
        rt.rooted_timer = 0.0;
        let steps = rt.ranks[slot].saturating_sub(1) as f32;
        let ability = &rt.kit.abilities[slot];
        let f = 1.0 + ability.duration_growth * steps;
        let actions: Vec<Action> = ability.recast.iter().map(|a| a.scaled(f, 0)).collect();
        let held = held.filter(|&u| self.units.get(u).is_some_and(|unit| unit.health > 0.0));
        let ctx = self.kit_ctx(pi, slot, target, held);
        for action in &actions {
            self.run_action(action, &ctx);
        }
        tracing::debug!("kit: unit {pi} recast slot {slot}");
        true
    }

    /// Turn a toggle off: its zones and empower end and its cooldown starts.
    fn kit_toggle_off(&mut self, pi: usize, slot: usize) {
        let Some(rt) = self.casters.get_mut(&pi) else {
            return;
        };
        rt.toggled[slot] = false;
        if rt.empower.as_ref().is_some_and(|e| e.slot == Some(slot)) {
            rt.empower = None;
        }
        let ability = &rt.kit.abilities[slot];
        let cooldown =
            rt.cooldown(slot, ability.cooldown) * self.units[pi].talent.cooldown_mult(slot);
        self.units[pi].cooldowns[slot] = cooldown;
        self.kit_world
            .zones
            .retain(|zone| !(zone.caster == pi && zone.slot == Some(slot)));
        tracing::debug!("kit: unit {pi} toggled slot {slot} off");
    }

    /// Cast with a vector's end point (the release) and a charge's share.
    fn cast_kit_with(
        &mut self,
        pi: usize,
        slot: usize,
        target: Vec3,
        vector_end: Option<Vec3>,
        charge: f32,
    ) -> bool {
        let Some(rt) = self.casters.get(&pi) else {
            return false;
        };
        let Some(ability) = rt.kit.abilities.get(slot).cloned() else {
            return false;
        };
        let passive = rt.kit.passive.clone();
        if rt.ranks[slot] == 0 || rt.dash.is_some() || rt.channel.is_some() {
            return false;
        }
        {
            let unit = &self.units[pi];
            if unit.health <= 0.0 || unit.stun_timer > 0.0 || unit.cc.silenced() {
                return false;
            }
        }
        // A second press inside the recast window runs the follow-up.
        if let Some((_, held)) = rt.recast[slot] {
            return self.kit_recast(pi, slot, target, held);
        }
        // A second press turns a toggle off.
        if ability.toggle && rt.toggled[slot] {
            self.kit_toggle_off(pi, slot);
            return true;
        }
        let talent = self.units[pi].talent;
        let cooldown = rt.cooldown(slot, ability.cooldown) * talent.cooldown_mult(slot);
        let actions = rt.actions_for(slot);
        let max_charges = rt.max_charges(slot);
        let range = if ability.cast_range > 0.0 {
            ability.cast_range * rt.reach_mult(slot)
        } else {
            rt.rank_entry(slot, rt.ranks[slot])
                .map_or(700.0, |entry| entry.range)
        } * talent.range_mult(slot);
        let unit = match ability.target {
            Targeting::Point => None,
            Targeting::Unit => match self.kit_pick_enemy(pi, target, range) {
                Some(unit) => Some(unit),
                None => return false,
            },
            Targeting::Ally => Some(self.kit_pick_ally(pi, target, range)),
        };
        {
            let player = &self.units[pi];
            // Tele Block grounds abilities that dash.
            let dashes = actions
                .iter()
                .any(|action| matches!(action, Action::Dash { .. }));
            if player.mana < ability.cost || (dashes && player.cc.no_mobility_timer > 0.0) {
                return false;
            }
        }
        let rt = self.casters.get_mut(&pi).expect("checked above");
        if max_charges > 1 {
            if rt.charges[slot] == 0 {
                return false;
            }
            if rt.charges[slot] >= max_charges {
                rt.recharge[slot] = cooldown;
            }
            rt.charges[slot] -= 1;
        } else if self.units[pi].cooldowns[slot] > 0.0 {
            return false;
        } else if ability.toggle {
            // A toggle's cooldown starts when it is turned off.
            self.units[pi].cooldowns[slot] = 0.4;
        } else {
            self.units[pi].cooldowns[slot] = cooldown;
        }
        self.units[pi].mana -= ability.cost;
        if !ability.recast.is_empty() {
            rt.recast[slot] = Some((ability.recast_window.max(0.5), unit));
        }
        if ability.toggle {
            rt.toggled[slot] = true;
        }

        if let Some(Passive::SpellCombo { max, window, .. }) = passive.as_ref() {
            if rt.combo_last != Some(slot) && rt.combo_timer > 0.0 {
                rt.combo_stacks = (rt.combo_stacks + 1).min(*max);
            } else if rt.combo_timer <= 0.0 {
                rt.combo_stacks = 0;
            }
            rt.combo_last = Some(slot);
            rt.combo_timer = *window;
        }

        let aim = unit.map_or(target, |u| self.units[u].pos);
        let mut ctx = self.kit_ctx(pi, slot, aim, unit);
        // A held charge: a short draw keeps part of the range and damage.
        if ability.charge > 0.0 {
            let lerp = |min: f32| min + (1.0 - min) * charge;
            ctx.mult *= lerp(ability.charge_damage_min);
            ctx.reach *= lerp(ability.charge_range_min);
        }
        // A drawn vector: laid from its start point toward the release.
        if let Some(end) = vector_end {
            let drawn = planar_delta(target, end);
            if drawn.length() > VECTOR_MIN {
                ctx.dir = drawn.normalize();
            }
        }
        let origin = ctx.origin;
        let dir = planar_delta(origin, aim);
        if dir.length_squared() > 1.0 {
            self.units[pi].yaw = dir.x.atan2(dir.z);
        }
        if !ctx.shared_sounds {
            let name = ["q_cast", "w_cast", "e_cast", "r_cast"][slot.min(3)];
            self.play_sound(name, origin);
        }
        if pi != self.player_index
            && let Some(clip) = self.units[pi]
                .cast_clip
                .clone()
                .or_else(|| self.units[pi].attack_clip.clone())
        {
            let hit = self.units[pi].attack_hit;
            self.units[pi].action = Some(super::ActionAnim::new(
                clip.clone(),
                super::action_rate(&clip, hit, 0.3),
            ));
        }
        self.effects.push(Effect {
            pos: origin,
            radius: 120.0,
            remaining: 0.3,
            duration: 0.3,
            color: ability.color,
        });
        for action in &actions {
            self.run_action(action, &ctx);
        }
        tracing::debug!("kit: unit {pi} cast slot {slot}");
        true
    }

    fn anchor_point(&self, at: Anchor, range: f32, ctx: &CastCtx) -> Vec3 {
        match at {
            Anchor::Caster => ctx.origin,
            Anchor::Front => ctx.origin + ctx.dir * 160.0,
            Anchor::Target => ctx
                .unit
                .and_then(|u| self.units.get(u))
                .map_or(ctx.target, |unit| unit.pos),
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
        let pi = ctx.caster;
        if ctx.shared_sounds
            && let Some(name) = action.sound_name()
        {
            let pos = self.units[pi].pos;
            self.play_sound(name, pos);
        }
        // A channel's repeated action plays its own cast each time.
        if !matches!(action, Action::Channel { .. })
            && let Some(id) = action.gfx().cast
        {
            let size = action.gfx().scale * self.gfx_size(pi);
            self.gfx_on(id, pi, size);
        }
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
                homing,
                visual,
                on_hit,
                gfx,
            } => {
                let count = (*count).max(1);
                let chase = ctx.unit.filter(|_| *homing);
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
                    self.kit_world.shots.push(KitShot {
                        caster: pi,
                        team: ctx.team,
                        pos: ctx.origin + Vec3::Y * height + dir * 40.0,
                        dir,
                        speed: *speed,
                        remaining: *range * ctx.reach,
                        width: *width,
                        damage: damage * ctx.mult,
                        pierce: *pierce,
                        explode: *explode,
                        on_hit: on_hit.clone(),
                        hit: Vec::new(),
                        color: ctx.color,
                        from: ctx.origin,
                        pure: ctx.pure,
                        basic: false,
                        look,
                        tint,
                        trail: Vec::new(),
                        age: 0.0,
                        ember_timer: 0.0,
                        target: chase,
                        gfx: gfx.clone(),
                        landed: false,
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
                gfx,
            } => {
                let center = self.anchor_point(*at, *range * ctx.reach, ctx);
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
                    self.kit_world.areas.push(PendingArea {
                        caster: pi,
                        team: ctx.team,
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
                        pure: ctx.pure,
                        sound: ctx.shared_sounds,
                        gfx: gfx.clone(),
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
                follow,
                self_damage,
                on_tick,
                gfx,
            } => {
                let center = self.anchor_point(*at, *range * ctx.reach, ctx);
                for i in 0..(*count).max(1) {
                    let pos = center + ctx.dir * (spacing * i as f32);
                    if *follow {
                        // An aura: no ground stain, the graphic rides along.
                        if let Some(id) = gfx.land {
                            let size = gfx.scale * self.gfx_size(pi);
                            self.gfx_hold(id, pi, *duration, size);
                        }
                    } else {
                        self.add_decal(pos, *radius, dim(ctx.color), *duration);
                        if let Some(id) = gfx.land {
                            self.gfx_at(id, pos, gfx.scale);
                        }
                    }
                    self.kit_world.zones.push(Zone {
                        caster: pi,
                        team: ctx.team,
                        pos,
                        radius: *radius,
                        grow: *grow,
                        remaining: *duration,
                        tick: tick.max(0.1),
                        timer: 0.0,
                        damage: damage * ctx.mult,
                        heal_caster: *heal_caster,
                        follow: *follow,
                        self_damage: *self_damage,
                        on_tick: on_tick.clone(),
                        color: ctx.color,
                        gfx: gfx.clone(),
                        pure: ctx.pure,
                        slot: Some(ctx.slot),
                    });
                }
            }
            Action::Dash {
                distance,
                speed,
                to_cursor,
                to_target,
                reset_below,
                backwards,
                damage,
                untargetable,
                on_hit,
                land,
                gfx,
            } => {
                let dir = if *backwards { -ctx.dir } else { ctx.dir };
                let mut distance = *distance * ctx.reach;
                if *to_cursor {
                    distance = distance.min(planar_delta(ctx.origin, ctx.target).length());
                }
                let chase = ctx.unit.filter(|_| *to_target);
                if let Some(unit) = chase {
                    let gap = planar_delta(ctx.origin, self.units[unit].pos).length();
                    distance = (gap - DASH_STOP).max(0.0);
                }
                let tailwind = match self.passive_of(pi) {
                    Some(Passive::Tailwind { attacks, .. }) => Some(*attacks),
                    _ => None,
                };
                let swift = match self.passive_of(pi) {
                    Some(Passive::Swift { dash_bonus, .. }) => Some(*dash_bonus),
                    _ => None,
                };
                let player = &mut self.units[pi];
                player.path.clear();
                player.path_index = 0;
                player.attack_target = None;
                player.attack_pending = false;
                let Some(rt) = self.casters.get_mut(&pi) else {
                    return;
                };
                if *untargetable {
                    rt.untargetable_timer = distance / speed.max(1.0) + 0.15;
                }
                rt.dash = Some(DashState {
                    dir,
                    remaining: distance,
                    speed: *speed,
                    damage: damage * ctx.mult,
                    on_hit: on_hit.clone(),
                    hit: Vec::new(),
                    land: land.clone(),
                    ctx: ctx.clone(),
                    gfx: gfx.clone(),
                    chase,
                    reset_below: *reset_below,
                });
                if let Some(attacks) = tailwind {
                    rt.tailwind = attacks;
                }
                if let Some(bonus) = swift {
                    rt.empower = Some(EmpowerState {
                        remaining: 4.0,
                        attacks: 1,
                        bonus,
                        missing_health: 0.0,
                        reset_on_kill: None,
                        heal_pct: 0.0,
                        mana_per_hit: 0.0,
                        max_health: 0.0,
                        on_hit: Vec::new(),
                        gfx: Gfx::default(),
                        slot: None,
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
                on_target,
                evasion,
                cleanse,
                allies,
                attack_damage,
                lifesteal,
                reflect_stun,
                jad_every,
                jad_windup,
                slam_stun,
                bonus_health,
                gfx: _,
            } => {
                let mult = ctx.mult;
                let who = if *on_target {
                    ctx.unit.unwrap_or(pi)
                } else {
                    pi
                };
                // Heals and shields reach allied heroes nearby too.
                let mut mended = vec![who];
                if *allies > 0.0 {
                    let center = self.units[who].pos;
                    let team = self.units[who].team;
                    mended.extend((0..self.units.len()).filter(|&i| {
                        let unit = &self.units[i];
                        i != who
                            && unit.team == team
                            && unit.health > 0.0
                            && unit.kind.is_champion()
                            && planar_delta(center, unit.pos).length() <= *allies
                    }));
                }
                for &i in &mended {
                    let unit = &mut self.units[i];
                    let missing = unit.max_health - unit.health;
                    unit.health =
                        (unit.health + heal * mult + missing * heal_missing).min(unit.max_health);
                    if *shield > 0.0 {
                        unit.shield = unit.shield.max(shield * mult);
                        unit.shield_timer = duration.max(3.0);
                    }
                    if *cleanse {
                        unit.cc.slow_pct = 0.0;
                        unit.cc.slow_timer = 0.0;
                        unit.cc.root_timer = 0.0;
                        unit.cc.chill_stacks = 0;
                    }
                    if heal + heal_missing > 0.0 {
                        let pos = unit.pos;
                        self.emit_burst(pos, [120, 255, 160], 18, 160.0, 14.0, 0.8);
                    }
                }
                {
                    let unit = &mut self.units[who];
                    if *evasion {
                        unit.cc.evasion_timer = unit.cc.evasion_timer.max(*duration);
                    }
                    if who != pi {
                        // An ally's tomb: it cannot act or be harmed.
                        if *untargetable > 0.0 {
                            unit.cc.untargetable_timer =
                                unit.cc.untargetable_timer.max(*untargetable);
                        }
                        if *rooted {
                            unit.stun_timer = unit.stun_timer.max(*duration);
                        }
                    }
                }
                let player = &mut self.units[pi];
                if *attack_speed > 0.0 && who == pi {
                    player.attack_speed_mult = 1.0 + attack_speed;
                    player.attack_speed_timer = *duration;
                }
                if *bonus_health > 0.0 {
                    let gain = bonus_health * mult;
                    player.max_health += gain;
                    player.health += gain;
                }
                let Some(rt) = self.casters.get_mut(&pi) else {
                    return;
                };
                if *bonus_health > 0.0 {
                    rt.bonus_health += bonus_health * mult;
                    rt.bonus_health_timer = *duration;
                }
                if who != pi {
                    return;
                }
                if *damage_reduction > 0.0 {
                    rt.damage_reduction = *damage_reduction;
                    rt.damage_reduction_timer = *duration;
                }
                if *reflect > 0.0 {
                    rt.reflect = *reflect;
                    rt.reflect_timer = *duration;
                }
                if *reflect_stun > 0.0 {
                    rt.reflect_stun = *reflect_stun;
                    rt.reflect_stun_timer = *duration;
                    rt.reflected.clear();
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
                if *attack_damage > 0.0 {
                    rt.attack_bonus = attack_damage * mult;
                    rt.attack_bonus_timer = *duration;
                }
                if *lifesteal > 0.0 {
                    rt.lifesteal = *lifesteal;
                    rt.lifesteal_timer = *duration;
                }
                if *jad_every > 0 {
                    rt.jad_every = *jad_every;
                    rt.jad_windup = *jad_windup;
                    rt.jad_timer = *duration;
                }
                if *slam_stun > 0.0 {
                    rt.slam_stun = *slam_stun;
                    rt.slam_timer = *duration;
                }
                if *evasion && let Some(Passive::Tailwind { attacks, .. }) = rt.kit.passive.as_ref()
                {
                    rt.tailwind = *attacks;
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
                armor,
                focus,
                guard,
                heal_owner,
                heal_allies,
                fragile,
                gfx,
            } => {
                let center = self.anchor_point(*at, 700.0, ctx);
                if let Some(id) = gfx.land {
                    self.gfx_at(id, center, gfx.scale);
                }
                for i in 0..(*count).max(1) {
                    let id = npcs.get(i as usize).copied().unwrap_or(*npc);
                    let Some(asset) = self.kit_world.npc_assets.get(&id) else {
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
                        ctx.team,
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
                    unit.cc.owner = Some(pi);
                    unit.cc.focus = ctx.unit.filter(|_| *focus);
                    unit.cc.guard = *guard;
                    unit.cc.heal_owner = *heal_owner;
                    unit.cc.heal_allies = *heal_allies;
                    unit.cc.fragile = *fragile;
                    unit.armor = *armor;
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
                let Some(rt) = self.casters.get_mut(&pi) else {
                    return;
                };
                rt.channel = Some(ChannelState {
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
                heal_pct,
                mana_per_hit,
                max_health,
                on_hit,
                gfx,
            } => {
                let Some(rt) = self.casters.get_mut(&pi) else {
                    return;
                };
                let toggle = rt.kit.abilities[ctx.slot].toggle;
                rt.empower = Some(EmpowerState {
                    remaining: *duration,
                    attacks: (*attacks).max(1),
                    bonus: bonus * ctx.mult,
                    missing_health: *missing_health,
                    reset_on_kill: *reset_on_kill,
                    heal_pct: *heal_pct,
                    mana_per_hit: *mana_per_hit,
                    max_health: *max_health,
                    on_hit: on_hit.clone(),
                    gfx: gfx.clone(),
                    slot: toggle.then_some(ctx.slot),
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
                armor,
                bonus_health,
                cycle,
                gfx: _,
            } => {
                let form = if cycle.is_empty() {
                    kit::FormStep {
                        npc: *npc,
                        scale: *scale,
                        damage: *damage,
                        range: *range,
                        move_speed: *move_speed,
                        armor: *armor,
                        bonus_health: *bonus_health,
                        on_hit: Vec::new(),
                        splash: 0.0,
                        evasion: false,
                    }
                } else {
                    let Some(rt) = self.casters.get_mut(&pi) else {
                        return;
                    };
                    let form = cycle[rt.form_index % cycle.len()].clone();
                    rt.form_index += 1;
                    form
                };
                self.kit_transform(pi, &form, *duration);
                let player = &mut self.units[pi];
                player.health = (player.health + heal * ctx.mult).min(player.max_health);
                self.emit_burst(ctx.origin, ctx.color, 30, 320.0, 18.0, 0.9);
            }
            Action::Strike {
                damage,
                heal_pct,
                caster_health,
                on_hit,
                gfx,
            } => {
                let Some(unit) = ctx.unit.filter(|&u| self.units[u].health > 0.0) else {
                    return;
                };
                let amount = (damage + caster_health * self.units[pi].max_health) * ctx.mult;
                let before = self.units[unit].health;
                self.kit_hit(
                    pi, ctx.team, unit, amount, on_hit, ctx.origin, gfx, ctx.pure,
                );
                self.chaos_passive(pi, unit, ctx.origin);
                let dealt = (before - self.units[unit].health).max(0.0);
                if *heal_pct > 0.0 && dealt > 0.0 {
                    let caster = &mut self.units[pi];
                    caster.health = (caster.health + dealt * heal_pct).min(caster.max_health);
                }
            }
            Action::Toss { distance, land } => {
                let Some(unit) = ctx.unit.filter(|&u| self.units[u].health > 0.0) else {
                    return;
                };
                let from = self.units[unit].pos;
                let throw = planar_delta(from, ctx.target);
                let reach = throw.length().min(*distance * ctx.reach);
                let landing = from + throw.normalize_or_zero() * reach;
                let moved = &mut self.units[unit];
                moved.cc.displace = (landing - from) / TOSS_SECONDS;
                moved.cc.displace.y = 0.0;
                moved.cc.displace_timer = TOSS_SECONDS;
                moved.cc.lift_timer = moved.cc.lift_timer.max(TOSS_SECONDS);
                moved.cc.lift_total = moved.cc.lift_timer;
                moved.attack_target = None;
                let mut at = ctx.clone();
                at.origin = landing;
                at.target = landing;
                for action in land {
                    self.run_action(action, &at);
                }
            }
        }
    }

    fn kit_transform(&mut self, pi: usize, form: &kit::FormStep, duration: f32) {
        let Some(asset) = self.kit_world.npc_assets.get(&form.npc) else {
            tracing::warn!("kit: transform npc {} not loaded", form.npc);
            return;
        };
        let (model, idle, walk) = (asset.model.clone(), asset.idle.clone(), asset.walk.clone());
        self.kit_revert_transform(pi);
        self.end_hex(pi);
        let player = &mut self.units[pi];
        let old_model = std::mem::replace(&mut player.model, model);
        let old_idle = std::mem::replace(&mut player.idle, idle);
        let old_walk = std::mem::replace(&mut player.walk, walk);
        let old_scale = std::mem::replace(&mut player.scale, form.scale);
        let attack_clip = player.attack_clip.take();
        let cast_clip = player.cast_clip.take();
        player.action = None;
        player.attack_range += form.range;
        player.attack_damage *= 1.0 + form.damage;
        player.armor += form.armor;
        player.max_health += form.bonus_health;
        player.health += form.bonus_health;
        let Some(rt) = self.casters.get_mut(&pi) else {
            return;
        };
        rt.transform = Some(TransformState {
            remaining: duration,
            model: old_model,
            idle: old_idle,
            walk: old_walk,
            scale: old_scale,
            range: form.range,
            damage: form.damage,
            move_speed: form.move_speed,
            attack_clip,
            cast_clip,
            armor: form.armor,
            bonus_health: form.bonus_health,
            on_hit: form.on_hit.clone(),
            splash: form.splash,
            evasion: form.evasion,
        });
    }

    /// Put `obj` (with its wield parts) in the player's hand, or empty it
    /// with `None`. A hero that cannot wield ignores this. During a
    /// transform the armed body waits in the saved model, so reverting
    /// shows the new weapon.
    pub fn equip_weapon(&mut self, obj: Option<u32>, parts: &[rift_cache::Model]) {
        if !self.player_can_wield {
            return;
        }
        self.player_weapon = obj;
        let clips = super::unit_clips(&self.units[self.player_index]);
        let model = Arc::new(crate::wield::armed_body(&self.player_body, parts, &clips));
        let pi = self.player_index;
        match self
            .casters
            .get_mut(&pi)
            .and_then(|rt| rt.transform.as_mut())
        {
            Some(transform) => transform.model = model,
            None => self.units[pi].model = model,
        }
        tracing::info!("wield: player holds {obj:?}");
    }

    /// Cache item id of the weapon in the player's hand.
    pub fn player_weapon(&self) -> Option<u32> {
        self.player_weapon
    }

    fn kit_revert_transform(&mut self, pi: usize) {
        let Some(state) = self.casters.get_mut(&pi).and_then(|rt| rt.transform.take()) else {
            return;
        };
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
        player.armor -= state.armor;
        player.max_health -= state.bonus_health;
        player.health = player.health.min(player.max_health);
    }

    /// Movement multiplier from `unit`'s kit buffs and passives.
    pub(super) fn kit_move_mult(&self, unit: usize) -> f32 {
        let Some(rt) = self.casters.get(&unit) else {
            return 1.0;
        };
        let swift = match rt.kit.passive.as_ref() {
            Some(Passive::Swift { move_speed, .. }) => *move_speed,
            _ => 0.0,
        };
        let haste = if rt.haste_timer > 0.0 { rt.haste } else { 0.0 };
        let transform = rt.transform.as_ref().map_or(0.0, |t| t.move_speed);
        1.0 + swift + haste + transform
    }

    /// A kit dash or buff makes `unit` untargetable right now.
    pub(super) fn kit_untargetable(&self, unit: usize) -> bool {
        self.units[unit].cc.untargetable_timer > 0.0
            || self
                .casters
                .get(&unit)
                .is_some_and(|rt| rt.untargetable_timer > 0.0)
    }

    /// True while a dash, channel or rooting buff moves `unit` for it.
    pub(super) fn kit_controls(&self, unit: usize) -> bool {
        self.casters
            .get(&unit)
            .is_some_and(KitRuntime::controls_movement)
    }

    /// Fraction of incoming damage `unit` takes. Zero while untargetable.
    pub(super) fn kit_incoming_mult(&self, unit: usize) -> f32 {
        let cc = &self.units[unit].cc;
        if cc.untargetable_timer > 0.0 {
            return 0.0;
        }
        let mark = cc.damage_taken_mult();
        let Some(rt) = self.casters.get(&unit) else {
            return mark;
        };
        if rt.untargetable_timer > 0.0 {
            return 0.0;
        }
        let reduce = if rt.damage_reduction_timer > 0.0 {
            1.0 - rt.damage_reduction.clamp(0.0, 0.9)
        } else {
            1.0
        };
        reduce * mark
    }

    /// True when a basic attack on `target` misses: evasion, or a ghost
    /// form attacks pass through.
    pub(super) fn attack_misses(&self, target: usize) -> bool {
        self.units[target].cc.evasion_timer > 0.0
            || self
                .casters
                .get(&target)
                .and_then(|rt| rt.transform.as_ref())
                .is_some_and(|form| form.evasion)
    }

    /// A duel keeps `attacker`'s damage off everyone but its opponent.
    pub(super) fn duel_blocks(&self, attacker: Option<usize>, target: usize) -> bool {
        attacker.is_some_and(|a| {
            let cc = &self.units[a].cc;
            cc.duel_timer > 0.0 && cc.duel_with != target && a != target
        })
    }

    /// Kill hooks: a duel won keeps its reward.
    pub(super) fn kit_on_kill(&mut self, victim: usize) {
        let cc = &self.units[victim].cc;
        if cc.duel_timer > 0.0 {
            let winner = cc.duel_with;
            self.units[victim].cc.duel_timer = 0.0;
            if self.units.get(winner).is_some_and(|u| u.health > 0.0) {
                let hero = &mut self.units[winner];
                hero.cc.duel_timer = 0.0;
                let reward = hero.cc.duel_reward;
                if reward > 0.0 {
                    hero.attack_damage += reward;
                    let pos = hero.pos;
                    self.emit_burst(pos + Vec3::Y * 80.0, [255, 210, 90], 30, 300.0, 16.0, 0.9);
                    if winner == self.player_index {
                        self.announcement = Some((format!("DUEL WON: +{reward:.0} DAMAGE"), 2.5));
                    }
                    tracing::info!("kit: unit {winner} won a duel (+{reward} damage)");
                }
            }
        }
    }

    /// Extra multiplier for projectiles that hit `unit`.
    pub(super) fn kit_projectile_taken(&self, unit: usize) -> f32 {
        match self.passive_of(unit) {
            Some(Passive::ThickHide { reduction }) => 1.0 - reduction.clamp(0.0, 0.9),
            _ => 1.0,
        }
    }

    /// Reflect and second-form hooks after kit caster `pi` took damage.
    pub(super) fn kit_after_hit(&mut self, pi: usize, amount: f32, aggressor: Option<usize>) {
        // Spiked Carapace: the first hit from each attacker in the window
        // comes back and stuns it.
        if let Some(attacker) = aggressor
            && attacker != pi
            && self.units[attacker].health > 0.0
            && let Some(rt) = self.casters.get_mut(&pi)
            && rt.reflect_stun_timer > 0.0
            && !rt.reflected.contains(&attacker)
        {
            rt.reflected.push(attacker);
            let stun = rt.reflect_stun;
            let back = amount * rt.reflect.max(1.0);
            let team = self.units[pi].team;
            self.damage_unit_mixed(attacker, 0.0, back, team, Some(pi));
            let from = self.units[pi].pos;
            self.land_statuses(
                attacker,
                &[Status::Stun { duration: stun }],
                from,
                &Gfx::default(),
                false,
                pi,
            );
        }
        let Some(rt) = self.casters.get(&pi) else {
            return;
        };
        if rt.reflect_timer > 0.0
            && let Some(attacker) = aggressor
            && attacker != pi
            && self.units[attacker].health > 0.0
            && self.units[attacker].attack_range < 300.0
        {
            let back = amount * rt.reflect;
            let team = self.units[pi].team;
            self.damage_unit_mixed(attacker, 0.0, back, team, Some(pi));
        }
        let Some(Passive::SecondForm {
            threshold,
            heal,
            npc,
            duration,
            scale,
            range,
            damage,
            armor,
            move_speed,
        }) = self.passive_of(pi).cloned()
        else {
            return;
        };
        let used = self.casters.get(&pi).is_none_or(|rt| rt.second_form_used);
        let player = &self.units[pi];
        if used || player.health <= 0.0 || player.health > player.max_health * threshold {
            return;
        }
        if let Some(rt) = self.casters.get_mut(&pi) {
            rt.second_form_used = true;
        }
        let player = &mut self.units[pi];
        player.health = (player.health + player.max_health * heal).min(player.max_health);
        let pos = player.pos;
        let form = kit::FormStep {
            npc,
            scale,
            damage,
            range,
            move_speed,
            armor,
            bonus_health: 0.0,
            on_hit: Vec::new(),
            splash: 0.0,
            evasion: false,
        };
        self.kit_transform(pi, &form, duration);
        self.emit_burst(pos, [255, 220, 120], 40, 360.0, 18.0, 1.0);
        if pi == self.player_index {
            self.announcement = Some(("SECOND FORM".to_string(), 2.0));
        }
        tracing::info!("kit: unit {pi} second form");
    }

    /// Allied minion damage multiplier from any nearby caster's aura.
    pub(super) fn kit_minion_aura(&self, minion: usize) -> f32 {
        let unit = &self.units[minion];
        let mut mult = 1.0;
        for (&caster, rt) in &self.casters {
            let Some(Passive::MinionAura { radius, damage }) = rt.kit.passive.as_ref() else {
                continue;
            };
            let hero = &self.units[caster];
            if unit.team == hero.team && hero.health > 0.0 && unit.pos.distance(hero.pos) <= *radius
            {
                mult = f32::max(mult, 1.0 + damage);
            }
        }
        mult
    }

    /// Experience multiplier from `unit`'s scholar passive.
    pub(super) fn kit_xp_mult(&self, unit: usize) -> f32 {
        match self.passive_of(unit) {
            Some(Passive::Scholar { xp }) => 1.0 + xp,
            _ => 1.0,
        }
    }

    /// A kit champion's basic attack: passives, empowers, splash, frost,
    /// forms, banked damage and Jad attacks all live here.
    pub(super) fn kit_land_attack(&mut self, pi: usize, target: usize) {
        let passive = self.passive_of(pi).cloned();
        let team = self.units[pi].team;
        let target_pos = self.units[target].pos;
        let (mut damage, ranged, from, level) = {
            let unit = &mut self.units[pi];
            unit.attack_cooldown = unit.attack_interval / unit.attack_speed_mult.max(0.1);
            if unit.cc.disarmed() {
                return;
            }
            let mut damage = unit.attack_damage;
            if unit.ranged_auto {
                damage *= unit.ranged_scale();
            }
            (
                damage,
                unit.ranged_auto,
                unit.pos + Vec3::Y * (unit.scale * 80.0),
                unit.level,
            )
        };
        if self.attack_misses(target) {
            let tint = self.hitsplat_tint(Some(pi));
            self.push_hitsplat(target, 0.0, super::HitsplatKind::Miss, tint);
            return;
        }
        let Some(rt) = self.casters.get_mut(&pi) else {
            return;
        };
        // Innates grow with hero level (SYS-4).
        let growth = 1.0 + rt.kit.innate_per_level * level.saturating_sub(1) as f32;
        let mut statuses: Vec<Status> = Vec::new();
        match &passive {
            Some(Passive::OnHit { statuses: s }) => {
                statuses.extend(s.iter().map(|status| match status {
                    Status::Slow { pct, duration } => Status::Slow {
                        pct: pct * growth,
                        duration: *duration,
                    },
                    other => other.clone(),
                }))
            }
            Some(Passive::FrostBite { pct, duration }) => statuses.push(Status::Chill {
                pct: pct * growth,
                duration: *duration,
            }),
            _ => {}
        }
        let mut splash = match &passive {
            Some(Passive::Splash { radius, pct }) => Some((*radius, *pct)),
            _ => None,
        };
        if let Some(form) = rt.transform.as_ref() {
            statuses.extend(form.on_hit.iter().cloned());
            if form.splash > 0.0 {
                splash = Some((220.0, form.splash));
            }
        }
        if rt.attack_bonus_timer > 0.0 {
            damage += rt.attack_bonus;
        }
        // Flesh heap: the whole bank goes into this hit.
        let banked = std::mem::take(&mut rt.flesh_heap);
        damage += banked;
        let lifesteal = if rt.lifesteal_timer > 0.0 {
            rt.lifesteal
        } else {
            0.0
        };
        let mut reset_slot = None;
        let mut heal_pct = 0.0;
        let mut gfx = Gfx::default();
        let mut empowered = false;
        if let Some(empower) = rt.empower.as_mut() {
            let unit = &mut self.units[pi];
            if empower.mana_per_hit > 0.0 && unit.mana < empower.mana_per_hit {
                rt.empower = None;
            } else {
                unit.mana -= empower.mana_per_hit;
                empowered = true;
                gfx = empower.gfx.clone();
                damage += empower.bonus;
                let target_unit = &self.units[target];
                damage += (target_unit.max_health - target_unit.health) * empower.missing_health;
                damage += target_unit.max_health * empower.max_health;
                statuses.extend(empower.on_hit.iter().cloned());
                reset_slot = empower.reset_on_kill;
                heal_pct = empower.heal_pct;
                if empower.slot.is_none() {
                    empower.attacks = empower.attacks.saturating_sub(1);
                    if empower.attacks == 0 {
                        rt.empower = None;
                    }
                }
            }
        }
        rt.attack_count += 1;
        let attack_count = rt.attack_count;
        // Jad attacks: every few attacks, or every second one in Inferno.
        let jad = match (&passive, rt.jad_timer > 0.0) {
            (_, true) => Some((rt.jad_every.max(1), rt.jad_windup, true)),
            (Some(Passive::JadRhythm { every, windup, .. }), false) => {
                Some((*every, *windup, false))
            }
            _ => None,
        };
        let jad_damage = match &passive {
            Some(Passive::JadRhythm { damage, .. }) => *damage * growth,
            _ => 60.0,
        };
        let jad_now = jad.and_then(|(every, windup, inferno)| {
            rt.jad_count += 1;
            (rt.jad_count % every.max(1) == 0).then_some((windup, inferno))
        });
        let both = jad_now.is_some_and(|(_, inferno)| inferno) && rt.mastered(3);
        let boulder = rt.jad_boulder_next;
        if jad_now.is_some() {
            rt.jad_boulder_next = !boulder;
        }
        let tailwind = if rt.tailwind > 0 && ranged {
            rt.tailwind -= 1;
            match &passive {
                Some(Passive::Tailwind { pct, .. }) => Some(pct * growth),
                _ => None,
            }
        } else {
            None
        };
        let slam = (rt.slam_timer > 0.0).then_some(rt.slam_stun);
        if let Some((windup, inferno)) = jad_now {
            let mult = self.kit_ability_mult(pi);
            self.jad_attack(
                pi,
                target,
                jad_damage * mult,
                windup,
                inferno,
                boulder,
                both,
            );
            return;
        }
        if empowered {
            self.emit_burst(
                target_pos + Vec3::Y * 60.0,
                [255, 240, 160],
                14,
                240.0,
                14.0,
                0.5,
            );
        }
        if banked > 0.0 {
            self.emit_burst(
                target_pos + Vec3::Y * 60.0,
                [170, 40, 30],
                18,
                220.0,
                12.0,
                0.6,
            );
        }
        // Bone crusher: slowed or taunted targets take a share of max health.
        if let Some(Passive::BoneCrusher { pct, cap }) = &passive {
            let foe = &self.units[target];
            if foe.cc.slow_timer > 0.0 || foe.cc.taunt_timer > 0.0 || foe.cc.root_timer > 0.0 {
                let mut bonus = foe.max_health * pct * growth;
                if !foe.kind.is_champion() {
                    bonus = bonus.min(*cap);
                }
                damage += bonus;
            }
        }
        if ranged {
            let aim = self.unit_chest(target);
            let dir = (aim - from).normalize_or_zero();
            let look = self.units[pi].look;
            let mut shots = vec![(target, damage)];
            if let Some(share) = tailwind
                && let Some(second) = self
                    .kit_enemies_in(team, target_pos, 600.0, Vec3::Z, 360.0)
                    .into_iter()
                    .find(|&i| i != target)
            {
                shots.push((second, damage * share));
            }
            for (goal, amount) in shots {
                let aim = self.unit_chest(goal);
                self.kit_world.shots.push(KitShot {
                    caster: pi,
                    team,
                    pos: from,
                    dir: (aim - from).normalize_or(dir),
                    speed: 1400.0,
                    remaining: (aim - from).length().max(1.0),
                    width: 80.0,
                    damage: amount,
                    pierce: false,
                    explode: splash.map_or(0.0, |(radius, _)| radius),
                    on_hit: statuses.clone(),
                    hit: Vec::new(),
                    color: fx::look(look).glow,
                    from: self.units[pi].pos,
                    pure: false,
                    basic: true,
                    look,
                    tint: None,
                    trail: Vec::new(),
                    age: 0.0,
                    ember_timer: 0.0,
                    target: Some(goal),
                    gfx: gfx.clone(),
                    landed: false,
                });
            }
            if lifesteal + heal_pct > 0.0 {
                let unit = &mut self.units[pi];
                unit.health = (unit.health + damage * (lifesteal + heal_pct)).min(unit.max_health);
            }
            self.muzzle_flash(from, look);
            return;
        }
        self.melee_slash(pi, target, [255, 235, 200]);
        let at = self.units[pi].pos;
        let before = self.units[target].health;
        if let Some(stun) = slam {
            statuses.push(Status::Stun { duration: stun });
        }
        self.kit_hit(pi, team, target, damage, &statuses, at, &gfx, false);
        let dealt = (before - self.units[target].health).max(0.0);
        if lifesteal + heal_pct > 0.0 && dealt > 0.0 {
            let unit = &mut self.units[pi];
            unit.health = (unit.health + dealt * (lifesteal + heal_pct)).min(unit.max_health);
        }
        let killed = self.units[target].health <= 0.0;
        if killed {
            self.units[pi].attack_target = None;
            if let Some(slot) = reset_slot {
                self.units[pi].cooldowns[slot] = 0.0;
                if let Some(rt) = self.casters.get_mut(&pi) {
                    rt.charges[slot] = rt.max_charges(slot);
                }
            }
        }
        let mut aoe: Option<(f32, f32)> = splash;
        if let Some(stun) = slam {
            aoe = Some((220.0, 0.6));
            let _ = stun;
        }
        if let Some(Passive::NthAttack {
            n,
            radius,
            damage: pct,
        }) = &passive
            && attack_count % n.max(&1) == 0
        {
            aoe = Some((*radius, *pct * growth));
            self.effects.push(Effect {
                pos: target_pos,
                radius: *radius,
                remaining: 0.4,
                duration: 0.4,
                color: [255, 120, 60],
            });
        }
        if let Some((radius, pct)) = aoe {
            let extra: Vec<Status> = slam
                .map(|stun| vec![Status::Stun { duration: stun }])
                .unwrap_or_default();
            for other in self.kit_enemies_in(team, target_pos, radius, Vec3::Z, 360.0) {
                if other != target {
                    self.kit_hit(
                        pi,
                        team,
                        other,
                        damage * pct,
                        &extra,
                        target_pos,
                        &Gfx::default(),
                        false,
                    );
                }
            }
        }
    }

    /// A Jad attack on `target`: a line of fire from Jad, or a boulder that
    /// falls where the target stood. `both` sends the two at once.
    #[allow(clippy::too_many_arguments)]
    fn jad_attack(
        &mut self,
        pi: usize,
        target: usize,
        damage: f32,
        windup: f32,
        inferno: bool,
        boulder: bool,
        both: bool,
    ) {
        let team = self.units[pi].team;
        let origin = self.units[pi].pos;
        let spot = self.units[target].pos;
        let line = planar_delta(origin, spot);
        let facing = line.normalize_or(Vec3::Z);
        if !boulder || both {
            // Fire: a narrow cone reaching past the target.
            self.kit_world.areas.push(PendingArea {
                caster: pi,
                team,
                pos: origin,
                radius: line.length() + 160.0,
                delay: windup,
                arc: JAD_FIRE_ARC,
                facing,
                damage,
                heal_per_hit: 0.0,
                on_hit: Vec::new(),
                color: [255, 120, 40],
                from: origin,
                pure: false,
                sound: true,
                gfx: Gfx::default(),
            });
            self.add_decal(
                origin + facing * (line.length() * 0.5),
                90.0,
                [150, 60, 20],
                windup + 0.1,
            );
        }
        if boulder || both {
            // The boulder lands on where the target stood; moving dodges.
            let fall = if inferno { 0.55 } else { 1.0 };
            self.add_decal(spot, 150.0, [110, 80, 50], windup + fall + 0.1);
            self.kit_world.areas.push(PendingArea {
                caster: pi,
                team,
                pos: spot,
                radius: 160.0,
                delay: windup + fall,
                arc: 360.0,
                facing,
                damage,
                heal_per_hit: 0.0,
                on_hit: vec![Status::Stun { duration: 0.4 }],
                color: [200, 140, 80],
                from: spot,
                pure: false,
                sound: true,
                gfx: Gfx::default(),
            });
        }
        // Jad rears up: he stays put through the wind-up.
        if let Some(rt) = self.casters.get_mut(&pi) {
            rt.rooted_timer = rt.rooted_timer.max(windup);
        }
        self.emit_burst(
            origin + Vec3::Y * 120.0,
            [255, 140, 60],
            16,
            200.0,
            14.0,
            windup,
        );
    }

    /// Advance every caster's kit state and every effect in flight. Called
    /// once per frame after the player moves.
    pub(super) fn update_kit(&mut self, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        if self.casters.is_empty() {
            return;
        }
        let pi = self.player_index;
        if let Some(rt) = self.casters.get_mut(&pi) {
            rt.ranks = self.ability_ranks;
        }
        for caster in self.caster_ids() {
            if caster != pi {
                let level = self.units[caster].level;
                if let Some(rt) = self.casters.get_mut(&caster) {
                    rt.ranks = std::array::from_fn(|slot| {
                        super::ranks::auto_rank(slot, level, rt.rank_table[slot].len())
                    });
                }
            }
            self.update_caster(caster, dt, height, collision);
        }
        self.update_kit_shots(dt, height);
        self.update_kit_reels(dt);
        self.update_kit_areas(dt);
        self.update_kit_zones(dt);
        self.update_hexes(dt);
        self.update_healers(dt);
    }

    /// Healer summons mend their owner (and, with the mastery, allied
    /// heroes near them) every frame.
    fn update_healers(&mut self, dt: f32) {
        let healers: Vec<(usize, f32, bool)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.cc.summoned && u.cc.heal_owner > 0.0 && u.health > 0.0)
            .filter_map(|(i, u)| u.cc.owner.map(|_| (i, u.cc.heal_owner, u.cc.heal_allies)))
            .collect();
        for (healer, share, allies) in healers {
            let Some(owner) = self.units[healer].cc.owner else {
                continue;
            };
            let at = self.units[healer].pos;
            let team = self.units[healer].team;
            let amount = share * self.units[owner].max_health * dt;
            for i in 0..self.units.len() {
                let unit = &mut self.units[i];
                let near = planar_delta(at, unit.pos).length() <= HEALER_REACH;
                let mine = i == owner || (allies && unit.kind.is_champion() && unit.team == team);
                if mine && near && unit.health > 0.0 {
                    unit.health = (unit.health + amount).min(unit.max_health);
                }
            }
        }
    }

    fn update_caster(
        &mut self,
        pi: usize,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) {
        let Some(rt) = self.casters.get_mut(&pi) else {
            return;
        };
        let charges: [(u8, f32); 4] = std::array::from_fn(|slot| {
            let ability = &rt.kit.abilities[slot];
            (rt.max_charges(slot), rt.cooldown(slot, ability.cooldown))
        });

        // Recharge multi-charge abilities; show the recharge only when empty.
        for (slot, (max, cooldown)) in charges.into_iter().enumerate() {
            if max <= 1 {
                continue;
            }
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

        for timer in [
            &mut rt.combo_timer,
            &mut rt.damage_reduction_timer,
            &mut rt.reflect_timer,
            &mut rt.haste_timer,
            &mut rt.untargetable_timer,
            &mut rt.rooted_timer,
            &mut rt.fury_timer,
            &mut rt.reflect_stun_timer,
            &mut rt.jad_timer,
            &mut rt.slam_timer,
            &mut rt.attack_bonus_timer,
            &mut rt.lifesteal_timer,
        ] {
            *timer = (*timer - dt).max(0.0);
        }
        for window in rt.recast.iter_mut() {
            if let Some((left, _)) = window {
                *left -= dt;
                if *left <= 0.0 {
                    *window = None;
                }
            }
        }
        if let Some(pending) = rt.pending.as_mut() {
            pending.held += dt;
        }
        if rt.bonus_health > 0.0 {
            rt.bonus_health_timer -= dt;
            if rt.bonus_health_timer <= 0.0 {
                let gone = std::mem::take(&mut rt.bonus_health);
                let unit = &mut self.units[pi];
                unit.max_health -= gone;
                unit.health = unit.health.min(unit.max_health).max(1.0);
            }
        }
        let rt = self.casters.get_mut(&pi).expect("caster");
        if let Some(empower) = rt.empower.as_mut() {
            empower.remaining -= dt;
            if empower.remaining <= 0.0 {
                rt.empower = None;
            }
        }
        let mut revert = false;
        if let Some(transform) = rt.transform.as_mut() {
            transform.remaining -= dt;
            revert = transform.remaining <= 0.0;
        }
        if revert {
            self.kit_revert_transform(pi);
        }

        // Death clears everything the champion was doing; respawn re-arms
        // the second form.
        if self.units[pi].health <= 0.0 {
            if let Some(rt) = self.casters.get_mut(&pi) {
                rt.dash = None;
                rt.channel = None;
                rt.empower = None;
                rt.rooted_timer = 0.0;
                rt.second_form_used = false;
                rt.toggled = [false; 4];
                rt.recast = [None; 4];
                rt.pending = None;
            }
            self.kit_revert_transform(pi);
        }

        self.update_kit_dash(pi, dt, height, collision);
        self.update_kit_channel(pi, dt);
    }

    fn update_kit_dash(
        &mut self,
        pi: usize,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) {
        let Some(mut dash) = self.casters.get_mut(&pi).and_then(|rt| rt.dash.take()) else {
            return;
        };
        let team = self.units[pi].team;
        if let Some(goal) = dash.chase {
            // A charge at a unit: stunned or rooted, it stops; else it
            // steers onto the target until beside it.
            let me = &self.units[pi];
            if me.stun_timer > 0.0 || me.cc.root_timer > 0.0 || self.units[goal].health <= 0.0 {
                return;
            }
            let to = planar_delta(me.pos, self.units[goal].pos);
            dash.dir = to.normalize_or(dash.dir);
            dash.remaining = (to.length() - DASH_STOP).max(0.0);
        }
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
            for index in self.kit_enemies_in(team, pos, 130.0, Vec3::Z, 360.0) {
                if !dash.hit.contains(&index) {
                    dash.hit.push(index);
                    let foe = &self.units[index];
                    if dash.reset_below > 0.0
                        && foe.kind.is_champion()
                        && foe.health < foe.max_health * dash.reset_below
                    {
                        self.units[pi].cooldowns[dash.ctx.slot] = 0.0;
                    }
                    self.kit_hit(
                        pi,
                        team,
                        index,
                        dash.damage,
                        &dash.on_hit,
                        pos,
                        &dash.gfx,
                        dash.ctx.pure,
                    );
                    self.chaos_passive(pi, index, pos);
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
            if pi != self.player_index {
                self.bot_forget_path(pi);
            }
        } else if let Some(rt) = self.casters.get_mut(&pi) {
            rt.dash = Some(dash);
        }
    }

    fn update_kit_channel(&mut self, pi: usize, dt: f32) {
        let Some(mut channel) = self.casters.get_mut(&pi).and_then(|rt| rt.channel.take()) else {
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
            if let Some(rt) = self.casters.get_mut(&pi) {
                rt.channel = Some(channel);
            }
        }
    }

    fn update_kit_shots(&mut self, dt: f32, height: &HeightGrid) {
        let shots = std::mem::take(&mut self.kit_world.shots);
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
                    if shot.hit.contains(&index) || !self.is_kit_enemy(shot.team, index) {
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
                if !shot.landed {
                    shot.landed = true;
                    if let Some(id) = shot.gfx.land {
                        let ground = Vec3::new(
                            shot.pos.x,
                            height.height_at(shot.pos.x, shot.pos.z),
                            shot.pos.z,
                        );
                        self.gfx_at(id, ground, shot.gfx.scale);
                    }
                }
                let (caster, team) = (shot.caster, shot.team);
                self.kit_hit(
                    caster,
                    team,
                    index,
                    shot.damage,
                    &shot.on_hit,
                    shot.from,
                    &shot.gfx,
                    shot.pure,
                );
                if !shot.basic {
                    self.chaos_passive(caster, index, shot.from);
                }
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
                        match self.passive_of(caster) {
                            Some(Passive::Splash { pct, .. }) => *pct,
                            _ => 0.5,
                        }
                    } else {
                        0.6
                    };
                    for other in self.kit_enemies_in(team, center, shot.explode, Vec3::Z, 360.0) {
                        if other != index {
                            self.kit_hit(
                                caster,
                                team,
                                other,
                                shot.damage * splash,
                                &shot.on_hit,
                                center,
                                &shot.gfx,
                                shot.pure,
                            );
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
            } else if look.rope.is_some() {
                let catch = hits.first().copied();
                self.kit_world.reels.push(KitReel {
                    caster: shot.caster,
                    pos: shot.pos,
                    look: shot.look,
                    catch,
                    latch: if catch.is_some() { REEL_LATCH } else { 0.0 },
                    age: shot.age,
                });
            }
        }
        self.kit_world.shots.extend(survivors);
    }

    /// Where a roped shot's rope ties on at its caster: the throwing
    /// height, a little in front of it.
    fn kit_rope_anchor(&self, caster: usize) -> Vec3 {
        let caster = &self.units[caster];
        let forward = Vec3::new(caster.yaw.sin(), 0.0, caster.yaw.cos());
        caster.pos + Vec3::Y * (caster.scale * 80.0) + forward * 40.0
    }

    /// Landed roped shots: stuck in their catch for a moment, then reeled
    /// home. Purely visual; the pull is a status the hit already applied.
    fn update_kit_reels(&mut self, dt: f32) {
        if self.kit_world.reels.is_empty() {
            return;
        }
        let mut reels = std::mem::take(&mut self.kit_world.reels);
        reels.retain_mut(|reel| {
            reel.age += dt;
            if self.units[reel.caster].health <= 0.0 {
                return false;
            }
            if let Some(catch) = reel.catch.filter(|_| reel.latch > 0.0) {
                reel.latch -= dt;
                if self.units.get(catch).is_some_and(|unit| unit.health > 0.0) {
                    reel.pos = self.unit_chest(catch);
                    return true;
                }
            }
            let hand = self.kit_rope_anchor(reel.caster);
            let to = hand - reel.pos;
            let distance = to.length();
            let step = REEL_SPEED * dt;
            if distance <= step + 40.0 {
                return false;
            }
            reel.pos += to / distance * step;
            true
        });
        self.kit_world.reels = reels;
    }

    /// Drawing data for every kit shot in flight, and for roped heads being
    /// reeled in (pointing away from the caster, as the rope pulls them).
    /// Shots the player cannot see stay hidden.
    pub(super) fn kit_shot_visuals(&self) -> Vec<ShotVisual> {
        let flying = self
            .kit_world
            .shots
            .iter()
            .filter(|shot| self.kit_effect_visible(shot.team, shot.pos))
            .map(|shot| ShotVisual {
                pos: shot.pos,
                dir: shot.dir,
                look: shot.look,
                trail: shot.trail.clone(),
                age: shot.age,
                tint: shot.tint,
                rope_from: fx::look(shot.look)
                    .rope
                    .map(|_| self.kit_rope_anchor(shot.caster)),
            });
        let reeling = self.kit_world.reels.iter().map(|reel| {
            let hand = self.kit_rope_anchor(reel.caster);
            ShotVisual {
                pos: reel.pos,
                dir: planar_delta(hand, reel.pos).normalize_or(Vec3::Z),
                look: reel.look,
                trail: Vec::new(),
                age: reel.age,
                tint: None,
                rope_from: Some(hand),
            }
        });
        flying.chain(reeling).collect()
    }

    /// The look a kit shot should be drawn and lit with.
    pub(super) fn kit_shot_lights(&self) -> Vec<(Vec3, fx::ProjectileLook)> {
        self.kit_world
            .shots
            .iter()
            .filter(|shot| self.kit_effect_visible(shot.team, shot.pos))
            .map(|shot| (shot.pos, fx::look(shot.look).tinted(shot.tint)))
            .collect()
    }

    /// Allied effects always show; enemy ones only where the player's team
    /// has vision.
    fn kit_effect_visible(&self, team: u8, pos: Vec3) -> bool {
        team == self.units[self.player_index].team || self.in_team_vision(pos)
    }

    fn update_kit_areas(&mut self, dt: f32) {
        let areas = std::mem::take(&mut self.kit_world.areas);
        let mut waiting = Vec::with_capacity(areas.len());
        for mut area in areas {
            area.delay -= dt;
            if area.delay > 0.0 {
                waiting.push(area);
                continue;
            }
            if area.sound {
                self.play_sound("kit_area", area.pos);
            }
            if let Some(id) = area.gfx.land {
                self.gfx_at(id, area.pos, area.gfx.scale);
            }
            let hits = self.kit_enemies_in(area.team, area.pos, area.radius, area.facing, area.arc);
            let count = hits.len();
            let (caster, team) = (area.caster, area.team);
            for index in hits {
                self.kit_hit(
                    caster,
                    team,
                    index,
                    area.damage,
                    &area.on_hit,
                    area.from,
                    &area.gfx,
                    area.pure,
                );
                self.chaos_passive(caster, index, area.from);
            }
            if area.heal_per_hit > 0.0 && count > 0 {
                let player = &mut self.units[caster];
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
        self.kit_world.areas.extend(waiting);
    }

    fn update_kit_zones(&mut self, dt: f32) {
        let zones = std::mem::take(&mut self.kit_world.zones);
        let mut alive = Vec::with_capacity(zones.len());
        for mut zone in zones {
            let pi = zone.caster;
            if zone.follow {
                if self.units[pi].health <= 0.0 {
                    continue;
                }
                zone.pos = self.units[pi].pos;
            }
            zone.remaining -= dt;
            zone.radius += zone.grow * dt;
            zone.timer -= dt;
            if zone.timer <= 0.0 {
                zone.timer += zone.tick;
                for index in self.kit_enemies_in(zone.team, zone.pos, zone.radius, Vec3::Z, 360.0) {
                    self.kit_hit(
                        pi,
                        zone.team,
                        index,
                        zone.damage,
                        &zone.on_tick,
                        zone.pos,
                        &zone.gfx,
                        zone.pure,
                    );
                }
                if zone.self_damage > 0.0 {
                    let player = &mut self.units[pi];
                    player.health = (player.health - zone.self_damage).max(1.0);
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
        self.kit_world.zones.extend(alive);
    }
}

/// How a bot should use an ability, read from its first actions.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BotPlan {
    /// Heals, shields and defences: when hurt.
    Care,
    /// Self buffs, empowers and transforms: in a fight with something big.
    Power,
    /// A dash or leap: to close on a hero up to this far away.
    Gap(f32),
    /// Damage or crowd control reaching this far.
    Hit(f32),
}

fn bot_plan(actions: &[Action], reach: f32) -> BotPlan {
    let mut hit: f32 = 0.0;
    let mut care = false;
    let mut power = false;
    for action in actions {
        match action {
            Action::Dash {
                distance,
                to_target: false,
                ..
            } => return BotPlan::Gap(distance * reach),
            Action::Dash { distance, .. } => hit = hit.max(distance * reach),
            Action::Strike { .. } => hit = hit.max(450.0 * reach),
            Action::Toss { .. } => hit = hit.max(600.0),
            Action::Projectile { range, .. } => hit = hit.max(range * reach * 0.9),
            Action::Area {
                at, range, radius, ..
            }
            | Action::Zone {
                at, range, radius, ..
            } => {
                let r = match at {
                    Anchor::Point => range * reach + radius * 0.5,
                    _ => *radius,
                };
                hit = hit.max(r);
            }
            Action::Channel { action, .. } => {
                if let BotPlan::Hit(r) = bot_plan(std::slice::from_ref(action), reach) {
                    hit = hit.max(r);
                }
            }
            Action::Summon { .. } => hit = hit.max(700.0),
            Action::Buff {
                heal,
                heal_missing,
                shield,
                damage_reduction,
                ..
            } => {
                if heal + heal_missing + shield + damage_reduction > 0.0 {
                    care = true;
                } else {
                    power = true;
                }
            }
            Action::Empower { .. } | Action::Transform { .. } => power = true,
        }
    }
    if hit > 0.0 {
        BotPlan::Hit(hit)
    } else if care {
        BotPlan::Care
    } else if power {
        BotPlan::Power
    } else {
        BotPlan::Hit(300.0)
    }
}

/// A muted shade of an ability colour for ground decals. Dark colours are
/// lifted so a red or black zone reads as a stain, not a hole in the map.
fn dim(color: [u8; 3]) -> [u8; 3] {
    color.map(|c| (c as f32 * 0.55 + 38.0).min(255.0) as u8)
}
