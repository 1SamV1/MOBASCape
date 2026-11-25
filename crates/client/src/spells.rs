//! Battle spells (docs/SPELLS.md, Part C): two picks for every MAGIC hero,
//! cast on D and F. Non-magic heroes get none; every hero still carries the
//! teleport scroll (`crate::scroll`).
//!
//! This module is plain data and pure rules; the scene runs the spells in
//! `animated/battle.rs`, for the player and the bots alike.

use glam::Vec3;
use serde::Deserialize;

use crate::hero_select::Lane;
use crate::kit::{Afflictions, Gfx, Status};

/// Where the spells live at runtime.
pub const PATH: &str = "data/spells/battle_spells.json";
/// The shipped spells, built in so a missing file never empties the book.
const SHIPPED: &str = include_str!("../../../data/spells/battle_spells.json");

/// `data/spells/battle_spells.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct SpellBook {
    pub spells: Vec<Spell>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Spell {
    /// Stable key: `--spells`, hero select and the theme's
    /// `icon.spell.<id>` role.
    pub id: String,
    pub name: String,
    /// The OSRS spellbook it comes from (Standard, Ancient or Lunar); the
    /// picker groups by it.
    pub book: String,
    /// Cache sprite of its spellbook icon.
    pub icon: u32,
    /// Seconds before it can be cast again.
    pub cooldown: f32,
    pub effect: Effect,
    #[serde(default)]
    pub gfx: Gfx,
}

/// What a battle spell does.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// Teleport up to `range` toward the aim, onto walkable ground. The
    /// `cast` graphic plays where you leave, `land` where you arrive.
    Blink { range: f32 },
    /// Heal yourself `own` and allied heroes within `radius` `allies`, as
    /// fractions of max health. Each ally shows the `hit` graphic.
    HealGroup { own: f32, allies: f32, radius: f32 },
    /// For `duration`, the next damage an enemy hero deals you is sent back
    /// at `reflect` of it, once, and you `say` so.
    Vengeance {
        reflect: f32,
        duration: f32,
        say: String,
    },
    /// A homing bolt at the enemy hero nearest the aim, within `range`.
    Bolt {
        range: f32,
        damage: f32,
        speed: f32,
        /// Projectile look from `fx::LOOKS`.
        visual: String,
        #[serde(default)]
        on_hit: Vec<Status>,
    },
    /// Everything hostile within `radius` of you.
    Nova {
        radius: f32,
        #[serde(default)]
        damage: f32,
        #[serde(default)]
        on_hit: Vec<Status>,
    },
}

impl SpellBook {
    /// The spells on disk, or the built-in copy (with a warning) when the
    /// file is missing or broken.
    pub fn load() -> Self {
        let parsed = std::fs::read_to_string(PATH)
            .map_err(anyhow::Error::from)
            .and_then(|text| serde_json::from_str(&text).map_err(anyhow::Error::from));
        let book = parsed.unwrap_or_else(|err| {
            tracing::warn!("spells: {PATH}: {err:#}; using the built-in battle spells");
            Self::shipped()
        });
        for visual in book.visuals() {
            if !crate::fx::LOOKS.iter().any(|look| look.name == visual) {
                tracing::warn!("spells: bolt look {visual:?} is not in fx::LOOKS");
            }
        }
        for id in book.spotanims() {
            if !crate::kit::spot::VERIFIED.contains(&id) {
                tracing::warn!("spells: spot animation {id} is not verified in docs/SPELLS.md");
            }
        }
        book
    }

    /// The battle spells the client was built with.
    pub fn shipped() -> Self {
        serde_json::from_str(SHIPPED).expect("the built-in battle spells parse")
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        let id = id.trim();
        self.spells
            .iter()
            .position(|spell| spell.id.eq_ignore_ascii_case(id))
    }

    pub fn get(&self, index: usize) -> Option<&Spell> {
        self.spells.get(index)
    }

