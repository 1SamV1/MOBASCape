//! Item passives and actives (docs/ITEMS.md). The numbers live in
//! `data/items/items.json`; `animated::gear` runs them. Every rare item
//! carries at least one of these so its identity is a behaviour, not just a
//! bigger stat line.

use serde::Deserialize;

use crate::kit::Status;

/// A passive with the name the shop shows for it.
#[derive(Debug, Clone, Deserialize)]
pub struct NamedPassive {
    pub name: String,
    #[serde(flatten)]
    pub effect: ItemPassive,
}

/// An active, used from its bag slot (1-6 with the shop closed).
#[derive(Debug, Clone, Deserialize)]
pub struct NamedActive {
    pub name: String,
    /// Seconds between uses.
    pub cooldown: f32,
    #[serde(flatten)]
    pub effect: ActiveEffect,
}

fn one() -> f32 {
    1.0
}
fn one_u32() -> u32 {
    1
}
fn special_window() -> f32 {
    6.0
}
fn ability_hit_gap() -> f32 {
    0.5
}

/// What owning the item does. Two copies of one item never stack: only the
/// first copy's passives count.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemPassive {
    /// A `chance` that a basic attack deals `mult` times its damage (the
    /// bonus pops as a gold max-hit mark).
    Crit { chance: f32, mult: f32 },
    /// Basic attacks heal this share of the damage they deal.
    Lifesteal { pct: f32 },
    /// Ability damage heals this share of the damage dealt.
    SpellVamp { pct: f32 },
    /// Melee basic attacks also hit enemies within `radius` of the target.
    Cleave { radius: f32, pct: f32 },
    /// Ranged basic attacks bounce on to `bounces` more enemies within
    /// `range` of the last one hit, for `pct` of the damage each.
    Ricochet { bounces: u32, pct: f32, range: f32 },
    /// Extra effects on every `every`th basic attack (with `chance`):
    /// statuses and bonus pure damage (flat, or a share of the target's
    /// current or max health).
    OnHit {
        #[serde(default)]
        statuses: Vec<Status>,
        #[serde(default = "one_u32")]
        every: u32,
        #[serde(default = "one")]
        chance: f32,
        #[serde(default)]
        flat: f32,
        #[serde(default)]
        max_health: f32,
        #[serde(default)]
        current_health: f32,
        /// Bonus pure damage is capped at this against non-heroes (0: no cap).
        #[serde(default)]
        cap: f32,
    },
    /// Ability damage also lands these statuses, at most once per `gap`
    /// seconds.
    AbilityHit {
        statuses: Vec<Status>,
        #[serde(default = "ability_hit_gap")]
        gap: f32,
    },
    /// Physical damage ignores this share of the target's armor.
    ArmorPen { pct: f32 },
    /// Up to `max` more damage as the owner's health runs out.
    Berserk { max: f32 },
    /// More damage against targets below `threshold` of their health.
    Execute { threshold: f32, pct: f32 },
    /// `per_100` more damage for every 100 max health the target has over
    /// the owner, up to `max`.
    GiantSlayer { per_100: f32, max: f32 },
    /// More damage against structures, jungle camps and bosses.
    BigGame { pct: f32 },
    /// Ability damage is raised by this share.
    AbilityAmp { pct: f32 },
    /// A `chance` that an incoming hit is cut by `pct`.
    Elysian { chance: f32, pct: f32 },
    /// All incoming damage is cut by this share.
    DamageReduction { pct: f32 },
    /// Damage from ranged attackers (reach 400 and over) is cut by this share.
    RangedWard { pct: f32 },
    /// Basic attacks taken send this share back as pure damage.
    Reflect { pct: f32 },
    /// Health and mana a second.
    Regen {
        #[serde(default)]
        health: f32,
        #[serde(default)]
        mana: f32,
    },
    /// Dropping under `threshold` health heals `heal` of max health, once
    /// per `cooldown` seconds.
    LastStand {
        threshold: f32,
        heal: f32,
        cooldown: f32,
    },
    /// Kill gold is raised by this share.
    Bounty { pct: f32 },
    /// Each basic attack adds `per_hit` attack speed, up to `max`; the
    /// stacks fall off `window` seconds after the last attack.
    Frenzy { per_hit: f32, max: f32, window: f32 },
    /// After an ability, the next basic attack within 10 s deals bonus
    /// pure damage: `damage` of attack damage plus `ability` of ability
    /// power (as points). Once per `cooldown` seconds.
    Spellblade {
        damage: f32,
        #[serde(default)]
        ability: f32,
        cooldown: f32,
    },
    /// Thralls raised from scrolls get `pct` more health and damage and
    /// last `duration` seconds longer.
    ThrallMaster { pct: f32, duration: f32 },
    /// Allied heroes within `radius` (the owner too) regain `health` a second.
    Aura { radius: f32, health: f32 },
}

