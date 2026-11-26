//! Ability ranks and skill points.
//!
//! Every ability starts unlearned (rank 0). Level 1 and every level after it
//! give one skill point, except the talent levels 10, 15, 20 and 25, which
//! give a talent pick instead (`crate::talents`). The player spends a point
//! on an ability slot's "+". Q, W and E go to rank 4, R to rank 3, and R's
//! ranks also need champion levels ([`ULTIMATE_RANK_LEVELS`]). A point with
//! nothing left to rank becomes a stat bump. The numbers per rank are hand-authored in the
//! champion file (`abilities[].ranks`) and applied here as ratios against
//! rank 1, so the kit's actions keep their own shape.

use super::AnimatedScene;
use crate::champion::AbilityRank;

/// Ranks of Q, W and E.
pub const BASIC_MAX_RANK: u8 = 4;
/// Ranks of R.
pub const ULTIMATE_MAX_RANK: u8 = 3;
/// Champion level needed for R rank 1, 2 and 3.
pub const ULTIMATE_RANK_LEVELS: [u32; 3] = [6, 11, 16];
/// What a spare point adds once nothing is left to rank.
pub const BUMP_HEALTH: f32 = 25.0;
pub const BUMP_DAMAGE: f32 = 2.0;
pub const BUMP_POWER: f32 = 3.0;

pub use crate::talents::is_talent_level;

/// Why a slot cannot take a point right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeBlock {
    NoPoints,
    MaxRank,
    /// The champion level the next rank needs.
    NeedsLevel(u32),
}