    /// Two picks from ids; unknown ids leave their slot empty.
    pub fn picks<S: AsRef<str>>(&self, ids: &[S]) -> [Option<usize>; 2] {
        std::array::from_fn(|n| ids.get(n).and_then(|id| self.index_of(id.as_ref())))
    }

    /// Every spot animation the spells play, including the held graphics
    /// their statuses bring, for preloading.
    pub fn spotanims(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for spell in &self.spells {
            out.extend([
                spell.gfx.cast,
                spell.gfx.land,
                spell.gfx.hit,
                spell.gfx.hold,
            ]);
            let statuses: &[Status] = match &spell.effect {
                Effect::Bolt { on_hit, .. } | Effect::Nova { on_hit, .. } => on_hit,
                _ => &[],
            };
            out.extend(statuses.iter().filter_map(|s| s.hold().map(|h| h.0)));
        }
        let mut out: Vec<u32> = out.into_iter().flatten().collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Projectile looks the bolts name.
    pub fn visuals(&self) -> impl Iterator<Item = &str> {
        self.spells.iter().filter_map(|spell| match &spell.effect {
            Effect::Bolt { visual, .. } => Some(visual.as_str()),
            _ => None,
        })
    }
}

impl Spell {
    /// What the spell does, in a sentence, written from its numbers so the
    /// picker never disagrees with the match.
    pub fn describe(&self) -> String {
        let pct = |f: f32| (f * 100.0).round();
        match &self.effect {
            Effect::Blink { range } => format!("Blink up to {range:.0} toward the cursor."),
            Effect::HealGroup {
                own,
                allies,
                radius,
            } => format!(
                "Heal {}% of max health, and allied heroes within {radius:.0} heal {}%.",
                pct(*own),
                pct(*allies)
            ),
            Effect::Vengeance {
                reflect, duration, ..
            } => format!(
                "For {duration:.0} s, the next hit an enemy hero lands on you sends {}% of it back.",
                pct(*reflect)
            ),
            Effect::Bolt {
                range,
                damage,
                on_hit,
                ..
            } => {
                let mut parts = Vec::new();
                if *damage > 0.0 {
                    parts.push(format!("deal {damage:.0} damage"));
                }
                parts.extend(on_hit.iter().map(Status::describe));
                format!(
                    "A bolt at an enemy hero within {range:.0}: {}.",
                    parts.join(" and ")
                )
            }
            Effect::Nova {
                radius,
                damage,
                on_hit,
            } => {
                let mut parts = Vec::new();
                if *damage > 0.0 {
                    parts.push(format!("deal {damage:.0} damage"));
                }
                parts.extend(on_hit.iter().map(Status::describe));
                format!(
                    "Enemies within {radius:.0} of you: {}.",
                    parts.join(" and ")
                )
            }
        }
    }
}

/// The spell's icon role in both HUDs' atlases (`icon.spell.<id>`).
pub fn icon_role(id: &str) -> String {
    format!("icon.spell.{id}")
}

/// Only MAGIC-class heroes carry battle spells.
pub fn is_magic(class: &str) -> bool {
    class.trim().eq_ignore_ascii_case("MAGIC")
}

/// The pair a lane call pre-fills. Never enforced: any spell can go in
/// either slot.
pub fn lane_defaults(lane: Option<Lane>) -> [&'static str; 2] {
    match lane {
        Some(Lane::Mid) => ["ancient_teleport", "ice_barrage"],
        Some(Lane::Support) => ["heal_group", "vengeance"],
        _ => ["ancient_teleport", "heal_group"],
    }
}

/// A hero's two battle spells and their cooldowns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Loadout {
    /// Indices into the spell book, D then F.
    pub picks: [Option<usize>; 2],
    pub cooldowns: [f32; 2],
}

impl Loadout {
    /// A loadout for a hero of `class`: the picks for a magic hero, none for
    /// anyone else.
    pub fn for_class(class: &str, picks: [Option<usize>; 2]) -> Self {
        if is_magic(class) {
            Self {
                picks,
                cooldowns: [0.0; 2],
            }
        } else {
            Self::default()
        }
    }