/// What using the item does.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActiveEffect {
    /// An OSRS special attack: the next basic attack within `window`
    /// seconds lands `extra` more hits (each a share of the hit), its
    /// statuses, heals `heal` of the damage, and splashes the extra hits
    /// to enemies within `splash` of the target.
    Special {
        #[serde(default)]
        extra: Vec<f32>,
        #[serde(default)]
        statuses: Vec<Status>,
        #[serde(default)]
        heal: f32,
        #[serde(default)]
        splash: f32,
        #[serde(default = "special_window")]
        window: f32,
    },
    /// A burst around the owner: damage (raised by ability power), statuses
    /// and a heal of `heal` of the damage dealt.
    Nova {
        radius: f32,
        damage: f32,
        #[serde(default)]
        statuses: Vec<Status>,
        #[serde(default)]
        heal: f32,
    },
    /// A homing bolt at the enemy hero nearest the owner within `range`
    /// (raised by ability power); `mana` is restored on use.
    Bolt {
        range: f32,
        damage: f32,
        #[serde(default)]
        statuses: Vec<Status>,
        #[serde(default)]
        mana: f32,
        /// Projectile look from `fx::LOOKS` (a trident bolt by default).
        #[serde(default)]
        look: Option<String>,
    },
    /// A shield of `amount` plus `max_health` of max health.
    Shield {
        amount: f32,
        #[serde(default)]
        max_health: f32,
        duration: f32,
    },
    /// Move speed raised by `pct` for `duration` seconds.
    Haste { pct: f32, duration: f32 },
    /// Attack speed raised by `pct` for `duration` seconds.
    Rapid { pct: f32, duration: f32 },
    /// Restore shares of max health and mana.
    Restore {
        #[serde(default)]
        health: f32,
        #[serde(default)]
        mana: f32,
    },
    /// Every ability cooldown drops by `seconds`.
    Refresh { seconds: f32 },
}

fn pct(share: f32) -> String {
    format!("{:.0}%", share * 100.0)
}

fn secs(seconds: f32) -> String {
    if (seconds - seconds.round()).abs() < 0.05 {
        format!("{:.0} s", seconds)
    } else {
        format!("{:.1} s", seconds)
    }
}

fn statuses_text(statuses: &[Status]) -> String {
    let parts: Vec<String> = statuses.iter().map(Status::describe).collect();
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        _ => {
            let (last, rest) = parts.split_last().expect("two or more");
            format!("{} and {last}", rest.join(", "))
        }
    }
}

fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

