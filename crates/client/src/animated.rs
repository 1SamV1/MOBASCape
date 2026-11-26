use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use glam::{Mat3, Vec3};
use rift_cache::{AnimationClip, AnimationScratch, Cache, Model, model::hsl_to_rgb};

pub mod awowogei;
mod battle;
pub mod bot_cast;
pub mod bots;
mod kit_runtime;
pub mod ranks;
mod teleport;
mod thrall;

use crate::champion::ChampionConfig;
use crate::fx::{self, FxMesh};
use crate::items::ItemConfig;
use crate::motion::PlayerMotion;
use crate::scene::{
    AmbientKind, CampBuff, HeightGrid, SceneData, SceneVertex, StructureKind, WorldCollision,
    model_dominant_texture, model_vertex_animated,
};

/// Highest champion level.
const MAX_LEVEL: u32 = 25;
const Q_COOLDOWN: f32 = 5.0;
const PLAYER_RESPAWN: f32 = 5.0;
/// The shared meteor lands at the cursor, clamped to this cast range.
const METEOR_RANGE: f32 = 1_100.0;
/// While a committed attack winds up, the target may drift this much past
/// attack range before the swing is abandoned.
const WINDUP_LEASH: f32 = 90.0;
/// Seconds a one-shot clip takes to blend in and out of the base pose.
const ACTION_BLEND_IN: f32 = 0.06;
const ACTION_BLEND_OUT: f32 = 0.12;
const MINION_AGGRO: f32 = 520.0;
/// Radius and share of experience for heroes near a kill they did not land.
const SHARE_RADIUS: f32 = 1400.0;
const SHARE_XP: f32 = 0.6;
/// Extra aggro reach minions have toward enemy structures, matching the
/// 5-tile clearance the scene keeps between towers and the lane centre.
const STRUCTURE_REACH: f32 = 400.0;
const MAX_MINIONS_PER_TEAM: usize = 36;
const DEATH_SINK_TIME: f32 = 0.45;
/// OSRS models are authored facing -Z, while a unit's `yaw` points +Z along
/// its movement (`dir.x.atan2(dir.z)`). Every unit is turned half a circle
/// when it is drawn; `model_yaw_offset` is only for genuine exceptions.
const CACHE_MODEL_FORWARD: f32 = std::f32::consts::PI;
/// Model vertex groups used by King Awowogei's arms and hands.
const MONKEY_ARM_GROUPS: &[u8] = &[11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 37];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Player,
    Ambient,
    Dummy,
    MeleeMinion,
    RangedMinion,
    Tower,
    Inhibitor,
    Nexus,
    Camp,
    Watchtower,
    /// Enemy champion controlled by the demo AI.
    BotChampion,
    /// Chest near the team spawn that holds remotely purchased items.
    StorageChest,
    /// Flying Armadylean courier that delivers stored purchases.
    DeliveryGuard,
}

/// Whether a kill of `kind` reaches the kill feed and the centre
/// announcements: heroes and structures always, the practice dummy only
/// when a hero (`by_hero`) landed it. Minion and camp kills would bury them
/// (the reference feed shows neither).
fn reaches_feed(kind: UnitKind, by_hero: bool) -> bool {
    kind.is_champion() || kind.is_structure() || (kind == UnitKind::Dummy && by_hero)
}

/// Short display name used by kill announcements and the kill feed.
fn short_unit_name(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Player => "YOUR CHAMPION",
        UnitKind::BotChampion => "A RIVAL CHAMPION",
        UnitKind::MeleeMinion | UnitKind::RangedMinion => "A MINION",
        UnitKind::Tower => "A TOWER",
        UnitKind::Inhibitor => "AN INHIBITOR",
        UnitKind::Nexus => "A NEXUS",
        UnitKind::Camp => "A CAMP",
        UnitKind::Watchtower => "A WATCHTOWER",
        UnitKind::Dummy => "A DUMMY",
        UnitKind::Ambient => "SCENERY",
        UnitKind::StorageChest => "THE CHEST",
        UnitKind::DeliveryGuard => "THE GUARD",
    }
}

impl UnitKind {
    fn is_minion(self) -> bool {
        matches!(self, Self::MeleeMinion | Self::RangedMinion)
    }

    pub fn is_structure(self) -> bool {
        matches!(self, Self::Tower | Self::Inhibitor | Self::Nexus)
    }

    pub fn is_champion(self) -> bool {
        matches!(self, Self::Player | Self::BotChampion)
    }
}

pub struct AnimatedUnit {
    pub model: Arc<Model>,
    pub idle: Arc<AnimationClip>,
    pub walk: Option<Arc<AnimationClip>>,
    /// Reused animation output and frame scratch, so a unit that does not
    /// blend clips does not allocate every tick.
    pose: Vec<[f32; 3]>,
    pose_scratch: AnimationScratch,
    pub pos: Vec3,
    pub yaw: f32,
    /// Model-space forward correction (usually 0 or PI).
    pub model_yaw_offset: f32,
    /// Weighty locomotion for the player's champion; `None` for everything
    /// else, which keeps minion movement cheap and predictable.
    pub motion: Option<PlayerMotion>,
    /// 0 = idle pose, 1 = walk pose. Player only; smooths the clip change.
    pub walk_blend: f32,
    /// Per-champion multiplier for local arm transforms while walking.
    pub walk_arm: f32,
    /// One pass of this unit's walk clip, and the distance one pass covers.
    pub walk_loop_ms: f32,
    pub walk_stride: f32,
    pub scale: f32,
    pub anim_time_ms: f32,
    pub moving: bool,
    pub kind: UnitKind,
    pub team: u8,
    pub health: f32,
    pub max_health: f32,
    pub hit_flash: f32,
    /// Counts down while a fresh corpse sinks and shrinks away.
    pub death_anim: f32,
    pub dead_timer: f32,
    pub target: Option<Vec3>,
    pub path: Vec<Vec3>,
    pub path_index: usize,
    pub attack_target: Option<usize>,
    pub attack_cooldown: f32,
    /// Time left before a committed player attack lands.
    pub attack_windup_timer: f32,
    pub attack_windup: f32,
    pub attack_pending: bool,
    pub attack_range: f32,
    pub attack_damage: f32,
    pub attack_interval: f32,
    pub speed: f32,
    /// Real armor: physical damage multiplier is 100 / (100 + armor).
    pub armor: f32,
    /// Positive stacks of armor reduction (kit shreds).
    pub armor_reduction: f32,
    pub armor_reduction_timer: f32,
    pub stun_timer: f32,
    /// Venom: damage per tick while the timer runs.
    pub venom_timer: f32,
    pub venom_damage: f32,
    pub venom_tick: f32,
    pub venom_team: u8,
    /// Attack speed multiplier from Footwork; 1.0 is the base rate.
    pub attack_speed_mult: f32,
    pub attack_speed_timer: f32,
    /// Ranged level, used to scale ranged damage.
    pub ranged: f32,
    pub is_champion: bool,
    /// Seconds before this unit becomes visible and starts acting. Used to
    /// stagger the members of a minion wave without changing the sim tick.
    pub spawn_delay: f32,
    pub hit_radius: f32,
    pub bar_height: f32,
    pub lane: Vec<Vec3>,
    pub lane_index: usize,
    pub respawn_time: f32,
    pub camp_buff: Option<CampBuff>,
    /// A region boss camp (bots take it as a team for the buff).
    pub camp_boss: bool,
    /// The camp creature's name from map.json, when it has one.
    pub camp_name: Option<String>,
    pub mana: f32,
    pub max_mana: f32,
    pub level: u32,
    pub xp: f32,
    pub gold: u32,
    pub shield: f32,
    pub shield_timer: f32,
    pub cooldowns: [f32; 4],
    pub visible: bool,
    pub ranged_auto: bool,
    pub q_damage: f32,
    pub r_damage: f32,
    pub e_heal: f32,
    pub e_shield: f32,
    pub w_distance: f32,
    pub ability_power: f32,
    pub structure_lane: Option<usize>,
    pub super_minion: bool,
    pub home: Vec3,
    pub aggro_range: f32,
    pub leash_range: f32,
    pub camp_aggro: bool,
    pub camp_wander_timer: f32,
    /// Scenery-creature variant and deterministic wander phase.
    pub ambient_kind: Option<AmbientKind>,
    pub ambient_seed: u32,
    /// Range at which a watchtower acquires its first target.
    pub lock_range: f32,
    /// Crowd control from kit abilities, and summon lifetime.
    pub cc: crate::kit::Afflictions,
    /// The teleport scroll every hero carries (unused by other units).
    pub scroll: crate::scroll::ScrollState,
    /// A magic hero's two battle spells (empty for everyone else).
    pub spells: crate::spells::Loadout,
    /// Vengeance waiting for an enemy hero's hit.
    pub vengeance: Option<crate::spells::Vengeance>,
    /// One-shot clips for basic attacks and ability casts.
    pub attack_clip: Option<Arc<AnimationClip>>,
    pub cast_clip: Option<Arc<AnimationClip>>,
    /// Fraction of `attack_clip` at which the blow connects.
    pub attack_hit: f32,
    /// The one-shot clip playing over idle/walk, if any.
    pub action: Option<ActionAnim>,
    /// A computer-controlled attack winding up toward its hit.
    pub strike: Option<PendingStrike>,
    /// Basic-attack projectile look (`fx::LOOKS` index).
    pub look: usize,
    /// Footprint radius used for edge-to-edge attack range. Smaller than
    /// `hit_radius`, which is a generous projectile hit box.
    pub body_radius: f32,
    /// Ability changes from the talents this hero picked.
    pub talent: crate::talents::TalentMods,
}

/// A one-shot animation (attack or cast) played over the base pose.
#[derive(Clone)]
pub struct ActionAnim {
    pub clip: Arc<AnimationClip>,
    pub time_ms: f32,
    /// Playback speed; attacks are time scaled to their wind-up.
    pub rate: f32,
    pub length_ms: f32,
    /// 1 while playing; falls to 0 when the clip is cancelled.
    pub fade: f32,
    pub cancelling: bool,
}

impl ActionAnim {
    fn new(clip: Arc<AnimationClip>, rate: f32) -> Self {
        let length_ms = clip.loop_ms();
        Self {
            clip,
            time_ms: 0.0,
            rate: rate.clamp(0.25, 4.0),
            length_ms,
            fade: 1.0,
            cancelling: false,
        }
    }

    /// Blend weight of the clip against the base pose.
    pub fn weight(&self) -> f32 {
        let secs = self.time_ms / 1000.0 / self.rate.max(0.01);
        let left = (self.length_ms - self.time_ms).max(0.0) / 1000.0 / self.rate.max(0.01);
        let intro = (secs / ACTION_BLEND_IN).clamp(0.0, 1.0);
        let outro = (left / ACTION_BLEND_OUT).clamp(0.0, 1.0);
        intro.min(outro).min(self.fade)
    }

    /// Advance; false once the clip is over.
    fn advance(&mut self, dt: f32) -> bool {
        self.time_ms += dt * 1000.0 * self.rate;
        if self.cancelling {
            self.fade -= dt / ACTION_BLEND_OUT;
        }
        self.time_ms < self.length_ms && self.fade > 0.0
    }
}

/// A unit's animation clocks at the start of a tick, so frame sounds can be
/// played for exactly the span the tick advanced.
struct AnimClock {
    base_ms: f32,
    /// The playing one-shot clip (by identity) and its time.
    action: Option<(*const AnimationClip, f32)>,
}

impl AnimClock {
    fn of(unit: &AnimatedUnit) -> Self {
        Self {
            base_ms: unit.anim_time_ms,
            action: unit
                .action
                .as_ref()
                .map(|action| (Arc::as_ptr(&action.clip), action.time_ms)),
        }
    }
}

/// Playback rate that makes `clip`'s `hit` fraction land after `windup`
/// seconds. Zero wind-up plays the clip at its own speed.
fn action_rate(clip: &AnimationClip, hit: f32, windup: f32) -> f32 {
    if windup <= 0.0 {
        return 1.0;
    }
    let hit_ms = clip.loop_ms() * hit.clamp(0.05, 1.0);
    (hit_ms / (windup * 1000.0)).clamp(0.25, 4.0)
}

/// A wind-up for a computer-controlled attack. The blow (or projectile)
/// lands when `timer` runs out.
#[derive(Clone, Copy)]
pub struct PendingStrike {
    pub target: usize,
    pub timer: f32,
    pub damage: f32,
    pub ranged: bool,
}

/// A cache spot animation prepared for drawing: model with palette swaps,
/// its clip, the pivot that centres it, and the definition's resize.
pub struct SpotModel {
    model: Model,
    clip: Option<AnimationClip>,
    /// Cache-space centre of the rest pose.
    center: [f32; 3],
    scale_xz: f32,
    scale_y: f32,
    length_ms: f32,
    rgb: Vec<[u8; 3]>,
}

impl SpotModel {
    fn load(cache: &Cache, id: u32) -> Result<Self> {
        let data = cache.config(rift_cache::spotanim::SPOTANIM_ARCHIVE, id)?;
        let def = rift_cache::SpotAnimDefinition::parse(id, &data)?;
        anyhow::ensure!(def.model_id > 0, "spotanim {id} has no model");
        let mut model = load_model(cache, def.model_id)?;
        if !def.recolors.is_empty() {
            let (find, replace): (Vec<i16>, Vec<i16>) = def
                .recolors
                .iter()
                .map(|&(f, r)| (f as i16, r as i16))
                .unzip();
            model.recolor(&find, &replace);
        }
        let clip = if def.animation_id >= 0 {
            AnimationClip::load(cache, def.animation_id as u32).ok()
        } else {
            None
        };
        let (min, max) = model.bounds();
        let center = [
            (min[0] + max[0]) as f32 * 0.5,
            (min[1] + max[1]) as f32 * 0.5,
            (min[2] + max[2]) as f32 * 0.5,
        ];
        let length_ms = clip.as_ref().map_or(600.0, |clip| clip.loop_ms());
        let rgb = model
            .face_colors
            .iter()
            .map(|&hsl| hsl_to_rgb(hsl, 1.0))
            .collect();
        let (scale_xz, scale_y) = def.scale();
        Ok(Self {
            model,
            clip,
            center,
            scale_xz,
            scale_y,
            length_ms,
            rgb,
        })
    }
}

/// How a spot animation model is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpotDraw {
    /// A projectile: looping clip, solid faces lit with the scene.
    Flying,
    /// An impact splash: plays once, every face added to the glow pass.
    Burst,
    /// A spell graphic (cast, land or hit): plays once with solid faces,
    /// standing on the ground the way the cache authored it.
    Spell,
    /// A status graphic held on a unit (stun stars, an ice block): loops
    /// with solid faces, standing on the unit's feet.
    Held,
}

impl SpotDraw {
    fn looping(self) -> bool {
        matches!(self, SpotDraw::Flying | SpotDraw::Held)
    }

    /// Spell graphics are authored from the ground up (cache y <= 0 is
    /// above the ground), so they keep their origin; projectiles and impact
    /// splashes are centred on their position.
    fn grounded(self) -> bool {
        matches!(self, SpotDraw::Spell | SpotDraw::Held)
    }
}

/// A spot animation played in the world: an impact splash, a spell graphic
/// or a held status graphic.
pub struct SpotBurst {
    pub spotanim: u32,
    /// World position, or the offset from the followed unit's feet.
    pub pos: Vec3,
    pub yaw: f32,
    pub time_ms: f32,
    pub scale: f32,
    pub mode: SpotDraw,
    /// Ride on this unit; `pos` is then an offset from its feet.
    pub follow: Option<usize>,
    /// Seconds a held graphic has left. Unused by the other modes.
    pub hold: f32,
    /// Drawn whatever the team can see (a teleport's destination marker).
    pub seen_by_all: bool,
}

impl SpotBurst {
    /// A graphic at the origin with nothing else set: scale 1, playing from
    /// the start, not riding a unit.
    pub fn new(spotanim: u32, mode: SpotDraw) -> Self {
        Self {
            spotanim,
            pos: Vec3::ZERO,
            yaw: 0.0,
            time_ms: 0.0,
            scale: 1.0,
            mode,
            follow: None,
            hold: 0.0,
            seen_by_all: false,
        }
    }
}

/// At most this many spot animations play at once; more are dropped.
const MAX_SPOT_BURSTS: usize = 64;

/// A bright arc streak across a target struck in melee.
pub struct Slash {
    pub pos: Vec3,
    pub dir: Vec3,
    pub color: [u8; 3],
    pub size: f32,
    pub remaining: f32,
    pub duration: f32,
}

/// Everything the renderer needs to draw one flying projectile.
pub struct ShotVisual {
    pub pos: Vec3,
    pub dir: Vec3,
    pub look: usize,
    pub trail: Vec<Vec3>,
    pub age: f32,
    /// Overrides the look's glow and trail colours (kit shots without a
    /// named look take their ability colour).
    pub tint: Option<[u8; 3]>,
    /// Where a roped look's rope ties on at the caster's end.
    pub rope_from: Option<Vec3>,
}

/// Shape drawn on the ground while the ability indicator is held.
#[derive(Debug, Clone, PartialEq)]
pub enum Indicator {
    /// A range circle around the caster.
    Range { radius: f32 },
    /// A skillshot lane from the caster toward the aim.
    Line { length: f32, width: f32, angle: f32 },
    /// A targeted circle at the aim, clamped to `range`.
    Target { range: f32, radius: f32 },
    /// A cone in front of the caster.
    Cone { radius: f32, arc: f32 },
    /// A circle around the caster's feet.
    SelfArea { radius: f32 },
    /// A dash or blink, drawn as a lane with an arrowhead. `to_cursor`
    /// dashes stop at the aim point.
    Dash {
        length: f32,
        backwards: bool,
        to_cursor: bool,
    },
    /// Bananarang's teardrop flight toward the aim: out on the right,
    /// back on the left.
    Boomerang { length: f32 },
    /// Vine Swing's curved arc toward the aim, always `length` long.
    Swing { length: f32 },
}

impl Indicator {
    /// Stretch the parts of the shape that follow an ability's reach (lane
    /// length, cast range, dash length); areas around the caster keep their
    /// size.
    pub fn scale_reach(&mut self, reach: f32) {
        match self {
            Indicator::Range { radius } => *radius *= reach,
            Indicator::Line { length, .. } => *length *= reach,
            Indicator::Target { range, .. } => *range *= reach,
            Indicator::Dash { length, .. }
            | Indicator::Boomerang { length }
            | Indicator::Swing { length } => *length *= reach,
            Indicator::Cone { .. } | Indicator::SelfArea { .. } => {}
        }
    }
}

/// What the shop does with a purchase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyOutcome {
    /// The hero was at the fountain, so the item went straight to the bag.
    Delivered,
    /// The hero was away from base, so the item is waiting in the chest.
    Stored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    Idle,
    FlyingToHero,
    Returning,
}

#[derive(Clone, Copy)]
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub level: u32,
    pub xp: f32,
    pub xp_next: f32,
    pub gold: u32,
    pub cooldowns: [f32; 4],
    pub shield: f32,
    pub shield_timer: f32,
    pub attack_damage: f32,
    pub armor: f32,
    /// Ranged level, shown in the OSRS-style stats panel.
    pub ranged: f32,
    /// Seconds between basic attacks, shown as Run-Attack.
    pub attack_interval: f32,
    /// Maximum cooldown per ability slot, used by the HUD fill bars.
    pub cooldown_maxima: [f32; 4],
    pub speed: f32,
    pub ability_power: f32,
    /// Personal match line for the scoreboard strip.
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    /// Creep score (lane minions last-hit by the player).
    pub cs: u32,
}

pub struct Effect {
    pub pos: Vec3,
    pub radius: f32,
    pub remaining: f32,
    pub duration: f32,
    pub color: [u8; 3],
}

/// A short-lived billboard spark. Particles are cosmetic and deterministic:
/// they use the scene RNG and never touch combat state.
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub color: [u8; 3],
    pub size: f32,
    pub remaining: f32,
    pub duration: f32,
    pub gravity: f32,
    pub drag: f32,
}

/// A ground scorch or lingering ability stain.
pub struct Decal {
    pub pos: Vec3,
    pub radius: f32,
    pub remaining: f32,
    pub duration: f32,
    pub color: [u8; 3],
}

/// A delayed ground impact with a visible telegraph. Meteor and similar
/// abilities land after `delay`, so opponents get a chance to step out.
pub struct PendingImpact {
    pub pos: Vec3,
    pub radius: f32,
    pub delay: f32,
    pub duration: f32,
    pub damage: f32,
    pub team: u8,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct SceneLight {
    pub pos: Vec3,
    pub color: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
}

pub struct VisibilityGrid {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<bool>,
}

impl VisibilityGrid {
    pub fn is_visible(&self, tx: i32, ty: i32) -> bool {
        tx >= 0
            && ty >= 0
            && (tx as usize) < self.width
            && (ty as usize) < self.height
            && self.tiles[ty as usize * self.width + tx as usize]
    }
}

#[derive(Clone, Copy)]
pub struct TeamBuff {
    pub remaining: f32,
    pub duration: f32,
    pub damage_mult: f32,
    pub speed_mult: f32,
    pub kind: CampBuff,
}

/// Which OSRS hitmark a floating combat number uses. The sprite and the
/// number colour both come from the reference client, so damage, poison,
/// venom, disease, healing and a blocked hit each read at a glance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitsplatKind {
    Damage,
    Poison,
    Venom,
    Disease,
    Heal,
    Miss,
    /// Gold-trimmed splat for a maximum/critical hit.
    MaxHit,
}

impl HitsplatKind {
    /// Key into the cache hitmark atlas.
    pub fn sprite_name(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Poison => "poison",
            Self::Venom => "venom",
            Self::Disease => "disease",
            Self::Heal => "heal",
            Self::Miss => "miss",
            Self::MaxHit => "max",
        }
    }

    /// Fallback splat colour when the cache has no hitmark sprites.
    pub fn fallback_color(self) -> [u8; 3] {
        match self {
            Self::Damage => [195, 30, 28],
            Self::Poison => [35, 160, 45],
            Self::Venom => [35, 120, 112],
            Self::Disease => [220, 145, 20],
            Self::Heal => [165, 45, 190],
            Self::Miss => [45, 65, 220],
            Self::MaxHit => [210, 45, 25],
        }
    }
}

/// How a floating number is drawn: an OSRS hitmark, a plain coloured
/// label (gold pickups and developer text), or overhead chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatStyle {
    Hitsplat(HitsplatKind),
    Label([u8; 3]),
    /// OSRS overhead chat: yellow text with a black shadow that rides above
    /// its unit's head (or stays put without one) and does not drift.
    Say,
}

/// OSRS overhead chat yellow.
pub const SAY_YELLOW: [u8; 3] = [255, 255, 0];
/// How long overhead chat stays up, like the reference client's 150 ticks
/// cut down to match pace.
pub const SAY_SECONDS: f32 = 2.4;

/// A number that pops over a unit when it is hit, like the reference client.
pub struct FloatText {
    pub pos: Vec3,
    pub text: String,
    pub style: FloatStyle,
    pub life: f32,
    /// Text scale. Bigger hits pop larger, like the reference client.
    pub scale: f32,
    /// Column slot for hitsplats sharing one unit: 0 is the top of the
    /// stack, so a combo reads downward like the reference client.
    pub stack: u8,
    /// The unit this number belongs to, for stacking and cleanup.
    pub unit: Option<usize>,
    /// Cool tint for hits that did not come from the local player, like the
    /// reference client's tinted other-player hitsplats.
    pub tint: Option<[u8; 3]>,
}

pub struct Projectile {
    pub pos: Vec3,
    pub dir: Vec3,
    pub speed: f32,
    pub life: f32,
    pub damage: f32,
    pub team: u8,
    /// The unit that fired the projectile, so kills and camp aggro can be
    /// attributed correctly.
    pub attacker: Option<usize>,
    /// Look from `fx::LOOKS`.
    pub look: usize,
    /// Homing basic attack: steers onto this unit and can hit nothing else.
    pub target: Option<usize>,
    /// Straight-line distance at launch; drives the lob of `arc` looks.
    pub flight: f32,
    pub travelled: f32,
    pub age: f32,
    /// Recent drawn positions, newest first, for the trail ribbon.
    pub trail: Vec<Vec3>,
    pub ember_timer: f32,
    /// Piercing shots fly on and can hit each unit only once.
    pub pierce: bool,
    pub hit: Vec<usize>,
    /// Venom damage per tick applied to whatever the shot hits.
    pub venom: f32,
    /// Extra damage against targets under the execute threshold.
    pub execute: f32,
    /// A battle spell bolt's statuses and graphics, landed on its target.
    pub spell: Option<Box<battle::SpellHit>>,
}

impl Projectile {
    /// A straight skillshot with no extras; set the other fields after.
    fn new(
        pos: Vec3,
        dir: Vec3,
        speed: f32,
        life: f32,
        damage: f32,
        team: u8,
        look: usize,
    ) -> Self {
        Self {
            pos,
            dir,
            speed,
            life,
            damage,
            team,
            attacker: None,
            look,
            target: None,
            flight: speed * life,
            travelled: 0.0,
            age: 0.0,
            trail: Vec::new(),
            ember_timer: 0.0,
            pierce: false,
            hit: Vec::new(),
            venom: 0.0,
            execute: 0.0,
            spell: None,
        }
    }

    /// A homing basic attack that cannot miss `target`.
    #[allow(clippy::too_many_arguments)]
    fn homing(
        from: Vec3,
        target: usize,
        target_pos: Vec3,
        speed: f32,
        damage: f32,
        team: u8,
        attacker: Option<usize>,
        look: usize,
    ) -> Self {
        let delta = target_pos - from;
        let mut shot = Self::new(
            from,
            delta.normalize_or_zero(),
            speed,
            8.0,
            damage,
            team,
            look,
        );
        shot.target = Some(target);
        shot.attacker = attacker;
        shot.flight = delta.length().max(1.0);
        shot
    }

    /// Where the shot is drawn: the path position lifted by its lob.
    pub fn draw_pos(&self) -> Vec3 {
        let arc = fx::look(self.look).arc;
        if arc <= 0.0 {
            return self.pos;
        }
        self.pos + Vec3::Y * fx::arc_offset(arc, self.travelled / self.flight.max(1.0))
    }
}

struct MinionTemplate {
    model: Arc<Model>,
    idle: Arc<AnimationClip>,
    walk: Arc<AnimationClip>,
    attack: Option<Arc<AnimationClip>>,
    health: f32,
    range: f32,
    damage: f32,
    interval: f32,
    speed: f32,
    scale: f32,
    /// Extra yaw for a model whose forward axis breaks the cache
    /// convention (see `CACHE_MODEL_FORWARD`). Zero for both minions.
    model_yaw: f32,
}

pub struct AnimatedScene {
    pub units: Vec<AnimatedUnit>,
    pub player_index: usize,
    pub vertices: Vec<SceneVertex>,
    pub indices: Vec<u32>,
    pub caster_vertices: Vec<SceneVertex>,
    pub caster_indices: Vec<u32>,
    pub shadow_vertices: Vec<SceneVertex>,
    pub shadow_indices: Vec<u32>,
    pub marker: Option<(Vec3, f32)>,
    pub projectiles: Vec<Projectile>,
    /// Additive effects rebuilt every frame (glows, trails, indicators).
    pub fx: FxMesh,
    /// Cache spot animations used by projectile looks, keyed by id.
    spot_models: HashMap<u32, Arc<SpotModel>>,
    /// Plain models flown by projectile looks (`fx::ProjectileLook::model`).
    look_models: HashMap<u32, Arc<Model>>,
    /// Impact splashes playing once.
    pub spot_bursts: Vec<SpotBurst>,
    /// Melee hit streaks.
    pub slashes: Vec<Slash>,
    /// Ability slot most recently cast, for the held range indicator.
    pub last_cast_slot: Option<usize>,
    /// Cursor ground point while the indicator is held (right mouse).
    pub indicator_aim: Option<Vec3>,
    /// Brief range flash after a cast: slot and seconds left.
    cast_flash: Option<(usize, f32)>,
    /// King Awowogei's bananas, vine, peels and Greegree form.
    pub awowogei: awowogei::AwowogeiState,
    /// The champion's named cache sounds, and sounds waiting to be played.
    sounds: std::collections::BTreeMap<String, crate::champion::SoundSpec>,
    /// Shared combat and interface sounds (`data/sounds.json`).
    default_sounds: crate::champion::SoundTable,
    /// The player champion's unarmed body parts, kept so a weapon can be
    /// merged on and swapped later.
    player_body: Vec<Model>,
    /// True when the champion animates on the player skeleton and can hold
    /// item wield models.
    pub player_can_wield: bool,
    /// Cache item id of the weapon in the player's hand.
    player_weapon: Option<u32>,
    /// The player's basic-attack release sound, by combat class.
    player_release_sound: &'static str,
    pub sound_cues: Vec<crate::audio::SoundCue>,
    rng_state: u32,
    pub effects: Vec<Effect>,
    /// Billboard sparks emitted by abilities, impacts and deaths.
    pub particles: Vec<Particle>,
    /// Ground stains that fade over several seconds.
    pub decals: Vec<Decal>,
    /// Telegraphed ground impacts waiting to land.
    pub pending_impacts: Vec<PendingImpact>,
    /// Floating damage numbers currently in the world.
    pub floats: Vec<FloatText>,
    /// Centre-screen announcement, e.g. "YOU HAVE SLAIN A MINION!".
    pub announcement: Option<(String, f32)>,
    /// Keeps the announcement on screen instead of replacing it with kills.
    pub announcement_pinned: bool,
    /// Recent kill messages, newest last.
    pub kill_feed: Vec<(String, f32)>,
    /// Minimap ping on the last enemy the player hit.
    pub ping: Option<(Vec3, f32)>,
    /// The teleport scroll's numbers (`data/spells/teleport_scroll.json`).
    pub scroll_config: crate::scroll::ScrollConfig,
    /// Teleport destinations on the minimap: point, team, seconds left.
    pub scroll_pings: Vec<(Vec3, u8, f32)>,
    /// The battle spells magic heroes pick from
    /// (`data/spells/battle_spells.json`).
    pub spell_book: crate::spells::SpellBook,
    /// True while Vengeance's own damage is being dealt, so it cannot set
    /// off another.
    reflecting: bool,
    pub wave_timer: f32,
    /// Monotonic wave counter used to stagger formation jitter deterministically.
    pub wave_serial: u32,
    /// Team scoreboard counters.
    pub kills: [u32; 2],
    pub deaths: [u32; 2],
    /// The local player's own match line, shown in the scoreboard strip.
    pub player_kills: u32,
    pub player_deaths: u32,
    pub player_assists: u32,
    /// Creep score: enemy lane minions last-hit by the player.
    pub player_cs: u32,
    /// Damage the player has taken recently: source, amount, seconds left.
    /// Used by the death overlay's breakdown.
    recent_damage: Vec<(String, f32, f32)>,
    pub wave_interval: f32,
    pub gold_income: f32,
    pub lanes: Vec<Vec<Vec3>>,
    pub nexus_positions: [Vec3; 2],
    pub player_spawn: Vec3,
    pub match_over: Option<u8>,
    pub minimap_bounds: (glam::Vec2, glam::Vec2),
    pub visibility: VisibilityGrid,
    pub visibility_timer: f32,
    /// Last frame's CPU split, for the benchmark phase log.
    pub last_shadow_ms: f32,
    pub last_mesh_ms: f32,
    pub last_mesh_units: usize,
    /// Base palette RGB per model, so face colours are not re-converted from
    /// HSL on every frame.
    /// Keyed by model address and face count: a re-armed body can reuse a
    /// freed model's address, and must not inherit its colours.
    face_rgb_cache: HashMap<(usize, usize), Vec<[u8; 3]>>,
    pub visibility_dirty: bool,
    pub brush: Vec<(f32, f32, f32, f32)>,
    /// Biome pockets the map is built from, used for the HUD label.
    pub biomes: Vec<crate::scene::BiomePocket>,
    /// Items bought away from base, waiting in the chest.
    pub delivery_pending: Vec<(usize, ItemConfig)>,
    pub delivery_state: DeliveryState,
    delivery_guard: Option<usize>,
    delivery_chest: Option<usize>,
    delivery_home: Vec3,
    delivered_queue: Vec<usize>,
    pub vision_origin: (i32, i32),
    pub team_buffs: [Option<TeamBuff>; 2],
    /// Bot heroes' minds (one per bot champion unit).
    pub bots: Vec<bots::BotBrain>,
    /// When each team next ganks or takes its region boss.
    pub team_plans: [bots::TeamPlan; 2],
    /// Seconds to the next bot status line in the log.
    pub bot_status_timer: f32,
    pub champion_name: String,
    /// File stem from `data/champions`; gates champion-specific kit pieces.
    champion_id: String,
    /// Data-driven kit, when the champion file has one.
    kit: Option<crate::kit::Kit>,
    /// Effects in flight from every kit caster, and their summon NPCs.
    kit_world: kit_runtime::KitWorld,
    /// Per-caster kit state, by unit index (the player and kit bots).
    casters: HashMap<usize, kit_runtime::KitRuntime>,
    /// Thrall NPCs the shop's scrolls can raise, loaded at start.
    thrall_assets: HashMap<u32, kit_runtime::NpcAssets>,
    /// The player's living thrall, if any (units are never removed, so the
    /// index stays good).
    thrall: Option<usize>,
    /// The champion file's Q, W, E and R entries: names, summaries and the
    /// hand-authored numbers per rank.
    ability_info: Vec<crate::champion::ChampionAbility>,
    /// Current rank of Q, W, E and R (every ability starts at rank 1).
    ability_ranks: [u8; 4],
    /// Unspent skill points: one per level gained.
    pub skill_points: u32,
    /// Talent levels reached whose talent has not been picked yet.
    pub talent_picks_pending: u32,
    /// The talent picked at each talent level: 0 = left, 1 = right.
    pub talents: [Option<u8>; 4],
    /// Spare points turned into stat bumps (no rank left to buy).
    pub stat_bumps: u32,
    /// Each champion unit's talent tiers (the player and bots), by index.
    unit_talents: HashMap<usize, Vec<[crate::talents::Talent; 2]>>,
    /// Seconds since the scene started, for interface shimmer.
    pub hud_time: f32,
    /// Cursor ground point for kit abilities whose entry point takes none.
    pub kit_aim: Vec3,
    /// Direction requested by hold-to-steer this frame. Consumed by
    /// `update_player` so both movement modes share the same weighted model.
    steer_wish: Option<Vec3>,
    melee_template: MinionTemplate,
    ranged_template: MinionTemplate,
}

