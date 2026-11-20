//! Stage 4's first live bridge: the score strip's slice of `HudState` from
//! the running match. Every other component still uses the stage 3 sample
//! until its own bridge is added.

use crate::hud::PlayerHud;
use rift_ui::hud::state::{AbilityState, ScoreState};

pub fn score(player: &PlayerHud) -> ScoreState {
    score_from(
        player.match_time,
        player.kills,
        (
            player.player_kills,
            player.player_deaths,
            player.player_assists,
        ),
        player.player_cs,
    )
}

/// The pure part, so the numbers and clock can be checked without a match.
pub fn score_from(
    match_time: f32,
    team_kills: (u32, u32),
    own: (u32, u32, u32),
    creep_score: u32,
) -> ScoreState {
    ScoreState {
        clock: crate::hud::format_clock(match_time),
        blue_kills: team_kills.0,
        red_kills: team_kills.1,
        kills: own.0,
        deaths: own.1,
        assists: own.2,
        creep_score,
    }
}

pub fn ability(player: &PlayerHud) -> AbilityState {
    ability_from(
        player.health,
        player.max_health,
        player.mana,
        player.max_mana,
        player.level,
        player.xp,
        player.xp_next,
        player.cooldowns,
    )
}

/// The pure part of the ability bar's bridge.
#[allow(clippy::too_many_arguments)]
pub fn ability_from(
    health: f32,
    max_health: f32,
    resource: f32,
    max_resource: f32,
    level: u32,
    experience: f32,
    experience_next: f32,
    cooldowns: [f32; 4],
) -> AbilityState {
    let max_health = count(max_health);
    let max_resource = count(max_resource);
    AbilityState {
        health: count(health).min(max_health),
        max_health,
        resource: count(resource).min(max_resource),
        max_resource,
        level: level.clamp(0, u8::MAX as u32) as u8,
        experience: if experience_next > 0.0 {
            (experience / experience_next * 100.0).clamp(0.0, 100.0) as u32
        } else {
            0
        },
        cooldowns: cooldowns.map(|seconds| seconds.max(0.0)),
    }
}

fn count(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_score_strip_reads_the_match_numbers_and_clock() {
        let score = score_from(125.0, (7, 5), (3, 1, 4), 86);
        assert_eq!(score.clock, "2:05");
        assert_eq!((score.blue_kills, score.red_kills), (7, 5));
        assert_eq!((score.kills, score.deaths, score.assists), (3, 1, 4));
        assert_eq!(score.creep_score, 86);
    }

    #[test]
    fn the_ability_bar_reads_health_resource_level_and_cooldowns() {
        let bar = ability_from(
            438.4,
            730.0,
            116.6,
            260.0,
            7,
            62.0,
            100.0,
            [1.2, 0.0, 3.9, 20.0],
        );
        assert_eq!((bar.health, bar.max_health), (438, 730));
        assert_eq!((bar.resource, bar.max_resource), (117, 260));
        assert_eq!(bar.level, 7);
        assert_eq!(bar.experience, 62);
        assert_eq!(bar.cooldowns, [1.2, 0.0, 3.9, 20.0]);

        // A stale health value cannot exceed its maximum.
        let bar = ability_from(900.0, 730.0, 0.0, 260.0, 300, 0.0, 0.0, [0.0; 4]);
        assert_eq!(bar.health, 730);
        assert_eq!(bar.level, 255);
        assert_eq!(bar.experience, 0);
    }
}