impl ItemPassive {
    /// The passive in a sentence, written from its numbers.
    pub fn describe(&self) -> String {
        match self {
            ItemPassive::Crit { chance, mult } => format!(
                "{} chance for a basic attack to deal {:.0}% damage.",
                pct(*chance),
                mult * 100.0
            ),
            ItemPassive::Lifesteal { pct: p } => {
                format!("Basic attacks heal {} of the damage dealt.", pct(*p))
            }
            ItemPassive::SpellVamp { pct: p } => {
                format!("Ability damage heals {} of the damage dealt.", pct(*p))
            }
            ItemPassive::Cleave { radius, pct: p } => format!(
                "Melee attacks also hit enemies within {radius:.0} of the target for {}.",
                pct(*p)
            ),
            ItemPassive::Ricochet {
                bounces, pct: p, ..
            } => format!(
                "Ranged attacks bounce to {bounces} more {} for {} damage.",
                if *bounces == 1 { "enemy" } else { "enemies" },
                pct(*p)
            ),
            ItemPassive::OnHit {
                statuses,
                every,
                chance,
                flat,
                max_health,
                current_health,
                ..
            } => {
                let when = match (*every, *chance < 1.0) {
                    (1, false) => "Basic attacks".to_string(),
                    (1, true) => format!("{} of basic attacks", pct(*chance)),
                    (n, _) => format!("Every {} basic attack", ordinal(n)),
                };
                let mut parts = Vec::new();
                if *flat > 0.0 {
                    parts.push(format!("deal {flat:.0} bonus damage"));
                }
                if *max_health > 0.0 {
                    parts.push(format!("deal {} of max health", pct(*max_health)));
                }
                if *current_health > 0.0 {
                    parts.push(format!("deal {} of current health", pct(*current_health)));
                }
                let effects = statuses_text(statuses);
                if !effects.is_empty() {
                    parts.push(effects);
                }
                format!("{when} {}.", parts.join(" and "))
            }
            ItemPassive::AbilityHit { statuses, .. } => {
                format!("Abilities {}.", statuses_text(statuses))
            }
            ItemPassive::ArmorPen { pct: p } => {
                format!("Physical damage ignores {} of armor.", pct(*p))
            }
            ItemPassive::Berserk { max } => {
                format!("Up to {} more damage as your health runs low.", pct(*max))
            }
            ItemPassive::Execute { threshold, pct: p } => format!(
                "{} more damage to targets below {} health.",
                pct(*p),
                pct(*threshold)
            ),
            ItemPassive::GiantSlayer { per_100, max } => format!(
                "{} more damage per 100 max health the target has over you (up to {}).",
                pct(*per_100),
                pct(*max)
            ),
            ItemPassive::BigGame { pct: p } => {
                format!("{} more damage to structures, camps and bosses.", pct(*p))
            }
            ItemPassive::AbilityAmp { pct: p } => format!("Ability damage +{}.", pct(*p)),
            ItemPassive::Elysian { chance, pct: p } => format!(
                "{} chance to cut an incoming hit by {}.",
                pct(*chance),
                pct(*p)
            ),
            ItemPassive::DamageReduction { pct: p } => {
                format!("Take {} less damage.", pct(*p))
            }
            ItemPassive::RangedWard { pct: p } => {
                format!("Take {} less damage from ranged attackers.", pct(*p))
            }
            ItemPassive::Reflect { pct: p } => {
                format!("Returns {} of basic attack damage taken.", pct(*p))
            }
            ItemPassive::Regen { health, mana } => match (*health > 0.0, *mana > 0.0) {
                (true, true) => format!("Regain {health:.0} health and {mana:.0} mana a second."),
                (true, false) => format!("Regain {health:.0} health a second."),
                _ => format!("Regain {mana:.0} mana a second."),
            },
            ItemPassive::LastStand {
                threshold,
                heal,
                cooldown,
            } => format!(
                "Falling below {} health heals {} of max health ({} cooldown).",
                pct(*threshold),
                pct(*heal),
                secs(*cooldown)
            ),
            ItemPassive::Bounty { pct: p } => format!("Kills pay {} more gold.", pct(*p)),
            ItemPassive::Frenzy {
                per_hit,
                max,
                window,
            } => format!(
                "Each attack adds {} attack speed, up to {} (fades after {}).",
                pct(*per_hit),
                pct(*max),
                secs(*window)
            ),
            ItemPassive::Spellblade {
                damage,
                ability,
                cooldown,
            } => {
                let ap = if *ability > 0.0 {
                    format!(" + {} of ability power", pct(*ability))
                } else {
                    String::new()
                };
                format!(
                    "After an ability, your next attack deals {} of attack damage{ap} as bonus damage ({}).",
                    pct(*damage),
                    secs(*cooldown)
                )
            }
            ItemPassive::ThrallMaster { pct: p, duration } => format!(
                "Thralls have {} more health and damage and last {} longer.",
                pct(*p),
                secs(*duration)
            ),
            ItemPassive::Aura { radius, health } => {
                format!("Allied heroes within {radius:.0} regain {health:.0} health a second.")
            }
        }
    }
}