/// The level a rank needs: R's ranks are gated, the others are not.
pub fn rank_level(slot: usize, rank: u8) -> u32 {
    if slot == 3 {
        ULTIMATE_RANK_LEVELS
            .get(rank.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(u32::MAX)
    } else {
        1
    }
}

/// Whether `slot` can go from `rank` to the next rank.
pub fn upgrade_block(
    slot: usize,
    rank: u8,
    max_rank: u8,
    level: u32,
    points: u32,
) -> Option<UpgradeBlock> {
    if rank >= max_rank {
        return Some(UpgradeBlock::MaxRank);
    }
    let needs = rank_level(slot, rank + 1);
    if level < needs {
        return Some(UpgradeBlock::NeedsLevel(needs));
    }
    if points == 0 {
        return Some(UpgradeBlock::NoPoints);
    }
    None
}

/// The rank a computer-played hero has in `slot` at `level`, out of `max`
/// ranks, spending its points the way a player would: R whenever its level
/// allows, one point each in Q, W and E first, then Q to max, then W, then
/// E. Talent levels give no point.
pub fn auto_rank(slot: usize, level: u32, max: usize) -> u8 {
    auto_ranks(level)[slot].min(max.min(u8::MAX as usize) as u8)
}

/// Every slot's rank under the bot skill order at `level`.
pub fn auto_ranks(level: u32) -> [u8; 4] {
    let mut ranks = [0u8; 4];
    for l in 1..=level {
        if is_talent_level(l) {
            continue;
        }
        let r_next = ranks[3] as usize;
        if r_next < ULTIMATE_RANK_LEVELS.len() && l >= ULTIMATE_RANK_LEVELS[r_next] {
            ranks[3] += 1;
            continue;
        }
        let pick = (0..3)
            .find(|&s| ranks[s] == 0)
            .or_else(|| (0..3).find(|&s| ranks[s] < BASIC_MAX_RANK));
        if let Some(s) = pick {
            ranks[s] += 1;
        }
    }
    ranks
}

/// Rank `rank`'s value over rank 1's, or 1 when either is missing or zero.
pub(super) fn ratio(current: Option<f32>, first: Option<f32>) -> f32 {
    match (current, first) {
        (Some(current), Some(first)) if first > 0.0 && current > 0.0 => current / first,
        _ => 1.0,
    }
}

/// A spare skill point's stat bump.
fn apply_stat_bump(unit: &mut super::AnimatedUnit) {
    unit.max_health += BUMP_HEALTH;
    unit.health += BUMP_HEALTH;
    unit.attack_damage += BUMP_DAMAGE;
    unit.ability_power += BUMP_POWER;
}

impl AnimatedScene {
    /// The champion file's entry for a slot.
    pub fn ability_info(&self, slot: usize) -> Option<&crate::champion::ChampionAbility> {
        self.ability_info.get(slot)
    }

    pub fn player_level(&self) -> u32 {
        self.units[self.player_index].level
    }

    pub fn ability_rank(&self, slot: usize) -> u8 {
        self.ability_ranks.get(slot).copied().unwrap_or(0)
    }

    pub fn ability_ranks(&self) -> [u8; 4] {
        self.ability_ranks
    }

    /// Ranks this slot has: the authored count, or the usual 4 / 3.
    pub fn max_rank(&self, slot: usize) -> u8 {
        match self.ability_info(slot).map(|info| info.ranks.len()) {
            Some(n) if n > 0 => n.min(u8::MAX as usize) as u8,
            _ if slot == 3 => ULTIMATE_MAX_RANK,
            _ => BASIC_MAX_RANK,
        }
    }

    pub fn upgrade_block(&self, slot: usize) -> Option<UpgradeBlock> {
        upgrade_block(
            slot,
            self.ability_rank(slot),
            self.max_rank(slot),
            self.units[self.player_index].level,
            self.skill_points,
        )
    }

    /// True when clicking the slot would spend a point on it.
    pub fn can_upgrade(&self, slot: usize) -> bool {
        slot < 4 && self.upgrade_block(slot).is_none()
    }

    /// Dev: gain levels up to `level`, earning their skill points.
    pub fn dev_level_to(&mut self, level: u32) {
        while self.units[self.player_index].level < level.min(super::MAX_LEVEL) {
            self.level_up_player();
        }
    }

    /// Turn skill points nothing can use into stat bumps: every slot is at
    /// its last rank, or only R is left and it waits for a level.
    pub(super) fn spend_spare_points(&mut self) {
        loop {
            if self.skill_points == 0 {
                return;
            }
            let basics_full = (0..3).all(|slot| self.ability_rank(slot) >= self.max_rank(slot));
            let r_waits = self.ability_rank(3) >= self.max_rank(3)
                || matches!(self.upgrade_block(3), Some(UpgradeBlock::NeedsLevel(_)));
            if !(basics_full && r_waits) {
                return;
            }
            self.skill_points -= 1;
            self.stat_bumps += 1;
            let pi = self.player_index;
            apply_stat_bump(&mut self.units[pi]);
            tracing::info!("skill point: stat bump {}", self.stat_bumps);
        }
    }

    /// The talent tiers of unit `index` (empty when it has none).
    pub fn talent_tiers(&self, index: usize) -> &[[crate::talents::Talent; 2]] {
        self.unit_talents
            .get(&index)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The player's Q, W, E and R names, for talent lines.
    pub fn player_ability_names(&self) -> Vec<String> {
        (0..4)
            .map(|slot| {
                self.ability_info(slot)
                    .map(|a| a.name.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// The player's talent tiers, for the HUD.
    pub fn player_talents(&self) -> &[[crate::talents::Talent; 2]] {
        self.talent_tiers(self.player_index)
    }

    /// The lowest talent tier the player has reached and not picked yet.
    pub fn open_talent_tier(&self) -> Option<usize> {
        if self.talent_picks_pending == 0 {
            return None;
        }
        let level = self.player_level();
        (0..crate::talents::TALENT_LEVELS.len()).find(|&tier| {
            level >= crate::talents::TALENT_LEVELS[tier]
                && self.talents[tier].is_none()
                && tier < self.player_talents().len()
        })
    }

    /// The player picks `choice` (0 = left, 1 = right) at the open tier.
    pub fn pick_talent(&mut self, choice: u8) -> bool {
        let Some(tier) = self.open_talent_tier() else {
            return false;
        };
        self.talents[tier] = Some(choice.min(1));
        self.talent_picks_pending -= 1;
        let pi = self.player_index;
        self.apply_talent(pi, tier, choice);
        let pos = self.units[pi].pos;
        self.play_sound("skill_point", pos);
        self.emit_burst(
            pos + glam::Vec3::Y * 60.0,
            [255, 214, 90],
            26,
            260.0,
            18.0,
            0.8,
        );
        true
    }

    /// Apply talent `choice` of `tier` to unit `index`: stats now, ability
    /// changes into its talent modifiers.
    pub(super) fn apply_talent(&mut self, index: usize, tier: usize, choice: u8) {
        use crate::talents::TalentEffect as T;
        let Some(talent) = self
            .talent_tiers(index)
            .get(tier)
            .map(|pair| pair[choice.min(1) as usize].clone())
        else {
            return;
        };
        let unit = &mut self.units[index];
        match talent.effect {
            T::Health { amount } => {
                unit.max_health += amount;
                unit.health += amount;
            }
            T::Damage { amount } => unit.attack_damage += amount,
            T::AbilityPower { amount } => unit.ability_power += amount,
            T::Armor { amount } => unit.armor += amount,
            T::Mana { amount } => {
                unit.max_mana += amount;
                unit.mana += amount;
            }
            T::MoveSpeed { pct } => unit.speed *= 1.0 + pct,
            T::AttackSpeed { pct } => unit.attack_interval /= 1.0 + pct,
            T::Cooldown { pct } => unit.talent.cooldown += pct,
            T::SlotDamage { slot, pct } => unit.talent.slot_damage[slot.min(3)] += pct,
            T::SlotCooldown { slot, pct } => unit.talent.slot_cooldown[slot.min(3)] += pct,
            T::SlotRange { slot, pct } => unit.talent.slot_range[slot.min(3)] += pct,
        }
        tracing::info!("talent: unit {index} tier {tier} took {}", talent.name);
    }

    /// Spend a skill point on `slot`. False when it cannot take one.
    pub fn upgrade_ability(&mut self, slot: usize) -> bool {
        if !self.can_upgrade(slot) {
            return false;
        }
        self.skill_points -= 1;
        self.ability_ranks[slot] += 1;
        self.spend_spare_points();
        let pos = self.units[self.player_index].pos;
        self.play_sound("skill_point", pos);
        let rank = self.ability_ranks[slot];
        // A shorter cooldown applies to a cooldown already running.
        let cooldown = self.rank_cooldown(slot, f32::MAX);
        let player = &mut self.units[self.player_index];
        player.cooldowns[slot] = player.cooldowns[slot].min(cooldown);
        let pos = player.pos + glam::Vec3::Y * 40.0;
        self.emit_burst(pos, [255, 214, 90], 18, 220.0, 16.0, 0.6);
        tracing::info!("ability rank: slot {slot} to rank {rank}");
        true
    }

    /// The authored numbers for `slot` at `rank` (1-based).
    pub fn rank_entry(&self, slot: usize, rank: u8) -> Option<AbilityRank> {
        self.ability_info(slot)?
            .ranks
            .get(rank.saturating_sub(1) as usize)
            .copied()
    }

    fn current_and_first(&self, slot: usize) -> Option<(AbilityRank, AbilityRank)> {
        Some((
            self.rank_entry(slot, self.ability_rank(slot))?,
            self.rank_entry(slot, 1)?,
        ))
    }

    /// Damage, heal and shield multiplier from the slot's rank.
    pub fn rank_damage_mult(&self, slot: usize) -> f32 {
        let talent = self.units[self.player_index].talent.damage_mult(slot);
        self.current_and_first(slot)
            .map(|(now, first)| ratio(now.damage, first.damage))
            .unwrap_or(1.0)
            * talent
    }

    /// Damage multiplier for the hand-written kits (Monkey, Ranger and the
    /// fallback): the rank plus the same +5% per level kits get through
    /// `kit::scaled_damage`. Ability power and the ranged scale are applied
    /// at each cast site.
    pub fn hand_damage_mult(&self, slot: usize) -> f32 {
        self.rank_damage_mult(slot) * crate::kit::scaled_damage(1.0, 0.0, self.player_level())
    }

    /// Reach multiplier (shot length, cast range, dash length) from the rank.
    pub fn rank_reach_mult(&self, slot: usize) -> f32 {
        let talent = self.units[self.player_index].talent.range_mult(slot);
        self.current_and_first(slot)
            .map(|(now, first)| ratio(Some(now.range), Some(first.range)))
            .unwrap_or(1.0)
            * talent
    }

    /// The slot's cooldown at its rank, or `fallback` without authored ranks.
    pub fn rank_cooldown(&self, slot: usize, fallback: f32) -> f32 {
        let talent = self.units[self.player_index].talent.cooldown_mult(slot);
        self.rank_entry(slot, self.ability_rank(slot).max(1))
            .map(|entry| entry.cooldown)
            .filter(|cooldown| *cooldown > 0.0)
            .unwrap_or(fallback)
            * talent
    }

    /// What the runtime multiplies authored damage by right now, apart from
    /// the rank (ability power, +5% per champion level for every hero, and
    /// the Ranger's ranged level), so the tooltip shows what a cast would
    /// actually do.
    pub fn ability_power_mult(&self) -> f32 {
        let player = &self.units[self.player_index];
        let base = crate::kit::scaled_damage(1.0, player.ability_power, player.level);
        if self.kit.is_none() && self.champion_id == "ranger" {
            base * player.ranged_scale()
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_abilities_take_points_until_rank_four() {
        assert_eq!(upgrade_block(0, 1, 4, 2, 1), None);
        assert_eq!(upgrade_block(0, 3, 4, 2, 1), None);
        assert_eq!(upgrade_block(0, 4, 4, 11, 3), Some(UpgradeBlock::MaxRank));
        assert_eq!(upgrade_block(2, 2, 4, 5, 0), Some(UpgradeBlock::NoPoints));
    }

    #[test]
    fn ultimate_ranks_wait_for_levels_six_eleven_and_sixteen() {
        assert_eq!(
            upgrade_block(3, 0, 3, 5, 4),
            Some(UpgradeBlock::NeedsLevel(6))
        );
        assert_eq!(upgrade_block(3, 0, 3, 6, 1), None);
        assert_eq!(
            upgrade_block(3, 1, 3, 10, 2),
            Some(UpgradeBlock::NeedsLevel(11))
        );
        assert_eq!(upgrade_block(3, 2, 3, 16, 1), None);
        assert_eq!(upgrade_block(3, 3, 3, 25, 1), Some(UpgradeBlock::MaxRank));
    }

    #[test]
    fn bots_follow_the_skill_order_and_skip_talent_levels() {
        assert_eq!(auto_ranks(1), [1, 0, 0, 0]);
        assert_eq!(auto_ranks(3), [1, 1, 1, 0]);
        assert_eq!(auto_ranks(6), [3, 1, 1, 1]);
        assert_eq!(auto_ranks(10), auto_ranks(9));
        assert_eq!(auto_ranks(11)[3], 2);
        assert_eq!(auto_ranks(25), [4, 4, 4, 3]);
    }

    #[test]
    fn ratios_ignore_missing_or_zero_numbers() {
        assert_eq!(ratio(Some(90.0), Some(45.0)), 2.0);
        assert_eq!(ratio(None, Some(45.0)), 1.0);
        assert_eq!(ratio(Some(10.0), Some(0.0)), 1.0);
    }
}