    pub fn has_spells(&self) -> bool {
        self.picks.iter().any(Option::is_some)
    }

    pub fn tick(&mut self, dt: f32) {
        for cooldown in &mut self.cooldowns {
            *cooldown = (*cooldown - dt).max(0.0);
        }
    }
}

/// Why a battle spell did not go off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Dead,
    /// The slot is empty (or the hero is not a mage).
    NoSpell,
    Cooldown,
    /// Stunned, feared, taunted, airborne or being moved.
    Disabled,
    /// A blink while rooted or tele blocked.
    Blocked,
    /// A bolt with no enemy hero in range.
    NoTarget,
}

/// Whether slot `slot` may be cast now; the spell's index when it may.
pub fn can_cast(
    loadout: &Loadout,
    slot: usize,
    health: f32,
    stun_timer: f32,
    cc: &Afflictions,
) -> Result<usize, Refusal> {
    let index = loadout
        .picks
        .get(slot)
        .copied()
        .flatten()
        .ok_or(Refusal::NoSpell)?;
    if health <= 0.0 {
        Err(Refusal::Dead)
    } else if loadout.cooldowns[slot] > 0.0 {
        Err(Refusal::Cooldown)
    } else if crate::scroll::disabled(stun_timer, cc) {
        Err(Refusal::Disabled)
    } else {
        Ok(index)
    }
}

/// A blink needs free feet: no root and no tele block.
pub fn can_blink(cc: &Afflictions) -> bool {
    cc.root_timer <= 0.0 && cc.no_mobility_timer <= 0.0
}

/// Where a blink of `range` toward `aim` lands, on the ground plane (the
/// caller snaps it to walkable ground). Aiming at your own feet blinks
/// `range` along `facing`.
pub fn blink_target(origin: Vec3, aim: Vec3, facing: Vec3, range: f32) -> Vec3 {
    let offset = Vec3::new(aim.x - origin.x, 0.0, aim.z - origin.z);
    let length = offset.length();
    if length < 1.0 {
        return origin + facing.normalize_or_zero() * range;
    }
    origin + offset / length * length.min(range)
}

/// How far from the aim a bolt still finds its hero, so a click near a
/// hero counts.
pub const BOLT_PICK: f32 = 450.0;

/// The bolt's target: of the `candidates` (unit index and position) within
/// `range` of the caster, the one nearest the aim, if it is within
/// [`BOLT_PICK`] of it.
pub fn bolt_target(
    candidates: &[(usize, Vec3)],
    caster: Vec3,
    aim: Vec3,
    range: f32,
) -> Option<usize> {
    let flat = |a: Vec3, b: Vec3| ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
    candidates
        .iter()
        .filter(|(_, pos)| flat(*pos, caster) <= range && flat(*pos, aim) <= BOLT_PICK)
        .min_by(|a, b| flat(a.1, aim).total_cmp(&flat(b.1, aim)))
        .map(|&(index, _)| index)
}

/// Vengeance waiting on a hero.
#[derive(Debug, Clone, PartialEq)]
pub struct Vengeance {
    pub reflect: f32,
    pub timer: f32,
    pub say: String,
}

/// Count vengeance down; it lapses unused when the time runs out.
pub fn tick_vengeance(vengeance: &mut Option<Vengeance>, dt: f32) {
    if let Some(v) = vengeance.as_mut() {
        v.timer -= dt;
        if v.timer <= 0.0 {
            *vengeance = None;
        }
    }
}