impl AnimatedScene {
    pub fn new(cache: &Cache, scene: &SceneData, champion: &ChampionConfig) -> Result<Self> {
        let mut assets = UnitAssets::new(cache);
        let mut units = Vec::new();

        // The scattered critters are weak neutral jungle camps: they aggro
        // when a hero walks into their patch instead of standing there as
        // decoration.
        for spawn in &scene.npc_spawns {
            let mut unit = load_unit(
                &mut assets,
                spawn.model_id,
                Some(spawn.idle_sequence),
                spawn.walk_sequence,
                spawn.pos,
                spawn.scale,
                UnitKind::Camp,
                2,
                150.0,
                150.0,
                9.0,
                1.7,
                260.0,
                160.0,
                260.0,
                Vec::new(),
            )?;
            unit.respawn_time = 60.0;
            unit.home = spawn.pos;
            unit.aggro_range = 420.0;
            unit.leash_range = 760.0;
            unit.camp_buff = None;
            tracing::info!("critter camp at {:?}", spawn.pos);
            units.push(unit);
        }

        // Non-combat scenery creatures. They use team 2 so every targeting
        // path treats them as neutral, and the health-bar pass explicitly
        // skips UnitKind::Ambient.
        for spawn in &scene.ambient_spawns {
            let speed = match spawn.kind {
                AmbientKind::Butterfly => 92.0,
                AmbientKind::Cow => 46.0,
                AmbientKind::Crab => 26.0,
            };
            let mut unit = load_unit(
                &mut assets,
                spawn.model_id,
                Some(spawn.idle_sequence),
                Some(spawn.walk_sequence),
                spawn.pos,
                spawn.scale,
                UnitKind::Ambient,
                2,
                1.0,
                0.0,
                0.0,
                0.0,
                speed,
                0.0,
                0.0,
                Vec::new(),
            )?;
            if !spawn.attachments.is_empty() {
                let parts = assets.merged_parts(spawn.model_id, &spawn.attachments)?;
                unit.model = Arc::new(bind_parts(&parts, &unit_clips(&unit), "ambient"));
            }
            for &(find, replace) in &spawn.recolors {
                Arc::make_mut(&mut unit.model).recolor(&[find], &[replace]);
            }
            unit.home = spawn.pos;
            unit.ambient_kind = Some(spawn.kind);
            unit.ambient_seed = spawn.pos.x.to_bits()
                ^ spawn.pos.z.to_bits().rotate_left(13)
                ^ (units.len() as u32).wrapping_mul(0x9E37_79B9);
            unit.anim_time_ms = (unit.ambient_seed % 4_000) as f32;
            units.push(unit);
        }

        let player_index = units.len();
        units.push(load_unit(
            &mut assets,
            champion.model,
            Some(champion.idle),
            Some(champion.walk),
            scene.player_spawn,
            champion.scale,
            UnitKind::Player,
            0,
            champion.health,
            champion.attack_range,
            champion.damage,
            champion.attack_interval,
            champion.speed,
            240.0,
            380.0,
            Vec::new(),
        )?);
        let player_body = assets.merged_parts(champion.model, &champion.attachments)?;
        let player_can_wield = crate::wield::can_wield(&units[player_index].idle);
        {
            if !champion.attachments.is_empty() {
                units[player_index].attack_clip = champion_clips(&mut assets, champion).0;
                let clips = unit_clips(&units[player_index]);
                units[player_index].model =
                    Arc::new(bind_parts(&player_body, &clips, &champion.name));
            }
            let player = &mut units[player_index];
            player.mana = champion.mana;
            player.max_mana = champion.mana;
            player.level = 1;
            player.ranged_auto = champion.ranged_auto;
            player.ranged = champion.ranged;
            player.model_yaw_offset = champion.model_yaw;
            player.armor = champion.armor;
            player.walk_arm = champion.walk_arm;
            player.attack_windup = champion.attack_windup;
            player.attack_hit = champion.attack_hit;
            player.look = champion_look(champion);
            player.q_damage = champion.q_damage;
            player.r_damage = champion.r_damage;
            player.e_heal = champion.e_heal;
            player.e_shield = champion.e_shield;
            player.w_distance = champion.w_distance;
        }
        if champion.walk_sway < 1.0 {
            if let Some(walk) = units[player_index].walk.as_mut() {
                Arc::make_mut(walk).damp_side_rotation(champion.walk_sway);
            }
        }
        let walk_loop_ms = units[player_index]
            .walk
            .as_ref()
            .map_or(1000.0, |walk| walk.loop_ms());
        let walk_stride = if champion.stride > 0.0 {
            champion.stride
        } else {
            champion.speed * walk_loop_ms / 1000.0
        };
        units[player_index].walk_loop_ms = walk_loop_ms;
        units[player_index].walk_stride = walk_stride;
        units[player_index].motion = Some(PlayerMotion::new(champion, walk_loop_ms));
        let (attack_clip, cast_clip) = champion_clips(&mut assets, champion);
        units[player_index].attack_clip = attack_clip;
        units[player_index].cast_clip = cast_clip;

        // A training dummy next to the spawn keeps combat testable without
        // waiting for the first wave to meet in the middle.
        let dummy_world = scene.player_spawn + Vec3::new(4.0 * 128.0, 0.0, 0.0);
        if let Some((tx, ty)) = scene
            .collision
            .nearest_walkable_world(dummy_world.x, dummy_world.z)
        {
            let dummy_pos = scene.collision.world_of_tile(tx, ty, &scene.height);
            units.push(load_unit(
                &mut assets,
                7744,
                Some(222),
                Some(219),
                dummy_pos,
                2.4,
                UnitKind::Dummy,
                1,
                120.0,
                0.0,
                0.0,
                0.0,
                0.0,
                130.0,
                300.0,
                Vec::new(),
            )?);
        }

        for structure in &scene.map.structures {
            tracing::info!(
                "structure: {:?} team {} model {} pos {:?}",
                structure.kind,
                structure.team,
                structure.model_id,
                structure.pos
            );
            let kind = match structure.kind {
                StructureKind::Tower => UnitKind::Tower,
                StructureKind::Inhibitor => UnitKind::Inhibitor,
                StructureKind::Nexus => UnitKind::Nexus,
                StructureKind::Watchtower => UnitKind::Watchtower,
            };
            let (hit_radius, bar_height) = match kind {
                UnitKind::Tower => (230.0, 500.0),
                UnitKind::Inhibitor => (230.0, 430.0),
                UnitKind::Nexus => (340.0, 575.0),
                UnitKind::Watchtower => (210.0, 720.0),
                _ => (150.0, 200.0),
            };
            let mut unit = load_unit(
                &mut assets,
                structure.model_id,
                None,
                None,
                structure.pos,
                structure.scale,
                kind,
                structure.team,
                structure.health,
                structure.range,
                structure.damage,
                structure.interval,
                0.0,
                hit_radius,
                bar_height,
                Vec::new(),
            )?;
            unit.structure_lane = structure.lane_index;
            if kind == UnitKind::Watchtower {
                // The lock range is the long one, the aggro radius is where
                // the tower first picks a target up.
                unit.lock_range = structure.aggro.max(1.0);
            }
            units.push(unit);
        }

        for camp in &scene.map.camps {
            tracing::info!(
                "camp: {} model {} pos {:?}",
                camp.id,
                camp.model_id,
                camp.pos
            );
            let mut unit = load_unit(
                &mut assets,
                camp.model_id,
                camp.idle_sequence,
                camp.walk_sequence,
                camp.pos,
                camp.scale,
                UnitKind::Camp,
                2,
                camp.health,
                camp.range,
                camp.damage,
                camp.interval,
                camp.speed,
                180.0,
                350.0,
                Vec::new(),
            )?;
            unit.respawn_time = camp.respawn;
            unit.camp_buff = Some(camp.buff);
            unit.camp_boss = camp.boss;
            unit.camp_name = camp.name.clone();
            if camp.range > 300.0 {
                unit.look = fx::look_by_name("earth_blast");
            }
            unit.home = camp.pos;
            unit.aggro_range = camp.aggro;
            unit.leash_range = camp.leash;
            units.push(unit);
        }

        // Storage chest and the flying Armadylean courier at the team base.
        let chest_hint = scene.player_spawn + Vec3::new(4.0 * 128.0, 0.0, 13.0 * 128.0);
        let chest_pos = Vec3::new(
            chest_hint.x,
            scene.height.height_at(chest_hint.x, chest_hint.z),
            chest_hint.z,
        );
        let chest_index = units.len();
        units.push(load_unit(
            &mut assets,
            1249,
            None,
            None,
            chest_pos,
            1.3,
            UnitKind::StorageChest,
            0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            90.0,
            180.0,
            Vec::new(),
        )?);
        tracing::info!("delivery: chest at {:?}", chest_pos);
        let guard_home = chest_pos + Vec3::new(2.6 * 128.0, 0.0, 0.0);
        let guard_index = units.len();
        units.push(load_unit(
            &mut assets,
            28019,
            Some(7166),
            Some(7167),
            guard_home,
            0.85,
            UnitKind::DeliveryGuard,
            0,
            1.0,
            0.0,
            0.0,
            0.0,
            1100.0,
            80.0,
            240.0,
            Vec::new(),
        )?);

        let nexus_positions = [
            find_nexus(&units, 0).unwrap_or(scene.player_spawn),
            find_nexus(&units, 1).unwrap_or(scene.player_spawn),
        ];

        // OSRS imp (melee) and fairy (ranged). The smaller imp models need a
        // larger scale to read at MOBA camera distance; fairies are taller.
        let melee_template = MinionTemplate {
            model: assets.model(19982)?,
            idle: assets.clip(171)?,
            walk: assets.clip(168)?,
            // Imp sword swing.
            attack: assets.clip(169).ok(),
            health: 90.0,
            range: 100.0,
            damage: 7.0,
            interval: 1.0,
            speed: 400.0,
            scale: 3.6,
            model_yaw: 0.0,
        };
        let ranged_template = MinionTemplate {
            model: assets.model(2857)?,
            idle: assets.clip(112)?,
            walk: assets.clip(106)?,
            // Fairy wand flick.
            attack: assets.clip(110).ok(),
            health: 65.0,
            range: 380.0,
            damage: 9.0,
            interval: 1.3,
            speed: 380.0,
            scale: 1.15,
            model_yaw: 0.0,
        };

        let mut min = glam::Vec2::splat(f32::MAX);
        let mut max = glam::Vec2::splat(f32::MIN);
        for lane in &scene.map.lanes {
            for p in &lane.waypoints {
                min = min.min(glam::Vec2::new(p.x, p.z));
                max = max.max(glam::Vec2::new(p.x, p.z));
            }
        }
        for unit in &units {
            if unit.kind.is_structure() || unit.kind == UnitKind::Camp {
                min = min.min(glam::Vec2::new(unit.pos.x, unit.pos.z));
                max = max.max(glam::Vec2::new(unit.pos.x, unit.pos.z));
            }
        }
        let pad = glam::Vec2::splat(3.0 * 128.0);
        let minimap_bounds = (min - pad, max + pad);

        let (vision_origin_x, vision_origin_y) = scene.collision.origin_tile();
        let visibility = VisibilityGrid {
            width: scene.collision.width() as usize,
            height: scene.collision.height() as usize,
            tiles: vec![false; (scene.collision.width() * scene.collision.height()) as usize],
        };
        let awowogei = if champion.id == "monkey" {
            awowogei::AwowogeiState::load(&mut assets)
        } else {
            awowogei::AwowogeiState::default()
        };

        // Projectile and impact models for every look. A missing spot
        // animation only costs that look its model; the glow still draws.
        let mut spot_models = HashMap::new();
        let scroll_config = crate::scroll::ScrollConfig::load();
        let spell_book = crate::spells::SpellBook::load();
        let kit_spots = champion.kit.as_ref().map(|kit| kit.spotanims());
        for id in kit_spots.iter().flatten() {
            if !crate::kit::spot::VERIFIED.contains(id) {
                tracing::warn!(
                    "kit: {} plays spot animation {id}, not yet verified in docs/SPELLS.md",
                    champion.name
                );
            }
        }
        let wanted = fx::spotanim_ids()
            .into_iter()
            .chain(crate::kit::spot::ALL)
            .chain(scroll_config.spotanims())
            .chain(spell_book.spotanims())
            .chain(std::iter::once(thrall::THRALL_RISE))
            .chain(kit_spots.into_iter().flatten());
        for id in wanted {
            if spot_models.contains_key(&id) {
                continue;
            }
            match SpotModel::load(cache, id) {
                Ok(spot) => {
                    spot_models.insert(id, Arc::new(spot));
                }
                Err(err) => tracing::warn!("fx: spotanim {id} unavailable: {err:#}"),
            }
        }

        let mut look_models = HashMap::new();
        for id in fx::model_ids() {
            match load_model(cache, id) {
                Ok(model) => {
                    look_models.insert(id, Arc::new(model));
                }
                Err(err) => tracing::warn!("fx: projectile model {id} unavailable: {err:#}"),
            }
        }

        let mut kit_world = kit_runtime::KitWorld::default();
        let mut casters = HashMap::new();
        if let Some(kit) = &champion.kit {
            kit_world.npc_assets = kit_runtime::load_npcs(cache, &mut assets, &kit.npcs(), "kit");
            casters.insert(
                player_index,
                kit_runtime::KitRuntime::new(kit, &champion.abilities),
            );
        }
        let thrall_assets = kit_runtime::load_npcs(
            cache,
            &mut assets,
            &crate::items::shipped_thrall_npcs(),
            "thrall",
        );
        let mut animated = Self {
            units,
            player_index,
            vertices: Vec::new(),
            indices: Vec::new(),
            caster_vertices: Vec::new(),
            caster_indices: Vec::new(),
            shadow_vertices: Vec::new(),
            shadow_indices: Vec::new(),
            marker: None,
            projectiles: Vec::new(),
            fx: FxMesh::default(),
            spot_models,
            look_models,
            spot_bursts: Vec::new(),
            slashes: Vec::new(),
            last_cast_slot: None,
            indicator_aim: None,
            cast_flash: None,
            awowogei,
            sounds: champion.sounds.clone(),
            default_sounds: crate::champion::load_default_sounds(),
            player_body,
            player_can_wield,
            player_weapon: None,
            player_release_sound: if champion.class == "MAGIC" {
                "magic_release"
            } else {
                "ranged_release"
            },
            sound_cues: Vec::new(),
            rng_state: 0x9E37_79B9,
            effects: Vec::new(),
            particles: Vec::new(),
            decals: Vec::new(),
            pending_impacts: Vec::new(),
            floats: Vec::new(),
            announcement: None,
            announcement_pinned: false,
            kill_feed: Vec::new(),
            ping: None,
            scroll_config,
            scroll_pings: Vec::new(),
            spell_book,
            reflecting: false,
            wave_timer: 0.0,
            wave_serial: 0,
            kills: [0, 0],
            deaths: [0, 0],
            player_kills: 0,
            player_deaths: 0,
            player_assists: 0,
            player_cs: 0,
            recent_damage: Vec::new(),
            wave_interval: scene.map.wave_interval,
            gold_income: 0.0,
            lanes: scene
                .map
                .lanes
                .iter()
                .map(|lane| lane.waypoints.clone())
                .collect(),
            nexus_positions,
            player_spawn: scene.player_spawn,
            match_over: None,
            minimap_bounds,
            visibility,
            visibility_timer: 0.0,
            visibility_dirty: true,
            last_shadow_ms: 0.0,
            last_mesh_ms: 0.0,
            last_mesh_units: 0,
            face_rgb_cache: HashMap::new(),
            brush: scene.map.brush.clone(),
            biomes: scene.map.biomes.clone(),
            delivery_pending: Vec::new(),
            delivery_state: DeliveryState::Idle,
            delivery_guard: Some(guard_index),
            delivery_chest: Some(chest_index),
            delivery_home: guard_home,
            delivered_queue: Vec::new(),
            vision_origin: (vision_origin_x, vision_origin_y),
            team_buffs: [None, None],
            bots: Vec::new(),
            team_plans: [bots::TeamPlan::default(); 2],
            bot_status_timer: 30.0,
            champion_name: champion.name.clone(),
            champion_id: champion.id.clone(),
            kit: champion.kit.clone(),
            kit_world,
            casters,
            thrall_assets,
            thrall: None,
            ability_info: champion.abilities.clone(),
            ability_ranks: [0; 4],
            // Level 1's point picks the first ability.
            skill_points: 1,
            talent_picks_pending: 0,
            talents: [None; 4],
            stat_bumps: 0,
            unit_talents: HashMap::from([(player_index, champion.talents.clone())]),
            hud_time: 0.0,
            kit_aim: Vec3::ZERO,
            steer_wish: None,
            melee_template,
            ranged_template,
        };
        // Persistent fountain rings mark the heal/shop area at spawn.
        animated.effects.push(Effect {
            pos: scene.player_spawn,
            radius: 240.0,
            remaining: 1.0e9,
            duration: 1.0e9,
            color: [70, 210, 130],
        });
        animated.effects.push(Effect {
            pos: scene.player_spawn,
            radius: 140.0,
            remaining: 1.0e9,
            duration: 1.0e9,
            color: [120, 255, 170],
        });
        // Blue rings around each castle dais, matching the reference base.
        for pos in animated.nexus_positions {
            animated.effects.push(Effect {
                pos,
                radius: 760.0,
                remaining: 1.0e9,
                duration: 1.0e9,
                color: [80, 165, 255],
            });
        }
        for lane_index in 0..animated.lanes.len() {
            animated.spawn_wave(0, lane_index, &scene.height);
            animated.spawn_wave(1, lane_index, &scene.height);
        }
        animated.wave_timer = animated.wave_interval;
        Ok(animated)
    }

    /// Dev hook: kill the local player so the respawn HUD can be inspected.
    pub fn demo_kill_player(&mut self) {
        let pi = self.player_index;
        let player = &mut self.units[pi];
        player.health = 0.0;
        player.dead_timer = PLAYER_RESPAWN;
        player.path.clear();
        player.path_index = 0;
        player.attack_pending = false;
        player.attack_target = None;
        self.announcement = Some(("YOU HAVE BEEN SLAIN".to_string(), 3.0));
    }

    /// Pop the reference client's combat feedback: floating numbers, a kill
    /// announcement, a kill feed and a minimap ping. Used by `--demo-fx`.
    pub fn demo_fx(&mut self) {
        let pi = self.player_index;
        self.floats.clear();
        let pos = self.units[pi].pos;
        // Three red damage splats, one per column slot, like a combo in the
        // reference client.
        for (stack, text) in ["46", "32", "38"].iter().enumerate() {
            self.floats.push(FloatText {
                pos: pos + Vec3::Y * (self.units[pi].bar_height + 30.0),
                text: text.to_string(),
                style: FloatStyle::Hitsplat(HitsplatKind::Damage),
                life: 9999.0,
                scale: 1.0,
                stack: stack as u8,
                unit: Some(pi),
                tint: None,
            });
        }
        self.floats.push(FloatText {
            pos: pos + Vec3::Y * 150.0,
            text: "1,502".to_string(),
            style: FloatStyle::Label([255, 225, 120]),
            life: 9999.0,
            scale: 1.7,
            stack: 0,
            unit: None,
            tint: None,
        });
        self.announcement = Some(("YOU HAVE SLAIN A MINION!".to_string(), 9999.0));
        self.announcement_pinned = true;
        self.kill_feed
            .push(("YOU SLAIN A MINION".to_string(), 9999.0));
        self.kill_feed
            .push(("BLOWPIPE RANGER HAS SLAIN A MINION".to_string(), 9999.0));
        self.ping = Some((pos + Vec3::new(600.0, 0.0, 300.0), 9999.0));
        // Also show the attack range and target rings.
        let pi = self.player_index;
        let mut best: Option<(usize, f32)> = None;
        for (index, unit) in self.units.iter().enumerate() {
            if index == pi || unit.team != 1 || unit.health <= 0.0 {
                continue;
            }
            let distance = unit.pos.distance(pos);
            if best.is_none_or(|(_, best)| distance < best) {
                best = Some((index, distance));
            }
        }
        if let Some((index, _)) = best {
            self.units[pi].attack_target = Some(index);
        }
    }

    /// Pin one hitmark of every OSRS kind over the hero. Used by
    /// `--demo-hitsplats` to check the cache sprites, the bold font and the
    /// column stacking in one frame.
    pub fn demo_hitsplats(&mut self) {
        let pi = self.player_index;
        self.floats.clear();
        let pos = self.units[pi].pos + Vec3::Y * (self.units[pi].bar_height + 30.0);
        let examples = [
            ("78", HitsplatKind::Damage),
            ("99", HitsplatKind::MaxHit),
            ("8", HitsplatKind::Poison),
            ("20", HitsplatKind::Venom),
            ("14", HitsplatKind::Disease),
            ("25", HitsplatKind::Heal),
            ("0", HitsplatKind::Miss),
        ];
        for (stack, (text, kind)) in examples.iter().enumerate() {
            self.floats.push(FloatText {
                pos,
                text: text.to_string(),
                style: FloatStyle::Hitsplat(*kind),
                life: 9999.0,
                scale: 1.0,
                stack: stack as u8,
                unit: Some(pi),
                tint: None,
            });
        }
    }

    pub fn champion_name(&self) -> &str {
        &self.champion_name
    }

    pub fn player_pos(&self) -> Vec3 {
        self.units[self.player_index].pos
    }

    pub fn player_stats(&self) -> PlayerStats {
        let p = &self.units[self.player_index];
        PlayerStats {
            health: p.health,
            max_health: p.max_health,
            mana: p.mana,
            max_mana: p.max_mana,
            level: p.level,
            xp: p.xp,
            xp_next: xp_for_level(p.level),
            gold: p.gold,
            cooldowns: p.cooldowns,
            shield: p.shield,
            shield_timer: p.shield_timer,
            attack_damage: p.attack_damage,
            armor: p.armor,
            ranged: p.ranged,
            attack_interval: p.attack_interval,
            cooldown_maxima: {
                let fallback = if let Some(kit) = &self.kit {
                    kit.cooldowns()
                } else if self.champion_id == "monkey" {
                    awowogei::COOLDOWNS
                } else {
                    [Q_COOLDOWN, 8.0, 12.0, 45.0]
                };
                std::array::from_fn(|slot| self.rank_cooldown(slot, fallback[slot]))
            },
            speed: p.speed,
            ability_power: p.ability_power,
            kills: self.player_kills,
            deaths: self.player_deaths,
            assists: self.player_assists,
            cs: self.player_cs,
        }
    }