impl ActiveEffect {
    /// The active in a sentence, written from its numbers.
    pub fn describe(&self) -> String {
        match self {
            ActiveEffect::Special {
                extra,
                statuses,
                heal,
                splash,
                ..
            } => {
                let mut parts = Vec::new();
                match extra.len() {
                    0 => {}
                    1 if extra[0] >= 0.99 => parts.push("hits twice".to_string()),
                    1 => parts.push(format!("deals {:.0}% damage", (1.0 + extra[0]) * 100.0)),
                    n => parts.push(format!(
                        "hits {} times for {:.0}% in all",
                        n + 1,
                        (1.0 + extra.iter().sum::<f32>()) * 100.0
                    )),
                }
                if *splash > 0.0 {
                    parts.push(format!("explodes within {splash:.0}"));
                }
                let effects = statuses_text(statuses);
                if !effects.is_empty() {
                    parts.push(effects);
                }
                if *heal > 0.0 {
                    parts.push(format!("heals {} of it", pct(*heal)));
                }
                format!("Your next attack {}.", parts.join(", "))
            }
            ActiveEffect::Nova {
                radius,
                damage,
                statuses,
                heal,
            } => {
                let mut text = format!("Blast enemies within {radius:.0} for {damage:.0} damage");
                let effects = statuses_text(statuses);
                if !effects.is_empty() {
                    text.push_str(&format!(" and {effects}"));
                }
                if *heal > 0.0 {
                    text.push_str(&format!(", healing {} of it", pct(*heal)));
                }
                text.push('.');
                text
            }
            ActiveEffect::Bolt {
                range,
                damage,
                statuses,
                mana,
                ..
            } => {
                let mut text = format!(
                    "Fire a bolt at the nearest enemy hero within {range:.0} for {damage:.0}"
                );
                let effects = statuses_text(statuses);
                if !effects.is_empty() {
                    text.push_str(&format!(" that will {effects}"));
                }
                if *mana > 0.0 {
                    text.push_str(&format!("; restore {mana:.0} mana"));
                }
                text.push('.');
                text
            }
            ActiveEffect::Shield {
                amount,
                max_health,
                duration,
            } => {
                let extra = if *max_health > 0.0 {
                    format!(" + {} max health", pct(*max_health))
                } else {
                    String::new()
                };
                format!(
                    "Shield yourself for {amount:.0}{extra} for {}.",
                    secs(*duration)
                )
            }
            ActiveEffect::Haste { pct: p, duration } => {
                format!("Move {} faster for {}.", pct(*p), secs(*duration))
            }
            ActiveEffect::Rapid { pct: p, duration } => {
                format!("Attack {} faster for {}.", pct(*p), secs(*duration))
            }
            ActiveEffect::Restore { health, mana } => match (*health > 0.0, *mana > 0.0) {
                (true, true) => format!(
                    "Restore {} of max health and {} of max mana.",
                    pct(*health),
                    pct(*mana)
                ),
                (true, false) => format!("Restore {} of max health.", pct(*health)),
                _ => format!("Restore {} of max mana.", pct(*mana)),
            },
            ActiveEffect::Refresh { seconds } => {
                format!("Your ability cooldowns drop by {}.", secs(*seconds))
            }
        }
    }

    /// True for actives a bot saves for a fight it is losing.
    pub fn defensive(&self) -> bool {
        matches!(
            self,
            ActiveEffect::Shield { .. } | ActiveEffect::Restore { .. } | ActiveEffect::Haste { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passives_and_actives_parse_flat_with_their_names() {
        let passive: NamedPassive = serde_json::from_str(
            r#"{"name": "Energy drain", "type": "on_hit", "every": 4,
                "statuses": [{"type": "slow", "pct": 0.2, "duration": 1.5}]}"#,
        )
        .expect("on_hit");
        assert_eq!(passive.name, "Energy drain");
        assert_eq!(
            passive.effect.describe(),
            "Every 4th basic attack slow by 20% for 1.5 s."
        );
        let active: NamedActive = serde_json::from_str(
            r#"{"name": "Puncture", "cooldown": 20, "type": "special", "extra": [1.0]}"#,
        )
        .expect("special");
        assert_eq!(active.cooldown, 20.0);
        assert_eq!(active.effect.describe(), "Your next attack hits twice.");
        let claws: ActiveEffect =
            serde_json::from_str(r#"{"type": "special", "extra": [0.5, 0.25, 0.25]}"#).unwrap();
        assert_eq!(
            claws.describe(),
            "Your next attack hits 4 times for 200% in all."
        );
    }
}
