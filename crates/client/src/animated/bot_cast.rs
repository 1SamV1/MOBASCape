//! Abilities for bot heroes.
//!
//! The player's kits run through `kit_runtime` and the hand-written casts,
//! which keep their state for one caster (the player). Bots instead cast a
//! reading of the same champion data every hero has: the Q/W/E/R rank
//! tables (damage, range, cooldown) and the description. Each ability is
//! sorted into one of four shapes:
//!
//! - **Care**: no damage in its ranks (heals, shields, stances). Heals a
//!   quarter of missing health, or shields 15% of max health when the label
//!   says "Shield". Cast below 60% health in a fight.
//! - **Nova**: damage, range 0. Hits everything around the caster.
//! - **Strike**: damage, short range (≤ 450). A blast at the target that
//!   splashes its neighbours.
//! - **Bolt**: damage, long range. A homing shot that cannot miss.
//!
//! Crowd control comes from the description: "stun" stuns, "root"/"snare"
//! roots, "slow"/"chill" slows. Ranks unlock with level like the player's
//! (Q at 1, W at 2, E at 3, R at 6; another rank every three levels), and
//! damage scales with ability power and level through `kit::scaled_damage`.

use glam::Vec3;

use super::{AnimatedScene, Effect, HitsplatKind, Projectile, UnitKind, planar_delta};
use crate::champion::{AbilityRank, ChampionAbility};
use crate::fx;

