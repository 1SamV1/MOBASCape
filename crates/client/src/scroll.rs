//! The teleport scroll every hero carries (docs/SPELLS.md, Part B). It
//! works like Dota 2's Town Portal: a channel, then you arrive beside an
//! allied tower, inhibitor or nexus. Crowd control breaks the channel, and
//! moving or casting cancels it.
//!
//! This module is plain data and pure helpers; the scene runs it in
//! `animated/teleport.rs`.

use glam::Vec3;
use serde::Deserialize;

use crate::kit::Afflictions;

/// Where the tunable numbers live.
pub const PATH: &str = "data/spells/teleport_scroll.json";

/// `data/spells/teleport_scroll.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScrollConfig {
    /// Name on tooltips.
    pub name: String,
    /// Cache item whose inventory icon is the slot's picture.
    pub item: u32,
    /// Seconds of channelling before the teleport.
    pub channel: f32,
    /// Seconds before the scroll can be used again, counted from the start
    /// of the channel (a broken channel still spends it, as in Dota 2).
    pub cooldown: f32,
    /// Distance from the structure's edge to the arrival point, on the side
    /// toward the team's base.
    pub offset: f32,
    /// Spot animation on the hero as the channel starts (a tablet breaking).
    pub origin_gfx: u32,
    /// Spot animation on the hero when they arrive.
    pub arrival_gfx: u32,
    /// Spot animation held at the destination while the channel runs, seen
    /// by both teams.
    pub marker_gfx: u32,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            name: "TELEPORT SCROLL".to_string(),
            item: 12642,
            channel: 3.0,
            cooldown: 80.0,
            offset: 160.0,
            origin_gfx: 678,
            arrival_gfx: 111,
            marker_gfx: 750,
        }
    }
}

impl ScrollConfig {
    /// Load the scroll's numbers. A missing or broken file keeps the
    /// defaults (the same numbers the shipped file has) and says so.
    pub fn load() -> Self {
        let parsed = std::fs::read_to_string(PATH)
            .map_err(anyhow::Error::from)
            .and_then(|text| serde_json::from_str(&text).map_err(anyhow::Error::from));
        parsed.unwrap_or_else(|err| {
            tracing::warn!("scroll: {PATH}: {err:#}; using the default teleport scroll");
            Self::default()
        })
    }

    /// Every spot animation the scroll plays, for preloading.
    pub fn spotanims(&self) -> [u32; 3] {
        [self.origin_gfx, self.arrival_gfx, self.marker_gfx]
    }
}

/// One hero's scroll: its cooldown and the channel in progress.
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pub cooldown: f32,
    pub channel: Option<Channel>,
}

/// A teleport being channelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Channel {
    pub remaining: f32,
    pub total: f32,
    /// Where the hero will arrive.
    pub dest: Vec3,
    /// The structure they chose (unit index).
    pub structure: usize,
}

impl Channel {
    /// 0 at the start of the channel, 1 at the teleport.
    pub fn progress(&self) -> f32 {
        (1.0 - self.remaining / self.total.max(0.001)).clamp(0.0, 1.0)
    }
}

/// Why a scroll did not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Dead,
    Cooldown,
    AlreadyChannelling,
    /// Stunned, feared, taunted, airborne or being moved.
    Disabled,
    NoDestination,
}

/// The candidate (unit index and position) nearest `aim`, on the ground
/// plane. This is how a hover over a structure, or a minimap point, picks
/// where to go.
pub fn nearest(candidates: &[(usize, Vec3)], aim: Vec3) -> Option<(usize, Vec3)> {
    let flat = |a: Vec3, b: Vec3| ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
    candidates
        .iter()
        .copied()
        .min_by(|a, b| flat(a.1, aim).total_cmp(&flat(b.1, aim)))
}

/// The arrival point beside a structure: `reach` (its footprint plus the
/// scroll's offset) from its centre, toward the team's `home`, so a hero
/// lands on the safe side of their tower.
pub fn arrival(structure: Vec3, home: Vec3, reach: f32) -> Vec3 {
    let toward = Vec3::new(home.x - structure.x, 0.0, home.z - structure.z);
    let dir = if toward.length_squared() > 1.0 {
        toward.normalize()
    } else {
        Vec3::X
    };
    structure + dir * reach
}

/// True when crowd control stops a channel this frame: a stun or knock-up
/// (`stun_timer`), fear, taunt or forced movement. A root does not: as in
/// Dota 2, a rooted hero may still scroll out.
pub fn disabled(stun_timer: f32, cc: &Afflictions) -> bool {
    stun_timer > 0.0
        || cc.fear_timer > 0.0
        || cc.taunt_timer > 0.0
        || cc.displace_timer > 0.0
        || cc.lift_timer > 0.0
}

/// Whether a scroll may start now, before a destination is looked for.
pub fn can_start(
    health: f32,
    state: &ScrollState,
    stun_timer: f32,
    cc: &Afflictions,
) -> Result<(), Refusal> {
    if health <= 0.0 {
        Err(Refusal::Dead)
    } else if state.channel.is_some() {
        Err(Refusal::AlreadyChannelling)
    } else if state.cooldown > 0.0 {
        Err(Refusal::Cooldown)
    } else if disabled(stun_timer, cc) {
        Err(Refusal::Disabled)
    } else {
        Ok(())
    }
}

/// What happened to a channel this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tick {
    /// Still channelling.
    Running,
    /// Crowd control broke it.
    Broken,
    /// It finished: teleport to this point.
    Arrive(Vec3),
}

