//! Talents: at levels 10, 15, 20 and 25 a hero picks one of two talents
//! instead of earning a skill point (`docs/ABILITY_REWORK.md`, SYS-7).
//!
//! A champion file lists four tiers of two (`talents`). A talent is a stat
//! (health, damage, ability power, armour, mana, move or attack speed) or a
//! change to one ability (its damage, cooldown or reach) or to all of them
//! (cooldown). Picking one applies it at once and for the rest of the match.

use serde::Deserialize;

/// Hero levels that award a talent pick, lowest tier first.
pub const TALENT_LEVELS: [u32; 4] = [10, 15, 20, 25];

/// True when reaching `level` awards a talent pick instead of a skill point.
pub fn is_talent_level(level: u32) -> bool {
    TALENT_LEVELS.contains(&level)
}

/// One talent: a name for the HUD and what it does.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Talent {
    pub name: String,
    #[serde(flatten)]
    pub effect: TalentEffect,
}

/// What a talent changes. Percentages are fractions (0.2 = 20%); `slot` is
/// 0..4 for Q..R.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalentEffect {
    Health {
        amount: f32,
    },
    Damage {
        amount: f32,
    },
    AbilityPower {
        amount: f32,
    },
    Armor {
        amount: f32,
    },
    Mana {
        amount: f32,
    },
    MoveSpeed {
        pct: f32,
    },
    AttackSpeed {
        pct: f32,
    },
    /// Every ability's cooldown is this much shorter.
    Cooldown {
        pct: f32,
    },
    SlotDamage {
        slot: usize,
        pct: f32,
    },
    SlotCooldown {
        slot: usize,
        pct: f32,
    },
    SlotRange {
        slot: usize,
        pct: f32,
    },
}

/// The ability modifiers a hero's talents add up to. Stats go straight onto
/// the unit when picked; these are read at cast time.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TalentMods {
    /// Fraction off every ability's cooldown.
    pub cooldown: f32,
    pub slot_damage: [f32; 4],
    pub slot_cooldown: [f32; 4],
    pub slot_range: [f32; 4],
}

impl TalentMods {
    /// The cooldown multiplier for `slot`, never below 40%.
    pub fn cooldown_mult(&self, slot: usize) -> f32 {
        (1.0 - self.cooldown - self.slot_cooldown.get(slot).copied().unwrap_or(0.0)).max(0.4)
    }

    pub fn damage_mult(&self, slot: usize) -> f32 {
        1.0 + self.slot_damage.get(slot).copied().unwrap_or(0.0)
    }

    pub fn range_mult(&self, slot: usize) -> f32 {
        1.0 + self.slot_range.get(slot).copied().unwrap_or(0.0)
    }
}

impl TalentEffect {
    /// One line for the HUD, naming `abilities[slot]` where it applies.
    pub fn describe(&self, abilities: &[String]) -> String {
        let pct = |p: f32| (p * 100.0).round() as i32;
        let name = |slot: usize| {
            abilities
                .get(slot)
                .cloned()
                .unwrap_or_else(|| ["Q", "W", "E", "R"][slot.min(3)].to_string())
        };
        match *self {
            TalentEffect::Health { amount } => format!("+{amount:.0} health"),
            TalentEffect::Damage { amount } => format!("+{amount:.0} attack damage"),
            TalentEffect::AbilityPower { amount } => format!("+{amount:.0} ability power"),
            TalentEffect::Armor { amount } => format!("+{amount:.0} armour"),
            TalentEffect::Mana { amount } => format!("+{amount:.0} mana"),
            TalentEffect::MoveSpeed { pct: p } => format!("+{}% move speed", pct(p)),
            TalentEffect::AttackSpeed { pct: p } => format!("+{}% attack speed", pct(p)),
            TalentEffect::Cooldown { pct: p } => format!("-{}% ability cooldowns", pct(p)),
            TalentEffect::SlotDamage { slot, pct: p } => {
                format!("+{}% {} damage", pct(p), name(slot))
            }
            TalentEffect::SlotCooldown { slot, pct: p } => {
                format!("-{}% {} cooldown", pct(p), name(slot))
            }
            TalentEffect::SlotRange { slot, pct: p } => {
                format!("+{}% {} range", pct(p), name(slot))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn talents_come_at_ten_fifteen_twenty_and_twenty_five() {
        assert!(is_talent_level(10));
        assert!(is_talent_level(25));
        assert!(!is_talent_level(11));
        assert!(!is_talent_level(1));
    }

    #[test]
    fn a_talent_reads_from_json() {
        let talent: Talent = serde_json::from_str(
            r#"{"name": "Quick hooks", "type": "slot_cooldown", "slot": 0, "pct": 0.15}"#,
        )
        .unwrap();
        assert_eq!(
            talent.effect,
            TalentEffect::SlotCooldown { slot: 0, pct: 0.15 }
        );
    }

    #[test]
    fn cooldowns_never_drop_below_forty_percent() {
        let mods = TalentMods {
            cooldown: 0.4,
            slot_cooldown: [0.4, 0.0, 0.0, 0.0],
            ..TalentMods::default()
        };
        assert_eq!(mods.cooldown_mult(0), 0.4);
        assert!((mods.cooldown_mult(1) - 0.6).abs() < 1e-6);
    }
}