    /// Sell an item from the bag for [`crate::items::sell_price`].
    pub fn sell_item(&mut self, item: &ItemConfig) -> Result<(), &'static str> {
        self.refund_item(item, crate::items::sell_price(item.cost))
    }

    /// Take an item out of the bag: its stats come off and `gold` is paid.
    /// Selling pays the sell price; Undo pays the full cost.
    pub fn refund_item(&mut self, item: &ItemConfig, gold: u32) -> Result<(), &'static str> {
        let pi = self.player_index;
        let player = &mut self.units[pi];
        if player.health <= 0.0 {
            return Err("dead");
        }
        player.gold = player.gold.saturating_add(gold);
        player.attack_damage = (player.attack_damage - item.damage).max(1.0);
        if item.health > 0.0 {
            player.max_health = (player.max_health - item.health).max(100.0);
            player.health = player.health.min(player.max_health);
        }
        if item.mana > 0.0 {
            player.max_mana = (player.max_mana - item.mana).max(50.0);
            player.mana = player.mana.min(player.max_mana);
        }
        player.speed = (player.speed - item.speed).max(100.0);
        player.ability_power = (player.ability_power - item.ability).max(0.0);
        let pos = player.pos;
        self.play_sound("shop_sell", pos);
        Ok(())
    }

    pub fn debug_grant_gold(&mut self, amount: u32) {
        let player = &mut self.units[self.player_index];
        player.gold = player.gold.saturating_add(amount);
    }

    pub fn debug_destroy_inhibitor(&mut self, team: u8, lane_index: usize, respawn: f32) -> bool {
        if let Some(unit) = self.units.iter_mut().find(|u| {
            u.kind == UnitKind::Inhibitor && u.team == team && u.structure_lane == Some(lane_index)
        }) {
            unit.health = 0.0;
            unit.dead_timer = respawn;
            unit.attack_target = None;
            true
        } else {
            false
        }
    }

    pub fn debug_inhibitor_state(&self, team: u8, lane_index: usize) -> Option<(f32, f32, f32)> {
        self.units
            .iter()
            .find(|u| {
                u.kind == UnitKind::Inhibitor
                    && u.team == team
                    && u.structure_lane == Some(lane_index)
            })
            .map(|u| (u.health, u.max_health, u.dead_timer))
    }

    pub fn super_minion_count(&self) -> usize {
        self.units
            .iter()
            .filter(|u| u.super_minion && u.health > 0.0)
            .count()
    }

    pub fn debug_refill_mana(&mut self) {
        let unit = &mut self.units[self.player_index];
        unit.mana = unit.max_mana;
    }

    /// Dev: give a unit this much health (current and maximum), so a demo
    /// target survives every ability aimed at it.
    pub fn debug_sturdy(&mut self, index: usize, health: f32) {
        if let Some(unit) = self.units.get_mut(index) {
            unit.max_health = health;
            unit.health = health;
        }
    }

    /// One line on what the kit did: hurt and crowd-controlled enemies,
    /// live summons, and the player's state.
    pub fn debug_kit_report(&self) -> String {
        let player = &self.units[self.player_index];
        let near: Vec<&AnimatedUnit> = self
            .units
            .iter()
            .filter(|u| u.team != 0 && u.team != 3 && u.pos.distance(player.pos) < 2200.0)
            .filter(|u| {
                matches!(
                    u.kind,
                    UnitKind::Camp
                        | UnitKind::MeleeMinion
                        | UnitKind::RangedMinion
                        | UnitKind::BotChampion
                )
            })
            .collect();
        let hurt = near.iter().filter(|u| u.health < u.max_health).count();
        let dead = near.iter().filter(|u| u.health <= 0.0).count();
        let cc = near
            .iter()
            .filter(|u| {
                u.stun_timer > 0.0
                    || u.cc.slow_timer > 0.0
                    || u.cc.root_timer > 0.0
                    || u.cc.fear_timer > 0.0
                    || u.cc.taunt_timer > 0.0
                    || u.venom_timer > 0.0
            })
            .count();
        let summons = self
            .units
            .iter()
            .filter(|u| u.cc.summoned && u.health > 0.0)
            .count();
        format!(
            "enemies near {} hurt {} dead {} afflicted {} | summons {} | hp {:.0}/{:.0} shield {:.0} range {:.0} scale {:.2} cds {:?}",
            near.len(),
            hurt,
            dead,
            cc,
            summons,
            player.health,
            player.max_health,
            player.shield,
            player.attack_range,
            player.scale,
            player.cooldowns
        )
    }

    pub fn debug_damage_player(&mut self, amount: f32) {
        let player = &mut self.units[self.player_index];
        player.health = (player.health - amount).max(1.0);
    }

    /// Buy an item at the fountain. The client applies stats locally for the
    /// prototype; the authoritative server will own this transaction later.
    /// Buy an item. At the fountain it goes straight into the bag; anywhere
    /// else it is stored in the chest and must be delivered by the guard.
    pub fn buy_item(
        &mut self,
        shop_index: usize,
        item: &ItemConfig,
    ) -> Result<BuyOutcome, &'static str> {
        let pi = self.player_index;
        let player = &mut self.units[pi];
        if player.health <= 0.0 {
            return Err("dead");
        }
        if player.gold < item.cost {
            return Err("not enough gold");
        }
        player.gold -= item.cost;
        let pos = player.pos;
        self.play_sound("shop_buy", pos);
        let player = &mut self.units[pi];
        let at_base = player.pos.distance(self.player_spawn) <= 900.0;
        if at_base {
            self.apply_item(item);
            Ok(BuyOutcome::Delivered)
        } else {
            self.delivery_pending.push((shop_index, item.clone()));
            Ok(BuyOutcome::Stored)
        }
    }

    /// Put an item's stats back on the player (Undo returning the parts an
    /// upgrade used up).
    pub fn grant_item(&mut self, item: &ItemConfig) {
        self.apply_item(item);
    }

    fn apply_item(&mut self, item: &ItemConfig) {
        let player = &mut self.units[self.player_index];
        player.attack_damage += item.damage;
        if item.health > 0.0 {
            player.max_health += item.health;
            player.health += item.health;
        }
        if item.mana > 0.0 {
            player.max_mana += item.mana;
            player.mana += item.mana;
        }
        player.speed += item.speed;
        player.ability_power += item.ability;
    }

    /// Undo a purchase still waiting in the chest: it leaves the delivery
    /// list and the full cost comes back. False when it is no longer there
    /// (already handed over).
    pub fn cancel_stored(&mut self, shop_index: usize) -> bool {
        let Some(pos) = self
            .delivery_pending
            .iter()
            .rposition(|(index, _)| *index == shop_index)
        else {
            return false;
        };
        let (_, item) = self.delivery_pending.remove(pos);
        let pi = self.player_index;
        self.units[pi].gold = self.units[pi].gold.saturating_add(item.cost);
        let pos = self.units[pi].pos;
        self.play_sound("shop_sell", pos);
        true
    }

    /// Send the guard to fetch the stored items.
    pub fn call_delivery(&mut self) -> bool {
        if self.delivery_pending.is_empty() || self.delivery_state != DeliveryState::Idle {
            return false;
        }
        self.delivery_state = DeliveryState::FlyingToHero;
        true
    }

    /// Shop indices of items the guard has just handed over.
    pub fn take_delivered(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.delivered_queue)
    }

    pub fn delivery_count(&self) -> usize {
        self.delivery_pending.len()
    }

    fn deliver_pending(&mut self) {
        let pending = std::mem::take(&mut self.delivery_pending);
        if pending.is_empty() {
            return;
        }
        for (index, item) in pending {
            self.apply_item(&item);
            self.delivered_queue.push(index);
        }
        let pos = self.player_pos();
        self.effects.push(Effect {
            pos,
            radius: 240.0,
            remaining: 1.2,
            duration: 1.2,
            color: [255, 220, 90],
        });
    }

    fn update_delivery(&mut self, dt: f32, height: &HeightGrid) {
        let Some(guard_index) = self.delivery_guard else {
            return;
        };
        let player_pos = self.player_pos();
        let home = self.delivery_home;
        match self.delivery_state {
            DeliveryState::Idle => {
                let ground = height.height_at(home.x, home.z);
                let guard = &mut self.units[guard_index];
                guard.pos = Vec3::new(home.x, ground + 190.0, home.z);
                guard.moving = false;
            }
            DeliveryState::FlyingToHero => {
                let target = Vec3::new(player_pos.x, player_pos.y + 150.0, player_pos.z);
                let guard_pos = self.units[guard_index].pos;
                let delta = target - guard_pos;
                let distance = delta.length();
                let step = 1100.0 * dt;
                if distance <= step.max(170.0) {
                    self.units[guard_index].pos = target;
                    self.units[guard_index].moving = false;
                    self.deliver_pending();
                    self.delivery_state = DeliveryState::Returning;
                } else {
                    let dir = delta / distance;
                    let guard = &mut self.units[guard_index];
                    guard.pos += dir * step;
                    guard.yaw = dir.x.atan2(dir.z);
                    guard.moving = true;
                }
                let guard = &mut self.units[guard_index];
                let floor = height.height_at(guard.pos.x, guard.pos.z) + 190.0;
                guard.pos.y = guard.pos.y.max(floor);
            }
            DeliveryState::Returning => {
                let ground = height.height_at(home.x, home.z);
                let target = Vec3::new(home.x, ground + 190.0, home.z);
                let guard_pos = self.units[guard_index].pos;
                let delta = target - guard_pos;
                let distance = delta.length();
                let step = 1100.0 * dt;
                if distance <= step.max(40.0) {
                    let guard = &mut self.units[guard_index];
                    guard.pos = target;
                    guard.moving = false;
                    self.delivery_state = DeliveryState::Idle;
                } else {
                    let dir = delta / distance;
                    let guard = &mut self.units[guard_index];
                    guard.pos += dir * step;
                    guard.yaw = dir.x.atan2(dir.z);
                    guard.moving = true;
                }
            }
        }
    }

    pub fn nexus_health(&self, team: u8) -> Option<(f32, f32)> {
        self.units
            .iter()
            .find(|u| u.kind == UnitKind::Nexus && u.team == team)
            .map(|u| (u.health, u.max_health))
    }

    /// Diagnostic snapshot used during lane-pathing investigations.
    pub fn debug_minion_states(&self) -> Vec<String> {
        self.units
            .iter()
            .filter(|unit| unit.kind.is_minion() && unit.health > 0.0)
            .map(|unit| {
                let dist = unit
                    .lane
                    .get(unit.lane_index)
                    .map(|waypoint| unit.pos.distance(*waypoint))
                    .unwrap_or(0.0);
                format!(
                    "t{} wp{}/{} pos({:.0},{:.0}) spd{:.0} d{:.0} moving={} target={:?} delay={:.1}",
                    unit.team,
                    unit.lane_index,
                    unit.lane.len(),
                    unit.pos.x / 128.0,
                    unit.pos.z / 128.0,
                    unit.speed,
                    dist,
                    unit.moving,
                    unit.attack_target,
                    unit.spawn_delay
                )
            })
            .collect()
    }

    pub fn minion_count(&self) -> usize {
        self.units
            .iter()
            .filter(|u| u.kind.is_minion() && u.health > 0.0)
            .count()
    }

    pub fn inhibitor_count(&self, team: u8) -> (usize, usize) {
        let all = self
            .units
            .iter()
            .filter(|u| u.kind == UnitKind::Inhibitor && u.team == team)
            .count();
        let alive = self
            .units
            .iter()
            .filter(|u| u.kind == UnitKind::Inhibitor && u.team == team && u.health > 0.0)
            .count();
        (alive, all)
    }

    /// Whether `team`'s nexus can be attacked. It always can: minions that
    /// reach the end of a lane, heroes and abilities all hit it (the user's
    /// call; an inhibitor shield left lanes stalled in front of it).
    /// Inhibitors still matter: a fallen one spawns super minions.
    pub fn nexus_vulnerable(&self, team: u8) -> bool {
        let _ = team;
        true
    }

    pub fn tower_healths(&self, team: u8) -> Vec<(f32, f32)> {
        self.units
            .iter()
            .filter(|u| u.kind == UnitKind::Tower && u.team == team)
            .map(|u| (u.health, u.max_health))
            .collect()
    }

    pub fn set_player_target(
        &mut self,
        target: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> usize {
        // Moving or attacking cancels a teleport scroll channel.
        self.cancel_scroll(self.player_index);
        self.steer_wish = None;
        let start = self.player_pos();
        let path = collision.path_world(start, target, height);
        let unit = &mut self.units[self.player_index];
        unit.path = path;
        unit.path_index = 0;
        unit.target = Some(target);
        unit.attack_target = None;
        unit.attack_pending = false;
        unit.attack_windup_timer = 0.0;
        self.marker = Some((target, 1.1));
        unit.path.len()
    }

    pub fn set_attack_target(
        &mut self,
        index: usize,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> bool {
        // Moving or attacking cancels a teleport scroll channel.
        self.cancel_scroll(self.player_index);
        if index >= self.units.len() {
            return false;
        }
        let target = &self.units[index];
        let targetable = target.team == 1 || (target.team == 2 && target.kind == UnitKind::Camp);
        if !targetable || target.health <= 0.0 || !target.visible {
            return false;
        }
        let target_pos = target.pos;
        self.steer_wish = None;
        let pi = self.player_index;
        let in_range = self.in_reach(pi, index, self.units[pi].attack_range);
        let player = &mut self.units[pi];
        player.attack_target = Some(index);
        player.attack_pending = false;
        player.attack_windup_timer = 0.0;
        player.target = Some(target_pos);
        if !in_range {
            player.path = collision.path_world(player.pos, target_pos, height);
            player.path_index = 0;
        } else {
            player.path.clear();
            player.path_index = 0;
        }
        self.marker = Some((target_pos, 1.1));
        true
    }

    /// Drop paths, steering and attack orders. Used by the animation demo
    /// and whenever the player should come to a clean stop.
    pub fn clear_player_orders(&mut self) {
        self.steer_wish = None;
        let unit = &mut self.units[self.player_index];
        unit.path.clear();
        unit.path_index = 0;
        unit.target = None;
        unit.attack_target = None;
        unit.attack_pending = false;
        unit.attack_windup_timer = 0.0;
    }

    pub fn steer_player(&mut self, direction: Vec3) {
        if direction.length_squared() > 1e-6 {
            self.cancel_scroll(self.player_index);
        }
        let flat = Vec3::new(direction.x, 0.0, direction.z);
        if flat.length_squared() < 1e-6 {
            self.steer_wish = None;
            return;
        }
        self.steer_wish = Some(flat.normalize());
        let unit = &mut self.units[self.player_index];
        unit.attack_pending = false;
        unit.attack_windup_timer = 0.0;
        unit.path.clear();
        unit.path_index = 0;
        unit.target = None;
        unit.attack_target = None;
        unit.attack_pending = false;
        unit.attack_windup_timer = 0.0;
    }

    /// Move the player with weight. Shared by tap-to-move, hold-to-steer and
    /// turning to attack so every mode feels the same.
    fn drive_player(
        &mut self,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
        desired: Option<Vec3>,
        top_speed: f32,
        speed_mult: f32,
    ) -> Option<crate::motion::StepResult> {
        let unit = &mut self.units[self.player_index];
        let mut motion = unit.motion.take()?;
        let result = motion.step(
            unit.pos, unit.yaw, desired, top_speed, speed_mult, dt, height, collision,
        );
        unit.pos = result.pos;
        unit.yaw = result.yaw;
        unit.moving = result.moving;
        unit.motion = Some(motion);
        Some(result)
    }

    /// Follow the player's A* path with the weighted motor. Waypoints are
    /// consumed at a radius instead of teleporting the champion onto them, so
    /// a heavy turn keeps its arc.
    fn drive_player_path(
        &mut self,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
        speed_mult: f32,
    ) {
        let pi = self.player_index;
        loop {
            if self.units[pi].path.is_empty() {
                return;
            }
            let index = self.units[pi].path_index;
            let waypoint = self.units[pi].path[index];
            let delta = planar_delta(self.units[pi].pos, waypoint);
            let distance = delta.length();
            if distance < 24.0 {
                self.units[pi].path_index += 1;
                if self.units[pi].path_index >= self.units[pi].path.len() {
                    self.units[pi].path.clear();
                    self.units[pi].path_index = 0;
                    self.units[pi].target = None;
                }
                continue;
            }
            let dir = delta / distance;
            let speed = self.units[pi].speed;
            self.drive_player(dt, height, collision, Some(dir), speed, speed_mult);
            return;
        }
    }

    fn player_land_attack(&mut self, target: usize) {
        if self.kit.is_some() {
            let pi = self.player_index;
            self.kit_land_attack(pi, target);
            return;
        }
        let pi = self.player_index;
        let unit = &mut self.units[pi];
        unit.attack_cooldown = unit.attack_interval / unit.attack_speed_mult.max(0.1);
        let mut damage = unit.attack_damage;
        if unit.ranged_auto {
            damage *= unit.ranged_scale();
        }
        let ranged = unit.ranged_auto;
        let from = unit.pos + Vec3::Y * (unit.scale * 80.0);
        let team = unit.team;
        let look = unit.look;
        let target_pos = self.unit_chest(target);
        if ranged {
            self.projectiles.push(Projectile::homing(
                from,
                target,
                target_pos,
                1400.0,
                damage,
                team,
                Some(pi),
                look,
            ));
            self.muzzle_flash(from, look);
            self.play_release_sound(pi, from);
        } else {
            self.melee_slash(pi, target, [255, 235, 200]);
            self.damage_unit_by(target, damage, 0, Some(pi));
            if pi == self.player_index {
                self.madness_attack(target);
            }
            if pi == self.player_index && self.awowogei.greegree.is_some() {
                self.greegree_slam(target);
            }
            if self.units[target].health <= 0.0 {
                self.units[pi].attack_target = None;
            }
        }
    }

    /// Planar centre distance minus the target's footprint: the gap an
    /// attack has to close. Height differences do not shorten reach.
    fn reach_gap(&self, attacker: usize, target: usize) -> f32 {
        let a = &self.units[attacker];
        let t = &self.units[target];
        planar_delta(a.pos, t.pos).length() - t.body_radius
    }

    /// True when `target`'s edge is within `range` of `attacker`'s centre.
    fn in_reach(&self, attacker: usize, target: usize, range: f32) -> bool {
        self.reach_gap(attacker, target) <= range
    }

    /// Where shots aim on a unit: about a third of the way up its bar.
    fn unit_chest(&self, index: usize) -> Vec3 {
        let unit = &self.units[index];
        let lift = if unit.kind.is_structure() {
            unit.bar_height * 0.45
        } else {
            (unit.bar_height * 0.30).clamp(40.0, 180.0)
        };
        unit.pos + Vec3::Y * lift
    }

    /// Launch point of a unit's ranged attack.
    fn unit_muzzle(&self, index: usize) -> Vec3 {
        let unit = &self.units[index];
        let forward = Vec3::new(unit.yaw.sin(), 0.0, unit.yaw.cos());
        let lift = match unit.kind {
            UnitKind::RangedMinion => 90.0,
            UnitKind::Camp => 140.0,
            _ => unit.scale * 80.0,
        };
        unit.pos + Vec3::Y * lift + forward * 30.0
    }

    /// A quick flash and a puff of embers where a projectile leaves.
    fn muzzle_flash(&mut self, pos: Vec3, look: usize) {
        let look = fx::look(look);
        self.effects.push(Effect {
            pos,
            radius: look.glow_size * 0.9,
            remaining: 0.16,
            duration: 0.16,
            color: look.glow,
        });
        let sparks = (look.impact_sparks / 3).max(2);
        self.emit_burst(
            pos,
            look.ember_color,
            sparks,
            160.0,
            look.glow_size * 0.16,
            0.3,
        );
    }

    /// A bright streak across `target` from `attacker`'s swing.
    fn melee_slash(&mut self, attacker: usize, target: usize, color: [u8; 3]) {
        let from = self.units[attacker].pos;
        let at = self.unit_chest(target);
        let dir = planar_delta(from, at).normalize_or_zero();
        let size = (self.units[target].body_radius * 1.6).clamp(70.0, 260.0);
        self.slashes.push(Slash {
            pos: at,
            dir,
            color,
            size,
            remaining: 0.22,
            duration: 0.22,
        });
        if self.slashes.len() > 48 {
            self.slashes.remove(0);
        }
        self.emit_burst(at, color, 6, 220.0, 10.0, 0.3);
    }

    /// Play a one-shot clip on a unit. `windup` is when the clip's hit
    /// moment should land (0 plays it at its own speed).
    fn start_action(&mut self, index: usize, cast: bool, windup: f32) {
        let unit = &mut self.units[index];
        let clip = if cast {
            unit.cast_clip.clone()
        } else {
            unit.attack_clip.clone()
        };
        let Some(clip) = clip else {
            return;
        };
        let rate = if cast {
            // Casts are snappy: no slower than real time, at most ~0.6 s.
            (clip.loop_ms() / 600.0).max(1.0)
        } else {
            action_rate(&clip, unit.attack_hit, windup)
        };
        unit.action = Some(ActionAnim::new(clip, rate));
    }

    /// Start blending a unit's one-shot clip out.
    fn cancel_action(&mut self, index: usize) {
        if let Some(action) = self.units[index].action.as_mut() {
            action.cancelling = true;
        }
    }

    /// Start a computer-controlled attack: the attack clip plays and the
    /// hit lands on its swing. A unit without a clip hits at once.
    fn begin_strike(&mut self, index: usize, target: usize, damage: f32, ranged: bool) {
        let unit = &mut self.units[index];
        // Disarmed (the Chaos Elemental's Madness): no basic attacks.
        if unit.cc.disarmed() {
            unit.attack_cooldown = unit.attack_cooldown.max(0.2);
            return;
        }
        let interval = unit.attack_interval / unit.attack_speed_mult.max(0.1);
        unit.attack_cooldown = interval;
        let strike = PendingStrike {
            target,
            timer: 0.0,
            damage,
            ranged,
        };
        match unit.attack_clip.clone() {
            Some(clip) => {
                let natural = clip.loop_ms() * unit.attack_hit.clamp(0.05, 1.0) / 1000.0;
                let windup = natural.min(interval * 0.55).max(0.08);
                unit.action = Some(ActionAnim::new(
                    clip.clone(),
                    action_rate(&clip, unit.attack_hit, windup),
                ));
                unit.strike = Some(PendingStrike {
                    timer: windup,
                    ..strike
                });
            }
            None => self.release_strike(index, strike),
        }
    }

    /// Count down every wind-up and land the ones that are due. A strike
    /// whose attacker or target died is dropped.
    fn update_strikes(&mut self, dt: f32) {
        for index in 0..self.units.len() {
            let Some(mut strike) = self.units[index].strike else {
                continue;
            };
            let attacker_ok = self.units[index].health > 0.0 && self.units[index].stun_timer <= 0.0;
            let target_ok = self
                .units
                .get(strike.target)
                .is_some_and(|unit| unit.health > 0.0);
            if !attacker_ok || !target_ok {
                self.units[index].strike = None;
                continue;
            }
            strike.timer -= dt;
            if strike.timer > 0.0 {
                self.units[index].strike = Some(strike);
                continue;
            }
            self.units[index].strike = None;
            self.release_strike(index, strike);
        }
    }

    fn release_strike(&mut self, index: usize, strike: PendingStrike) {
        // Kit heroes' attacks carry their passives and empowers.
        if index != self.player_index && self.casters.contains_key(&index) {
            let cooldown = self.units[index].attack_cooldown;
            self.kit_land_attack(index, strike.target);
            self.units[index].attack_cooldown = cooldown;
            return;
        }
        let team = self.units[index].team;
        if strike.ranged {
            let from = self.unit_muzzle(index);
            let target_pos = self.unit_chest(strike.target);
            let speed = match self.units[index].kind {
                UnitKind::RangedMinion => 950.0,
                UnitKind::Camp => 850.0,
                _ => 1400.0,
            };
            let look = self.units[index].look;
            self.projectiles.push(Projectile::homing(
                from,
                strike.target,
                target_pos,
                speed,
                strike.damage,
                team,
                Some(index),
                look,
            ));
            self.muzzle_flash(from, look);
            self.play_release_sound(index, from);
        } else {
            let color = match team {
                0 => [200, 225, 255],
                1 => [255, 200, 190],
                _ => [255, 230, 170],
            };
            self.melee_slash(index, strike.target, color);
            self.damage_unit_by(strike.target, strike.damage, team, Some(index));
        }
    }

    /// Cast the Q slot. King Awowogei throws his Bananarang, the Blowpipe
    /// Ranger looses a poisoned dart, kits run their data, and everyone else
    /// keeps the shared arcane bolt aimed at the cursor.
    pub fn cast_q(&mut self, target: Vec3) -> bool {
        let ok = self.ability_rank(0) > 0 && self.cast_q_inner(target);
        self.after_cast(0, ok);
        ok
    }

    pub fn cast_w(
        &mut self,
        target: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> bool {
        let ok = self.ability_rank(1) > 0 && self.cast_w_inner(target, collision, height);
        self.after_cast(1, ok);
        ok
    }

    pub fn cast_e(&mut self, collision: &WorldCollision, height: &HeightGrid) -> bool {
        let ok = self.ability_rank(2) > 0 && self.cast_e_inner(collision, height);
        self.after_cast(2, ok);
        ok
    }

    /// Cast the R slot at the cursor (King Awowogei's Greegree ignores it).
    pub fn cast_r(&mut self, target: Vec3) -> bool {
        let ok = self.ability_rank(3) > 0 && self.cast_r_inner(target);
        self.after_cast(3, ok);
        ok
    }

    /// A successful cast plays the cast clip, remembers the slot for the
    /// held indicator and flashes its range briefly.
    fn after_cast(&mut self, slot: usize, ok: bool) {
        if !ok {
            return;
        }
        let pi = self.player_index;
        // Casting cancels a teleport scroll channel.
        self.cancel_scroll(pi);
        self.last_cast_slot = Some(slot);
        self.cast_flash = Some((slot, 0.45));
        // A cast interrupts a basic attack that has not landed yet.
        self.units[pi].attack_pending = false;
        self.units[pi].attack_windup_timer = 0.0;
        self.start_action(pi, true, 0.0);
        // Kits score their casts per action (`kit_*` sounds) instead.
        if self.kit.is_none() && !self.slot_has_own_sounds(slot) {
            let pos = self.units[pi].pos;
            self.play_sound("cast", pos);
        }
    }

    /// Show the held range indicator for the last ability at `aim`, or hide
    /// it with `None`.
    pub fn set_indicator_aim(&mut self, aim: Option<Vec3>) {
        self.indicator_aim = aim;
    }

    /// Ground shapes that describe an ability slot's reach.
    pub fn ability_indicators(&self, slot: usize) -> Vec<Indicator> {
        let reach = self.rank_reach_mult(slot);
        let mut shapes = self.base_ability_indicators(slot);
        if (reach - 1.0).abs() > 1e-3 {
            for shape in &mut shapes {
                shape.scale_reach(reach);
            }
        }
        shapes
    }

    /// The slot's shapes at rank 1.
    fn base_ability_indicators(&self, slot: usize) -> Vec<Indicator> {
        if let Some(kit) = self.kit.as_ref() {
            return kit
                .abilities
                .get(slot)
                .map(|ability| {
                    kit_indicators(&ability.actions, self.units[self.player_index].attack_range)
                })
                .unwrap_or_default();
        }
        let player = &self.units[self.player_index];
        match (self.champion_id.as_str(), slot) {
            ("monkey", slot) => awowogei::indicators(slot),
            ("ranger", 0) => vec![Indicator::Line {
                length: 1500.0 * 1.2,
                width: 70.0,
                angle: 0.0,
            }],
            ("ranger", 1) => (-2..=2)
                .map(|i| Indicator::Line {
                    length: 1400.0 * 1.1,
                    width: 60.0,
                    angle: i as f32 * 0.22,
                })
                .collect(),
            ("ranger", 2) => vec![Indicator::Dash {
                length: 512.0,
                backwards: true,
                to_cursor: false,
            }],
            ("ranger", 3) => vec![Indicator::Line {
                length: 1800.0 * 1.6,
                width: 90.0,
                angle: 0.0,
            }],
            (_, 0) => vec![Indicator::Line {
                length: 950.0 * 1.4,
                width: 90.0,
                angle: 0.0,
            }],
            (_, 1) => vec![Indicator::Dash {
                length: player.w_distance,
                backwards: false,
                to_cursor: true,
            }],
            (_, 2) => vec![Indicator::SelfArea { radius: 160.0 }],
            _ => vec![Indicator::Target {
                range: METEOR_RANGE,
                radius: 384.0,
            }],
        }
    }

    /// Colour of an ability slot's indicator.
    fn ability_color(&self, slot: usize) -> [u8; 3] {
        self.kit
            .as_ref()
            .and_then(|kit| kit.abilities.get(slot))
            .map_or([120, 200, 255], |ability| ability.color)
    }

    fn next_random(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng_state = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    /// Emit a deterministic burst of billboard sparks. Kept under a hard cap
    /// so a teamfight cannot grow the cosmetic buffer without bound.
    fn emit_burst(
        &mut self,
        pos: Vec3,
        color: [u8; 3],
        count: usize,
        speed: f32,
        size: f32,
        life: f32,
    ) {
        for _ in 0..count {
            if self.particles.len() >= 320 {
                break;
            }
            let angle = self.next_random() * std::f32::consts::TAU;
            let radial = speed * (0.30 + self.next_random() * 0.70);
            let upward = speed * (0.25 + self.next_random() * 0.95);
            let particle_size = size * (0.55 + self.next_random() * 0.85);
            let duration = life * (0.55 + self.next_random() * 0.75);
            self.particles.push(Particle {
                pos: pos + Vec3::Y * (size * 0.35),
                vel: Vec3::new(angle.cos() * radial, upward, angle.sin() * radial),
                color,
                size: particle_size,
                remaining: duration,
                duration,
                gravity: -520.0,
                drag: 1.4,
            });
        }
    }

    /// Queue the champion's sound `name` at `pos`. Unnamed sounds are
    /// silently skipped, so kits can ask for sounds a file does not list.
    /// Unnamed sounds fall back to the shared table in `data/sounds.json`.
    pub(crate) fn play_sound(&mut self, name: &str, pos: Vec3) {
        let spec = self
            .sounds
            .get(name)
            .or_else(|| self.default_sounds.get(name))
            .copied();
        if let Some(spec) = spec {
            self.queue_sound(spec, pos);
        }
    }

    /// Queue a sound from the shared table only: for units other than the
    /// player's champion, whose own file must not change their sounds.
    fn play_shared_sound(&mut self, name: &str, pos: Vec3) {
        if let Some(spec) = self.default_sounds.get(name).copied() {
            self.queue_sound(spec, pos);
        }
    }

    /// One copy of a sound per frame: an area spell striking five minions,
    /// or a kit hit sound landing with the shared hit sound, plays once.
    fn queue_sound(&mut self, spec: crate::champion::SoundSpec, pos: Vec3) {
        if self.sound_cues.len() >= 32 || self.sound_cues.iter().any(|cue| cue.id == spec.id) {
            return;
        }
        self.sound_cues.push(crate::audio::SoundCue {
            id: spec.id,
            pos,
            volume: spec.volume,
            speed: spec.speed,
        });
    }

    /// True when the champion file names any sound for ability `slot`
    /// (`q_*`, `w_*`, ...), so the shared cast whoosh stays out of a kit
    /// that scores its own casts.
    pub(crate) fn slot_has_own_sounds(&self, slot: usize) -> bool {
        let prefix = ["q_", "w_", "e_", "r_"][slot.min(3)];
        self.sounds.keys().any(|name| name.starts_with(prefix))
    }

    /// A champion's basic attack left its hand: a whoosh for the shot.
    /// Hidden enemies stay silent so sound never leaks through fog.
    fn play_release_sound(&mut self, index: usize, from: Vec3) {
        if index == self.player_index {
            let name = self.player_release_sound;
            self.play_sound(name, from);
        } else if self.units[index].is_champion && self.units[index].visible {
            self.play_shared_sound("ranged_release", from);
        }
    }

    /// Every sound id the champion file and the shared table name, plus the
    /// frame sounds of every loaded unit clip, for preloading.
    pub fn sound_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .sounds
            .values()
            .chain(
                self.default_sounds
                    .iter()
                    .filter(|(name, _)| name.as_str() != "frame_sound")
                    .map(|(_, spec)| spec),
            )
            .map(|spec| spec.id)
            .collect();
        for unit in &self.units {
            let clips = [
                Some(&unit.idle),
                unit.walk.as_ref(),
                unit.attack_clip.as_ref(),
                unit.cast_clip.as_ref(),
            ];
            for clip in clips.into_iter().flatten() {
                ids.extend(clip.sequence.sounds.iter().map(|sound| sound.id as u32));
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn add_decal(&mut self, pos: Vec3, radius: f32, color: [u8; 3], duration: f32) {
        if self.decals.len() >= 48 {
            self.decals.remove(0);
        }
        self.decals.push(Decal {
            pos,
            radius,
            remaining: duration,
            duration,
            color,
        });
    }

    fn cast_q_inner(&mut self, target: Vec3) -> bool {
        if self.kit.is_some() {
            return self.cast_kit(0, target);
        }
        match self.champion_id.as_str() {
            "monkey" => self.cast_bananarang(target),
            "ranger" => self.cast_toxic_dart(target),
            _ => self.cast_arcane_bolt(target),
        }
    }

    /// Blowpipe Ranger Q: a fast poisoned dart that stacks venom.
    fn cast_toxic_dart(&mut self, target: Vec3) -> bool {
        let pi = self.player_index;
        let (rank_damage, rank_reach, rank_cooldown) = (
            self.hand_damage_mult(0),
            self.rank_reach_mult(0),
            self.rank_cooldown(0, 5.0),
        );
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[0] > 0.0
            || player.mana < 30.0
        {
            return false;
        }
        let flat = Vec3::new(target.x - player.pos.x, 0.0, target.z - player.pos.z);
        if flat.length_squared() < 1.0 {
            return false;
        }
        let dir = flat.normalize();
        let damage = player.q_damage
            * player.ranged_scale()
            * (1.0 + player.ability_power / 100.0)
            * rank_damage;
        let pos = player.pos + Vec3::Y * (player.scale * 80.0) + dir * 40.0;
        let team = player.team;
        player.mana -= 30.0;
        player.cooldowns[0] = rank_cooldown;
        let mut shot = Projectile::new(
            pos,
            dir,
            1500.0,
            1.2 * rank_reach,
            damage,
            team,
            fx::look_by_name("dart"),
        );
        shot.attacker = Some(pi);
        shot.venom = 4.0;
        self.projectiles.push(shot);
        self.emit_burst(pos, [120, 255, 120], 8, 190.0, 14.0, 0.4);
        true
    }

    /// Blowpipe Ranger W: a fan of darts that poisons everything it hits.
    fn cast_dart_volley(&mut self, target: Vec3) -> bool {
        let pi = self.player_index;
        let (rank_damage, rank_reach, rank_cooldown) = (
            self.hand_damage_mult(1),
            self.rank_reach_mult(1),
            self.rank_cooldown(1, 12.0),
        );
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[1] > 0.0
            || player.mana < 50.0
        {
            return false;
        }
        let flat = Vec3::new(target.x - player.pos.x, 0.0, target.z - player.pos.z);
        if flat.length_squared() < 1.0 {
            return false;
        }
        let dir = flat.normalize();
        let damage = player.q_damage
            * 0.55
            * player.ranged_scale()
            * (1.0 + player.ability_power / 100.0)
            * rank_damage;
        let team = player.team;
        let origin = player.pos + Vec3::Y * (player.scale * 80.0);
        player.mana -= 50.0;
        player.cooldowns[1] = rank_cooldown;
        for index in -2..=2 {
            let angle = index as f32 * 0.22;
            let (sin, cos) = angle.sin_cos();
            let rotated = Vec3::new(dir.x * cos - dir.z * sin, 0.0, dir.x * sin + dir.z * cos);
            let mut shot = Projectile::new(
                origin + rotated * 40.0,
                rotated,
                1400.0,
                1.1 * rank_reach,
                damage,
                team,
                fx::look_by_name("dart"),
            );
            shot.attacker = Some(pi);
            shot.venom = 3.0;
            self.projectiles.push(shot);
        }
        true
    }

    /// Blowpipe Ranger E: a backwards hop that speeds up attacks, the way an
    /// OSRS ranger kites with a blowpipe.
    fn cast_footwork(&mut self, collision: &WorldCollision, height: &HeightGrid) -> bool {
        let pi = self.player_index;
        let (rank_damage, rank_reach, rank_cooldown) = (
            self.hand_damage_mult(2),
            self.rank_reach_mult(2),
            self.rank_cooldown(2, 14.0),
        );
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[2] > 0.0
            || player.mana < 25.0
        {
            return false;
        }
        let _ = (rank_damage, rank_reach);
        let forward = Vec3::new(player.yaw.sin(), 0.0, player.yaw.cos());
        let mut destination = player.pos;
        for step in (1..=4).rev() {
            let p = player.pos - forward * (step as f32 * 128.0);
            if collision.is_walkable_world(p.x, p.z) {
                destination = Vec3::new(p.x, height.height_at(p.x, p.z), p.z);
                break;
            }
        }
        player.pos = destination;
        player.attack_speed_mult = 1.4;
        player.attack_speed_timer = 5.0;
        player.mana -= 25.0;
        player.cooldowns[2] = rank_cooldown;
        player.path.clear();
        player.path_index = 0;
        let pos = player.pos;
        self.effects.push(Effect {
            pos,
            radius: 140.0,
            remaining: 0.35,
            duration: 0.35,
            color: [120, 255, 170],
        });
        self.emit_burst(pos, [140, 255, 170], 16, 260.0, 18.0, 0.55);
        self.add_decal(pos, 90.0, [40, 82, 52], 1.6);
        true
    }

    /// Blowpipe Ranger R: a piercing shot that executes wounded targets.
    fn cast_deadshot(&mut self, target: Vec3) -> bool {
        let pi = self.player_index;
        let (rank_damage, rank_reach, rank_cooldown) = (
            self.hand_damage_mult(3),
            self.rank_reach_mult(3),
            self.rank_cooldown(3, 75.0),
        );
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[3] > 0.0
            || player.mana < 80.0
        {
            return false;
        }
        let flat = Vec3::new(target.x - player.pos.x, 0.0, target.z - player.pos.z);
        if flat.length_squared() < 1.0 {
            return false;
        }
        let dir = flat.normalize();
        let damage = player.r_damage
            * player.ranged_scale()
            * (1.0 + player.ability_power / 100.0)
            * rank_damage;
        let pos = player.pos + Vec3::Y * (player.scale * 80.0) + dir * 40.0;
        let team = player.team;
        player.mana -= 80.0;
        player.cooldowns[3] = rank_cooldown;
        let mut shot = Projectile::new(
            pos,
            dir,
            1800.0,
            1.6 * rank_reach,
            damage,
            team,
            fx::look_by_name("blood"),
        );
        shot.attacker = Some(pi);
        shot.pierce = true;
        shot.venom = 2.0;
        shot.execute = 0.35;
        self.projectiles.push(shot);
        self.emit_burst(pos, [255, 90, 70], 14, 260.0, 16.0, 0.5);
        true
    }

    fn cast_arcane_bolt(&mut self, target: Vec3) -> bool {
        let pi = self.player_index;
        let (rank_damage, rank_reach, rank_cooldown) = (
            self.hand_damage_mult(0),
            self.rank_reach_mult(0),
            self.rank_cooldown(0, Q_COOLDOWN),
        );
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[0] > 0.0
            || player.mana < 25.0
        {
            return false;
        }
        let flat = Vec3::new(target.x - player.pos.x, 0.0, target.z - player.pos.z);
        if flat.length_squared() < 1.0 {
            return false;
        }
        let dir = flat.normalize();
        let pos = player.pos + Vec3::Y * (player.scale * 80.0) + dir * 40.0;
        let team = player.team;
        player.mana -= 25.0;
        player.cooldowns[0] = rank_cooldown;
        let damage = player.q_damage * (1.0 + player.ability_power / 100.0) * rank_damage;
        self.projectiles.push(Projectile::new(
            pos,
            dir,
            950.0,
            1.4 * rank_reach,
            damage,
            team,
            fx::look_by_name("fire_blast"),
        ));
        self.effects.push(Effect {
            pos,
            radius: 110.0,
            remaining: 0.22,
            duration: 0.22,
            color: [255, 120, 40],
        });
        self.emit_burst(pos, [255, 150, 60], 10, 210.0, 15.0, 0.4);
        true
    }

    fn cast_w_inner(
        &mut self,
        target: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> bool {
        if self.kit.is_some() {
            return self.cast_kit(1, target);
        }
        if self.champion_id == "ranger" {
            return self.cast_dart_volley(target);
        }
        if self.champion_id == "monkey" {
            return self.cast_vine_swing(target, collision, height);
        }
        let (rank_reach, rank_cooldown) = (self.rank_reach_mult(1), self.rank_cooldown(1, 8.0));
        let pi = self.player_index;
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[1] > 0.0
            || player.mana < 30.0
        {
            return false;
        }
        let flat = Vec3::new(target.x - player.pos.x, 0.0, target.z - player.pos.z);
        if flat.length_squared() < 1.0 {
            return false;
        }
        let dir = flat.normalize();
        let start = player.pos;
        let mut destination = start;
        let distance = player.w_distance * rank_reach;
        let steps = (distance / 64.0).ceil().max(1.0) as i32;
        for step in (1..=steps).rev() {
            let p = start + dir * (step as f32 * 64.0);
            if collision.is_walkable_world(p.x, p.z) {
                destination = Vec3::new(p.x, height.height_at(p.x, p.z), p.z);
                break;
            }
        }
        if destination.distance(start) < 8.0 {
            return false;
        }
        player.pos = destination;
        player.mana -= 30.0;
        player.cooldowns[1] = rank_cooldown;
        player.path.clear();
        player.path_index = 0;
        self.effects.push(Effect {
            pos: start,
            radius: 140.0,
            remaining: 0.35,
            duration: 0.35,
            color: [90, 220, 255],
        });
        self.effects.push(Effect {
            pos: destination,
            radius: 170.0,
            remaining: 0.42,
            duration: 0.42,
            color: [120, 235, 255],
        });
        self.emit_burst(start, [120, 225, 255], 18, 300.0, 18.0, 0.55);
        self.emit_burst(destination, [170, 245, 255], 20, 320.0, 18.0, 0.6);
        true
    }

    fn cast_e_inner(&mut self, collision: &WorldCollision, height: &HeightGrid) -> bool {
        if self.kit.is_some() {
            let target = self.kit_aim;
            return self.cast_kit(2, target);
        }
        if self.champion_id == "ranger" {
            return self.cast_footwork(collision, height);
        }
        if self.champion_id == "monkey" {
            let target = self.kit_aim;
            return self.cast_banana_peel(target, height);
        }
        let (rank_damage, rank_cooldown) = (self.hand_damage_mult(2), self.rank_cooldown(2, 12.0));
        let pi = self.player_index;
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[2] > 0.0
            || player.mana < 40.0
        {
            return false;
        }
        player.mana -= 40.0;
        player.cooldowns[2] = rank_cooldown;
        let ability_mult = (1.0 + player.ability_power / 100.0) * rank_damage;
        let before = player.health;
        player.health = (player.health + player.e_heal * ability_mult).min(player.max_health);
        let healed = player.health - before;
        player.shield = player.e_shield * ability_mult;
        player.shield_timer = 4.0;
        let pos = player.pos;
        if healed > 0.5 {
            self.push_hitsplat(pi, healed, HitsplatKind::Heal, None);
        }
        self.effects.push(Effect {
            pos,
            radius: 160.0,
            remaining: 0.5,
            duration: 0.5,
            color: [90, 255, 140],
        });
        self.emit_burst(pos, [120, 255, 160], 26, 170.0, 16.0, 0.9);
        self.add_decal(pos, 130.0, [36, 90, 52], 2.6);
        true
    }

    fn cast_r_inner(&mut self, target: Vec3) -> bool {
        if self.kit.is_some() {
            return self.cast_kit(3, target);
        }
        match self.champion_id.as_str() {
            "monkey" => self.cast_greegree(),
            "ranger" => self.cast_deadshot(target),
            _ => self.cast_meteor(target),
        }
    }

    fn cast_meteor(&mut self, target: Vec3) -> bool {
        let pi = self.player_index;
        let (rank_damage, rank_reach, rank_cooldown) = (
            self.hand_damage_mult(3),
            self.rank_reach_mult(3),
            self.rank_cooldown(3, 45.0),
        );
        let player = &mut self.units[pi];
        if player.health <= 0.0
            || player.stun_timer > 0.0
            || player.cooldowns[3] > 0.0
            || player.mana < 80.0
        {
            return false;
        }
        let offset = planar_delta(player.pos, target);
        let reach = METEOR_RANGE * rank_reach;
        let target = if offset.length() > reach {
            let p = player.pos + offset.normalize_or_zero() * reach;
            Vec3::new(p.x, target.y, p.z)
        } else {
            target
        };
        player.mana -= 80.0;
        player.cooldowns[3] = rank_cooldown;
        let radius = 384.0;
        let damage = player.r_damage * (1.0 + player.ability_power / 100.0) * rank_damage;
        // Telegraph first, damage later. The rock is visible falling for the
        // whole delay, so a mobile target can walk out of the circle.
        self.pending_impacts.push(PendingImpact {
            pos: target,
            radius,
            delay: 0.9,
            duration: 0.9,
            damage,
            team: 0,
            color: [255, 110, 50],
        });
        self.effects.push(Effect {
            pos: target,
            radius: radius * 0.35,
            remaining: 0.3,
            duration: 0.3,
            color: [255, 190, 80],
        });
        true
    }

    pub fn pick_enemy_champion(&self, origin: Vec3, dir: Vec3) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if !unit.is_champion
                || unit.team == self.units[self.player_index].team
                || unit.health <= 0.0
                || !unit.visible
            {
                continue;
            }
            let center = unit.pos + Vec3::Y * (unit.bar_height * 0.45);
            let radius = unit.hit_radius.max(80.0);
            let oc = center - origin;
            let t = oc.dot(dir);
            if t < 0.0 {
                continue;
            }
            let closest = origin + dir * t;
            if closest.distance(center) <= radius && best.is_none_or(|(_, best_t)| t < best_t) {
                best = Some((i, t));
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn pick_unit(&self, origin: Vec3, dir: Vec3) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if !(unit.team == 1 || (unit.team == 2 && unit.kind == UnitKind::Camp))
                || unit.health <= 0.0
                || !unit.visible
            {
                continue;
            }
            let center = unit.pos + Vec3::Y * (unit.bar_height * 0.45);
            let radius = unit.hit_radius;
            let oc = center - origin;
            let t = oc.dot(dir);
            if t < 0.0 {
                continue;
            }
            let closest = origin + dir * t;
            if closest.distance(center) <= radius && best.is_none_or(|(_, best_t)| t < best_t) {
                best = Some((i, t));
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn nearest_enemy(&self, pos: Vec3) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if unit.team != 1 || unit.health <= 0.0 {
                continue;
            }
            let d = unit.pos.distance_squared(pos);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Closest watchtower of the given team, used by the verification demo.
    pub fn nearest_watchtower(&self, team: u8, pos: Vec3) -> Option<usize> {
        self.nearest_of_kind(UnitKind::Watchtower, team, pos)
    }

    /// The nearest living lane tower of `team`.
    pub fn nearest_tower(&self, team: u8, pos: Vec3) -> Option<usize> {
        self.nearest_of_kind(UnitKind::Tower, team, pos)
    }

    fn nearest_of_kind(&self, kind: UnitKind, team: u8, pos: Vec3) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if unit.kind != kind || unit.team != team || unit.health <= 0.0 {
                continue;
            }
            let d = unit.pos.distance_squared(pos);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    pub fn unit_attack_target(&self, index: usize) -> Option<usize> {
        self.units.get(index).and_then(|unit| unit.attack_target)
    }

    pub fn nearest_camp(&self, pos: Vec3) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if unit.kind != UnitKind::Camp || unit.health <= 0.0 {
                continue;
            }
            let d = unit.pos.distance_squared(pos);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Spawn a sparring enemy champion, used by `--demo-duel`.
    pub fn spawn_enemy_champion(
        &mut self,
        cache: &Cache,
        champion: &ChampionConfig,
        pos: Vec3,
    ) -> Result<usize> {
        self.spawn_champion_unit(cache, champion, 1, pos)
    }

    /// Spawn a bot-controlled champion for `team` at `pos`.
    pub fn spawn_champion_unit(
        &mut self,
        cache: &Cache,
        champion: &ChampionConfig,
        team: u8,
        pos: Vec3,
    ) -> Result<usize> {
        let mut assets = UnitAssets::new(cache);
        let mut unit = load_unit(
            &mut assets,
            champion.model,
            Some(champion.idle),
            Some(champion.walk),
            pos,
            champion.scale,
            UnitKind::BotChampion,
            team,
            champion.health,
            champion.attack_range,
            champion.damage,
            champion.attack_interval,
            champion.speed,
            240.0,
            380.0,
            Vec::new(),
        )?;
        if !champion.attachments.is_empty() {
            let parts = assets.merged_parts(champion.model, &champion.attachments)?;
            unit.attack_clip = champion_clips(&mut assets, champion).0;
            unit.model = Arc::new(bind_parts(&parts, &unit_clips(&unit), &champion.name));
        }
        unit.armor = champion.armor;
        unit.is_champion = true;
        unit.home = pos;
        unit.level = 1;
        // Bots cast from the champion's mana pool (`bot_cast`).
        unit.max_mana = champion.mana;
        unit.mana = champion.mana;
        unit.model_yaw_offset = champion.model_yaw;
        unit.walk_arm = champion.walk_arm;
        unit.attack_windup = champion.attack_windup;
        unit.attack_hit = champion.attack_hit;
        unit.ranged_auto = champion.ranged_auto;
        unit.look = champion_look(champion);
        let (attack_clip, cast_clip) = champion_clips(&mut assets, champion);
        unit.attack_clip = attack_clip;
        unit.cast_clip = cast_clip;
        if champion.walk_sway < 1.0 {
            if let Some(walk) = unit.walk.as_mut() {
                Arc::make_mut(walk).damp_side_rotation(champion.walk_sway);
            }
        }
        self.units.push(unit);
        let index = self.units.len() - 1;
        self.unit_talents.insert(index, champion.talents.clone());
        if let Some(kit) = &champion.kit {
            self.add_kit_caster(cache, &mut assets, index, kit, &champion.abilities);
        }
        Ok(index)
    }

    /// OSRS overhead chat over a unit ("Taste vengeance!").
    pub fn say(&mut self, unit: usize, text: &str) {
        let Some(speaker) = self.units.get(unit) else {
            return;
        };
        let pos = say_anchor(speaker);
        // One line per speaker: a new line replaces the old one.
        self.floats
            .retain(|float| !(float.style == FloatStyle::Say && float.unit == Some(unit)));
        self.floats.push(FloatText {
            pos,
            text: text.to_string(),
            style: FloatStyle::Say,
            life: SAY_SECONDS,
            scale: 1.0,
            stack: 0,
            unit: Some(unit),
            tint: None,
        });
    }

    /// Play a spell graphic once riding `unit` (a cast on the caster, a hit
    /// on a target). Ids that did not load are skipped.
    pub(crate) fn gfx_on(&mut self, id: u32, unit: usize, scale: f32) {
        let mut burst = SpotBurst::new(id, SpotDraw::Spell);
        burst.follow = Some(unit);
        burst.scale = scale;
        self.push_spot(burst);
    }

    /// Play a spell graphic once standing at `pos` (an area landing).
    pub(crate) fn gfx_at(&mut self, id: u32, pos: Vec3, scale: f32) {
        let mut burst = SpotBurst::new(id, SpotDraw::Spell);
        burst.pos = pos;
        burst.scale = scale;
        self.push_spot(burst);
    }

    /// Hold a looping status graphic on `unit` for `seconds`. The same
    /// graphic already on that unit is extended instead of stacked.
    pub(crate) fn gfx_hold(&mut self, id: u32, unit: usize, seconds: f32, scale: f32) {
        if seconds <= 0.0 {
            return;
        }
        if let Some(held) = self.spot_bursts.iter_mut().find(|burst| {
            burst.mode == SpotDraw::Held && burst.follow == Some(unit) && burst.spotanim == id
        }) {
            held.hold = held.hold.max(seconds);
            return;
        }
        let mut burst = SpotBurst::new(id, SpotDraw::Held);
        burst.follow = Some(unit);
        burst.hold = seconds;
        burst.scale = scale;
        self.push_spot(burst);
    }

    /// Hold a looping graphic standing at `pos` for `seconds`, seen by both
    /// teams through the fog (a teleport's destination).
    pub(crate) fn gfx_hold_at(&mut self, id: u32, pos: Vec3, seconds: f32, scale: f32) {
        let mut burst = SpotBurst::new(id, SpotDraw::Held);
        burst.pos = pos;
        burst.hold = seconds;
        burst.scale = scale;
        burst.seen_by_all = true;
        self.push_spot(burst);
    }

    fn push_spot(&mut self, mut burst: SpotBurst) {
        if !self.spot_models.contains_key(&burst.spotanim)
            || self.spot_bursts.len() >= MAX_SPOT_BURSTS
        {
            return;
        }
        if let Some(unit) = burst.follow.and_then(|index| self.units.get(index)) {
            burst.yaw = unit.yaw;
        }
        self.spot_bursts.push(burst);
    }

    /// Load spot animations that no projectile look names, so a demo or a
    /// spell can play them. Ids already loaded are skipped.
    pub fn preload_spotanims(&mut self, cache: &Cache, ids: &[u32]) {
        for &id in ids {
            if self.spot_models.contains_key(&id) {
                continue;
            }
            match SpotModel::load(cache, id) {
                Ok(spot) => {
                    self.spot_models.insert(id, Arc::new(spot));
                }
                Err(err) => tracing::warn!("fx: spotanim {id} unavailable: {err:#}"),
            }
        }
    }

    /// Developer preview (`--demo-gfx`): play each spot animation once in a
    /// grid around `centre`, each labelled with its id. Returns how long the
    /// longest one runs, in seconds, so the caller can loop the grid.
    pub fn demo_gfx(&mut self, ids: &[u32], centre: Vec3, height: &HeightGrid) -> f32 {
        let length_of = |id: &u32| {
            self.spot_models
                .get(id)
                .map(|model| model.length_ms / 1000.0)
        };
        let longest = ids.iter().filter_map(length_of).fold(0.6f32, f32::max);
        let lengths: Vec<Option<f32>> = ids.iter().map(length_of).collect();
        for ((&id, length), spot) in
            ids.iter()
                .zip(lengths)
                .zip(demo_grid(centre, ids.len(), 230.0))
        {
            let ground = Vec3::new(spot.x, height.height_at(spot.x, spot.z), spot.z);
            if length.is_some() {
                // Drawn the way spells draw them: solid, standing on the
                // ground at their authored height.
                self.spot_bursts.push(SpotBurst {
                    spotanim: id,
                    pos: ground,
                    yaw: 0.0,
                    time_ms: 0.0,
                    scale: 1.0,
                    mode: SpotDraw::Spell,
                    follow: None,
                    hold: 0.0,
                    seen_by_all: false,
                });
            }
            // A missing id keeps its label, marked, so the sheet shows it.
            let label = if length.is_some() {
                id.to_string()
            } else {
                format!("{id}?")
            };
            self.floats.push(FloatText {
                pos: ground - Vec3::Y * 10.0,
                text: label,
                style: FloatStyle::Say,
                life: longest + 0.4,
                scale: 1.0,
                stack: 0,
                unit: None,
                tint: None,
            });
        }
        longest
    }

    pub fn teleport_player(&mut self, pos: Vec3) {
        let unit = &mut self.units[self.player_index];
        unit.pos = pos;
        unit.path.clear();
        unit.path_index = 0;
        unit.moving = false;
        unit.attack_pending = false;
        unit.attack_windup_timer = 0.0;
        if let Some(motion) = unit.motion.as_mut() {
            motion.reset();
        }
    }

    /// Name of the biome pocket the hero is standing in.
    pub fn current_biome(&self) -> Option<String> {
        let pos = self.player_pos();
        let tile_x = (pos.x / 128.0).floor() as i32;
        let tile_y = (pos.z / 128.0).floor() as i32;
        self.biomes
            .iter()
            .find(|biome| {
                tile_x >= biome.origin.0
                    && tile_y >= biome.origin.1
                    && tile_x < biome.origin.0 + biome.rect.2 as i32
                    && tile_y < biome.origin.1 + biome.rect.3 as i32
            })
            .map(|biome| biome.label.clone())
    }

    /// Aggregate the recent player damage by source, largest first.
    pub fn damage_sources(&self) -> Vec<(String, u32)> {
        damage_totals(
            &self
                .recent_damage
                .iter()
                .map(|(name, amount, _)| (name.clone(), *amount))
                .collect::<Vec<_>>(),
        )
    }

    fn record_damage(&mut self, label: &str, amount: f32) {
        if let Some((_, total, timer)) = self
            .recent_damage
            .iter_mut()
            .find(|(name, _, _)| name == label)
        {
            *total += amount;
            *timer = 10.0;
        } else {
            self.recent_damage.push((label.to_string(), amount, 10.0));
        }
    }

    fn damage_label(&self, attacker_team: u8, aggressor: Option<usize>) -> String {
        if let Some(index) = aggressor
            && let Some(unit) = self.units.get(index)
        {
            return match unit.kind {
                UnitKind::Player | UnitKind::BotChampion => "Rival champion",
                UnitKind::Tower => "Tower",
                UnitKind::Camp => "Camp",
                UnitKind::Watchtower => "Watchtower",
                UnitKind::MeleeMinion | UnitKind::RangedMinion => "Minion",
                _ => "The Rift",
            }
            .to_string();
        }
        match attacker_team {
            1 => "Enemy",
            2 => "Neutral",
            _ => "The Rift",
        }
        .to_string()
    }

    /// Team scoreboard: (kills, deaths) per team.
    pub fn team_scores(&self) -> ((u32, u32), (u32, u32)) {
        (
            (self.kills[0], self.deaths[0]),
            (self.kills[1], self.deaths[1]),
        )
    }

    /// Display name, level and a Dota style flavour line for a unit, used by
    /// the hover tooltip.
    pub fn unit_info(&self, index: usize) -> Option<(String, u32, &'static str)> {
        let unit = self.units.get(index)?;
        let name = match unit.kind {
            UnitKind::Player => self.champion_name.clone(),
            UnitKind::Camp if unit.camp_name.is_some() => {
                unit.camp_name.as_deref().unwrap_or_default().to_uppercase()
            }
            UnitKind::Camp => {
                let buff = match unit.camp_buff {
                    Some(CampBuff::Might) => "MIGHT",
                    Some(CampBuff::Swift) => "SWIFT",
                    None => "NEUTRAL",
                };
                format!("{buff} JUNGLE CREATURE")
            }
            UnitKind::Tower => {
                if unit.team == 0 {
                    "BLUE TOWER".to_string()
                } else {
                    "RED TOWER".to_string()
                }
            }
            UnitKind::Inhibitor => {
                if unit.team == 0 {
                    "BLUE INHIBITOR".to_string()
                } else {
                    "RED INHIBITOR".to_string()
                }
            }
            UnitKind::Nexus => {
                if unit.team == 0 {
                    "BLUE NEXUS".to_string()
                } else {
                    "RED NEXUS".to_string()
                }
            }
            UnitKind::MeleeMinion => "IMP".to_string(),
            UnitKind::RangedMinion => "FAIRY".to_string(),
            UnitKind::Dummy => "TRAINING DUMMY".to_string(),
            UnitKind::Ambient => "AMBIENT NPC".to_string(),
            UnitKind::Watchtower => {
                if unit.team == 0 {
                    "BLUE WATCHTOWER".to_string()
                } else {
                    "RED WATCHTOWER".to_string()
                }
            }
            UnitKind::BotChampion => self
                .bot_of(index)
                .map(|bot| bot.name.to_uppercase())
                .unwrap_or_else(|| "RIVAL CHAMPION".to_string()),
            UnitKind::StorageChest => "ITEM STORAGE".to_string(),
            UnitKind::DeliveryGuard => "ARMADYLEAN GUARD".to_string(),
        };
        let level = match unit.kind {
            UnitKind::Player | UnitKind::BotChampion => unit.level,
            UnitKind::Camp => 2,
            _ => 1,
        };
        let description = match unit.kind {
            UnitKind::Player => "YOUR CHAMPION. CLICK TO TARGET, HOLD TO STEER.",
            UnitKind::Camp => "CLEAR IT FOR GOLD, XP AND A TEAM BUFF.",
            UnitKind::Tower => "STANDS IN THE LANE. FALLS BEFORE THE NEXUS.",
            UnitKind::Inhibitor => "WHILE IT IS DOWN, ITS LANE SPAWNS SUPER MINIONS.",
            UnitKind::Nexus => "DESTROY IT TO END THE MATCH.",
            UnitKind::MeleeMinion | UnitKind::RangedMinion => {
                "LANE MINION. KILL IT FOR GOLD AND EXPERIENCE."
            }
            UnitKind::Dummy => "TRAINING DUMMY. PRACTISE YOUR COMBOS HERE.",
            UnitKind::Ambient => "A PASSER BY OF THE RIFT.",
            UnitKind::Watchtower => {
                "FIRES A SLOW BOLT AT THE FIRST ENEMY IN RANGE AND KEEPS FIRING UNTIL IT LEAVES."
            }
            UnitKind::BotChampion if unit.team == 0 => "AN ALLIED BOT HERO.",
            UnitKind::BotChampion => "AN ENEMY CHAMPION.",
            UnitKind::StorageChest => "REMOTE PURCHASES WAIT HERE. CALL THE GUARD TO DELIVER THEM.",
            UnitKind::DeliveryGuard => {
                "FLIES YOUR STORED ITEMS OUT TO YOU, THEN RETURNS TO THE CHEST."
            }
        };
        Some((name, level, description))
    }

    /// Health, max health, team and kind of a unit for the hover tooltip.
    pub fn unit_status(&self, index: usize) -> (f32, f32, u8, UnitKind) {
        match self.units.get(index) {
            Some(unit) => (unit.health, unit.max_health, unit.team, unit.kind),
            None => (0.0, 1.0, 2, UnitKind::Ambient),
        }
    }

    pub fn unit_pos(&self, index: usize) -> Option<Vec3> {
        self.units.get(index).map(|u| u.pos)
    }

    pub fn unit_home(&self, index: usize) -> Option<Vec3> {
        self.units.get(index).map(|u| u.home)
    }

    pub fn unit_camp_aggro(&self, index: usize) -> Option<bool> {
        self.units.get(index).map(|u| u.camp_aggro)
    }

    pub fn unit_health(&self, index: usize) -> Option<f32> {
        self.units.get(index).map(|u| u.health)
    }

    pub fn update(&mut self, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        // Frame sounds play for the span each clip advances this tick.
        let start_clocks: Vec<AnimClock> = self.units.iter().map(AnimClock::of).collect();
        self.hud_time += dt;
        // Floating numbers drift up and fade, announcements and the kill feed
        // time out, and the minimap ping decays.
        let units = &self.units;
        self.floats.retain_mut(|float| {
            float.life -= dt;
            // Hitmarks hang a little longer than economy labels, like the
            // reference client's one-to-two tick splats. Overhead chat
            // follows its speaker instead of drifting.
            let drift = match float.style {
                FloatStyle::Hitsplat(_) => 26.0,
                FloatStyle::Label(_) => 46.0,
                FloatStyle::Say => {
                    if let Some(unit) = float.unit.and_then(|index| units.get(index)) {
                        if unit.health <= 0.0 {
                            return false;
                        }
                        float.pos = say_anchor(unit);
                    }
                    0.0
                }
            };
            float.pos.y += dt * drift;
            float.life > 0.0
        });
        if let Some((_, timer)) = &mut self.announcement {
            *timer -= dt;
            if *timer <= 0.0 {
                self.announcement = None;
            }
        }
        self.kill_feed.retain_mut(|(_, timer)| {
            *timer -= dt;
            *timer > 0.0
        });
        self.recent_damage.retain_mut(|(_, _, timer)| {
            *timer -= dt;
            *timer > 0.0
        });
        if let Some((_, timer)) = &mut self.ping {
            *timer -= dt;
            if *timer <= 0.0 {
                self.ping = None;
            }
        }
        let pi = self.player_index;
        let player_spawn = self.player_spawn;
        {
            let player = &mut self.units[pi];
            for cd in &mut player.cooldowns {
                *cd = (*cd - dt).max(0.0);
            }
            player.mana = (player.mana + 8.0 * dt).min(player.max_mana);
            if player.health > 0.0 && player.pos.distance(player_spawn) <= 900.0 {
                player.health =
                    (player.health + player.max_health * 0.06 * dt).min(player.max_health);
                player.mana = (player.mana + player.max_mana * 0.06 * dt).min(player.max_mana);
            }
            if player.shield_timer > 0.0 {
                player.shield_timer -= dt;
                if player.shield_timer <= 0.0 {
                    player.shield = 0.0;
                }
            }
        }
        self.gold_income += 8.0 * dt;
        if self.gold_income >= 1.0 {
            let amount = self.gold_income.floor() as u32;
            self.gold_income -= amount as f32;
            let player = &mut self.units[pi];
            if player.health > 0.0 {
                player.gold = player.gold.saturating_add(amount);
            }
        }
        for effect in &mut self.effects {
            effect.remaining -= dt;
        }
        self.effects.retain(|effect| effect.remaining > 0.0);
        if self.effects.len() > 160 {
            let excess = self.effects.len() - 160;
            self.effects.drain(..excess);
        }
        for slash in &mut self.slashes {
            slash.remaining -= dt;
        }
        self.slashes.retain(|slash| slash.remaining > 0.0);
        let spot_models = &self.spot_models;
        let units = &self.units;
        self.spot_bursts.retain_mut(|burst| {
            burst.time_ms += dt * 1000.0;
            // A graphic riding a unit goes with it when it dies.
            if let Some(index) = burst.follow
                && units.get(index).is_none_or(|unit| unit.health <= 0.0)
            {
                return false;
            }
            if burst.mode == SpotDraw::Held {
                burst.hold -= dt;
                return burst.hold > 0.0;
            }
            spot_models
                .get(&burst.spotanim)
                .is_some_and(|spot| burst.time_ms < spot.length_ms)
        });
        if let Some((_, remaining)) = self.cast_flash.as_mut() {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.cast_flash = None;
            }
        }

        for particle in &mut self.particles {
            particle.remaining -= dt;
            particle.vel.y += particle.gravity * dt;
            particle.vel *= (1.0 - particle.drag * dt).max(0.0);
            particle.pos += particle.vel * dt;
            let floor = height.height_at(particle.pos.x, particle.pos.z) + 2.0;
            if particle.pos.y < floor {
                particle.pos.y = floor;
                particle.vel.y *= -0.25;
                particle.vel.x *= 0.55;
                particle.vel.z *= 0.55;
            }
        }
        self.particles.retain(|particle| particle.remaining > 0.0);

        for decal in &mut self.decals {
            decal.remaining -= dt;
        }
        self.decals.retain(|decal| decal.remaining > 0.0);

        let impacts = std::mem::take(&mut self.pending_impacts);
        let mut surviving_impacts = Vec::with_capacity(impacts.len());
        for mut impact in impacts {
            impact.delay -= dt;
            if impact.delay > 0.0 {
                surviving_impacts.push(impact);
                continue;
            }
            let hits = self
                .units
                .iter()
                .enumerate()
                .filter(|(_, unit)| {
                    unit.health > 0.0
                        && unit.team != impact.team
                        && unit.team != 2
                        && unit.pos.distance(impact.pos)
                            <= impact.radius + unit.hit_radius.max(40.0)
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            for index in hits {
                self.damage_unit_by(index, impact.damage, impact.team, Some(self.player_index));
            }
            self.effects.push(Effect {
                pos: impact.pos,
                radius: impact.radius,
                remaining: 0.65,
                duration: 0.65,
                color: impact.color,
            });
            self.emit_burst(impact.pos, impact.color, 40, 620.0, 34.0, 0.95);
            self.add_decal(impact.pos, impact.radius * 0.72, [54, 30, 20], 9.0);
        }
        self.pending_impacts = surviving_impacts;
        if let Some((_, ref mut remaining)) = self.marker {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.marker = None;
            }
        }

        if self.match_over.is_none() {
            self.wave_timer -= dt;
            if self.wave_timer <= 0.0 {
                for lane_index in 0..self.lanes.len() {
                    self.spawn_wave(0, lane_index, height);
                    self.spawn_wave(1, lane_index, height);
                }
                self.wave_timer = self.wave_interval;
            }
        }

        for buff in &mut self.team_buffs {
            if let Some(b) = buff {
                b.remaining -= dt;
                if b.remaining <= 0.0 {
                    *buff = None;
                }
            }
        }

        let mut venom_ticks: Vec<(usize, f32, u8)> = Vec::new();
        for (unit_index, unit) in self.units.iter_mut().enumerate() {
            unit.hit_flash = (unit.hit_flash - dt).max(0.0);
            unit.death_anim = (unit.death_anim - dt).max(0.0);
            unit.attack_cooldown = (unit.attack_cooldown - dt).max(0.0);
            unit.attack_windup_timer = (unit.attack_windup_timer - dt).max(0.0);
            unit.stun_timer = (unit.stun_timer - dt).max(0.0);
            if unit.cc.tick(dt) && unit.health > 0.0 {
                // A kit summon's time is up.
                unit.health = 0.0;
                unit.death_anim = DEATH_SINK_TIME;
            }
            if unit.attack_speed_timer > 0.0 {
                unit.attack_speed_timer -= dt;
                if unit.attack_speed_timer <= 0.0 {
                    unit.attack_speed_mult = 1.0;
                }
            }
            if unit.venom_timer > 0.0 {
                unit.venom_timer -= dt;
                unit.venom_tick -= dt;
                if unit.venom_tick <= 0.0 {
                    unit.venom_tick = 2.0;
                    venom_ticks.push((unit_index, unit.venom_damage, unit.venom_team));
                }
                if unit.venom_timer <= 0.0 {
                    unit.venom_damage = 0.0;
                }
            }
            if unit.armor_reduction_timer > 0.0 {
                unit.armor_reduction_timer -= dt;
                if unit.armor_reduction_timer <= 0.0 {
                    unit.armor_reduction = 0.0;
                }
            }
            unit.anim_time_ms += dt * 1000.0;
            if let Some(action) = unit.action.as_mut() {
                // Walking off cancels the backswing (and the swing itself;
                // the player's order code clears the pending hit).
                if unit.moving || unit.health <= 0.0 {
                    action.cancelling = true;
                }
                if !action.advance(dt) {
                    unit.action = None;
                }
            }
            if unit.kind == UnitKind::Player {
                let target = if unit.moving { 1.0 } else { 0.0 };
                let rate = if target > unit.walk_blend { 9.0 } else { 6.0 };
                unit.walk_blend += (target - unit.walk_blend) * (1.0 - (-rate * dt).exp());
                unit.walk_blend = unit.walk_blend.clamp(0.0, 1.0);
            }
            if unit.health <= 0.0
                && matches!(
                    unit.kind,
                    UnitKind::Dummy | UnitKind::Camp | UnitKind::Inhibitor | UnitKind::BotChampion
                )
                && unit.dead_timer > 0.0
            {
                unit.dead_timer -= dt;
                if unit.dead_timer <= 0.0 {
                    unit.health = unit.max_health;
                    unit.dead_timer = 0.0;
                    unit.death_anim = 0.0;
                    unit.attack_target = None;
                    unit.path.clear();
                    unit.path_index = 0;
                    if matches!(unit.kind, UnitKind::Camp | UnitKind::BotChampion) {
                        unit.pos = unit.home;
                        unit.camp_aggro = false;
                        unit.target = None;
                        unit.camp_wander_timer = 2.0;
                    }
                }
            }
        }

        let start_positions: Vec<Vec3> = self.units.iter().map(|unit| unit.pos).collect();
        self.update_bot_brains(dt);
        self.update_player(dt, height, collision);
        self.update_kit(dt, height, collision);
        self.update_scrolls(dt);
        self.update_spells(dt);
        self.update_delivery(dt, height);

        let unit_count = self.units.len();
        for i in 0..unit_count {
            let kind = self.units[i].kind;
            if self.units[i].health <= 0.0 {
                continue;
            }
            if self.units[i].spawn_delay > 0.0 {
                self.units[i].spawn_delay = (self.units[i].spawn_delay - dt).max(0.0);
                continue;
            }
            if self.kit_afflicted(i, dt, height) {
                continue;
            }
            if self.units[i].stun_timer > 0.0 {
                self.units[i].moving = false;
                continue;
            }
            if kind.is_minion() {
                self.update_minion(i, dt, height);
            } else if kind == UnitKind::Tower {
                self.update_tower(i);
            } else if kind == UnitKind::Camp {
                self.update_camp(i, dt, height);
            } else if kind == UnitKind::Ambient {
                self.update_ambient(i, dt, height);
            } else if kind == UnitKind::Watchtower {
                self.update_watchtower(i);
            } else if kind == UnitKind::BotChampion {
                if self.bots.iter().any(|bot| bot.unit == i) {
                    self.update_bot(i, dt, height, collision);
                } else {
                    self.update_enemy_champion(i, dt, height, collision);
                }
            }
        }

        self.update_strikes(dt);
        self.update_awowogei(dt, height, collision);
        self.update_projectiles(dt, height);

        for (index, damage, team) in venom_ticks {
            self.damage_unit_typed(index, damage, team, HitsplatKind::Venom);
        }

        // Replace the clock's share of every moving unit's animation time
        // with the distance it actually covered. This fixes foot sliding for
        // minions, camps and ambient creatures as well as the player.
        // Units summoned this frame have no start position and skip it.
        for (unit, start) in self.units.iter_mut().zip(&start_positions) {
            let moved = planar_delta(*start, unit.pos).length();
            let limit = unit.speed * dt * 4.0 + 64.0;
            if moved > 0.01 && moved <= limit && unit.walk_loop_ms > 0.0 && unit.walk_stride > 0.0 {
                unit.anim_time_ms += moved / unit.walk_stride * unit.walk_loop_ms - dt * 1000.0;
            }
        }
        self.play_frame_sounds(&start_clocks);
    }

    /// Play the sounds the cache attaches to animation frames (opcode 14):
    /// footsteps, swings and roars on the clips units actually played this
    /// tick. Only units the player can see are heard, and each sound keeps
    /// to its cache audible range.
    fn play_frame_sounds(&mut self, start: &[AnimClock]) {
        let Some(spec) = self.default_sounds.get("frame_sound").copied() else {
            return;
        };
        // Sounds on the walk/idle loop are footsteps: they repeat every
        // stride, so they play quieter than one-off swings and roars.
        let step_spec =
            self.default_sounds
                .get("footstep")
                .copied()
                .unwrap_or(crate::champion::SoundSpec {
                    volume: spec.volume * 0.33,
                    ..spec
                });
        let listener = self.units[self.player_index].pos;
        let mut queued = Vec::new();
        for (index, (unit, before)) in self.units.iter().zip(start).enumerate() {
            if !unit.visible || unit.spawn_delay > 0.0 || unit.health <= 0.0 {
                continue;
            }
            let base = if unit.moving {
                unit.walk.as_ref().unwrap_or(&unit.idle)
            } else {
                &unit.idle
            };
            let steps = base
                .sequence
                .sounds_between(before.base_ms, unit.anim_time_ms, true);
            for (k, sound) in steps.into_iter().enumerate() {
                let range = if sound.range == 0 {
                    crate::audio::HEARING_RANGE
                } else {
                    sound.range as f32 * 128.0
                };
                if planar_delta(listener, unit.pos).length() <= range {
                    // Each step a little different in pitch and weight, so
                    // the same stomp does not drum identically every stride.
                    let (pitch, weight) = step_variation(index, unit.anim_time_ms, k);
                    queued.push((
                        crate::champion::SoundSpec {
                            id: sound.id as u32,
                            volume: step_spec.volume * weight,
                            speed: step_spec.speed * pitch,
                        },
                        unit.pos,
                    ));
                }
            }
            let mut sounds = Vec::new();
            if let Some(action) = &unit.action {
                let from = match before.action {
                    Some((clip, time)) if std::ptr::eq(clip, Arc::as_ptr(&action.clip)) => time,
                    _ => 0.0,
                };
                sounds.extend(
                    action
                        .clip
                        .sequence
                        .sounds_between(from, action.time_ms, false),
                );
            }
            for sound in sounds {
                let range = if sound.range == 0 {
                    crate::audio::HEARING_RANGE
                } else {
                    sound.range as f32 * 128.0
                };
                if planar_delta(listener, unit.pos).length() <= range {
                    let id = sound.id as u32;
                    queued.push((crate::champion::SoundSpec { id, ..spec }, unit.pos));
                }
            }
        }
        for (spec, pos) in queued {
            self.queue_sound(spec, pos);
        }
    }

    fn update_player(&mut self, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        let pi = self.player_index;
        let speed_mult = self.team_speed_mult(0) * self.kit_move_mult(self.player_index);
        let steer = self.steer_wish.take();
        if self.units[pi].stun_timer > 0.0 {
            self.units[pi].moving = false;
            return;
        }
        if (self.kit_controls(self.player_index) || self.awowogei.controls_player())
            && self.units[pi].health > 0.0
        {
            // A dash, channel or rooted buff owns the champion's movement.
            return;
        }
        if self.units[pi].health <= 0.0 {
            self.units[pi].dead_timer -= dt;
            if self.units[pi].dead_timer <= 0.0 {
                let spawn = self.player_spawn;
                let unit = &mut self.units[pi];
                unit.pos = spawn;
                unit.health = unit.max_health;
                unit.dead_timer = 0.0;
                unit.death_anim = 0.0;
                unit.attack_target = None;
                unit.attack_pending = false;
                unit.attack_windup_timer = 0.0;
                unit.path.clear();
                unit.path_index = 0;
                unit.moving = false;
                unit.anim_time_ms = 0.0;
                if let Some(motion) = unit.motion.as_mut() {
                    motion.reset();
                }
            }
            return;
        }

        // Hold-to-steer and tap-to-move share the same weighted motor.
        if let Some(direction) = steer {
            let speed = self.units[pi].speed;
            self.drive_player(dt, height, collision, Some(direction), speed, speed_mult);
            return;
        }

        let target_index = self.units[pi].attack_target;
        if let Some(ti) = target_index {
            let target_alive = self.units.get(ti).is_some_and(|u| {
                u.health > 0.0 && (u.team == 1 || (u.team == 2 && u.kind == UnitKind::Camp))
            });
            if target_alive {
                let target_pos = self.units[ti].pos;
                let player_pos = self.units[pi].pos;
                // A committed swing tolerates a little drift before it is
                // abandoned, so a target stepping back does not eat it.
                let leash = if self.units[pi].attack_pending {
                    WINDUP_LEASH
                } else {
                    0.0
                };
                if !self.in_reach(pi, ti, self.units[pi].attack_range + leash) {
                    if self.units[pi].attack_pending {
                        self.cancel_action(pi);
                    }
                    let unit = &mut self.units[pi];
                    unit.attack_pending = false;
                    unit.attack_windup_timer = 0.0;
                    if unit.path.is_empty()
                        || unit
                            .path
                            .last()
                            .is_none_or(|p| p.distance(target_pos) > 48.0)
                    {
                        unit.path = collision.path_world(player_pos, target_pos, height);
                        unit.path_index = 0;
                    }
                    self.drive_player_path(dt, height, collision, speed_mult);
                } else {
                    let dir = target_pos - player_pos;
                    let desired = (dir.length_squared() > 1e-6).then_some(dir);
                    let facing = self
                        .drive_player(dt, height, collision, desired, 0.0, speed_mult)
                        .map_or(0.0, |step| step.facing_error);
                    self.units[pi].path.clear();
                    self.units[pi].path_index = 0;
                    let pending = self.units[pi].attack_pending;
                    let timer = self.units[pi].attack_windup_timer;
                    let cooldown = self.units[pi].attack_cooldown;
                    let windup = self.units[pi].attack_windup;
                    if pending {
                        if timer <= 0.0 && facing < 0.35 {
                            self.player_land_attack(ti);
                            self.units[pi].attack_pending = false;
                        }
                    } else if cooldown <= 0.0 && facing < 0.35 {
                        // Wind-up shortens with attack speed, like the
                        // interval does.
                        let windup = windup / self.units[pi].attack_speed_mult.max(0.1);
                        self.start_action(pi, false, windup);
                        if windup > 0.0 {
                            self.units[pi].attack_pending = true;
                            self.units[pi].attack_windup_timer = windup;
                        } else {
                            self.player_land_attack(ti);
                        }
                    }
                }
                return;
            } else {
                self.units[pi].attack_target = None;
                self.units[pi].attack_pending = false;
                self.units[pi].attack_windup_timer = 0.0;
            }
        }

        if !self.units[pi].path.is_empty() {
            self.drive_player_path(dt, height, collision, speed_mult);
        } else {
            // No command: coast to a stop in the current facing.
            let speed = self.units[pi].speed;
            self.drive_player(dt, height, collision, None, speed, speed_mult);
        }
    }

    fn update_ambient(&mut self, index: usize, dt: f32, height: &HeightGrid) {
        let unit = &mut self.units[index];
        let kind = unit.ambient_kind.unwrap_or(AmbientKind::Butterfly);
        let time = unit.anim_time_ms * 0.001;
        let phase = (unit.ambient_seed % 4096) as f32 * 0.0015;
        let home = unit.home;

        // Cows graze for roughly half of their cycle and only amble between
        // nearby patches. Butterflies stay in constant fluttering motion.
        if kind == AmbientKind::Cow && (time * 0.45 + phase).sin() < 0.1 {
            unit.moving = false;
            return;
        }

        let (radius, speed, hover) = match kind {
            AmbientKind::Butterfly => (55.0 + (unit.ambient_seed % 5) as f32 * 22.0, 92.0, 30.0),
            AmbientKind::Cow => (4.5 * 128.0, 46.0, 0.0),
            AmbientKind::Crab => (1.2 * 128.0, 26.0, 0.0),
        };
        let target = home
            + Vec3::new(
                (time * 0.73 + phase).sin() * radius,
                0.0,
                (time * 0.51 + phase * 1.7).cos() * radius,
            );
        let delta = planar_delta(unit.pos, target);
        let distance = delta.length();
        if distance > 8.0 {
            let dir = delta / distance;
            unit.pos += dir * (speed * dt).min(distance);
            unit.yaw = if kind == AmbientKind::Crab {
                // Crabs scuttle sideways.
                dir.x.atan2(dir.z) + std::f32::consts::FRAC_PI_2
            } else {
                dir.x.atan2(dir.z)
            };
            unit.moving = true;
        } else {
            unit.moving = false;
        }
        let flutter = if kind == AmbientKind::Butterfly {
            hover + (time * 5.1 + phase).sin() * 9.0
        } else {
            0.0
        };
        unit.pos.y = height.height_at(unit.pos.x, unit.pos.z) + flutter;
    }

    fn update_minion(&mut self, index: usize, dt: f32, height: &HeightGrid) {
        let aura = self.kit_minion_aura(index);
        let team = self.units[index].team;
        let speed_mult = self.team_speed_mult(team);
        let pos = self.units[index].pos;
        // Summons and thralls attack what their hero attacks (as OSRS
        // thralls do), camps included, then fall back to minion aggro.
        let owner = self.units[index].cc.owner.unwrap_or(self.player_index);
        let focus = self.units[index]
            .cc
            .focus
            .filter(|&t| self.units[t].health > 0.0);
        let led = if self.units[index].cc.summoned {
            focus.or_else(|| {
                self.units[owner]
                    .attack_target
                    .filter(|&t| self.units[t].health > 0.0 && self.units[t].team != team)
            })
        } else {
            None
        };
        // Guards (healers) never fight.
        let target = if self.units[index].cc.guard {
            None
        } else {
            led.or_else(|| self.find_target(index, team, MINION_AGGRO))
        };

        if let Some(target) = target {
            let target_pos = self.units[target].pos;
            let _ = pos;
            if self.in_reach(index, target, self.units[index].attack_range) {
                let unit = &mut self.units[index];
                unit.moving = false;
                unit.attack_target = Some(target);
                let dir = target_pos - unit.pos;
                if dir.length_squared() > 1e-6 {
                    turn_toward(unit, dir.x.atan2(dir.z), dt);
                }
                if unit.attack_cooldown <= 0.0 && unit.strike.is_none() {
                    let damage = unit.attack_damage * aura;
                    let ranged = unit.kind == UnitKind::RangedMinion;
                    self.begin_strike(index, target, damage, ranged);
                }
            } else {
                self.units[index].attack_target = Some(target);
                self.move_unit_toward(index, target_pos, dt, height, speed_mult);
            }
        } else if self.units[index].cc.summoned {
            // Kit summons stay at the champion's heel between fights.
            self.units[index].attack_target = None;
            let heel = self.units[owner].pos;
            if self.units[index].pos.distance(heel) > 320.0 {
                self.move_unit_toward(index, heel, dt, height, 1.2);
            } else {
                self.units[index].moving = false;
            }
        } else {
            self.units[index].attack_target = None;
            self.follow_lane(index, dt, height, speed_mult);
        }
    }

    fn update_enemy_champion(
        &mut self,
        index: usize,
        dt: f32,
        height: &HeightGrid,
        _collision: &WorldCollision,
    ) {
        let team = self.units[index].team;
        let target = self.find_target(index, team, 600.0);
        let Some(target) = target else {
            self.units[index].moving = false;
            self.units[index].attack_target = None;
            return;
        };
        let target_pos = self.units[target].pos;
        let _ = team;
        let (in_range, cooldown_ready, damage, ranged) = {
            let unit = &self.units[index];
            (
                self.in_reach(index, target, unit.attack_range),
                unit.attack_cooldown <= 0.0 && unit.strike.is_none(),
                unit.attack_damage,
                unit.ranged_auto,
            )
        };
        if in_range {
            let unit = &mut self.units[index];
            unit.moving = false;
            unit.attack_target = Some(target);
            let dir = target_pos - unit.pos;
            if dir.length_squared() > 1e-6 {
                turn_toward(unit, dir.x.atan2(dir.z), dt);
            }
            if cooldown_ready {
                self.begin_strike(index, target, damage, ranged);
            }
        } else {
            self.units[index].attack_target = Some(target);
            self.move_unit_toward(index, target_pos, dt, height, 1.0);
        }
    }

    fn update_camp(&mut self, index: usize, dt: f32, height: &HeightGrid) {
        let home = self.units[index].home;
        let leash = self.units[index].leash_range.max(1.0);

        if self.units[index].camp_aggro {
            let target = self.units[index]
                .attack_target
                .filter(|&ti| {
                    self.units.get(ti).is_some_and(|u| {
                        matches!(u.kind, UnitKind::Player | UnitKind::BotChampion) && u.health > 0.0
                    })
                })
                .or_else(|| self.find_camp_target(index));
            let Some(ti) = target else {
                self.units[index].camp_aggro = false;
                self.units[index].attack_target = None;
                self.units[index].target = Some(home);
                return;
            };

            let target_pos = self.units[ti].pos;
            if self.units[index].pos.distance(home) > leash {
                // Pulled too far: drop the hero and walk home to reset.
                self.units[index].camp_aggro = false;
                self.units[index].attack_target = None;
                self.units[index].target = Some(home);
                return;
            }

            if self.in_reach(index, ti, self.units[index].attack_range) {
                self.units[index].attack_target = Some(ti);
                self.units[index].moving = false;
                let dir = target_pos - self.units[index].pos;
                if dir.length_squared() > 1e-6 {
                    let yaw = dir.x.atan2(dir.z);
                    turn_toward(&mut self.units[index], yaw, dt);
                }
                if self.units[index].attack_cooldown <= 0.0 && self.units[index].strike.is_none() {
                    let damage = self.units[index].attack_damage;
                    let ranged = self.units[index].attack_range > 300.0;
                    self.begin_strike(index, ti, damage, ranged);
                }
            } else {
                self.units[index].attack_target = Some(ti);
                self.move_unit_toward(index, target_pos, dt, height, 1.0);
            }
            return;
        }

        // Return to the spawn area after a leash reset, then heal to full.
        if self.units[index].pos.distance(home) > 10.0 {
            self.move_unit_toward(index, home, dt, height, 1.0);
            if self.units[index].pos.distance(home) <= 10.0 {
                let unit = &mut self.units[index];
                unit.pos = home;
                unit.health = unit.max_health;
                unit.target = None;
                unit.moving = false;
                unit.camp_wander_timer = 2.0;
            }
            return;
        }

        // Heroes walking into the camp's aggro radius pull it.
        if let Some(ti) = self.find_camp_target(index) {
            self.units[index].camp_aggro = true;
            self.units[index].attack_target = Some(ti);
            self.units[index].target = None;
            return;
        }

        // Small idle wander inside the spawn area, Dota-style.
        if self.units[index].camp_wander_timer > 0.0 {
            self.units[index].camp_wander_timer -= dt;
        }
        let wander = self.units[index].target;
        if let Some(target) = wander {
            if self.units[index].pos.distance(target) > 6.0 {
                self.move_unit_toward(index, target, dt, height, 1.0);
            } else {
                self.units[index].target = None;
                self.units[index].moving = false;
                self.units[index].camp_wander_timer = 2.0;
            }
        } else if self.units[index].camp_wander_timer <= 0.0 {
            let phase = self.units[index].anim_time_ms * 0.001 + index as f32 * 1.7;
            let offset = Vec3::new(phase.sin() * 72.0, 0.0, phase.cos() * 72.0);
            self.units[index].target = Some(home + offset);
            self.units[index].camp_wander_timer = 2.5;
        }
    }

    fn find_camp_target(&self, index: usize) -> Option<usize> {
        let camp = &self.units[index];
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if !matches!(unit.kind, UnitKind::Player | UnitKind::BotChampion)
                || unit.health <= 0.0
                || (i == self.player_index && self.player_untargetable())
            {
                continue;
            }
            let d = camp.pos.distance_squared(unit.pos);
            if d <= camp.aggro_range * camp.aggro_range && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Watchtower behaviour: it picks up the first enemy inside its short
    /// aggro radius, then keeps firing slow bolts at that target until the
    /// target dies or walks out of the longer lock range.
    fn update_watchtower(&mut self, index: usize) {
        let team = self.units[index].team;
        let lock_range = self.units[index].attack_range;
        let aggro_range = self.units[index].lock_range.max(1.0);
        let tower_pos = self.units[index].pos;

        let mut target = self.units[index].attack_target.filter(|&ti| {
            let unit = &self.units[ti];
            unit.health > 0.0
                && unit.team != team
                && unit.pos.distance(tower_pos) <= lock_range
                && !(ti == self.player_index && self.player_untargetable())
        });
        if target.is_none() {
            target = self.find_watchtower_target(index, aggro_range);
        }

        let Some(ti) = target else {
            self.units[index].attack_target = None;
            return;
        };
        self.units[index].attack_target = Some(ti);
        let target_pos = self.units[ti].pos;
        let dir = target_pos - tower_pos;
        if dir.length_squared() > 1e-6 {
            self.units[index].yaw = dir.x.atan2(dir.z);
        }
        if self.units[index].attack_cooldown > 0.0 {
            return;
        }
        self.units[index].attack_cooldown = self.units[index].attack_interval;
        let damage = self.units[index].attack_damage;
        let from = tower_pos + Vec3::Y * 240.0;
        let shot_dir = (target_pos - from).normalize_or_zero();
        // Watchtower bolts stay slow dodgeable skillshots by design.
        let look = fx::look_by_name("watchtower");
        let mut shot = Projectile::new(from, shot_dir, 280.0, 3.5, damage, team, look);
        shot.attacker = Some(index);
        self.projectiles.push(shot);
        self.muzzle_flash(from, look);
    }

    /// Watchtowers prefer heroes, then lane minions, then jungle camps.
    fn find_watchtower_target(&self, index: usize, range: f32) -> Option<usize> {
        let tower = &self.units[index];
        let mut best: Option<(usize, u8, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if i == index
                || unit.health <= 0.0
                || unit.team == tower.team
                || (i == self.player_index && self.player_untargetable())
            {
                continue;
            }
            if unit.kind == UnitKind::Nexus || unit.kind == UnitKind::Inhibitor {
                continue;
            }
            let priority = match unit.kind {
                UnitKind::Player => 0u8,
                UnitKind::MeleeMinion | UnitKind::RangedMinion => 1,
                UnitKind::Camp => 2,
                _ => 3,
            };
            if priority == 3 {
                continue;
            }
            let d = tower.pos.distance_squared(unit.pos);
            if d > range * range {
                continue;
            }
            match best {
                Some((_, best_priority, best_d))
                    if best_priority < priority || (best_priority == priority && best_d <= d) => {}
                _ => best = Some((i, priority, d)),
            }
        }
        best.map(|(i, _, _)| i)
    }

    fn update_tower(&mut self, index: usize) {
        let team = self.units[index].team;
        let pos = self.units[index].pos;
        let range = self.units[index].attack_range;
        let target = self.find_target(index, team, range);
        if let Some(target) = target {
            if self.units[index].attack_cooldown <= 0.0 {
                self.units[index].attack_cooldown = self.units[index].attack_interval;
                let target_pos = self.unit_chest(target);
                let damage = self.units[index].attack_damage;
                let look = if team == 0 {
                    fx::look_by_name("tower_blue")
                } else {
                    fx::look_by_name("tower_red")
                };
                let from = pos + Vec3::Y * 380.0;
                // Tower shots home like basic attacks; the attacker stays
                // unset so tower kills never pull a camp.
                let shot =
                    Projectile::homing(from, target, target_pos, 900.0, damage, team, None, look);
                self.projectiles.push(shot);
                self.muzzle_flash(from, look);
            }
        }
    }

    fn find_target(&self, index: usize, team: u8, range: f32) -> Option<usize> {
        let me = &self.units[index];
        if me.cc.taunt_timer > 0.0
            && self
                .units
                .get(me.cc.taunt_by)
                .is_some_and(|unit| unit.health > 0.0 && unit.team != team)
        {
            return Some(me.cc.taunt_by);
        }
        let pos = self.units[index].pos;
        let minion = me.kind.is_minion();
        let mut best: Option<(usize, f32)> = None;
        for (i, unit) in self.units.iter().enumerate() {
            if i == index
                || unit.team == team
                || unit.team == 2
                || unit.health <= 0.0
                || (i == self.player_index && self.player_untargetable())
                || matches!(unit.kind, UnitKind::DeliveryGuard | UnitKind::StorageChest)
            {
                continue;
            }
            // Big structures are seen from their edge, and by minions from
            // further still: the scene keeps every tower 5 tiles off the lane
            // centre, so waves on the far side of the formation used to walk
            // past a live tower while it shot them, then hit the inhibitor.
            let edge = if unit.kind.is_structure() {
                unit.hit_radius + if minion { STRUCTURE_REACH } else { 0.0 }
            } else {
                0.0
            };
            let d = (pos.distance(unit.pos) - edge).max(0.0);
            if d <= range && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    fn move_unit_toward(
        &mut self,
        index: usize,
        target: Vec3,
        dt: f32,
        height: &HeightGrid,
        speed_mult: f32,
    ) {
        let unit = &mut self.units[index];
        // Lateral lane offsets sit at different terrain heights from the
        // original waypoint; always move in the XZ plane and let the height
        // grid place the unit afterwards.
        let delta = planar_delta(unit.pos, target);
        let distance = delta.length();
        if distance < 4.0 {
            unit.moving = false;
            return;
        }
        let dir = delta / distance;
        let step = (unit.speed * speed_mult * unit.cc.move_mult() * dt).min(distance);
        unit.pos += dir * step;
        unit.pos.y = height.height_at(unit.pos.x, unit.pos.z);
        turn_toward(unit, dir.x.atan2(dir.z), dt);
        unit.moving = true;
    }

    fn follow_lane(&mut self, index: usize, dt: f32, height: &HeightGrid, speed_mult: f32) {
        let unit = &mut self.units[index];
        if unit.lane.is_empty() {
            unit.moving = false;
            return;
        }
        if unit.lane_index >= unit.lane.len() {
            let target = unit.lane.last().copied().unwrap_or(unit.pos);
            let dir = target - unit.pos;
            if dir.length_squared() > 1e-6 {
                unit.yaw = dir.x.atan2(dir.z);
            }
            unit.moving = false;
            return;
        }
        let waypoint = unit.lane[unit.lane_index];
        let delta = planar_delta(unit.pos, waypoint);
        let distance = delta.length();
        if distance < 10.0 {
            unit.lane_index += 1;
            return;
        }
        let dir = delta / distance;
        let step = (unit.speed * speed_mult * unit.cc.move_mult() * dt).min(distance);
        unit.pos += dir * step;
        unit.pos.y = height.height_at(unit.pos.x, unit.pos.z);
        unit.yaw = dir.x.atan2(dir.z);
        unit.moving = true;
    }

    /// Cool tint for a champion's hit that was not the local player's. It is
    /// the reference client's tinted other-player hitsplat, kept subtle so
    /// the splat colour still reads.
    fn hitsplat_tint(&self, aggressor: Option<usize>) -> Option<[u8; 3]> {
        let aggressor = aggressor?;
        if aggressor == self.player_index {
            return None;
        }
        // The reference client's tinted splat is roughly a third brightness
        // with a cool cast, not a faint wash.
        self.units
            .get(aggressor)
            .is_some_and(|unit| unit.is_champion)
            .then_some([100, 90, 125])
    }

    /// Pop an OSRS hitmark over a unit. Hits on one unit occupy a downward
    /// column: slot 0 sits at the unit's head, later hits stack below it.
    fn push_hitsplat(
        &mut self,
        index: usize,
        amount: f32,
        kind: HitsplatKind,
        tint: Option<[u8; 3]>,
    ) {
        let Some(unit) = self.units.get(index) else {
            return;
        };
        let stack = self
            .floats
            .iter()
            .filter(|float| {
                float.unit == Some(index)
                    && float.life > 0.0
                    && matches!(float.style, FloatStyle::Hitsplat(_))
            })
            .count()
            .min(3) as u8;
        let pos = unit.pos + Vec3::Y * (unit.bar_height + 30.0);
        let amount = amount.max(0.0).round() as u32;
        self.floats.push(FloatText {
            pos,
            text: amount.to_string(),
            style: FloatStyle::Hitsplat(kind),
            life: 1.2,
            scale: 1.0,
            stack,
            unit: Some(index),
            tint,
        });
    }

    /// Damage with a specific hitmark, used by venom ticks.
    fn damage_unit_typed(
        &mut self,
        index: usize,
        amount: f32,
        attacker_team: u8,
        kind: HitsplatKind,
    ) {
        self.damage_unit_mixed_kind(index, amount, 0.0, attacker_team, None, kind);
    }

    /// Apply damage and attribute it to a specific attacker. The aggressor
    /// matters for jungle camps: only a hero pulls a camp, a watchtower
    /// bolt just hurts it.
    fn damage_unit_by(
        &mut self,
        index: usize,
        amount: f32,
        attacker_team: u8,
        aggressor: Option<usize>,
    ) {
        self.damage_unit_mixed(index, amount, 0.0, attacker_team, aggressor);
    }

    /// Physical damage is reduced by armor; pure damage bypasses it.
    fn damage_unit_mixed(
        &mut self,
        index: usize,
        physical: f32,
        pure: f32,
        attacker_team: u8,
        aggressor: Option<usize>,
    ) {
        self.damage_unit_mixed_kind(
            index,
            physical,
            pure,
            attacker_team,
            aggressor,
            HitsplatKind::Damage,
        );
    }

    /// The shared damage path, with the hitmark to pop.
    fn damage_unit_mixed_kind(
        &mut self,
        index: usize,
        physical: f32,
        pure: f32,
        attacker_team: u8,
        aggressor: Option<usize>,
        hitsplat: HitsplatKind,
    ) {
        if index >= self.units.len() {
            return;
        }
        if matches!(
            self.units[index].kind,
            UnitKind::DeliveryGuard | UnitKind::StorageChest
        ) {
            return;
        }
        let incoming = if index == self.player_index && self.player_untargetable() {
            0.0
        } else {
            self.kit_incoming_mult(index)
        };
        if incoming <= 0.0 || self.duel_blocks(aggressor, index) {
            return;
        }
        // A fragile summon (Jad's healers) dies to any hero's hit.
        let physical = if self.units[index].cc.fragile
            && aggressor.is_some_and(|a| self.units[a].kind.is_champion())
        {
            physical.max(self.units[index].health * 100.0 + 1.0e4)
        } else {
            physical
        };
        let team_mult = self.team_damage_mult(attacker_team) * incoming;
        let (target_kind, camp_buff, respawn_time, killed, amount) = {
            let unit = &mut self.units[index];
            if unit.health <= 0.0 {
                return;
            }
            let armor = (unit.armor - unit.armor_reduction).clamp(-80.0, 500.0);
            let multiplier = armor_damage_multiplier(armor);
            let mut amount = (physical * multiplier + pure) * team_mult;
            if unit.shield > 0.0 {
                let absorbed = unit.shield.min(amount);
                unit.shield -= absorbed;
                amount -= absorbed;
            }
            unit.health = (unit.health - amount).max(0.0);
            unit.hit_flash = 0.2;
            if unit.health <= 0.0 && !unit.kind.is_structure() {
                unit.death_anim = DEATH_SINK_TIME;
                unit.visible = true;
            }
            (
                unit.kind,
                unit.camp_buff,
                unit.respawn_time,
                unit.health <= 0.0,
                amount,
            )
        };
        if index == self.player_index && amount > 0.0 {
            let label = self.damage_label(attacker_team, aggressor);
            self.record_damage(&label, amount);
        }
        if amount > 0.0 && self.casters.contains_key(&index) {
            self.kit_after_hit(index, amount, aggressor);
        }
        let hero_aggressor = match aggressor {
            Some(aggressor) => self.units[aggressor].kind == UnitKind::Player,
            None => attacker_team == 0,
        };
        let hit_kind = if amount > 0.5 {
            hitsplat
        } else {
            HitsplatKind::Miss
        };
        let tint = self.hitsplat_tint(aggressor);
        self.push_hitsplat(index, amount, hit_kind, tint);
        self.play_hit_sound(index, aggressor, amount);
        if amount > 0.0 {
            let color = if index == self.player_index {
                [255, 110, 100]
            } else if hero_aggressor {
                [255, 225, 120]
            } else {
                [235, 235, 235]
            };
            let pos = self.units[index].pos + Vec3::Y * (self.units[index].bar_height + 40.0);
            if hero_aggressor || index == self.player_index {
                self.emit_burst(pos, color, 3, 130.0, 7.0, 0.3);
            }
        }
        if index != self.player_index && self.units[index].team == 1 && hero_aggressor {
            // Ping the target on the minimap, like the reference client.
            self.ping = Some((self.units[index].pos, 6.0));
        }
        if !killed && amount > 0.0 && !self.reflecting {
            self.vengeance_check(index, aggressor, amount);
        }
        if !killed {
            // A camp turns on the hero who hit it (the player or a bot).
            let striker = aggressor.filter(|&a| {
                matches!(self.units[a].kind, UnitKind::Player | UnitKind::BotChampion)
                    && self.units[a].health > 0.0
            });
            if target_kind == UnitKind::Camp && attacker_team < 2 {
                if let Some(striker) = striker {
                    self.units[index].camp_aggro = true;
                    self.units[index].attack_target = Some(striker);
                    self.units[index].target = None;
                }
            }
            return;
        }
        if target_kind.is_champion() || target_kind.is_structure() {
            let name = |i: usize| {
                self.bot_of(i)
                    .map(|bot| bot.name.clone())
                    .unwrap_or_else(|| format!("{:?}", self.units[i].kind))
            };
            tracing::info!(
                "death: {} (team {}) by {} (team {attacker_team})",
                name(index),
                self.units[index].team,
                aggressor.map_or_else(|| "the rift".to_string(), name)
            );
        }
        if attacker_team == 0 && hero_aggressor {
            self.grant_reward(target_kind);
        }
        if let Some(bot) = aggressor.filter(|&a| self.units[a].kind == UnitKind::BotChampion) {
            self.bot_reward(bot, target_kind);
        }
        self.share_xp(index, target_kind, attacker_team, aggressor);
        self.kit_flesh_heap(index);
        self.kit_on_kill(index);
        self.greegree_kill(index, aggressor);
        if reaches_feed(target_kind, hero_aggressor) {
            let victim = short_unit_name(target_kind);
            let killer = aggressor
                .filter(|&a| a != index)
                .map(|a| short_unit_name(self.units[a].kind))
                .unwrap_or("THE RIFT");
            if index == self.player_index {
                if !self.announcement_pinned {
                    self.announcement = Some((format!("YOU HAVE BEEN SLAIN BY {killer}"), 4.0));
                }
            } else if hero_aggressor {
                if !self.announcement_pinned {
                    self.announcement = Some((format!("YOU HAVE SLAIN {victim}!"), 4.0));
                }
                self.kill_feed.push((format!("YOU SLAIN {victim}"), 8.0));
            } else if attacker_team == 0 {
                self.kill_feed
                    .push((format!("{killer} HAS SLAIN {victim}"), 8.0));
            }
        }
        // Personal scoreboard line. Only damage attributed directly to the
        // player counts, so allied minions do not feed the player's CS.
        if aggressor == Some(self.player_index) {
            if matches!(target_kind, UnitKind::MeleeMinion | UnitKind::RangedMinion) {
                self.player_cs += 1;
            }
            if matches!(target_kind, UnitKind::Player | UnitKind::BotChampion) {
                self.player_kills += 1;
            }
        }
        if index == self.player_index {
            self.player_deaths += 1;
        }
        if attacker_team < 2 && target_kind != UnitKind::Camp {
            self.kills[attacker_team as usize] += 1;
            let victim_team = self.units[index].team;
            if victim_team < 2 {
                self.deaths[victim_team as usize] += 1;
            }
        }
        if target_kind.is_structure() {
            let pos = self.units[index].pos;
            self.play_shared_sound("structure_fall", pos);
        }
        match target_kind {
            UnitKind::Nexus => {
                self.match_over = Some(attacker_team);
            }
            UnitKind::Player => {
                let unit = &mut self.units[index];
                unit.dead_timer = PLAYER_RESPAWN;
                unit.attack_target = None;
                unit.path.clear();
                unit.path_index = 0;
            }
            UnitKind::Tower => {
                self.units[index].dead_timer = 0.0;
            }
            UnitKind::Inhibitor => {
                self.units[index].dead_timer = 120.0;
            }
            UnitKind::Dummy => {
                self.units[index].dead_timer = 4.0;
            }
            UnitKind::BotChampion => {
                let unit = &mut self.units[index];
                unit.dead_timer = 6.0 + unit.level as f32 * 1.5;
                unit.attack_target = None;
                unit.path.clear();
                unit.path_index = 0;
            }
            UnitKind::Camp => {
                let unit = &mut self.units[index];
                unit.dead_timer = respawn_time;
                unit.camp_aggro = false;
                unit.attack_target = None;
                unit.target = None;
                unit.path.clear();
                unit.path_index = 0;
                if let Some(kind) = camp_buff {
                    let (damage_mult, speed_mult) = match kind {
                        CampBuff::Might => (1.25, 1.0),
                        CampBuff::Swift => (1.0, 1.2),
                    };
                    if attacker_team < 2 {
                        self.team_buffs[attacker_team as usize] = Some(TeamBuff {
                            remaining: 60.0,
                            duration: 60.0,
                            damage_mult,
                            speed_mult,
                            kind,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    /// The hit sound for one landed blow. Like the reference client, only
    /// fights the player is in make hit noises (the player struck, or was
    /// struck), plus visible champion duels nearby; minion and tower trades
    /// stay quiet so a lane does not drown the player's own hits.
    fn play_hit_sound(&mut self, index: usize, aggressor: Option<usize>, amount: f32) {
        let pos = self.units[index].pos;
        if index == self.player_index {
            if amount > 0.5 {
                self.play_sound("player_hurt", pos);
            }
            return;
        }
        let from_player = aggressor == Some(self.player_index);
        let champion_duel = aggressor.is_some_and(|a| self.units[a].is_champion)
            && self.units[index].is_champion
            && self.units[index].visible;
        if !from_player && !champion_duel {
            return;
        }
        let ranged = aggressor.is_some_and(|a| self.units[a].ranged_auto);
        let name = match (amount > 0.5, ranged) {
            (false, _) => "blocked_hit",
            (true, true) => "ranged_hit",
            (true, false) => "melee_hit",
        };
        if from_player {
            self.play_sound(name, pos);
        } else {
            self.play_shared_sound(name, pos);
        }
    }

    fn grant_reward(&mut self, kind: UnitKind) {
        let (xp, gold) = reward_for(kind);
        if xp <= 0.0 {
            return;
        }
        let pi = self.player_index;
        self.units[pi].gold = self.units[pi].gold.saturating_add(gold);
        self.grant_xp(xp);
    }

    /// Nearby heroes learn from a kill they did not land: every living hero
    /// of the killing side within `SHARE_RADIUS` of the victim, other than
    /// the killer, gets `SHARE_XP` of its experience (no gold). Same rule for
    /// the player and the bots, so laners level without last-hitting.
    fn share_xp(&mut self, victim: usize, kind: UnitKind, team: u8, killer: Option<usize>) {
        let (xp, _) = reward_for(kind);
        if xp <= 0.0 || team > 1 {
            return;
        }
        let at = self.units[victim].pos;
        let heroes: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                Some(i) != killer
                    && i != victim
                    && u.team == team
                    && u.health > 0.0
                    && matches!(u.kind, UnitKind::Player | UnitKind::BotChampion)
                    && planar_delta(u.pos, at).length() <= SHARE_RADIUS
            })
            .collect();
        for i in heroes {
            if i == self.player_index {
                self.grant_xp(xp * SHARE_XP);
            } else {
                self.bot_xp(i, xp * SHARE_XP);
            }
        }
    }

    fn grant_xp(&mut self, xp: f32) {
        let pi = self.player_index;
        let xp = xp * self.kit_xp_mult(pi);
        self.units[pi].xp += xp;
        loop {
            let player = &mut self.units[pi];
            if player.level >= MAX_LEVEL || player.xp < xp_for_level(player.level) {
                break;
            }
            player.xp -= xp_for_level(player.level);
            self.level_up_player();
        }
    }

    /// One level gained: stats grow, and the level earns a skill point or,
    /// at a talent level, a talent pick.
    fn level_up_player(&mut self) {
        let pi = self.player_index;
        level_up_stats(&mut self.units[pi]);
        let level = self.units[pi].level;
        if ranks::is_talent_level(level) {
            self.talent_picks_pending += 1;
            self.announcement = Some((format!("TALENT AVAILABLE (LEVEL {level})"), 2.5));
        } else {
            self.skill_points += 1;
        }
        self.spend_spare_points();
    }

    pub fn refresh_visibility(&mut self, dt: f32, collision: &WorldCollision) {
        self.visibility_timer += dt;
        if self.visibility_timer < 0.15 {
            return;
        }
        self.visibility_timer = 0.0;
        self.update_visibility(collision);
        self.visibility_dirty = true;
    }

    fn update_visibility(&mut self, collision: &WorldCollision) {
        self.visibility.tiles.fill(false);
        for unit in &mut self.units {
            unit.visible = unit.team == 0;
        }

        let sources = self
            .units
            .iter()
            .filter(|u| u.team == 0 && u.health > 0.0)
            .map(|u| {
                let radius: f32 = match u.kind {
                    UnitKind::Player | UnitKind::BotChampion => 1500.0,
                    UnitKind::MeleeMinion | UnitKind::RangedMinion => 900.0,
                    UnitKind::Tower | UnitKind::Inhibitor | UnitKind::Nexus => 1100.0,
                    _ => 800.0,
                };
                (u.pos, radius)
            })
            .collect::<Vec<_>>();

        let (ox, oy) = self.vision_origin;
        for (pos, radius) in &sources {
            let rt = (*radius / 128.0).ceil() as i32;
            let ctx = (pos.x / 128.0).floor() as i32 - ox;
            let cty = (pos.z / 128.0).floor() as i32 - oy;
            for dy in -rt..=rt {
                for dx in -rt..=rt {
                    if dx * dx + dy * dy > rt * rt {
                        continue;
                    }
                    let tx = ctx + dx;
                    let ty = cty + dy;
                    if tx < 0
                        || ty < 0
                        || (tx as usize) >= self.visibility.width
                        || (ty as usize) >= self.visibility.height
                    {
                        continue;
                    }
                    if !los_clear(collision, ctx, cty, tx, ty) {
                        continue;
                    }
                    self.visibility.tiles[ty as usize * self.visibility.width + tx as usize] = true;
                }
            }
        }

        let player_pos = self.player_pos();
        let brush = self.brush.clone();
        for unit in &mut self.units {
            if unit.team == 0 {
                continue;
            }
            let tx = (unit.pos.x / 128.0).floor() as i32 - ox;
            let ty = (unit.pos.z / 128.0).floor() as i32 - oy;
            if !self.visibility.is_visible(tx, ty) {
                continue;
            }
            // Neutral jungle camps are never hidden by brush: they are
            // stationary objectives, not ambushers, and should be visible as
            // soon as the team has line of sight to their camp.
            if unit.team != 2
                && point_in_brush(&brush, unit.pos)
                && player_pos.distance(unit.pos) > 500.0
            {
                continue;
            }
            unit.visible = true;
        }
        // Neutral camps are revealed by proximity alone: they are static
        // objectives, so trees or brush should not make them invisible until
        // the hero is standing on top of them.
        let player_pos = self.player_pos();
        for unit in &mut self.units {
            if unit.team == 2
                && unit.kind == UnitKind::Camp
                && unit.health > 0.0
                && player_pos.distance(unit.pos) <= 1500.0
            {
                unit.visible = true;
            }
        }
        // Structures are permanent map features: the player always knows
        // where the towers, inhibitors, nexus and watchtowers stand, even
        // when they sit outside the current line of sight.
        for unit in &mut self.units {
            if unit.kind.is_structure() || unit.kind == UnitKind::Watchtower {
                unit.visible = true;
            }
        }
        // The training dummy is a dev-only target; keep it visible so the
        // deterministic combat demo is not blocked by LOS.
        for unit in &mut self.units {
            if unit.kind == UnitKind::Dummy && unit.health > 0.0 {
                unit.visible = true;
            }
        }
    }

    /// Dynamic units farther than 9000 units from the camera focus are off
    /// screen at every supported camera pitch. Skipping their animation and
    /// mesh build is the cheapest large win on the CPU side.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_render(
        &mut self,
        height: &HeightGrid,
        camera_target: Vec3,
        cull_radius: f32,
        camera_right: Vec3,
        camera_up: Vec3,
        camera_forward: Vec3,
    ) {
        let shadows = Instant::now();
        self.rebuild_shadows(height, camera_target, cull_radius);
        self.last_shadow_ms = shadows.elapsed().as_secs_f32() * 1000.0;
        let mesh = Instant::now();
        self.rebuild_mesh(
            height,
            camera_target,
            cull_radius,
            camera_right,
            camera_up,
            camera_forward,
        );
        self.last_mesh_ms = mesh.elapsed().as_secs_f32() * 1000.0;
    }

    /// True when a world position is inside the team's line of sight.
    pub fn in_team_vision(&self, pos: Vec3) -> bool {
        let tx = (pos.x / 128.0).floor() as i32 - self.vision_origin.0;
        let ty = (pos.z / 128.0).floor() as i32 - self.vision_origin.1;
        self.visibility.is_visible(tx, ty)
    }

    pub fn visible_enemy_count(&self) -> usize {
        // Structures are always visible, so only count mobile enemies here.
        self.units
            .iter()
            .filter(|u| {
                u.team != 0
                    && u.health > 0.0
                    && u.visible
                    && !u.kind.is_structure()
                    && u.kind != UnitKind::Watchtower
                    && u.kind != UnitKind::Ambient
            })
            .count()
    }

    fn team_damage_mult(&self, team: u8) -> f32 {
        self.team_buffs
            .get(team as usize)
            .and_then(|b| *b)
            .map(|b| b.damage_mult)
            .unwrap_or(1.0)
    }

    fn team_speed_mult(&self, team: u8) -> f32 {
        self.team_buffs
            .get(team as usize)
            .and_then(|b| *b)
            .map(|b| b.speed_mult)
            .unwrap_or(1.0)
    }

    pub fn team_buff(&self, team: u8) -> Option<(f32, f32, CampBuff)> {
        self.team_buffs
            .get(team as usize)
            .and_then(|b| *b)
            .map(|b| (b.remaining, b.duration, b.kind))
    }

    fn update_projectiles(&mut self, dt: f32, height: &HeightGrid) {
        let projectiles = std::mem::take(&mut self.projectiles);
        let mut survivors = Vec::with_capacity(projectiles.len());

        for mut projectile in projectiles {
            let look = *fx::look(projectile.look);
            projectile.age += dt;
            projectile.life -= dt;

            // Homing basic attacks steer onto the target every frame and
            // land when this frame's step reaches it, so they never miss.
            if let Some(ti) = projectile.target {
                if self.units.get(ti).is_none_or(|unit| unit.health <= 0.0) {
                    self.projectile_fizzle(&projectile);
                    continue;
                }
                let aim = self.unit_chest(ti);
                let to = aim - projectile.pos;
                let distance = to.length();
                if distance > 1.0 {
                    projectile.dir = to / distance;
                }
                let step = projectile.speed * dt;
                if distance <= step + 6.0 {
                    projectile.pos = aim;
                    projectile.travelled = projectile.flight;
                    self.apply_projectile_hit(&projectile, ti);
                    self.projectile_impact(&projectile, aim);
                    continue;
                }
                projectile.pos += projectile.dir * step;
                projectile.travelled += step;
                // A long chase stretches the lob instead of snapping it.
                projectile.flight = projectile
                    .flight
                    .max(projectile.travelled + distance - step);
                if projectile.life <= 0.0 {
                    continue;
                }
            } else {
                let step = projectile.speed * dt;
                projectile.pos += projectile.dir * step;
                projectile.travelled += step;
                if projectile.life <= 0.0
                    || projectile.pos.y <= height.height_at(projectile.pos.x, projectile.pos.z)
                {
                    self.projectile_fizzle(&projectile);
                    continue;
                }
            }

            let head = projectile.draw_pos();
            advance_trail(
                &mut projectile.trail,
                head,
                look.trail_points,
                projectile.speed,
            );
            projectile.ember_timer -= dt;
            if look.embers > 0.0 && projectile.ember_timer <= 0.0 {
                projectile.ember_timer = 1.0 / look.embers;
                let pos = projectile.draw_pos();
                let dir = projectile.dir;
                self.shed_ember(pos, dir, &look);
            }

            if projectile.target.is_some() {
                survivors.push(projectile);
                continue;
            }

            let mut hits: Vec<usize> = Vec::new();
            for (i, unit) in self.units.iter().enumerate() {
                if unit.team == projectile.team
                    || unit.health <= 0.0
                    || (unit.team == 2 && unit.kind != UnitKind::Camp)
                    || projectile.hit.contains(&i)
                    || (i == self.player_index && self.player_untargetable())
                {
                    continue;
                }
                let radius = unit.hit_radius.max(70.0);
                let center = unit.pos + Vec3::Y * (unit.bar_height * 0.35);
                if projectile.pos.distance(center) > radius {
                    continue;
                }
                hits.push(i);
                if !projectile.pierce {
                    break;
                }
            }
            let hit_count = hits.len();
            for index in hits {
                projectile.hit.push(index);
                self.apply_projectile_hit(&projectile, index);
                self.projectile_impact(&projectile, projectile.pos);
            }
            if hit_count == 0 || projectile.pierce {
                survivors.push(projectile);
            }
        }
        self.projectiles = survivors;
    }

    /// Damage, venom and execute from a projectile reaching `index`.
    fn apply_projectile_hit(&mut self, projectile: &Projectile, index: usize) {
        if index == self.player_index && self.player_untargetable() {
            return;
        }
        let mut damage = projectile.damage;
        {
            let unit = &self.units[index];
            if projectile.execute > 0.0 && unit.health < unit.max_health * 0.25 {
                damage *= 1.0 + projectile.execute;
            }
        }
        if projectile.venom > 0.0 {
            let poisoned = &mut self.units[index];
            if poisoned.venom_timer <= 0.0 {
                poisoned.venom_tick = 1.0;
            }
            poisoned.venom_timer = 6.0;
            poisoned.venom_damage = poisoned.venom_damage.max(projectile.venom);
            poisoned.venom_team = projectile.team;
        }
        let damage = damage * self.kit_projectile_taken(index);
        let splashed = damage > 0.0 && self.units[index].shield > 0.0;
        self.damage_unit_by(index, damage, projectile.team, projectile.attacker);
        if let Some(hit) = projectile.spell.as_deref()
            && self.units[index].health > 0.0
        {
            let caster = projectile.attacker.unwrap_or(self.player_index);
            self.land_spell(index, hit, splashed, caster);
        }
    }

    /// Flash, shock ring, sparks, light and the look's impact model.
    fn projectile_impact(&mut self, projectile: &Projectile, pos: Vec3) {
        let look = *fx::look(projectile.look);
        self.impact_burst(pos, &look);
        if projectile.venom > 0.0 {
            let ground = Vec3::new(pos.x, pos.y - 60.0, pos.z);
            self.add_decal(ground, 70.0, [34, 78, 44], 3.2);
        }
    }

    /// The shared impact for a look, also used by kit shots.
    fn impact_burst(&mut self, pos: Vec3, look: &fx::ProjectileLook) {
        self.effects.push(Effect {
            pos,
            radius: look.impact_radius,
            remaining: 0.45,
            duration: 0.45,
            color: look.impact_color,
        });
        self.emit_burst(
            pos,
            look.impact_color,
            look.impact_sparks,
            look.impact_radius * 1.5,
            (look.glow_size * 0.18).clamp(6.0, 22.0),
            0.5,
        );
        if let Some(id) = look.hit_spotanim
            && self.spot_models.contains_key(&id)
            && self.spot_bursts.len() < 32
        {
            let yaw = self.next_random() * std::f32::consts::TAU;
            self.spot_bursts.push(SpotBurst {
                spotanim: id,
                pos,
                yaw,
                time_ms: 0.0,
                scale: look.hit_scale.unwrap_or(look.model_scale),
                mode: SpotDraw::Burst,
                follow: None,
                hold: 0.0,
                seen_by_all: false,
            });
        }
    }

    /// A shot that ran out or hit the ground dissolves into a small puff.
    fn projectile_fizzle(&mut self, projectile: &Projectile) {
        let look = *fx::look(projectile.look);
        let pos = projectile.draw_pos();
        self.effects.push(Effect {
            pos,
            radius: look.glow_size * 0.8,
            remaining: 0.22,
            duration: 0.22,
            color: look.glow,
        });
        self.emit_burst(pos, look.ember_color, 4, 120.0, look.glow_size * 0.12, 0.35);
    }

    /// One drifting ember behind a projectile.
    fn shed_ember(&mut self, pos: Vec3, dir: Vec3, look: &fx::ProjectileLook) {
        if self.particles.len() >= 300 {
            return;
        }
        let jitter = Vec3::new(
            self.next_random() - 0.5,
            self.next_random() - 0.3,
            self.next_random() - 0.5,
        ) * 90.0;
        let life = 0.25 + self.next_random() * 0.25;
        let spread = look.glow_size * 0.35;
        let offset = Vec3::new(
            self.next_random() - 0.5,
            self.next_random() - 0.5,
            self.next_random() - 0.5,
        ) * spread;
        self.particles.push(Particle {
            pos: pos + offset,
            vel: -dir * 60.0 + jitter,
            color: look.ember_color,
            size: (look.glow_size * 0.14).clamp(4.0, 18.0),
            remaining: life,
            duration: life,
            gravity: 40.0,
            drag: 2.4,
        });
    }

    fn spawn_wave(&mut self, team: u8, lane_index: usize, height: &HeightGrid) {
        if self.match_over.is_some() {
            return;
        }
        // Living minions only: units are never removed, and counting the
        // dead stopped every lane's waves for good about two minutes in.
        if self
            .units
            .iter()
            .filter(|u| u.is_minion_of(team) && u.health > 0.0)
            .count()
            >= MAX_MINIONS_PER_TEAM
        {
            return;
        }
        let Some(lane_waypoints) = self.lanes.get(lane_index).cloned() else {
            return;
        };

        let nexus = self.nexus_positions[team as usize];
        let lane = if team == 0 {
            lane_waypoints
        } else {
            let mut reversed = lane_waypoints;
            reversed.reverse();
            reversed
        };
        // Spawns follow the first lane segment so the three lanes leave the
        // nexus in different directions instead of all stacking on the same
        // tile. Units are then arranged in a loose wedge: two melee in front,
        // the ranged minion behind, and a super minion ahead of the formation.
        let first = lane.first().copied().unwrap_or(nexus);
        let next = lane.get(1).copied().unwrap_or(first + Vec3::Z);
        let mut forward = Vec3::new(next.x - first.x, 0.0, next.z - first.z);
        if forward.length_squared() < 1.0 {
            forward = Vec3::Z;
        } else {
            forward = forward.normalize();
        }
        let across = Vec3::new(-forward.z, 0.0, forward.x);
        let serial = self.wave_serial;
        self.wave_serial = self.wave_serial.wrapping_add(1);

        let enemy_team = 1 - team;
        let super_lane = self.units.iter().any(|u| {
            u.kind == UnitKind::Inhibitor
                && u.team == enemy_team
                && u.structure_lane == Some(lane_index)
                && u.health <= 0.0
        });
        let formation = wave_formation_positions(nexus, forward, across, serial, super_lane);

        for (index, (mut pos, lane_offset, ranged, super_minion)) in
            formation.into_iter().enumerate()
        {
            let spawn_delay = index as f32 * 0.35;
            let unit_lane = offset_lane(&lane, lane_offset * 128.0);
            let template = if ranged {
                &self.ranged_template
            } else {
                &self.melee_template
            };
            let health = if super_minion {
                template.health * 2.5
            } else {
                template.health
            };
            let damage = if super_minion {
                template.damage * 2.0
            } else {
                template.damage
            };
            let scale = if super_minion {
                template.scale * 1.6
            } else {
                template.scale
            };
            let speed = if super_minion {
                template.speed * 0.9
            } else {
                template.speed
            };
            pos.y = height.height_at(pos.x, pos.z);
            let walk_loop_ms = template.walk.loop_ms();
            let walk_stride = speed * walk_loop_ms / 1000.0;
            let unit = AnimatedUnit {
                model: template.model.clone(),
                idle: template.idle.clone(),
                walk: Some(template.walk.clone()),
                pose: Vec::new(),
                pose_scratch: AnimationScratch::default(),
                pos,
                yaw: if team == 0 { 0.0 } else { std::f32::consts::PI },
                model_yaw_offset: template.model_yaw,
                motion: None,
                walk_blend: 0.0,
                walk_arm: 1.0,
                walk_loop_ms,
                walk_stride,
                scale,
                anim_time_ms: 0.0,
                moving: false,
                kind: if ranged {
                    UnitKind::RangedMinion
                } else {
                    UnitKind::MeleeMinion
                },
                team,
                health,
                max_health: health,
                hit_flash: 0.0,
                death_anim: 0.0,
                dead_timer: 0.0,
                target: None,
                path: Vec::new(),
                path_index: 0,
                attack_target: None,
                attack_cooldown: 0.0,
                attack_windup_timer: 0.0,
                attack_windup: 0.0,
                attack_pending: false,
                attack_range: template.range,
                attack_damage: damage,
                attack_interval: template.interval,
                speed,
                armor: 0.0,
                armor_reduction: 0.0,
                armor_reduction_timer: 0.0,
                stun_timer: 0.0,
                venom_timer: 0.0,
                venom_damage: 0.0,
                venom_tick: 0.0,
                venom_team: 0,
                attack_speed_mult: 1.0,
                talent: crate::talents::TalentMods::default(),
                attack_speed_timer: 0.0,
                ranged: 0.0,
                is_champion: false,
                spawn_delay,
                hit_radius: if super_minion {
                    140.0
                } else if ranged {
                    90.0
                } else {
                    105.0
                },
                bar_height: if super_minion { 450.0 } else { 330.0 },
                lane: unit_lane.clone(),
                lane_index: 1.min(unit_lane.len()),
                respawn_time: 0.0,
                camp_buff: None,
                camp_boss: false,
                camp_name: None,
                mana: 0.0,
                max_mana: 0.0,
                level: 1,
                xp: 0.0,
                gold: 0,
                shield: 0.0,
                shield_timer: 0.0,
                cooldowns: [0.0; 4],
                visible: true,
                ranged_auto: false,
                q_damage: 0.0,
                r_damage: 0.0,
                e_heal: 0.0,
                e_shield: 0.0,
                w_distance: 0.0,
                ability_power: 0.0,
                structure_lane: None,
                super_minion,
                home: pos,
                aggro_range: 0.0,
                leash_range: 0.0,
                camp_aggro: false,
                camp_wander_timer: 0.0,
                ambient_kind: None,
                ambient_seed: 0,
                lock_range: 0.0,
                cc: Default::default(),
                scroll: Default::default(),
                spells: Default::default(),
                vengeance: None,
                attack_clip: template.attack.clone(),
                cast_clip: None,
                attack_hit: 0.5,
                action: None,
                strike: None,
                look: if team == 0 {
                    fx::look_by_name("minion_blue")
                } else {
                    fx::look_by_name("minion_red")
                },
                body_radius: if super_minion { 55.0 } else { 40.0 },
            };
            self.place_minion(unit);
        }
    }

    /// Put a new minion in the slot of a long-dead one when there is one, so
    /// the unit list stops growing with every wave; otherwise append it.
    /// Indices of other units stay stable; stale targets on the slot are
    /// cleared first.
    fn place_minion(&mut self, unit: AnimatedUnit) {
        let slot = self.units.iter().position(|u| {
            u.kind.is_minion() && u.health <= 0.0 && u.death_anim <= 0.0 && u.dead_timer <= 0.0
        });
        let Some(slot) = slot else {
            self.units.push(unit);
            return;
        };
        for other in &mut self.units {
            if other.attack_target == Some(slot) {
                other.attack_target = None;
            }
            if other.strike.is_some_and(|strike| strike.target == slot) {
                other.strike = None;
            }
        }
        self.projectiles
            .retain(|p| p.target != Some(slot) && p.attacker != Some(slot));
        self.units[slot] = unit;
    }

    fn rebuild_shadows(&mut self, height: &HeightGrid, camera_target: Vec3, cull_radius: f32) {
        self.shadow_vertices.clear();
        self.shadow_indices.clear();
        for unit in &self.units {
            if unit.health <= 0.0
                || !unit.visible
                || unit.spawn_delay > 0.0
                || !in_mesh_range(unit.pos, camera_target, cull_radius)
            {
                continue;
            }
            let size = (unit.hit_radius * 1.15).clamp(55.0, 480.0);
            let y = height.height_at(unit.pos.x, unit.pos.z) + 4.0;
            let base = self.shadow_vertices.len() as u32;
            let p = |x: f32, z: f32, uv: [f32; 2]| SceneVertex::blob(Vec3::new(x, y, z), 0.38, uv);
            self.shadow_vertices
                .push(p(unit.pos.x - size, unit.pos.z - size, [0.0, 0.0]));
            self.shadow_vertices
                .push(p(unit.pos.x + size, unit.pos.z - size, [1.0, 0.0]));
            self.shadow_vertices
                .push(p(unit.pos.x - size, unit.pos.z + size, [0.0, 1.0]));
            self.shadow_vertices
                .push(p(unit.pos.x + size, unit.pos.z + size, [1.0, 1.0]));
            self.shadow_indices.extend_from_slice(&[
                base,
                base + 1,
                base + 2,
                base + 2,
                base + 1,
                base + 3,
            ]);
        }
    }

    pub fn lights(&self) -> Vec<SceneLight> {
        let mut lights = Vec::new();
        lights.push(SceneLight {
            pos: self.player_spawn + Vec3::Y * 60.0,
            color: [0.25, 1.0, 0.45],
            radius: 800.0,
            intensity: 0.35,
        });
        for effect in &self.effects {
            let color = [
                effect.color[0] as f32 / 255.0,
                effect.color[1] as f32 / 255.0,
                effect.color[2] as f32 / 255.0,
            ];
            let fade = (effect.remaining / effect.duration.max(0.01)).clamp(0.0, 1.0);
            lights.push(SceneLight {
                pos: effect.pos + Vec3::Y * 40.0,
                color,
                radius: (effect.radius * 3.0).max(220.0),
                intensity: 1.2 * fade,
            });
        }
        let shots = self
            .projectiles
            .iter()
            .map(|projectile| (projectile.draw_pos(), *fx::look(projectile.look)))
            .chain(self.kit_shot_lights());
        for (pos, look) in shots {
            // A dark iron anchor carries no light of its own.
            if !self.in_team_vision(pos) || look.light_radius <= 0.0 {
                continue;
            }
            lights.push(SceneLight {
                pos: pos + Vec3::Y * 20.0,
                color: look.light,
                radius: look.light_radius,
                intensity: 1.5,
            });
        }
        for impact in &self.pending_impacts {
            let progress = 1.0 - (impact.delay / impact.duration.max(0.01)).clamp(0.0, 1.0);
            lights.push(SceneLight {
                pos: impact.pos + Vec3::Y * 80.0,
                color: [
                    impact.color[0] as f32 / 255.0,
                    impact.color[1] as f32 / 255.0,
                    impact.color[2] as f32 / 255.0,
                ],
                radius: impact.radius * 3.0,
                intensity: 0.5 + progress * 2.4,
            });
        }
        lights
    }

    /// A projectile: trail ribbon, layered glow and its cache model; a
    /// roped look (the Bloat's anchor) instead draws its model and the rope
    /// back to its caster.
    fn draw_shot(&mut self, shot: &ShotVisual, right: Vec3, up: Vec3, forward: Vec3) {
        let look = fx::look(shot.look).tinted(shot.tint);
        if let Some(rope) = look.rope {
            let heading = Vec3::new(shot.dir.x, 0.0, shot.dir.z).normalize_or(Vec3::Z);
            if let Some(model) = look.model.and_then(|id| self.look_models.get(&id)).cloned() {
                // Flat, front leading, with a slight wobble in flight.
                let yaw = heading.x.atan2(heading.z) - look.model_yaw;
                let wobble = 0.12 * (shot.age * 9.0).sin();
                let rot = Mat3::from_rotation_y(yaw) * Mat3::from_rotation_x(wobble);
                append_banana_model(self, &model, shot.pos, rot, look.model_scale);
            }
            if let Some(from) = shot.rope_from {
                let tail = shot.pos - heading * (look.model_tail * look.model_scale);
                draw_rope(self, from, tail, rope, forward);
            }
            return;
        }
        // Trails start at the head so the ribbon never lags the model.
        let mut trail = Vec::with_capacity(shot.trail.len() + 1);
        trail.push(shot.pos);
        trail.extend(shot.trail.iter().skip(1).copied());
        if trail.len() >= 2 {
            self.fx
                .ribbon(&trail, look.trail_width, look.trail, 1.8, forward);
            self.fx.ribbon(
                &trail,
                look.trail_width * 0.35,
                [255, 255, 255],
                0.45,
                forward,
            );
        }
        let flicker = 1.0 + 0.12 * (shot.age * 41.0).sin() + 0.06 * (shot.age * 97.0).sin();
        self.fx.glow(
            shot.pos,
            look.glow_size * 1.7,
            look.glow,
            0.45 * look.glow_intensity,
            right,
            up,
        );
        self.fx.glow(
            shot.pos,
            look.glow_size * 0.75 * flicker,
            look.glow,
            look.glow_intensity * flicker,
            right,
            up,
        );
        if let Some(spot) = look
            .spotanim
            .and_then(|id| self.spot_models.get(&id))
            .cloned()
        {
            let basis = flight_basis(shot.dir, shot.age * look.spin);
            self.append_spot_model(
                &spot,
                shot.age * 1000.0,
                shot.pos,
                basis,
                look.model_scale,
                1.0,
                SpotDraw::Flying,
            );
        }
    }

    /// Draw a spot animation model. Opaque faces join the lit scene;
    /// translucent faces (spell shells) are added to the glow pass.
    #[allow(clippy::too_many_arguments)]
    fn append_spot_model(
        &mut self,
        spot: &SpotModel,
        time_ms: f32,
        pos: Vec3,
        basis: Mat3,
        scale: f32,
        glow: f32,
        mode: SpotDraw,
    ) {
        let looping = mode.looping();
        let time = if looping {
            time_ms.rem_euclid(spot.length_ms.max(1.0))
        } else {
            time_ms.min(spot.length_ms - 1.0).max(0.0)
        };
        let positions = match spot.clip.as_ref() {
            Some(clip) => spot.model.animated_positions(clip, time),
            None => spot
                .model
                .animated_positions(&AnimationClip::default(), 0.0),
        };
        let [cx, cy, cz] = if mode.grounded() {
            [0.0; 3]
        } else {
            spot.center
        };
        let sxz = spot.scale_xz * scale;
        let sy = spot.scale_y * scale;
        let place = |i: u32| -> Vec3 {
            let p = positions.get(i as usize).copied().unwrap_or([cx, cy, cz]);
            pos + basis * Vec3::new((p[0] - cx) * sxz, -(p[1] - cy) * sy, (p[2] - cz) * sxz)
        };
        for (face, &[a, b, c]) in spot.model.faces.iter().enumerate() {
            let (pa, pb, pc) = (place(a), place(b), place(c));
            let rgb = spot.rgb.get(face).copied().unwrap_or([255, 255, 255]);
            let alpha = spot
                .model
                .face_alphas
                .as_ref()
                .and_then(|alphas| alphas.get(face))
                .copied()
                .unwrap_or(0);
            if alpha > 0 || mode == SpotDraw::Burst {
                let opacity = 1.0 - alpha as f32 / 255.0;
                self.fx.triangle([pa, pb, pc], rgb, opacity * 1.5 * glow);
                continue;
            }
            let normal = (pb - pa).cross(pc - pa).normalize_or_zero();
            let (va, vb, vc) = if mode.grounded() {
                // Spell and status graphics stand in the world (claws, a
                // pillar of fire, an ice block): flat-shaded per face with
                // the scene, the way the cache draws them, so a solid shape
                // keeps its form instead of reading as a flat cut-out.
                // Faces are wound either way, so light whichever side
                // faces up.
                let lit = if normal.y < 0.0 { -normal } else { normal };
                (
                    SceneVertex::new(pa, lit, rgb),
                    SceneVertex::new(pb, lit, rgb),
                    SceneVertex::new(pc, lit, rgb),
                )
            } else {
                // Projectiles read as light sources, so their solid faces
                // are drawn unlit and slightly boosted.
                let lifted = rgb.map(|c| (c as f32 * 1.12).min(255.0) as u8);
                let mut va = SceneVertex::unlit(pa, lifted);
                let mut vb = SceneVertex::unlit(pb, lifted);
                let mut vc = SceneVertex::unlit(pc, lifted);
                for v in [&mut va, &mut vb, &mut vc] {
                    v.normal = normal.to_array();
                }
                (va, vb, vc)
            };
            let base = self.vertices.len() as u32;
            self.vertices.extend_from_slice(&[va, vb, vc]);
            self.indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
    }

    /// Ground indicators: the held right-mouse preview of the last ability
    /// and the brief range flash after every cast.
    fn draw_indicators(&mut self, height: &HeightGrid) {
        let pi = self.player_index;
        if self.units[pi].health <= 0.0 {
            return;
        }
        let origin = self.units[pi].pos;
        let yaw = self.units[pi].yaw;
        let facing = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        if let Some(aim) = self.indicator_aim {
            let slot = self.last_cast_slot.unwrap_or(0);
            let ready = self.units[pi].cooldowns[slot] <= 0.0;
            let mut color = self.ability_color(slot);
            if !ready {
                // On cooldown: same shapes, drained of colour.
                let grey = (color[0] as u32 + color[1] as u32 + color[2] as u32) / 3;
                color = [grey as u8; 3].map(|c| (c as f32 * 0.55 + 40.0) as u8);
            }
            let intensity = if ready { 1.0 } else { 0.55 };
            for indicator in self.ability_indicators(slot) {
                self.draw_indicator(height, &indicator, origin, facing, aim, color, intensity);
            }
        }
        if let Some((slot, remaining)) = self.cast_flash {
            if self.indicator_aim.is_none() {
                let fade = (remaining / 0.45).clamp(0.0, 1.0);
                let color = self.ability_color(slot);
                if let Some(radius) = self
                    .ability_indicators(slot)
                    .iter()
                    .map(indicator_reach)
                    .fold(None, |best: Option<f32>, r| {
                        Some(best.map_or(r, |b| b.max(r)))
                    })
                {
                    self.fx
                        .ground_ring(height, origin, radius, 10.0, color, 0.9 * fade);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_indicator(
        &mut self,
        height: &HeightGrid,
        indicator: &Indicator,
        origin: Vec3,
        facing: Vec3,
        aim: Vec3,
        color: [u8; 3],
        intensity: f32,
    ) {
        let to_aim = planar_delta(origin, aim);
        let aim_dir = if to_aim.length_squared() > 1.0 {
            to_aim.normalize()
        } else {
            facing
        };
        match *indicator {
            Indicator::Range { radius } => {
                self.fx
                    .ground_ring(height, origin, radius, 9.0, color, 0.9 * intensity);
            }
            Indicator::Line {
                length,
                width,
                angle,
            } => {
                let (sin, cos) = angle.sin_cos();
                let dir = Vec3::new(
                    aim_dir.x * cos - aim_dir.z * sin,
                    0.0,
                    aim_dir.x * sin + aim_dir.z * cos,
                );
                self.fx.ground_rect(
                    height,
                    origin,
                    dir,
                    length,
                    width * 0.5,
                    color,
                    0.9 * intensity,
                );
                self.fx.ground_arrow(
                    height,
                    origin + dir * length,
                    dir,
                    (width * 0.9).clamp(40.0, 120.0),
                    color,
                    1.2 * intensity,
                );
            }
            Indicator::Target { range, radius } => {
                self.fx
                    .ground_ring(height, origin, range, 8.0, color, 0.55 * intensity);
                let reach = to_aim.length().min(range);
                let at = origin + aim_dir * reach;
                self.fx
                    .ground_area(height, at, radius, 360.0, aim_dir, color, intensity);
            }
            Indicator::Cone { radius, arc } => {
                self.fx
                    .ground_area(height, origin, radius, arc, aim_dir, color, intensity);
            }
            Indicator::SelfArea { radius } => {
                self.fx
                    .ground_area(height, origin, radius, 360.0, aim_dir, color, intensity);
            }
            Indicator::Dash {
                length,
                backwards,
                to_cursor,
            } => {
                let dir = if backwards { -facing } else { aim_dir };
                let length = if to_cursor {
                    length.min(to_aim.length()).max(64.0)
                } else {
                    length
                };
                self.fx
                    .ground_ring(height, origin, length, 7.0, color, 0.4 * intensity);
                self.fx
                    .ground_rect(height, origin, dir, length, 42.0, color, 0.8 * intensity);
                self.fx.ground_arrow(
                    height,
                    origin + dir * length,
                    dir,
                    90.0,
                    color,
                    1.3 * intensity,
                );
            }
            Indicator::Boomerang { length } => {
                let (out, back) = awowogei::boomerang_preview(origin, aim_dir, length);
                for (path, alpha) in [(out, 1.0), (back, 0.55)] {
                    for pair in path.windows(2) {
                        let delta = planar_delta(pair[0], pair[1]);
                        self.fx.ground_strip(
                            height,
                            pair[0],
                            delta,
                            delta.length(),
                            24.0,
                            color,
                            intensity * alpha,
                        );
                    }
                }
            }
            Indicator::Swing { length } => {
                // The swing always goes its full length; the cursor aims.
                let path = awowogei::swing_preview(origin, aim_dir, length);
                for pair in path.windows(2) {
                    let delta = planar_delta(pair[0], pair[1]);
                    self.fx.ground_strip(
                        height,
                        pair[0],
                        delta,
                        delta.length(),
                        34.0,
                        color,
                        0.85 * intensity,
                    );
                }
                if let [.., before, last] = path.as_slice() {
                    let dir = planar_delta(*before, *last).normalize_or_zero();
                    self.fx
                        .ground_arrow(height, *last, dir, 90.0, color, 1.3 * intensity);
                }
                self.fx
                    .ground_ring(height, origin, length, 7.0, color, 0.4 * intensity);
            }
        }
    }

    fn rebuild_mesh(
        &mut self,
        height: &HeightGrid,
        camera_target: Vec3,
        cull_radius: f32,
        camera_right: Vec3,
        camera_up: Vec3,
        camera_forward: Vec3,
    ) {
        self.vertices.clear();
        self.indices.clear();
        // The sun-shadow caster mesh is rebuilt every frame as well. Without
        // this reset the shadow map kept every previous position of every
        // unit, which stacked shadows into a trail and grew the draw call
        // without bound.
        self.caster_vertices.clear();
        self.caster_indices.clear();
        self.fx.clear();

        let mut units = std::mem::take(&mut self.units);
        let mut mesh_units = 0usize;
        // Indexed so the pose buffer can be borrowed mutably, then the same
        // unit borrowed immutably for append_unit after the animation writes.
        #[allow(clippy::needless_range_loop)]
        for index in 0..units.len() {
            let unit = &units[index];
            let alive = unit.health > 0.0;
            if (alive || unit.death_anim > 0.0)
                && (unit.visible || unit.death_anim > 0.0)
                && unit.spawn_delay <= 0.0
                && in_mesh_range(unit.pos, camera_target, cull_radius)
            {
                mesh_units += 1;
                let action = unit
                    .action
                    .as_ref()
                    .filter(|_| alive)
                    .map(|action| (action.clip.clone(), action.time_ms, action.weight()));
                if let Some((clip, time_ms, weight)) = action.as_ref()
                    && *weight >= 0.999
                {
                    let unit = &mut units[index];
                    unit.model.animated_positions_into(
                        clip,
                        *time_ms,
                        &mut unit.pose,
                        &mut unit.pose_scratch,
                    );
                    append_unit(self, &units[index], &units[index].pose);
                    continue;
                }
                let blends = unit.walk.is_some()
                    && ((unit.kind == UnitKind::Player && unit.walk_arm < 0.999)
                        || (unit.walk_blend > 0.001 && unit.walk_blend < 0.999));
                if !blends {
                    let clip = if !alive || !unit.moving {
                        unit.idle.clone()
                    } else {
                        unit.walk.clone().unwrap_or_else(|| unit.idle.clone())
                    };
                    let unit = &mut units[index];
                    unit.model.animated_positions_into(
                        &clip,
                        unit.anim_time_ms,
                        &mut unit.pose,
                        &mut unit.pose_scratch,
                    );
                    if let Some((clip, time_ms, weight)) = action.as_ref() {
                        let acting = unit.model.animated_positions(clip, *time_ms);
                        blend_pose(&mut unit.pose, &acting, *weight);
                    }
                    append_unit(self, &units[index], &units[index].pose);
                    continue;
                }
                let positions = if !alive {
                    unit.model.animated_positions(&unit.idle, unit.anim_time_ms)
                } else if let Some(walk) = unit.walk.as_ref() {
                    if unit.kind == UnitKind::Player && unit.walk_arm < 0.999 {
                        let idle = unit.model.animated_positions(&unit.idle, unit.anim_time_ms);
                        let normal = unit.model.animated_positions(walk, unit.anim_time_ms);
                        let masked = unit.model.animated_positions_masked(
                            walk,
                            unit.anim_time_ms,
                            MONKEY_ARM_GROUPS,
                        );
                        let arm = unit.walk_arm.clamp(0.0, 1.0);
                        let mut walking = normal.clone();
                        for ((out, normal_pose), masked) in
                            walking.iter_mut().zip(&normal).zip(&masked)
                        {
                            for axis in 0..3 {
                                let rigid = normal_pose[axis]
                                    + (masked[axis] - normal_pose[axis]) * (1.0 - arm);
                                out[axis] = rigid;
                            }
                        }
                        if unit.walk_blend >= 0.999 {
                            walking
                        } else {
                            let blend = unit.walk_blend.clamp(0.0, 1.0);
                            let mut out = idle.clone();
                            for ((out, idle_pose), walking) in
                                out.iter_mut().zip(&idle).zip(&walking)
                            {
                                for axis in 0..3 {
                                    out[axis] =
                                        idle_pose[axis] + (walking[axis] - idle_pose[axis]) * blend;
                                }
                            }
                            out
                        }
                    } else {
                        let clip = if unit.moving { walk } else { &unit.idle };
                        let mut positions = unit.model.animated_positions(clip, unit.anim_time_ms);
                        if unit.walk_blend > 0.001 && unit.walk_blend < 0.999 {
                            let idle = unit.model.animated_positions(&unit.idle, unit.anim_time_ms);
                            let walking = unit.model.animated_positions(walk, unit.anim_time_ms);
                            let blend = unit.walk_blend;
                            for ((out, idle), walking) in
                                positions.iter_mut().zip(&idle).zip(&walking)
                            {
                                for axis in 0..3 {
                                    out[axis] = idle[axis] + (walking[axis] - idle[axis]) * blend;
                                }
                            }
                        }
                        positions
                    }
                } else {
                    unit.model.animated_positions(&unit.idle, unit.anim_time_ms)
                };
                let mut positions = positions;
                if let Some((clip, time_ms, weight)) = action.as_ref() {
                    let acting = unit.model.animated_positions(clip, *time_ms);
                    blend_pose(&mut positions, &acting, *weight);
                }
                append_unit(self, unit, &positions);
            }
        }
        self.units = units;
        self.last_mesh_units = mesh_units;

        // Minions and camps keep cheap world-space bars. Champions,
        // structures and dummies use the readable screen-space plates built
        // by the client, so drawing both would double up the health bars.
        let player_damage = self.units[self.player_index].attack_damage;
        let player_bar_pos = self.units[self.player_index].pos;
        let bars = self
            .units
            .iter()
            .filter(|unit| {
                unit.health > 0.0
                    && unit.spawn_delay <= 0.0
                    && matches!(
                        unit.kind,
                        UnitKind::MeleeMinion | UnitKind::RangedMinion | UnitKind::Camp
                    )
            })
            .map(|unit| {
                let (width, height) = match unit.kind {
                    UnitKind::Nexus => (190.0 * (unit.scale / 2.8).clamp(0.7, 1.5), 14.0),
                    UnitKind::Tower => (150.0 * (unit.scale / 2.8).clamp(0.7, 1.5), 14.0),
                    UnitKind::Player | UnitKind::BotChampion => (160.0, 16.0),
                    UnitKind::Camp => (145.0 * (unit.scale / 1.4).clamp(0.9, 1.3), 20.0),
                    _ => (55.0, 12.0),
                };
                (
                    unit.pos + Vec3::Y * unit.bar_height,
                    width,
                    height,
                    (unit.health / unit.max_health).clamp(0.0, 1.0),
                    if unit.team == 1
                        && unit.kind.is_minion()
                        && unit.health <= player_damage
                        && unit.pos.distance(player_bar_pos) < 1400.0
                    {
                        // Last-hit window: the bar turns white when one
                        // auto-attack would secure the minion.
                        [255, 255, 255]
                    } else if unit.team == 0 {
                        [95u8, 225, 120]
                    } else if unit.team == 2 {
                        [242u8, 64, 52]
                    } else {
                        [235u8, 70, 60]
                    },
                )
            })
            .collect::<Vec<_>>();
        for (center, width, height, fill, color) in bars {
            append_bar(
                self,
                center,
                width,
                height,
                fill,
                color,
                camera_right,
                camera_up,
                camera_forward,
            );
        }

        // Attacks are only drawn inside the team's line of sight, so enemy
        // tower bolts do not reveal themselves through the fog of war.
        let mut shots = self
            .projectiles
            .iter()
            .filter(|projectile| self.in_team_vision(projectile.pos))
            .map(|projectile| ShotVisual {
                pos: projectile.draw_pos(),
                dir: projectile.dir,
                look: projectile.look,
                trail: projectile.trail.clone(),
                age: projectile.age,
                tint: None,
                rope_from: None,
            })
            .collect::<Vec<_>>();
        shots.extend(self.kit_shot_visuals());
        for shot in &shots {
            self.draw_shot(shot, camera_right, camera_up, camera_forward);
        }

        // Sparks and embers glow additively and fade by dimming.
        let particles = self
            .particles
            .iter()
            .filter(|p| self.in_team_vision(p.pos))
            .map(|p| {
                let t = (p.remaining / p.duration.max(0.01)).clamp(0.0, 1.0);
                (
                    p.pos,
                    p.size * (0.55 + 0.6 * t),
                    p.color,
                    1.3 * t * t.sqrt(),
                )
            })
            .collect::<Vec<_>>();
        for (pos, size, color, intensity) in particles {
            self.fx
                .glow(pos, size, color, intensity, camera_right, camera_up);
        }

        // Melee streaks: a bright arc swept across the struck unit.
        let slashes = self
            .slashes
            .iter()
            .map(|slash| {
                let t = 1.0 - (slash.remaining / slash.duration.max(0.01)).clamp(0.0, 1.0);
                (slash.pos, slash.dir, slash.size, slash.color, t)
            })
            .collect::<Vec<_>>();
        for (pos, dir, size, color, t) in slashes {
            let side = Vec3::Y.cross(dir).normalize_or_zero();
            let sweep = (t * 1.8).min(1.0);
            let points = (0..9)
                .map(|i| {
                    let a = 1.2 - 2.4 * sweep * (i as f32 / 8.0);
                    pos + (side * a.cos() + Vec3::Y * (a.sin() * 0.7) + dir * 0.15) * size * 0.5
                })
                .collect::<Vec<_>>();
            let fade = (1.0 - t).powf(1.3);
            self.fx
                .ribbon(&points, size * 0.09, color, 2.2 * fade, camera_forward);
            self.fx
                .glow(pos, size * 0.35, color, 0.9 * fade, camera_right, camera_up);
        }

        // Impact splashes, spell graphics and held status graphics.
        let bursts = self
            .spot_bursts
            .iter()
            .filter_map(|burst| {
                let pos = match burst.follow {
                    Some(index) => {
                        let unit = self.units.get(index)?;
                        if !unit.visible {
                            return None;
                        }
                        unit.pos + burst.pos
                    }
                    None => burst.pos,
                };
                if !burst.seen_by_all && !self.in_team_vision(pos) {
                    return None;
                }
                let spot = self.spot_models.get(&burst.spotanim)?.clone();
                Some((spot, pos, burst))
            })
            .map(|(spot, pos, burst)| {
                (spot, pos, burst.yaw, burst.time_ms, burst.scale, burst.mode)
            })
            .collect::<Vec<_>>();
        for (spot, pos, yaw, time_ms, scale, mode) in bursts {
            let basis = Mat3::from_rotation_y(yaw);
            if mode == SpotDraw::Burst {
                let fade = (1.0 - time_ms / spot.length_ms.max(1.0)).clamp(0.0, 1.0);
                // Impact models are drawn entirely as light: their solid
                // faces read as flat cardboard once the splash grows past
                // the target.
                let grow = 0.7 + 0.5 * (time_ms / spot.length_ms.max(1.0)).min(1.0);
                self.append_spot_model(
                    &spot,
                    time_ms,
                    pos,
                    basis,
                    scale * 0.75 * grow,
                    1.1 * fade,
                    SpotDraw::Burst,
                );
            } else {
                self.append_spot_model(&spot, time_ms, pos, basis, scale, 1.0, mode);
            }
        }

        self.draw_awowogei(height, camera_right, camera_up);

        let player = &self.units[self.player_index];
        let player_pos = player.pos;
        let player_range = player.attack_range;
        let player_target = player.attack_target;
        let attack_pending = player.attack_pending;
        let attack_windup = player.attack_windup;
        let windup_timer = player.attack_windup_timer;
        if player.health > 0.0 {
            append_ring(self, height, player_pos, 95.0, [90, 180, 255]);
        }
        // Attack range and target rings, like the reference client shows while
        // the champion is fighting.
        if player_target.is_some() {
            append_ring_width(self, height, player_pos, player_range, [220, 60, 50], 0.975);
            if let Some(target) = player_target {
                let target_pos = self.units[target].pos;
                append_ring_width(self, height, target_pos, 72.0, [250, 220, 90], 0.86);
            }
        }
        if attack_pending {
            let progress = 1.0 - (windup_timer / attack_windup.max(0.001)).clamp(0.0, 1.0);
            append_ring(
                self,
                height,
                player_pos,
                100.0 + 55.0 * progress,
                [255, 150, 55],
            );
        }

        // Small team-coloured rings under lane imps/fairies mirror the Dota
        // style reference and make the two sides readable at a glance.
        let minion_rings = self
            .units
            .iter()
            .filter(|u| u.visible && u.kind.is_minion() && u.health > 0.0)
            .map(|u| (u.pos, u.team))
            .collect::<Vec<_>>();
        for (pos, team) in minion_rings {
            append_ring(
                self,
                height,
                pos,
                46.0,
                if team == 0 {
                    [90, 160, 255]
                } else {
                    [255, 90, 80]
                },
            );
        }

        let aggro_minions = self
            .units
            .iter()
            .filter(|u| {
                u.visible
                    && u.kind.is_minion()
                    && u.health > 0.0
                    && u.team == 1
                    && u.attack_target == Some(self.player_index)
            })
            .map(|u| u.pos)
            .collect::<Vec<_>>();
        for pos in aggro_minions {
            append_ring_width(self, height, pos, 66.0, [255, 70, 55], 0.92);
        }

        let structure_rings = self
            .units
            .iter()
            .filter(|u| {
                u.visible
                    && matches!(u.kind, UnitKind::Tower | UnitKind::Inhibitor)
                    && u.health > 0.0
            })
            .map(|u| (u.pos, u.team))
            .collect::<Vec<_>>();
        for (pos, team) in structure_rings {
            append_ring(
                self,
                height,
                pos,
                140.0,
                if team == 0 {
                    [90, 160, 255]
                } else {
                    [255, 90, 80]
                },
            );
        }

        let tower_danger = self
            .units
            .iter()
            .filter(|u| {
                u.visible
                    && u.kind == UnitKind::Tower
                    && u.team == 1
                    && u.health > 0.0
                    && u.pos.distance(player_pos) < u.attack_range * 1.35
            })
            .map(|u| {
                (
                    u.pos,
                    u.attack_range,
                    u.attack_target == Some(self.player_index),
                )
            })
            .collect::<Vec<_>>();
        for (pos, range, aggro) in tower_danger {
            append_ring_width(
                self,
                height,
                pos,
                range,
                if aggro { [255, 60, 45] } else { [190, 90, 70] },
                0.985,
            );
        }

        let camp_rings = self
            .units
            .iter()
            .filter(|u| u.visible && u.kind == UnitKind::Camp && u.health > 0.0)
            .map(|u| u.pos)
            .collect::<Vec<_>>();
        for pos in camp_rings {
            append_ring(self, height, pos, 120.0, [230, 180, 70]);
        }
        if !self.delivery_pending.is_empty() {
            if let Some(chest) = self.delivery_chest.and_then(|index| self.units.get(index)) {
                append_ring(self, height, chest.pos, 150.0, [255, 210, 80]);
            }
        }

        // Lingering ground stains under the effects layer.
        let decals = self
            .decals
            .iter()
            .map(|d| {
                let t = (d.remaining / d.duration.max(0.01)).clamp(0.0, 1.0);
                let color = [
                    (d.color[0] as f32 * (0.35 + t * 0.65)) as u8,
                    (d.color[1] as f32 * (0.35 + t * 0.65)) as u8,
                    (d.color[2] as f32 * (0.35 + t * 0.65)) as u8,
                ];
                (d.pos, d.radius, color)
            })
            .collect::<Vec<_>>();
        for (pos, radius, color) in decals {
            append_ring_width(self, height, pos, radius, color, 0.22);
        }

        // Telegraphed impacts: a pulsing danger circle plus the incoming rock.
        let impacts = self
            .pending_impacts
            .iter()
            .map(|i| {
                let progress = 1.0 - (i.delay / i.duration.max(0.01)).clamp(0.0, 1.0);
                let pulse = 0.72 + (i.delay * 18.0).sin().abs() * 0.28;
                (i.pos, i.radius, progress, pulse)
            })
            .collect::<Vec<_>>();
        for (pos, radius, progress, pulse) in impacts {
            append_ring(self, height, pos, radius, [255, 80, 35]);
            append_ring_width(self, height, pos, radius * pulse, [255, 150, 60], 0.60);
            append_ring_width(
                self,
                height,
                pos,
                radius * (0.22 + progress * 0.68),
                [255, 210, 120],
                0.86,
            );
            let fall = 1500.0 * (1.0 - progress);
            let meteor = pos + Vec3::Y * (fall + 140.0);
            append_billboard(self, meteor, 96.0, [255, 130, 45], camera_right, camera_up);
            append_billboard(self, meteor, 58.0, [255, 235, 170], camera_right, camera_up);
        }

        // Effects: an expanding soft shock ring on the ground and a flash
        // that blooms at the start.
        let effects = self
            .effects
            .iter()
            .filter(|e| self.in_team_vision(e.pos))
            .map(|e| {
                let t = (1.0 - e.remaining / e.duration.max(0.01)).clamp(0.0, 1.0);
                (e.pos, e.radius, e.color, t)
            })
            .collect::<Vec<_>>();
        for (pos, radius, color, t) in effects {
            let ease = 1.0 - (1.0 - t) * (1.0 - t);
            let ring = radius * (0.30 + 0.75 * ease);
            let fade = (1.0 - t).powf(1.2);
            self.fx.ground_ring(
                height,
                pos,
                ring,
                (radius * 0.07).clamp(8.0, 40.0) * (1.0 - 0.5 * t),
                color,
                1.6 * fade,
            );
            if t < 0.35 {
                let flash = 1.0 - t / 0.35;
                self.fx.glow(
                    pos + Vec3::Y * 30.0,
                    (radius * 0.55).clamp(30.0, 260.0),
                    color,
                    1.4 * flash,
                    camera_right,
                    camera_up,
                );
            }
        }

        self.draw_indicators(height);

        let shield_pos = self.units[self.player_index].pos;
        let shield = self.units[self.player_index].shield;
        if shield > 0.0 {
            append_ring(self, height, shield_pos, 120.0, [90, 255, 140]);
        }

        if let Some((marker, remaining)) = self.marker {
            let pulse = (remaining * 6.0).sin().abs() * 0.25 + 0.75;
            append_ring(self, height, marker, 55.0 * pulse, [120, 255, 120]);
        }
    }
}

impl AnimatedUnit {
    /// Ranged level scaling, matching the OSRS stat panel.
    fn ranged_scale(&self) -> f32 {
        1.0 + self.ranged / 100.0
    }

    fn is_minion_of(&self, team: u8) -> bool {
        self.kind.is_minion() && self.team == team
    }
}

fn los_clear(collision: &WorldCollision, ox: i32, oy: i32, tx: i32, ty: i32) -> bool {
    let dx = (tx - ox).abs();
    let dy = (ty - oy).abs();
    let sx = if ox < tx { 1 } else { -1 };
    let sy = if oy < ty { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = ox;
    let mut y = oy;
    loop {
        if (x != ox || y != oy) && (x != tx || y != ty) && collision.blocks_sight_tile(x, y) {
            return false;
        }
        if x == tx && y == ty {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    true
}

fn point_in_brush(brush: &[(f32, f32, f32, f32)], pos: Vec3) -> bool {
    brush.iter().any(|&(x0, z0, x1, z1)| {
        pos.x >= x0.min(x1) && pos.x <= x0.max(x1) && pos.z >= z0.min(z1) && pos.z <= z0.max(z1)
    })
}

/// Sum damage entries by source, largest first, then by name.
/// Where overhead chat is anchored: the unit's plate anchor. The HUD lifts
/// the line above the name in screen space, so it clears the plate at any
/// zoom.
fn say_anchor(unit: &AnimatedUnit) -> Vec3 {
    unit.pos + Vec3::Y * (unit.bar_height + 12.0)
}

/// Grid positions for `count` items centred on `centre`, `spacing` apart,
/// in rows as close to square as the count allows.
pub fn demo_grid(centre: Vec3, count: usize, spacing: f32) -> Vec<Vec3> {
    if count == 0 {
        return Vec::new();
    }
    let columns = (count as f32).sqrt().ceil() as usize;
    let rows = count.div_ceil(columns);
    (0..count)
        .map(|index| {
            let column = (index % columns) as f32 - (columns as f32 - 1.0) * 0.5;
            let row = (index / columns) as f32 - (rows as f32 - 1.0) * 0.5;
            centre + Vec3::new(column * spacing, 0.0, row * spacing)
        })
        .collect()
}

pub fn damage_totals(entries: &[(String, f32)]) -> Vec<(String, u32)> {
    let mut totals: Vec<(String, f32)> = Vec::new();
    for (name, amount) in entries {
        if let Some((_, total)) = totals.iter_mut().find(|(label, _)| label == name) {
            *total += *amount;
        } else {
            totals.push((name.clone(), *amount));
        }
    }
    totals.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    totals
        .into_iter()
        .map(|(name, amount)| (name, amount.round() as u32))
        .collect()
}

/// Experience from `level` to the next. Gentler than the old 11-level
/// curve so a long match reaches the level 25 cap.
fn xp_for_level(level: u32) -> f32 {
    100.0 + (level.saturating_sub(1) as f32) * 40.0
}

/// Stats a hero gains with each level.
pub const LEVEL_HEALTH: f32 = 32.0;
pub const LEVEL_MANA: f32 = 16.0;
pub const LEVEL_DAMAGE: f32 = 1.6;

/// Raise a hero one level: health, mana and attack damage grow.
fn level_up_stats(unit: &mut AnimatedUnit) {
    unit.level += 1;
    unit.max_health += LEVEL_HEALTH;
    unit.health = (unit.health + LEVEL_HEALTH).min(unit.max_health);
    unit.max_mana += LEVEL_MANA;
    unit.mana = (unit.mana + LEVEL_MANA).min(unit.max_mana);
    unit.attack_damage += LEVEL_DAMAGE;
}

/// Experience and gold for the last hit on a `kind` (player and bots).
fn reward_for(kind: UnitKind) -> (f32, u32) {
    match kind {
        UnitKind::MeleeMinion | UnitKind::RangedMinion => (20.0, 10),
        UnitKind::Tower => (100.0, 100),
        UnitKind::Inhibitor => (150.0, 150),
        UnitKind::Camp => (80.0, 50),
        UnitKind::Dummy => (50.0, 20),
        UnitKind::Player | UnitKind::BotChampion => (200.0, 300),
        _ => (0.0, 0),
    }
}

fn find_nexus(units: &[AnimatedUnit], team: u8) -> Option<Vec3> {
    units
        .iter()
        .find(|u| u.kind == UnitKind::Nexus && u.team == team)
        .map(|u| u.pos)
}

/// A champion's attack and cast clips. A clip that fails to load is logged
/// and skipped so a bad id never stops the match from starting.
fn champion_clips(
    assets: &mut UnitAssets<'_>,
    champion: &ChampionConfig,
) -> (Option<Arc<AnimationClip>>, Option<Arc<AnimationClip>>) {
    let mut load = |sequence: Option<u32>| {
        let sequence = sequence?;
        match assets.clip(sequence) {
            Ok(clip) if !clip.frames.is_empty() => Some(clip),
            Ok(_) => {
                tracing::warn!("{}: sequence {sequence} has no frames", champion.name);
                None
            }
            Err(err) => {
                tracing::warn!("{}: sequence {sequence}: {err}", champion.name);
                None
            }
        }
    };
    let attack = load(champion.attack_anim);
    let cast = load(champion.cast_anim).or_else(|| attack.clone());
    (attack, cast)
}

fn load_model(cache: &Cache, model_id: u32) -> Result<Model> {
    let group = cache.group(7, model_id)?;
    Ok(Model::load(model_id, &group.data)?)
}

/// Decoded models and animation clips, memoised by id.
///
/// A scene builds hundreds of units out of a handful of distinct assets:
/// every butterfly, crab, cow, imp and fairy re-decoded the same model and
/// the same animation frames. Decoding one clip costs about 100 ms, so the
/// repeats dominated the ~25 s stall when a match started. The maps are
/// shared behind an `Arc` because nothing mutates them once loaded.
struct UnitAssets<'a> {
    cache: &'a Cache,
    models: HashMap<u32, Arc<Model>>,
    clips: HashMap<u32, Arc<AnimationClip>>,
}

impl<'a> UnitAssets<'a> {
    fn new(cache: &'a Cache) -> Self {
        Self {
            cache,
            models: HashMap::new(),
            clips: HashMap::new(),
        }
    }

    fn model(&mut self, model_id: u32) -> Result<Arc<Model>> {
        if let Some(model) = self.models.get(&model_id) {
            return Ok(Arc::clone(model));
        }
        let model = Arc::new(load_model(self.cache, model_id)?);
        self.models.insert(model_id, Arc::clone(&model));
        Ok(model)
    }

    fn clip(&mut self, sequence: u32) -> Result<Arc<AnimationClip>> {
        if let Some(clip) = self.clips.get(&sequence) {
            return Ok(Arc::clone(clip));
        }
        let clip = Arc::new(AnimationClip::load(self.cache, sequence)?);
        self.clips.insert(sequence, Arc::clone(&clip));
        Ok(clip)
    }

    /// Owned copies of a model and its attachments, ready for `Model::merge`.
    fn merged_parts(&mut self, model_id: u32, attachments: &[u32]) -> Result<Vec<Model>> {
        let mut parts = Vec::with_capacity(1 + attachments.len());
        parts.push((*self.model(model_id)?).clone());
        for &attachment in attachments {
            parts.push((*self.model(attachment)?).clone());
        }
        Ok(parts)
    }
}

/// Pitch (0.92–1.08) and volume (0.75–1.0) for one footstep of unit
/// `index`, from a hash of the unit and its animation time.
fn step_variation(index: usize, anim_ms: f32, k: usize) -> (f32, f32) {
    let mut h = (index as u32).wrapping_mul(0x9E37_79B1)
        ^ (anim_ms as u32).wrapping_mul(0x85EB_CA77)
        ^ (k as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    let a = (h & 0xFFFF) as f32 / 65535.0;
    let b = (h >> 16) as f32 / 65535.0;
    (0.92 + 0.16 * a, 0.75 + 0.25 * b)
}

/// The clips a unit plays most (idle, walk and attack when it has them),
/// used to score how its parts bind to the skeleton.
pub(crate) fn unit_clips(unit: &AnimatedUnit) -> Vec<Arc<AnimationClip>> {
    std::iter::once(Arc::clone(&unit.idle))
        .chain(unit.walk.iter().cloned())
        .chain(unit.attack_clip.iter().cloned())
        .collect()
}

/// Merge a hero's or NPC's parts and bind each one to the skeleton (see
/// `rift_cache::skeleton`): parts on the body's skeleton keep their labels,
/// foreign or unlabelled parts follow the nearest limb, so nothing is left
/// frozen while the body animates.
pub(crate) fn bind_parts(parts: &[Model], clips: &[Arc<AnimationClip>], who: &str) -> Model {
    let refs: Vec<&AnimationClip> = clips.iter().map(|clip| clip.as_ref()).collect();
    let (model, reports) = rift_cache::skeleton::assemble(parts, &refs);
    for report in reports.iter().skip(1) {
        tracing::debug!(
            "skeleton: {who} part {} (model {}) {:?}: drift native {:.3} follow {:.3} frozen {:.3}, tear {:.3}",
            report.index,
            report.model_id,
            report.binding,
            report.native_drift,
            report.follow_drift,
            report.static_drift,
            report.native_tear
        );
    }
    model
}

#[allow(clippy::too_many_arguments)]
fn load_unit(
    assets: &mut UnitAssets<'_>,
    model_id: u32,
    idle_sequence: Option<u32>,
    walk_sequence: Option<u32>,
    pos: Vec3,
    scale: f32,
    kind: UnitKind,
    team: u8,
    health: f32,
    attack_range: f32,
    attack_damage: f32,
    attack_interval: f32,
    speed: f32,
    hit_radius: f32,
    bar_height: f32,
    lane: Vec<Vec3>,
) -> Result<AnimatedUnit> {
    let model = assets.model(model_id)?;
    let idle = match idle_sequence {
        Some(seq) => assets.clip(seq)?,
        None => Arc::new(AnimationClip::default()),
    };
    let walk = match walk_sequence {
        Some(seq) => Some(assets.clip(seq)?),
        None => None,
    };
    Ok(build_unit(
        model,
        idle,
        walk,
        pos,
        scale,
        kind,
        team,
        health,
        attack_range,
        attack_damage,
        attack_interval,
        speed,
        hit_radius,
        bar_height,
        lane,
    ))
}

/// Build a unit from already loaded model and clips. Kit summons use this at
/// cast time, when the cache is no longer at hand.
#[allow(clippy::too_many_arguments)]
fn build_unit(
    model: Arc<Model>,
    idle: Arc<AnimationClip>,
    walk: Option<Arc<AnimationClip>>,
    pos: Vec3,
    scale: f32,
    kind: UnitKind,
    team: u8,
    health: f32,
    attack_range: f32,
    attack_damage: f32,
    attack_interval: f32,
    speed: f32,
    hit_radius: f32,
    bar_height: f32,
    lane: Vec<Vec3>,
) -> AnimatedUnit {
    let walk_loop_ms = walk.as_ref().map_or(0.0, |clip| clip.loop_ms());
    let walk_stride = speed * walk_loop_ms / 1000.0;
    AnimatedUnit {
        model,
        idle,
        walk,
        pose: Vec::new(),
        pose_scratch: AnimationScratch::default(),
        pos,
        yaw: 0.0,
        model_yaw_offset: 0.0,
        motion: None,
        walk_blend: 0.0,
        walk_arm: 1.0,
        walk_loop_ms,
        walk_stride,
        scale,
        anim_time_ms: 0.0,
        moving: false,
        kind,
        team,
        health,
        max_health: health,
        hit_flash: 0.0,
        death_anim: 0.0,
        dead_timer: 0.0,
        target: None,
        path: Vec::new(),
        path_index: 0,
        attack_target: None,
        attack_cooldown: 0.0,
        attack_windup_timer: 0.0,
        attack_windup: 0.0,
        attack_pending: false,
        attack_range,
        attack_damage,
        attack_interval,
        speed,
        armor: 0.0,
        armor_reduction: 0.0,
        armor_reduction_timer: 0.0,
        stun_timer: 0.0,
        venom_timer: 0.0,
        venom_damage: 0.0,
        venom_tick: 0.0,
        venom_team: 0,
        attack_speed_mult: 1.0,
        talent: crate::talents::TalentMods::default(),
        attack_speed_timer: 0.0,
        ranged: 0.0,
        is_champion: kind.is_champion(),
        spawn_delay: 0.0,
        hit_radius,
        bar_height,
        lane,
        lane_index: 0,
        respawn_time: 0.0,
        camp_buff: None,
        camp_boss: false,
        camp_name: None,
        mana: 0.0,
        max_mana: 0.0,
        level: 1,
        xp: 0.0,
        gold: 0,
        shield: 0.0,
        shield_timer: 0.0,
        cooldowns: [0.0; 4],
        visible: true,
        ranged_auto: false,
        q_damage: 0.0,
        r_damage: 0.0,
        e_heal: 0.0,
        e_shield: 0.0,
        w_distance: 0.0,
        ability_power: 0.0,
        structure_lane: None,
        super_minion: false,
        home: pos,
        aggro_range: 0.0,
        leash_range: 0.0,
        camp_aggro: false,
        camp_wander_timer: 0.0,
        ambient_kind: None,
        ambient_seed: 0,
        lock_range: 0.0,
        cc: Default::default(),
        scroll: Default::default(),
        spells: Default::default(),
        vengeance: None,
        attack_clip: None,
        cast_clip: None,
        attack_hit: 0.5,
        action: None,
        strike: None,
        look: fx::DEFAULT_LOOK,
        body_radius: default_body_radius(kind, hit_radius),
    }
}

/// Footprint radius for edge-to-edge attack range.
fn default_body_radius(kind: UnitKind, hit_radius: f32) -> f32 {
    match kind {
        UnitKind::Player | UnitKind::BotChampion => 65.0,
        UnitKind::MeleeMinion | UnitKind::RangedMinion => 40.0,
        UnitKind::Tower => 150.0,
        UnitKind::Inhibitor => 140.0,
        UnitKind::Nexus => 230.0,
        UnitKind::Watchtower => 120.0,
        UnitKind::Camp => (hit_radius * 0.5).clamp(50.0, 140.0),
        UnitKind::Dummy => 60.0,
        _ => 40.0,
    }
}

/// Basic-attack look for a champion file: its `projectile`, else a class
/// default.
fn champion_look(champion: &ChampionConfig) -> usize {
    match champion.projectile.as_deref() {
        Some(name) => fx::look_by_name(name),
        None if champion.class.eq_ignore_ascii_case("RANGED") => fx::look_by_name("arrow"),
        None => fx::DEFAULT_LOOK,
    }
}

/// Deterministic loose wedge for a minion wave. Melee minions take the front
/// row, the ranged minion trails, and an optional super minion leads. Jitter
/// changes each wave so repeated waves do not line up on the same exact tiles.
fn wave_formation_positions(
    nexus: Vec3,
    forward: Vec3,
    across: Vec3,
    serial: u32,
    super_lane: bool,
) -> Vec<(Vec3, f32, bool, bool)> {
    let mut formation = vec![
        // forward tiles, lateral tiles, ranged, super
        (1.20f32, -1.85f32, false, false),
        (2.60, 1.85, false, false),
        (0.20, 0.0, true, false),
    ];
    if super_lane {
        formation.push((3.60, 0.0, false, true));
    }
    formation
        .into_iter()
        .enumerate()
        .map(
            |(index, (forward_tiles, lateral_tiles, ranged, super_minion))| {
                let jitter = ((serial
                    .wrapping_mul(1_103_515_245)
                    .wrapping_add(index as u32 * 12_345)
                    >> 16
                    & 0xff) as f32
                    / 255.0
                    - 0.5)
                    * 0.24;
                let lateral = lateral_tiles + jitter;
                let pos = nexus + forward * (forward_tiles * 128.0) + across * (lateral * 128.0);
                (pos, lateral, ranged, super_minion)
            },
        )
        .collect()
}

/// Copy a lane centre line shifted sideways by `offset` world units. Each
/// minion keeps its own parallel track so a wave does not collapse into a
/// single-file clump on the way to the enemy nexus.
fn offset_lane(lane: &[Vec3], offset: f32) -> Vec<Vec3> {
    let mut out = Vec::with_capacity(lane.len());
    for index in 0..lane.len() {
        let previous = lane[index.saturating_sub(1)];
        let next = lane[(index + 1).min(lane.len() - 1)];
        let dir = Vec3::new(next.x - previous.x, 0.0, next.z - previous.z);
        let perpendicular = if dir.length_squared() > 1e-6 {
            Vec3::new(-dir.z, 0.0, dir.x).normalize()
        } else {
            Vec3::X
        };
        out.push(lane[index] + perpendicular * offset);
    }
    out
}

/// Horizontal (XZ) delta between two world positions. Lane offsets sit at
/// slightly different terrain heights than their source waypoints, so a
/// 3D distance can never fall below the waypoint threshold and leaves units
/// oscillating in place.
/// The indicator for a circle or cone anchored at `at`.
fn area_indicator(at: crate::kit::Anchor, radius: f32, arc: f32, range: f32) -> Indicator {
    use crate::kit::Anchor;
    match at {
        Anchor::Point => Indicator::Target { range, radius },
        Anchor::Caster if arc < 359.0 => Indicator::Cone { radius, arc },
        Anchor::Caster => Indicator::SelfArea { radius },
        // `front` areas sit 160 units ahead of the caster.
        Anchor::Front => Indicator::Cone {
            radius: radius + 160.0,
            arc: arc.min(150.0),
        },
        // On the unit the ability targets: its cast range.
        Anchor::Target => Indicator::Range { radius: range },
    }
}

/// Indicator shapes for a kit ability's actions. The first action that
/// reaches beyond the caster decides the main shape; self buffs fall back
/// to a small circle.
fn kit_indicators(actions: &[crate::kit::Action], attack_range: f32) -> Vec<Indicator> {
    use crate::kit::{Action, Anchor};
    let mut out = Vec::new();
    for action in actions {
        match action {
            Action::Projectile {
                range,
                width,
                count,
                spread,
                ..
            } => {
                let count = (*count).max(1);
                for i in 0..count {
                    let angle = if count == 1 {
                        0.0
                    } else {
                        spread.to_radians() * (i as f32 / (count - 1) as f32 - 0.5)
                    };
                    out.push(Indicator::Line {
                        length: *range,
                        width: *width,
                        angle,
                    });
                }
            }
            Action::Area {
                at,
                radius,
                arc,
                range,
                scatter,
                ..
            } => out.push(area_indicator(*at, radius + scatter, *arc, *range)),
            Action::Zone {
                at, radius, range, ..
            } => out.push(area_indicator(*at, *radius, 360.0, *range)),
            Action::Dash {
                distance,
                to_cursor,
                backwards,
                land,
                ..
            } => {
                out.push(Indicator::Dash {
                    length: *distance,
                    backwards: *backwards,
                    to_cursor: *to_cursor,
                });
                // A slam on landing shows its circle at the end of the leap.
                for landing in land {
                    if let Action::Area { radius, .. } = landing {
                        out.push(Indicator::Target {
                            range: *distance,
                            radius: *radius,
                        });
                    }
                }
            }
            Action::Summon { at, .. } => {
                if matches!(at, Anchor::Point) {
                    out.push(Indicator::Range { radius: 700.0 });
                }
            }
            Action::Channel { action, .. } => {
                out.extend(kit_indicators(std::slice::from_ref(action), attack_range));
            }
            Action::Empower { .. } | Action::Transform { .. } => {
                out.push(Indicator::Range {
                    radius: attack_range,
                });
            }
            Action::Strike { .. } => out.push(Indicator::Range { radius: 450.0 }),
            Action::Toss { distance, .. } => out.push(Indicator::Range { radius: *distance }),
            Action::Buff { .. } => {}
        }
        if !out.is_empty() {
            break;
        }
    }
    if out.is_empty() {
        out.push(Indicator::SelfArea { radius: 140.0 });
    }
    out
}

/// The furthest an indicator reaches from the caster.
fn indicator_reach(indicator: &Indicator) -> f32 {
    match *indicator {
        Indicator::Range { radius } => radius,
        Indicator::Line { length, .. } => length,
        Indicator::Target { range, radius } => range + radius,
        Indicator::Cone { radius, .. } | Indicator::SelfArea { radius } => radius,
        Indicator::Dash { length, .. } => length,
        Indicator::Boomerang { length } | Indicator::Swing { length } => length,
    }
}

/// Blend `pose` toward `other` by `weight` in place.
fn blend_pose(pose: &mut [[f32; 3]], other: &[[f32; 3]], weight: f32) {
    let w = weight.clamp(0.0, 1.0);
    for (out, target) in pose.iter_mut().zip(other) {
        for axis in 0..3 {
            out[axis] += (target[axis] - out[axis]) * w;
        }
    }
}

/// A rotation that turns a cache model's forward axis (-Z) along `dir`,
/// rolled `roll` radians around it.
fn flight_basis(dir: Vec3, roll: f32) -> Mat3 {
    let forward = dir.normalize_or_zero();
    if forward == Vec3::ZERO {
        return Mat3::from_rotation_z(roll);
    }
    let back = -forward;
    let reference = if back.y.abs() > 0.98 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let right = reference.cross(back).normalize();
    let up = back.cross(right);
    Mat3::from_cols(right, up, back) * Mat3::from_rotation_z(roll)
}

/// Keep a trail of drawn positions, newest first, spaced by distance so its
/// length does not depend on the frame rate.
fn advance_trail(trail: &mut Vec<Vec3>, head: Vec3, points: usize, speed: f32) {
    let spacing = (speed * 0.018).max(10.0);
    if trail.len() < 2 || trail[1].distance(head) >= spacing {
        trail.insert(0, head);
    } else {
        trail[0] = head;
    }
    trail.truncate(points.max(2));
}

fn planar_delta(from: Vec3, to: Vec3) -> Vec3 {
    Vec3::new(to.x - from.x, 0.0, to.z - from.z)
}

/// Shortest horizontal distance from a point to a line segment. Used by fast
/// skillshots so a low frame rate or a high timescale cannot tunnel through a
/// target between two sampled positions.
fn in_mesh_range(pos: Vec3, center: Vec3, radius: f32) -> bool {
    let dx = pos.x - center.x;
    let dz = pos.z - center.z;
    dx * dx + dz * dz <= radius * radius
}

fn segment_distance_xz(point: Vec3, from: Vec3, to: Vec3) -> f32 {
    let ab = planar_delta(from, to);
    let ap = planar_delta(from, point);
    let denom = ab.length_squared();
    let t = if denom > 1e-6 {
        (ap.dot(ab) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (ap - ab * t).length()
}

fn armor_damage_multiplier(armor: f32) -> f32 {
    if armor >= 0.0 {
        100.0 / (100.0 + armor)
    } else {
        2.0 - 100.0 / (100.0 - armor)
    }
}

/// Turn a unit toward a heading at a species-appropriate rate instead of
/// snapping. The player has its own weighted motor.
fn turn_toward(unit: &mut AnimatedUnit, target_yaw: f32, dt: f32) {
    let rate = match unit.kind {
        UnitKind::MeleeMinion | UnitKind::RangedMinion => 12.0,
        UnitKind::Camp => 7.0,
        UnitKind::DeliveryGuard => 10.0,
        _ => 15.0,
    };
    let delta = (target_yaw - unit.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    unit.yaw += delta.clamp(-rate * dt, rate * dt);
}

fn append_unit(scene: &mut AnimatedScene, unit: &AnimatedUnit, positions: &[[f32; 3]]) {
    append_unit_model(scene, unit, &unit.model, positions);
}

fn append_unit_model(
    scene: &mut AnimatedScene,
    unit: &AnimatedUnit,
    model: &Model,
    positions: &[[f32; 3]],
) {
    let rot = Mat3::from_rotation_y(unit.yaw + unit.model_yaw_offset + CACHE_MODEL_FORWARD);
    let dominant_texture = model_dominant_texture(model);
    // Key by the model instance, not the cache id: a recoloured camp/champion
    // can share the id while carrying a different palette.
    let model_key = (model as *const Model as usize, model.faces.len());
    scene.face_rgb_cache.entry(model_key).or_insert_with(|| {
        model
            .face_colors
            .iter()
            .map(|&hsl| hsl_to_rgb(hsl, 1.0))
            .collect()
    });
    // The step spring is drawing only: bars and selection rings remain on
    // the ground while the body lifts. Corpses shrink and sink briefly.
    let death = if unit.health <= 0.0 && unit.death_anim > 0.0 {
        1.0 - (unit.death_anim / DEATH_SINK_TIME).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let scale = unit.scale * (1.0 - 0.30 * death);
    let origin = unit.pos
        + Vec3::Y * (unit.motion.as_ref().map_or(0.0, |motion| motion.lift) - 72.0 * death);

    for (face_idx, &[a, b, c]) in model.faces.iter().enumerate() {
        let pa = model_vertex_animated(positions, a as usize, origin, rot, scale);
        let pb = model_vertex_animated(positions, b as usize, origin, rot, scale);
        let pc = model_vertex_animated(positions, c as usize, origin, rot, scale);

        let mut normal = (pb - pa).cross(pc - pa).normalize_or_zero();
        if normal.length_squared() < 1e-8 {
            normal = rot
                * Vec3::from_array(
                    model
                        .face_normals
                        .get(face_idx)
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]),
                );
        }

        let texture_id = model
            .face_textures
            .as_ref()
            .and_then(|t| t.get(face_idx))
            .copied()
            .map(|t| t as u32)
            .filter(|&t| t < 256)
            .or(dominant_texture);
        let textured = texture_id.is_some();
        let mut rgb = if textured {
            // The sampled model texture supplies the detail; vertex colour is
            // only a team/hit-flash tint.
            [255, 255, 255]
        } else if unit.kind == UnitKind::Tower {
            [125, 117, 102]
        } else if model.face_colors[face_idx] == 127 {
            [84, 142, 58]
        } else {
            // Use the cached cache-palette RGB. An old darkening clamp here
            // turned bright NPC palettes (the snakeling's white hood, for
            // example) into muddy near-black shapes.
            scene.face_rgb_cache[&model_key]
                .get(face_idx)
                .copied()
                .unwrap_or([84, 142, 58])
        };
        if unit.kind.is_minion() {
            let tint = if unit.team == 0 {
                [0.80f32, 0.95, 1.25]
            } else {
                [1.25, 0.85, 0.80]
            };
            rgb = [
                (rgb[0] as f32 * tint[0]).clamp(0.0, 255.0) as u8,
                (rgb[1] as f32 * tint[1]).clamp(0.0, 255.0) as u8,
                (rgb[2] as f32 * tint[2]).clamp(0.0, 255.0) as u8,
            ];
        }
        if matches!(unit.kind, UnitKind::Nexus | UnitKind::Inhibitor) && unit.team == 1 {
            rgb[0] = rgb[0].saturating_add(90);
            rgb[1] = (rgb[1] as f32 * 0.5) as u8;
            rgb[2] = (rgb[2] as f32 * 0.8) as u8;
        }
        if unit.hit_flash > 0.0 {
            rgb[0] = 255;
            rgb[1] = (rgb[1] as f32 * 0.55) as u8;
            rgb[2] = (rgb[2] as f32 * 0.55) as u8;
        }

        let ao_at = |p: Vec3| {
            (0.74 + 0.26 * ((p.y - origin.y).max(0.0) / (scale * 220.0)).clamp(0.0, 1.0))
                .clamp(0.66, 1.0)
        };
        let mut va = SceneVertex::new(pa, normal, rgb);
        let mut vb = SceneVertex::new(pb, normal, rgb);
        let mut vc = SceneVertex::new(pc, normal, rgb);
        let material = if let Some(texture_id) = texture_id {
            let abs_normal = normal.abs();
            let uv_for = |p: Vec3| {
                let local = p - origin;
                if abs_normal.y >= abs_normal.x && abs_normal.y >= abs_normal.z {
                    [local.x / 128.0, local.z / 128.0]
                } else if abs_normal.x >= abs_normal.z {
                    [local.z / 128.0, local.y / 128.0]
                } else {
                    [local.x / 128.0, local.y / 128.0]
                }
            };
            va.texture_id = texture_id;
            vb.texture_id = texture_id;
            vc.texture_id = texture_id;
            va.uv = uv_for(pa);
            vb.uv = uv_for(pb);
            vc.uv = uv_for(pc);
            5
        } else {
            3
        };
        va.material = material;
        vb.material = material;
        vc.material = material;
        va.ao = ao_at(pa);
        vb.ao = ao_at(pb);
        vc.ao = ao_at(pc);
        let base = scene.vertices.len() as u32;
        scene.vertices.push(va);
        scene.vertices.push(vb);
        scene.vertices.push(vc);
        scene.indices.extend_from_slice(&[base, base + 1, base + 2]);

        if matches!(
            unit.kind,
            UnitKind::Player
                | UnitKind::Tower
                | UnitKind::Inhibitor
                | UnitKind::Nexus
                | UnitKind::Camp
        ) {
            let cbase = scene.caster_vertices.len() as u32;
            scene.caster_vertices.push(va);
            scene.caster_vertices.push(vb);
            scene.caster_vertices.push(vc);
            scene
                .caster_indices
                .extend_from_slice(&[cbase, cbase + 1, cbase + 2]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_bar(
    scene: &mut AnimatedScene,
    center: Vec3,
    width: f32,
    height: f32,
    fill: f32,
    color: [u8; 3],
    right: Vec3,
    up: Vec3,
    forward: Vec3,
) {
    // Pull the whole bar several units toward the camera. The previous 1.5u
    // fill offset was below the depth-buffer epsilon at MOBA camera distance,
    // which made the fill flicker against its background while units moved.
    let center = center + forward * 12.0;
    let bg = [18u8, 18, 20];
    append_quad(
        scene,
        center,
        right * (width * 0.5),
        up * (height * 0.5),
        bg,
    );
    let fill_center = center + forward * 6.0 + right * (-width * 0.5 + width * fill * 0.5);
    append_quad(
        scene,
        fill_center,
        right * (width * fill * 0.5),
        up * (height * 0.45),
        color,
    );
}

fn append_banana_model(
    scene: &mut AnimatedScene,
    model: &Model,
    center: Vec3,
    rot: Mat3,
    scale: f32,
) {
    let vertex = |index: usize| -> Vec3 {
        let x = model.vertex_x.get(index).copied().unwrap_or(0) as f32;
        let y = model.vertex_y.get(index).copied().unwrap_or(0) as f32;
        let z = model.vertex_z.get(index).copied().unwrap_or(0) as f32;
        center + rot * (Vec3::new(x, -y, z) * scale)
    };

    for (face_idx, &[a, b, c]) in model.faces.iter().enumerate() {
        let pa = vertex(a as usize);
        let pb = vertex(b as usize);
        let pc = vertex(c as usize);
        let mut normal = (pb - pa).cross(pc - pa).normalize_or_zero();
        if normal.length_squared() < 1e-8 {
            normal = rot
                * Vec3::from_array(
                    model
                        .face_normals
                        .get(face_idx)
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]),
                );
        }
        let palette = model.face_colors.get(face_idx).copied().unwrap_or(0);
        let rgb = hsl_to_rgb(palette, 1.0);
        let mut va = SceneVertex::new(pa, normal, rgb);
        let mut vb = SceneVertex::new(pb, normal, rgb);
        let mut vc = SceneVertex::new(pc, normal, rgb);
        va.material = 3;
        vb.material = 3;
        vc.material = 3;
        let base = scene.vertices.len() as u32;
        scene.vertices.push(va);
        scene.vertices.push(vb);
        scene.vertices.push(vc);
        scene.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }
}

fn append_billboard(
    scene: &mut AnimatedScene,
    center: Vec3,
    size: f32,
    color: [u8; 3],
    right: Vec3,
    up: Vec3,
) {
    append_quad(
        scene,
        center,
        right * (size * 0.5),
        up * (size * 0.5),
        color,
    );
}

fn append_ring(
    scene: &mut AnimatedScene,
    height: &HeightGrid,
    center: Vec3,
    radius: f32,
    color: [u8; 3],
) {
    append_ring_width(scene, height, center, radius, color, 0.78);
}

/// A ground ring with an explicit band width. `inner` is the inner radius as a
/// fraction of `radius`; 0.97 gives the thin range marker the reference
/// client draws around a champion.
fn append_ring_width(
    scene: &mut AnimatedScene,
    height: &HeightGrid,
    center: Vec3,
    radius: f32,
    color: [u8; 3],
    inner: f32,
) {
    let segments = 48u32;
    let base = scene.vertices.len() as u32;
    // Sample the terrain per vertex so the band hugs slopes instead of
    // slicing through them.
    let lift = |x: f32, z: f32| height.height_at(x, z) + 6.0;
    for i in 0..segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let b = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let (ax, az) = (center.x + a.cos() * radius, center.z + a.sin() * radius);
        let (bx, bz) = (center.x + b.cos() * radius, center.z + b.sin() * radius);
        let (cx, cz) = (
            center.x + a.cos() * radius * inner,
            center.z + a.sin() * radius * inner,
        );
        let (dx, dz) = (
            center.x + b.cos() * radius * inner,
            center.z + b.sin() * radius * inner,
        );
        let p0 = Vec3::new(ax, lift(ax, az), az);
        let p1 = Vec3::new(bx, lift(bx, bz), bz);
        let p2 = Vec3::new(cx, lift(cx, cz), cz);
        let p3 = Vec3::new(dx, lift(dx, dz), dz);
        scene.vertices.push(SceneVertex::unlit(p0, color));
        scene.vertices.push(SceneVertex::unlit(p1, color));
        scene.vertices.push(SceneVertex::unlit(p2, color));
        scene.vertices.push(SceneVertex::unlit(p3, color));
        let o = base + i * 4;
        scene
            .indices
            .extend_from_slice(&[o, o + 1, o + 2, o + 2, o + 1, o + 3]);
    }
}

fn append_quad(scene: &mut AnimatedScene, center: Vec3, right: Vec3, up: Vec3, color: [u8; 3]) {
    let base = scene.vertices.len() as u32;
    scene
        .vertices
        .push(SceneVertex::unlit(center - right - up, color));
    scene
        .vertices
        .push(SceneVertex::unlit(center + right - up, color));
    scene
        .vertices
        .push(SceneVertex::unlit(center - right + up, color));
    scene
        .vertices
        .push(SceneVertex::unlit(center + right + up, color));
    scene
        .indices
        .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
}

/// A taut braided rope from `from` to `to`: two alternating strand shades,
/// sagging a little more the longer it runs.
fn draw_rope(scene: &mut AnimatedScene, from: Vec3, to: Vec3, color: [u8; 3], forward: Vec3) {
    let along = to - from;
    let length = along.length();
    if length < 1.0 {
        return;
    }
    let segments = ((length / 40.0).ceil() as usize).clamp(4, 48);
    let side = along.cross(forward).normalize_or(Vec3::X);
    let sag = Vec3::Y * -(length * 0.04).min(60.0);
    let point = |i: usize| {
        let s = i as f32 / segments as f32;
        from + along * s + sag * (4.0 * s * (1.0 - s))
    };
    let dark = color.map(|c| (c as f32 * 0.62) as u8);
    for i in 0..segments {
        let (a, b) = (point(i), point(i + 1));
        let half = (b - a) * 0.5;
        let shade = if i % 2 == 0 { color } else { dark };
        append_quad(scene, a + half, half, side * 6.5, shade);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_demo_grid_is_square_ish_and_centred() {
        assert!(demo_grid(Vec3::ZERO, 0, 100.0).is_empty());
        let one = demo_grid(Vec3::new(5.0, 0.0, 7.0), 1, 100.0);
        assert_eq!(one, vec![Vec3::new(5.0, 0.0, 7.0)]);
        // Ten items: four columns, three rows, centred on the origin.
        let ten = demo_grid(Vec3::ZERO, 10, 100.0);
        assert_eq!(ten.len(), 10);
        assert_eq!(ten[0], Vec3::new(-150.0, 0.0, -100.0));
        assert_eq!(ten[3], Vec3::new(150.0, 0.0, -100.0));
        assert_eq!(ten[9], Vec3::new(-50.0, 0.0, 100.0));
    }

    #[test]
    fn only_hero_structure_and_dummy_kills_reach_the_feed() {
        for kind in [
            UnitKind::Player,
            UnitKind::BotChampion,
            UnitKind::Tower,
            UnitKind::Inhibitor,
            UnitKind::Nexus,
        ] {
            assert!(reaches_feed(kind, false), "{kind:?}");
        }
        assert!(reaches_feed(UnitKind::Dummy, true));
        assert!(
            !reaches_feed(UnitKind::Dummy, false),
            "a minion's dummy kill"
        );
        for kind in [
            UnitKind::MeleeMinion,
            UnitKind::RangedMinion,
            UnitKind::Camp,
            UnitKind::Ambient,
        ] {
            assert!(!reaches_feed(kind, true), "{kind:?}");
        }
    }

    #[test]
    fn hitmark_kinds_map_to_distinct_osrs_sprites() {
        let kinds = [
            HitsplatKind::Damage,
            HitsplatKind::Poison,
            HitsplatKind::Venom,
            HitsplatKind::Disease,
            HitsplatKind::Heal,
            HitsplatKind::Miss,
            HitsplatKind::MaxHit,
        ];
        let mut names: Vec<&str> = kinds.iter().map(|kind| kind.sprite_name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), kinds.len());
        let mut colours: Vec<[u8; 3]> = kinds.iter().map(|kind| kind.fallback_color()).collect();
        colours.sort_unstable();
        colours.dedup();
        assert_eq!(colours.len(), kinds.len());
    }

    #[test]
    fn wave_formation_spaces_minions_apart() {
        let nexus = Vec3::new(1000.0, 10.0, 2000.0);
        let forward = Vec3::Z;
        let across = Vec3::X;
        let formation = wave_formation_positions(nexus, forward, across, 7, false);
        assert_eq!(formation.len(), 3);
        for i in 0..formation.len() {
            for j in i + 1..formation.len() {
                let distance = formation[i].0.distance(formation[j].0);
                assert!(
                    distance > 220.0,
                    "wave minions {i} and {j} spawned only {distance:.0} units apart"
                );
            }
        }
        let ranged = formation.iter().filter(|(_, _, ranged, _)| *ranged).count();
        assert_eq!(ranged, 1);
    }

    #[test]
    fn super_wave_adds_a_staggered_leader() {
        let nexus = Vec3::new(1000.0, 10.0, 2000.0);
        let formation = wave_formation_positions(nexus, Vec3::Z, Vec3::X, 3, true);
        assert_eq!(formation.len(), 4);
        assert_eq!(
            formation
                .iter()
                .filter(|(_, _, _, super_minion)| *super_minion)
                .count(),
            1
        );
        let super_pos = formation
            .iter()
            .find(|(_, _, _, super_minion)| *super_minion)
            .expect("super minion")
            .0;
        assert!(super_pos.z > nexus.z + 128.0);
    }

    #[test]
    fn armor_reduces_and_amplifies_physical_damage() {
        assert!((armor_damage_multiplier(0.0) - 1.0).abs() < 1e-6);
        assert!((armor_damage_multiplier(100.0) - 0.5).abs() < 1e-6);
        assert!(armor_damage_multiplier(-100.0) > 1.0);
    }

    #[test]
    fn segment_distance_catches_fast_projectiles() {
        let from = Vec3::ZERO;
        let to = Vec3::new(100.0, 0.0, 0.0);
        assert!(segment_distance_xz(Vec3::new(50.0, 0.0, 0.0), from, to) < 0.01);
        let offset = segment_distance_xz(Vec3::new(50.0, 0.0, 30.0), from, to);
        assert!((offset - 30.0).abs() < 0.01);
        let behind = segment_distance_xz(Vec3::new(-25.0, 0.0, 0.0), from, to);
        assert!((behind - 25.0).abs() < 0.01);
    }

    #[test]
    fn planar_delta_ignores_height() {
        let from = Vec3::new(100.0, 50.0, 200.0);
        let to = Vec3::new(100.0, 900.0, 260.0);
        let delta = planar_delta(from, to);
        assert_eq!(delta.y, 0.0);
        assert!((delta.length() - 60.0).abs() < 1e-4);
    }

    #[test]
    fn damage_totals_merge_by_source_and_sort_largest_first() {
        let entries = vec![
            ("Tower".to_string(), 40.0),
            ("Rival champion".to_string(), 120.0),
            ("Tower".to_string(), 30.0),
            ("Minion".to_string(), 10.0),
        ];
        assert_eq!(
            damage_totals(&entries),
            vec![
                ("Rival champion".to_string(), 120),
                ("Tower".to_string(), 70),
                ("Minion".to_string(), 10),
            ]
        );
    }

    #[test]
    fn offset_lanes_keep_parallel_tracks() {
        let lane = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1000.0),
            Vec3::new(1000.0, 0.0, 2000.0),
        ];
        let left = offset_lane(&lane, -200.0);
        let right = offset_lane(&lane, 200.0);
        for (a, b) in left.iter().zip(&right) {
            assert!(a.distance(*b) > 350.0);
        }
    }

    fn clip(lengths: &[u16]) -> AnimationClip {
        let mut clip = AnimationClip::default();
        clip.sequence.frame_lengths = lengths.to_vec();
        clip
    }

    #[test]
    fn footsteps_vary_within_a_narrow_band() {
        let mut pitches = std::collections::BTreeSet::new();
        for step in 0..64 {
            let (pitch, weight) = step_variation(3, step as f32 * 640.0, 0);
            assert!((0.92..=1.08).contains(&pitch), "pitch {pitch}");
            assert!((0.75..=1.0).contains(&weight), "weight {weight}");
            pitches.insert((pitch * 100.0) as i32);
        }
        assert!(pitches.len() > 8, "steps should not all sound the same");
    }

    #[test]
    fn an_attack_clip_is_time_scaled_so_its_hit_lands_at_the_windup() {
        // 50 ticks = 1000 ms; the hit at 40% is 400 ms into the clip.
        let clip = clip(&[25, 25]);
        let rate = action_rate(&clip, 0.4, 0.2);
        assert!((rate - 2.0).abs() < 1e-4, "rate {rate}");
        assert_eq!(action_rate(&clip, 0.4, 0.0), 1.0);
        // Absurd wind-ups are clamped rather than freezing or blurring.
        assert_eq!(action_rate(&clip, 0.5, 100.0), 0.25);
        assert_eq!(action_rate(&clip, 0.5, 0.001), 4.0);
    }

    #[test]
    fn a_one_shot_blends_in_holds_and_blends_out() {
        let mut action = ActionAnim::new(Arc::new(clip(&[50])), 1.0);
        assert_eq!(action.weight(), 0.0);
        assert!(action.advance(0.5));
        assert!((action.weight() - 1.0).abs() < 1e-4);
        assert!(action.advance(0.45));
        assert!(action.weight() < 1.0, "blending out near the end");
        assert!(!action.advance(0.1), "over after its length");
    }

    #[test]
    fn a_cancelled_one_shot_fades_quickly() {
        let mut action = ActionAnim::new(Arc::new(clip(&[200])), 1.0);
        action.advance(0.2);
        action.cancelling = true;
        assert!(action.advance(ACTION_BLEND_OUT * 0.5));
        assert!(action.weight() < 0.6);
        assert!(!action.advance(ACTION_BLEND_OUT));
    }

    #[test]
    fn blend_pose_moves_toward_the_other_pose_by_weight() {
        let mut pose = vec![[0.0, 0.0, 0.0], [10.0, 10.0, 10.0]];
        blend_pose(&mut pose, &[[10.0, 20.0, 30.0], [10.0, 10.0, 10.0]], 0.5);
        assert_eq!(pose[0], [5.0, 10.0, 15.0]);
        assert_eq!(pose[1], [10.0, 10.0, 10.0]);
    }

    #[test]
    fn flight_basis_points_the_model_forward_axis_along_the_flight() {
        for dir in [
            Vec3::X,
            Vec3::Z,
            Vec3::new(1.0, 0.4, -2.0).normalize(),
            Vec3::Y,
        ] {
            let basis = flight_basis(dir, 0.7);
            let forward = basis * Vec3::NEG_Z;
            assert!(forward.distance(dir) < 1e-4, "{dir:?} -> {forward:?}");
            assert!((basis.determinant() - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn trails_keep_newest_first_and_stay_bounded() {
        let mut trail = Vec::new();
        for i in 0..40 {
            advance_trail(&mut trail, Vec3::X * (i as f32 * 50.0), 8, 1000.0);
        }
        assert_eq!(trail.len(), 8);
        assert!(trail[0].x > trail[7].x);
        // Tiny steps update the head instead of piling up points.
        let mut still = Vec::new();
        for i in 0..20 {
            advance_trail(&mut still, Vec3::X * i as f32, 8, 1000.0);
        }
        assert!(still.len() <= 3);
        assert_eq!(still[0].x, 19.0);
    }

    #[test]
    fn body_radius_is_tighter_than_the_projectile_hit_box() {
        assert_eq!(default_body_radius(UnitKind::Player, 240.0), 65.0);
        assert_eq!(default_body_radius(UnitKind::Nexus, 340.0), 230.0);
        assert_eq!(default_body_radius(UnitKind::Camp, 160.0), 80.0);
        assert_eq!(default_body_radius(UnitKind::Camp, 1000.0), 140.0);
    }

    #[test]
    fn homing_shots_lob_only_when_their_look_arcs() {
        let dart = fx::look_by_name("dart");
        let mut shot =
            Projectile::homing(Vec3::ZERO, 3, Vec3::X * 1000.0, 1000.0, 5.0, 0, None, dart);
        assert_eq!(shot.target, Some(3));
        assert!((shot.flight - 1000.0).abs() < 1e-3);
        shot.travelled = 500.0;
        assert!(shot.draw_pos().y > 0.0);
        let mut orb = Projectile::homing(
            Vec3::ZERO,
            3,
            Vec3::X * 1000.0,
            1000.0,
            5.0,
            0,
            None,
            fx::DEFAULT_LOOK,
        );
        orb.travelled = 500.0;
        assert_eq!(orb.draw_pos(), Vec3::ZERO);
    }

    #[test]
    fn kit_indicators_follow_the_first_reaching_action() {
        let parse = |json: &str| -> Vec<crate::kit::Action> { serde_json::from_str(json).unwrap() };
        let line = kit_indicators(
            &parse(
                r#"[{"type":"projectile","speed":1000,"range":900,"damage":1,"width":80,"count":3,"spread":30}]"#,
            ),
            300.0,
        );
        assert_eq!(line.len(), 3);
        assert!(
            matches!(line[1], Indicator::Line { length, angle, .. } if length == 900.0 && angle.abs() < 1e-6)
        );
        let target = kit_indicators(
            &parse(
                r#"[{"type":"buff","heal":5},{"type":"area","radius":200,"range":800,"scatter":50}]"#,
            ),
            300.0,
        );
        assert_eq!(
            target,
            vec![Indicator::Target {
                range: 800.0,
                radius: 250.0
            }]
        );
        let cone = kit_indicators(
            &parse(r#"[{"type":"area","at":"caster","radius":400,"arc":90}]"#),
            300.0,
        );
        assert_eq!(
            cone,
            vec![Indicator::Cone {
                radius: 400.0,
                arc: 90.0
            }]
        );
        let leap = kit_indicators(
            &parse(
                r#"[{"type":"dash","distance":600,"to_cursor":true,"land":[{"type":"area","at":"caster","radius":250}]}]"#,
            ),
            300.0,
        );
        assert_eq!(leap.len(), 2);
        assert!(
            matches!(leap[0], Indicator::Dash { length, to_cursor: true, .. } if length == 600.0)
        );
        let selfish = kit_indicators(&parse(r#"[{"type":"buff","shield":50}]"#), 300.0);
        assert_eq!(selfish, vec![Indicator::SelfArea { radius: 140.0 }]);
        let empower = kit_indicators(&parse(r#"[{"type":"empower","bonus":10}]"#), 300.0);
        assert_eq!(empower, vec![Indicator::Range { radius: 300.0 }]);
    }

    #[test]
    fn indicator_reach_is_the_furthest_point() {
        assert_eq!(
            indicator_reach(&Indicator::Target {
                range: 800.0,
                radius: 100.0
            }),
            900.0
        );
        assert_eq!(
            indicator_reach(&Indicator::Line {
                length: 700.0,
                width: 50.0,
                angle: 0.3
            }),
            700.0
        );
    }
}