/// Splash radius of a strike and reach of a nova.
const STRIKE_SPLASH: f32 = 170.0;
const NOVA_RADIUS: f32 = 320.0;
/// Range at or under which a damaging ability is a strike, not a bolt.
const STRIKE_RANGE: f32 = 450.0;
/// Health fraction under which a bot uses its care ability.
const CARE_AT: f32 = 0.6;
/// Mana: basic abilities cost `BASIC_COST + 10 per rank`, the ultimate
/// `ULT_COST`. Bots regenerate `MANA_REGEN` of max mana per second.
const BASIC_COST: f32 = 25.0;
const ULT_COST: f32 = 80.0;
pub(super) const MANA_REGEN: f32 = 0.02;
/// Short pause after a cast before the next one, so spells read one at a time.
const CAST_GAP: f32 = 0.6;
/// Bolt projectile speed.
const BOLT_SPEED: f32 = 1400.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpellShape {
    Care { shield: bool },
    Nova,
    Strike,
    Bolt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpellCc {
    Stun(f32),
    Root(f32),
    Slow(f32, f32),
}

#[derive(Debug, Clone)]
pub struct BotSpell {
    pub slot: usize,
    pub name: String,
    pub shape: SpellShape,
    pub cc: Option<SpellCc>,
    pub ranks: Vec<AbilityRank>,
}

impl BotSpell {
    /// Read one hero-select ability as a bot spell. `slot` is 0..4 (Q..R).
    pub fn from_ability(slot: usize, ability: &ChampionAbility) -> Option<Self> {
        if ability.ranks.is_empty() {
            return None;
        }
        let damage = ability.ranks.iter().any(|r| r.damage.unwrap_or(0.0) > 0.0);
        let label = ability
            .damage_label
            .as_deref()
            .unwrap_or("Damage")
            .to_ascii_lowercase();
        let range = ability.ranks[0].range;
        let shape = if !damage || label == "heal" || label == "shield" {
            SpellShape::Care {
                shield: label == "shield",
            }
        } else if range <= 1.0 {
            SpellShape::Nova
        } else if range <= STRIKE_RANGE {
            SpellShape::Strike
        } else {
            SpellShape::Bolt
        };
        let text = ability.description.to_ascii_lowercase();
        let ult = slot == 3;
        let cc = if matches!(shape, SpellShape::Care { .. }) {
            None
        } else if text.contains("stun") {
            Some(SpellCc::Stun(if ult { 1.2 } else { 0.8 }))
        } else if text.contains("root") || text.contains("snare") {
            Some(SpellCc::Root(1.0))
        } else if text.contains("slow") || text.contains("chill") {
            Some(SpellCc::Slow(0.35, 2.0))
        } else {
            None
        };
        Some(Self {
            slot,
            name: ability.name.clone(),
            shape,
            cc,
            ranks: ability.ranks.clone(),
        })
    }

    /// Rank known at `level` (0 = not learned yet).
    pub fn rank_at(&self, level: u32) -> usize {
        let (first, step) = if self.slot == 3 {
            (6, 3)
        } else {
            (self.slot as u32 + 1, 3)
        };
        if level < first {
            return 0;
        }
        (1 + ((level - first) / step) as usize).min(self.ranks.len())
    }

    fn cost(&self, rank: usize) -> f32 {
        if self.slot == 3 {
            ULT_COST
        } else {
            BASIC_COST + 10.0 * rank as f32
        }
    }

    fn color(&self) -> [u8; 3] {
        match self.shape {
            SpellShape::Care { shield: true } => [150, 200, 255],
            SpellShape::Care { shield: false } => [110, 230, 120],
            _ => [
                [255, 190, 80],
                [120, 200, 255],
                [200, 140, 255],
                [255, 90, 70],
            ][self.slot.min(3)],
        }
    }
}

/// Every castable ability of a champion, Q to R.
pub fn spells_for(abilities: &[ChampionAbility]) -> Vec<BotSpell> {
    abilities
        .iter()
        .take(4)
        .enumerate()
        .filter_map(|(slot, ability)| BotSpell::from_ability(slot, ability))
        .collect()
}

impl AnimatedScene {
    /// Try one ability on `target` (an enemy the bot is fighting). Returns
    /// true when something was cast. Ultimates are saved for heroes; damage
    /// spells on minions only when the bot has mana to spare.
    pub(super) fn bot_try_cast(&mut self, b: usize, target: usize) -> bool {
        let index = self.bots[b].unit;
        if self.bots[b].cast_gap > 0.0 {
            return false;
        }
        let (level, mana, max_mana, hp_frac, pos, stunned) = {
            let u = &self.units[index];
            (
                u.level,
                u.mana,
                u.max_mana.max(1.0),
                u.health / u.max_health.max(1.0),
                u.pos,
                u.stun_timer > 0.0,
            )
        };
        if stunned || self.units[target].health <= 0.0 {
            return false;
        }
        let target_kind = self.units[target].kind;
        let hero = matches!(target_kind, UnitKind::Player | UnitKind::BotChampion);
        let big = hero || (target_kind == UnitKind::Camp && self.units[target].camp_boss);
        let distance = (planar_delta(pos, self.units[target].pos).length()
            - self.units[target].hit_radius)
            .max(0.0);
        let spells = self.bots[b].spells.clone();
        for spell in spells.iter().rev() {
            let rank = spell.rank_at(level);
            if rank == 0 || self.units[index].cooldowns[spell.slot] > 0.0 {
                continue;
            }
            let cost = spell.cost(rank);
            if mana < cost {
                continue;
            }
            let numbers = spell.ranks[rank - 1];
            let wanted = match spell.shape {
                SpellShape::Care { .. } => hp_frac < CARE_AT,
                SpellShape::Nova | SpellShape::Strike | SpellShape::Bolt => {
                    let reach = if spell.shape == SpellShape::Nova {
                        NOVA_RADIUS
                    } else {
                        numbers.range.max(STRIKE_SPLASH)
                    };
                    distance <= reach
                        && (hero || (spell.slot < 3 && (big || mana > max_mana * 0.6)))
                }
            };
            if !wanted {
                continue;
            }
            self.units[index].cooldowns[spell.slot] = numbers.cooldown;
            self.units[index].mana -= cost;
            self.bots[b].cast_gap = CAST_GAP;
            self.cast_bot_spell(index, target, spell, numbers);
            return true;
        }
        false
    }

    fn cast_bot_spell(&mut self, index: usize, target: usize, spell: &BotSpell, rank: AbilityRank) {
        let (pos, team, ap, level) = {
            let u = &self.units[index];
            (u.pos, u.team, u.ability_power, u.level)
        };
        let color = spell.color();
        let damage = crate::kit::scaled_damage(rank.damage.unwrap_or(0.0), ap, level);
        let target_pos = self.units[target].pos;
        let dir = planar_delta(pos, target_pos);
        if dir.length_squared() > 1.0 {
            self.units[index].yaw = dir.x.atan2(dir.z);
        }
        // The hero's own attack swing sells the cast.
        if let Some(clip) = self.units[index].attack_clip.clone() {
            let hit = self.units[index].attack_hit;
            self.units[index].action = Some(super::ActionAnim::new(
                clip.clone(),
                super::action_rate(&clip, hit, 0.3),
            ));
        }
        self.effects.push(Effect {
            pos,
            radius: 110.0,
            remaining: 0.3,
            duration: 0.3,
            color,
        });
        tracing::debug!(
            "bots: {} casts {} ({:?}) for {damage:.0}",
            self.bot_of(index).map_or("?", |bot| bot.name.as_str()),
            spell.name,
            spell.shape
        );
        match spell.shape {
            SpellShape::Care { shield } => {
                let u = &mut self.units[index];
                if shield {
                    u.shield += u.max_health * 0.15;
                } else {
                    let healed = ((u.max_health - u.health) * 0.25).max(0.0);
                    u.health = (u.health + healed).min(u.max_health);
                    self.push_hitsplat(index, healed, HitsplatKind::Heal, None);
                }
                self.effects.push(Effect {
                    pos,
                    radius: 160.0,
                    remaining: 0.6,
                    duration: 0.6,
                    color,
                });
            }
            SpellShape::Nova => {
                self.bot_spell_area(index, pos, NOVA_RADIUS, damage, spell.cc, color);
            }
            SpellShape::Strike => {
                self.bot_spell_area(index, target_pos, STRIKE_SPLASH, damage, spell.cc, color);
            }
            SpellShape::Bolt => {
                let look = if spell.slot == 3 {
                    fx::look_by_name("fire_wave")
                } else if self.units[index].ranged_auto {
                    self.units[index].look
                } else {
                    fx::look_by_name("earth_blast")
                };
                let from = pos + Vec3::Y * 90.0;
                let shot = Projectile::homing(
                    from,
                    target,
                    target_pos + Vec3::Y * 60.0,
                    BOLT_SPEED,
                    damage,
                    team,
                    Some(index),
                    look,
                );
                self.projectiles.push(shot);
                // The bolt's crowd control lands up front: homing shots
                // cannot miss, and projectiles carry no affliction.
                if let Some(cc) = spell.cc {
                    self.apply_bot_cc(target, cc);
                }
            }
        }
    }

    /// Damage (and crowd control) on every enemy of `caster` in a circle.
    fn bot_spell_area(
        &mut self,
        caster: usize,
        at: Vec3,
        radius: f32,
        damage: f32,
        cc: Option<SpellCc>,
        color: [u8; 3],
    ) {
        let team = self.units[caster].team;
        self.effects.push(Effect {
            pos: at,
            radius,
            remaining: 0.45,
            duration: 0.45,
            color,
        });
        let hits: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.health > 0.0
                    && u.team != team
                    && !u.kind.is_structure()
                    && (u.team < 2 || u.kind == UnitKind::Camp)
                    && (u.visible || u.team == 2 || team == 0)
                    && planar_delta(at, u.pos).length() <= radius + u.hit_radius
            })
            .collect();
        for i in hits {
            self.damage_unit_mixed_kind(i, damage, 0.0, team, Some(caster), HitsplatKind::Damage);
            if let Some(cc) = cc {
                self.apply_bot_cc(i, cc);
            }
        }
    }

    fn apply_bot_cc(&mut self, target: usize, cc: SpellCc) {
        let u = &mut self.units[target];
        if u.health <= 0.0 || u.kind.is_structure() {
            return;
        }
        match cc {
            SpellCc::Stun(t) => u.stun_timer = u.stun_timer.max(t),
            SpellCc::Root(t) => u.cc.root_timer = u.cc.root_timer.max(t),
            SpellCc::Slow(pct, t) => {
                u.cc.slow_pct = u.cc.slow_pct.max(pct);
                u.cc.slow_timer = u.cc.slow_timer.max(t);
            }
        }
    }

    /// Cooldowns, cast gap and mana for every bot (alive or not).
    pub(super) fn tick_bot_spells(&mut self, dt: f32) {
        for b in 0..self.bots.len() {
            self.bots[b].cast_gap = (self.bots[b].cast_gap - dt).max(0.0);
            let u = &mut self.units[self.bots[b].unit];
            for cd in &mut u.cooldowns {
                *cd = (*cd - dt).max(0.0);
            }
            if u.health > 0.0 {
                u.mana = (u.mana + u.max_mana * MANA_REGEN * dt).min(u.max_mana);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ability(desc: &str, label: Option<&str>, ranks: &[(Option<f32>, f32)]) -> ChampionAbility {
        ChampionAbility {
            key: "Q".into(),
            name: "TEST".into(),
            description: desc.into(),
            ranks: ranks
                .iter()
                .map(|&(damage, range)| AbilityRank {
                    damage,
                    range,
                    cooldown: 5.0,
                })
                .collect(),
            damage_label: label.map(str::to_string),
            damage_note: None,
        }
    }

    #[test]
    fn shapes_come_from_rank_numbers() {
        let heal = ability("Heals.", Some("Heal"), &[(None, 0.0)]);
        let nova = ability("Slams the ground.", None, &[(Some(40.0), 0.0)]);
        let strike = ability("A swipe that stuns.", None, &[(Some(40.0), 260.0)]);
        let bolt = ability("A slowing bolt.", None, &[(Some(40.0), 900.0)]);
        let shapes: Vec<_> = [heal, nova, strike, bolt]
            .iter()
            .enumerate()
            .map(|(slot, a)| {
                let s = BotSpell::from_ability(slot, a).unwrap();
                (s.shape, s.cc)
            })
            .collect();
        assert_eq!(shapes[0], (SpellShape::Care { shield: false }, None));
        assert_eq!(shapes[1], (SpellShape::Nova, None));
        assert_eq!(shapes[2], (SpellShape::Strike, Some(SpellCc::Stun(0.8))));
        assert_eq!(
            shapes[3],
            (SpellShape::Bolt, Some(SpellCc::Slow(0.35, 2.0)))
        );
    }

    #[test]
    fn ranks_unlock_with_level() {
        let q = BotSpell::from_ability(0, &ability("", None, &[(Some(1.0), 300.0); 4])).unwrap();
        let r = BotSpell::from_ability(3, &ability("", None, &[(Some(1.0), 300.0); 3])).unwrap();
        assert_eq!(
            (q.rank_at(1), q.rank_at(4), q.rank_at(10), q.rank_at(11)),
            (1, 2, 4, 4)
        );
        assert_eq!(
            (r.rank_at(5), r.rank_at(6), r.rank_at(9), r.rank_at(11)),
            (0, 1, 2, 2)
        );
    }

    #[test]
    fn every_champion_has_bot_spells() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/champions");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            let champion: crate::champion::ChampionConfig = serde_json::from_str(&text).unwrap();
            let spells = spells_for(&champion.abilities);
            assert_eq!(spells.len(), 4, "{}", path.display());
        }
    }
}