/// Damage `amount` just landed. From an enemy hero, a waiting vengeance
/// fires once: the damage to send back and the line to say.
pub fn trigger_vengeance(
    vengeance: &mut Option<Vengeance>,
    amount: f32,
    from_enemy_hero: bool,
) -> Option<(f32, String)> {
    if !from_enemy_hero || amount <= 0.0 {
        return None;
    }
    let v = vengeance.take()?;
    Some((amount * v.reflect, v.say))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_book_parses_validates_and_has_the_first_wave() {
        let book = SpellBook::shipped();
        let ids: Vec<&str> = book.spells.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "ancient_teleport",
                "heal_group",
                "vengeance",
                "ice_barrage",
                "entangle",
                "tele_block"
            ]
        );
        for spell in &book.spells {
            assert!(spell.cooldown > 0.0, "{} has no cooldown", spell.id);
            assert!(spell.icon > 0, "{} has no icon", spell.id);
            assert!(
                ["Standard", "Ancient", "Lunar"].contains(&spell.book.as_str()),
                "{} names an unknown book",
                spell.id
            );
            assert!(!spell.describe().is_empty());
        }
        for id in book.spotanims() {
            assert!(
                crate::kit::spot::VERIFIED.contains(&id),
                "battle spell graphic {id} is not verified in docs/SPELLS.md"
            );
        }
        for visual in book.visuals() {
            assert!(
                crate::fx::LOOKS.iter().any(|look| look.name == visual),
                "bolt look {visual} is not in fx::LOOKS"
            );
        }
        // Tele Block brings its own held graphic.
        assert!(book.spotanims().contains(&342));
    }

    #[test]
    fn the_runtime_file_matches_the_built_in_copy() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(PATH);
        assert_eq!(std::fs::read_to_string(path).unwrap(), SHIPPED);
    }

    #[test]
    fn lane_calls_prefill_a_pair_and_ids_resolve_loosely() {
        let book = SpellBook::shipped();
        assert_eq!(
            lane_defaults(Some(Lane::Mid)),
            ["ancient_teleport", "ice_barrage"]
        );
        assert_eq!(
            lane_defaults(Some(Lane::Support)),
            ["heal_group", "vengeance"]
        );
        assert_eq!(lane_defaults(Some(Lane::Top)), lane_defaults(None));
        let picks = book.picks(&[" Ice_Barrage", "nonsense"]);
        assert_eq!(picks, [Some(3), None]);
        for lane in [None, Some(Lane::Mid), Some(Lane::Support)] {
            let picks = book.picks(&lane_defaults(lane));
            assert!(
                picks.iter().all(Option::is_some),
                "{lane:?} defaults resolve"
            );
        }
    }

    #[test]
    fn battle_spells_are_refused_for_non_magic_heroes() {
        let picks = [Some(0), Some(1)];
        assert!(!Loadout::for_class("MELEE", picks).has_spells());
        assert!(!Loadout::for_class("RANGED", picks).has_spells());
        let mage = Loadout::for_class(" magic ", picks);
        assert!(mage.has_spells());
        let free = Afflictions::default();
        let melee = Loadout::for_class("MELEE", picks);
        assert_eq!(
            can_cast(&melee, 0, 100.0, 0.0, &free),
            Err(Refusal::NoSpell)
        );
        assert_eq!(can_cast(&mage, 1, 100.0, 0.0, &free), Ok(1));
    }

    #[test]
    fn casting_waits_on_cooldown_life_and_crowd_control() {
        let free = Afflictions::default();
        let mut mage = Loadout::for_class("MAGIC", [Some(2), None]);
        assert_eq!(can_cast(&mage, 1, 100.0, 0.0, &free), Err(Refusal::NoSpell));
        assert_eq!(can_cast(&mage, 0, 0.0, 0.0, &free), Err(Refusal::Dead));
        assert_eq!(
            can_cast(&mage, 0, 100.0, 0.3, &free),
            Err(Refusal::Disabled)
        );
        mage.cooldowns[0] = 2.0;
        assert_eq!(
            can_cast(&mage, 0, 100.0, 0.0, &free),
            Err(Refusal::Cooldown)
        );
        mage.tick(2.5);
        assert_eq!(mage.cooldowns[0], 0.0);
        // A root still lets a spell go off, but not a blink.
        let rooted = Afflictions {
            root_timer: 1.0,
            ..Default::default()
        };
        assert_eq!(can_cast(&mage, 0, 100.0, 0.0, &rooted), Ok(2));
        assert!(!can_blink(&rooted));
        assert!(can_blink(&free));
    }

    #[test]
    fn tele_block_stops_a_blink_and_a_scroll() {
        let blocked = Afflictions {
            no_mobility_timer: 2.0,
            ..Default::default()
        };
        assert!(!can_blink(&blocked));
        let ready = crate::scroll::ScrollState::default();
        assert_eq!(
            crate::scroll::can_start(100.0, &ready, 0.0, &blocked),
            Err(crate::scroll::Refusal::Disabled)
        );
        // Battle spells themselves still go off under tele block.
        let mage = Loadout::for_class("MAGIC", [Some(1), None]);
        assert_eq!(can_cast(&mage, 0, 100.0, 0.0, &blocked), Ok(1));
    }

    #[test]
    fn a_blink_stops_at_the_cursor_or_its_range() {
        let origin = Vec3::new(0.0, 10.0, 0.0);
        let near = blink_target(origin, Vec3::new(200.0, 0.0, 0.0), Vec3::Z, 450.0);
        assert_eq!(near, Vec3::new(200.0, 10.0, 0.0));
        let far = blink_target(origin, Vec3::new(0.0, 0.0, 1000.0), Vec3::Z, 450.0);
        assert_eq!(far, Vec3::new(0.0, 10.0, 450.0));
        let feet = blink_target(origin, origin, Vec3::X, 450.0);
        assert_eq!(feet, Vec3::new(450.0, 10.0, 0.0));
    }

    #[test]
    fn a_bolt_finds_the_hero_nearest_the_aim_within_range() {
        let heroes = [
            (4, Vec3::new(600.0, 0.0, 0.0)),
            (7, Vec3::new(500.0, 0.0, 300.0)),
            (9, Vec3::new(1500.0, 0.0, 0.0)),
        ];
        let caster = Vec3::ZERO;
        assert_eq!(
            bolt_target(&heroes, caster, Vec3::new(520.0, 0.0, 280.0), 700.0),
            Some(7)
        );
        // Out of range, even with the cursor on top of it.
        assert_eq!(
            bolt_target(&heroes, caster, Vec3::new(1500.0, 0.0, 0.0), 700.0),
            None
        );
        // Nothing near the cursor.
        assert_eq!(
            bolt_target(&heroes, caster, Vec3::new(-900.0, 0.0, 0.0), 700.0),
            None
        );
    }

    #[test]
    fn vengeance_reflects_once_then_is_gone() {
        let mut v = Some(Vengeance {
            reflect: 0.75,
            timer: 6.0,
            say: "Taste vengeance!".into(),
        });
        // A minion's hit does not spend it.
        assert_eq!(trigger_vengeance(&mut v, 40.0, false), None);
        assert!(v.is_some());
        let fired = trigger_vengeance(&mut v, 100.0, true);
        assert_eq!(fired, Some((75.0, "Taste vengeance!".to_string())));
        assert_eq!(trigger_vengeance(&mut v, 100.0, true), None);
        // Unused, it lapses.
        let mut v = Some(Vengeance {
            reflect: 0.75,
            timer: 1.0,
            say: String::new(),
        });
        tick_vengeance(&mut v, 0.6);
        assert!(v.is_some());
        tick_vengeance(&mut v, 0.6);
        assert!(v.is_none());
    }

    #[test]
    fn descriptions_come_from_the_numbers() {
        let book = SpellBook::shipped();
        let text = |id: &str| book.get(book.index_of(id).unwrap()).unwrap().describe();
        assert_eq!(
            text("heal_group"),
            "Heal 15% of max health, and allied heroes within 600 heal 10%."
        );
        assert_eq!(
            text("ice_barrage"),
            "A bolt at an enemy hero within 700: deal 90 damage and root for 1.5 s."
        );
        assert_eq!(
            text("tele_block"),
            "A bolt at an enemy hero within 700: block dashes, blinks and teleports for 4 s and halve healing for 4 s."
        );
        assert_eq!(
            text("entangle"),
            "Enemies within 320 of you: root for 1.25 s."
        );
    }
}