/// Count a scroll down by `dt`: the cooldown, and the channel if one runs.
pub fn tick(state: &mut ScrollState, dt: f32, stun_timer: f32, cc: &Afflictions) -> Option<Tick> {
    state.cooldown = (state.cooldown - dt).max(0.0);
    let channel = state.channel.as_mut()?;
    if disabled(stun_timer, cc) {
        state.channel = None;
        return Some(Tick::Broken);
    }
    channel.remaining -= dt;
    if channel.remaining <= 0.0 {
        let dest = channel.dest;
        state.channel = None;
        return Some(Tick::Arrive(dest));
    }
    Some(Tick::Running)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(remaining: f32) -> ScrollState {
        ScrollState {
            cooldown: 80.0,
            channel: Some(Channel {
                remaining,
                total: 3.0,
                dest: Vec3::new(10.0, 0.0, 20.0),
                structure: 7,
            }),
        }
    }

    #[test]
    fn the_shipped_file_parses_and_matches_the_defaults() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(PATH);
        let text = std::fs::read_to_string(path).unwrap();
        let config: ScrollConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(config, ScrollConfig::default());
        for id in config.spotanims() {
            assert!(
                crate::kit::spot::VERIFIED.contains(&id),
                "scroll graphic {id} is not verified in docs/SPELLS.md"
            );
        }
    }

    #[test]
    fn the_nearest_structure_to_the_cursor_is_chosen() {
        let towers = [
            (3, Vec3::new(0.0, 0.0, 0.0)),
            (5, Vec3::new(1000.0, 400.0, 0.0)),
            (9, Vec3::new(0.0, 0.0, 2000.0)),
        ];
        // Height does not count: only the ground plane.
        assert_eq!(nearest(&towers, Vec3::new(900.0, 0.0, 0.0)).unwrap().0, 5);
        assert_eq!(
            nearest(&towers, Vec3::new(0.0, 900.0, 1100.0)).unwrap().0,
            9
        );
        assert_eq!(nearest(&[], Vec3::ZERO), None);
    }

    #[test]
    fn heroes_arrive_on_their_base_side_of_the_structure() {
        let at = arrival(
            Vec3::new(1000.0, 50.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            300.0,
        );
        assert_eq!(at, Vec3::new(700.0, 50.0, 0.0));
        // A structure on top of home still gives a point.
        let at = arrival(Vec3::ZERO, Vec3::ZERO, 300.0);
        assert_eq!(at, Vec3::new(300.0, 0.0, 0.0));
    }

    #[test]
    fn a_scroll_refuses_while_dead_cooling_down_channelling_or_disabled() {
        let free = Afflictions::default();
        let ready = ScrollState::default();
        assert_eq!(can_start(100.0, &ready, 0.0, &free), Ok(()));
        assert_eq!(can_start(0.0, &ready, 0.0, &free), Err(Refusal::Dead));
        let cooling = ScrollState {
            cooldown: 5.0,
            channel: None,
        };
        assert_eq!(
            can_start(100.0, &cooling, 0.0, &free),
            Err(Refusal::Cooldown)
        );
        assert_eq!(
            can_start(100.0, &channel(1.0), 0.0, &free),
            Err(Refusal::AlreadyChannelling)
        );
        assert_eq!(can_start(100.0, &ready, 0.5, &free), Err(Refusal::Disabled));
        let feared = Afflictions {
            fear_timer: 1.0,
            ..Default::default()
        };
        assert_eq!(
            can_start(100.0, &ready, 0.0, &feared),
            Err(Refusal::Disabled)
        );
        // A rooted hero may scroll out.
        let rooted = Afflictions {
            root_timer: 1.0,
            ..Default::default()
        };
        assert_eq!(can_start(100.0, &ready, 0.0, &rooted), Ok(()));
    }

    #[test]
    fn a_channel_runs_then_arrives() {
        let free = Afflictions::default();
        let mut state = channel(3.0);
        assert_eq!(tick(&mut state, 1.0, 0.0, &free), Some(Tick::Running));
        let progress = state.channel.unwrap().progress();
        assert!((progress - 1.0 / 3.0).abs() < 1e-5);
        assert_eq!(tick(&mut state, 1.0, 0.0, &free), Some(Tick::Running));
        assert_eq!(
            tick(&mut state, 1.5, 0.0, &free),
            Some(Tick::Arrive(Vec3::new(10.0, 0.0, 20.0)))
        );
        assert!(state.channel.is_none());
        // The cooldown counted down the whole time.
        assert_eq!(state.cooldown, 76.5);
        assert_eq!(tick(&mut state, 1.0, 0.0, &free), None);
    }

    #[test]
    fn a_stun_or_a_knockback_breaks_the_channel() {
        let mut state = channel(2.0);
        assert_eq!(
            tick(&mut state, 0.1, 0.4, &Afflictions::default()),
            Some(Tick::Broken)
        );
        assert!(state.channel.is_none());
        assert_eq!(
            state.cooldown, 79.9,
            "a broken channel still spends the scroll"
        );
        let pushed = Afflictions {
            displace_timer: 0.25,
            ..Default::default()
        };
        let mut state = channel(2.0);
        assert_eq!(tick(&mut state, 0.1, 0.0, &pushed), Some(Tick::Broken));
        // A slow does not break it.
        let slowed = Afflictions {
            slow_pct: 0.5,
            slow_timer: 2.0,
            ..Default::default()
        };
        let mut state = channel(2.0);
        assert_eq!(tick(&mut state, 0.1, 0.0, &slowed), Some(Tick::Running));
    }
}
