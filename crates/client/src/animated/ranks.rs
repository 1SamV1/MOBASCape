//! Ability ranks and skill points.
//!
//! Every ability starts at rank 1. Each level gained gives one skill point;
//! the player spends it by clicking a glowing ability slot. Q, W and E go
//! to rank 4, R to rank 3, and R's ranks also need champion levels
//! ([`ULTIMATE_RANK_LEVELS`]). The numbers per rank are hand-authored in the
//! champion file (`abilities[].ranks`) and applied here as ratios against
//! rank 1, so the kit's actions keep their own shape.

use super::AnimatedScene;
use crate::champion::AbilityRank;

/// Ranks of Q, W and E.
pub const BASIC_MAX_RANK: u8 = 4;
/// Ranks of R.
pub const ULTIMATE_MAX_RANK: u8 = 3;
/// Champion level needed for R rank 1, 2 and 3.
pub const ULTIMATE_RANK_LEVELS: [u32; 3] = [1, 6, 9];

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

/// Rank `rank`'s value over rank 1's, or 1 when either is missing or zero.
fn ratio(current: Option<f32>, first: Option<f32>) -> f32 {
    match (current, first) {
        (Some(current), Some(first)) if first > 0.0 && current > 0.0 => current / first,
        _ => 1.0,
    }
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
        self.ability_ranks.get(slot).copied().unwrap_or(1)
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

    /// Spend a skill point on `slot`. False when it cannot take one.
    pub fn upgrade_ability(&mut self, slot: usize) -> bool {
        if !self.can_upgrade(slot) {
            return false;
        }
        self.skill_points -= 1;
        self.ability_ranks[slot] += 1;
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
        self.current_and_first(slot)
            .map(|(now, first)| ratio(now.damage, first.damage))
            .unwrap_or(1.0)
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
        self.current_and_first(slot)
            .map(|(now, first)| ratio(Some(now.range), Some(first.range)))
            .unwrap_or(1.0)
    }

    /// The slot's cooldown at its rank, or `fallback` without authored ranks.
    pub fn rank_cooldown(&self, slot: usize, fallback: f32) -> f32 {
        self.rank_entry(slot, self.ability_rank(slot))
            .map(|entry| entry.cooldown)
            .filter(|cooldown| *cooldown > 0.0)
            .unwrap_or(fallback)
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
    fn ultimate_ranks_wait_for_levels_six_and_nine() {
        assert_eq!(
            upgrade_block(3, 1, 3, 5, 4),
            Some(UpgradeBlock::NeedsLevel(6))
        );
        assert_eq!(upgrade_block(3, 1, 3, 6, 1), None);
        assert_eq!(
            upgrade_block(3, 2, 3, 8, 2),
            Some(UpgradeBlock::NeedsLevel(9))
        );
        assert_eq!(upgrade_block(3, 2, 3, 9, 1), None);
        assert_eq!(upgrade_block(3, 3, 3, 11, 1), Some(UpgradeBlock::MaxRank));
        assert_eq!(rank_level(0, 4), 1);
    }

    #[test]
    fn ratios_ignore_missing_or_zero_numbers() {
        assert_eq!(ratio(Some(90.0), Some(45.0)), 2.0);
        assert_eq!(ratio(None, Some(45.0)), 1.0);
        assert_eq!(ratio(Some(10.0), Some(0.0)), 1.0);
    }
}
