//! The teleport scroll on the scene (docs/SPELLS.md, Part B): starting a
//! channel toward an allied structure, breaking or cancelling it, and the
//! arrival. The rules themselves are pure, in `crate::scroll`.

use glam::Vec3;

use super::{AnimatedScene, UnitKind};
use crate::scene::{HeightGrid, WorldCollision};
use crate::scroll::{self, Channel, Refusal, Tick};

/// Team colours for the destination ring both teams see.
const RING: [[u8; 3]; 2] = [[90, 150, 255], [255, 90, 80]];
/// A scroll destination stays on the minimap this long after it starts.
const PING_SECONDS: f32 = 4.0;

impl AnimatedScene {
    /// Living allied structures a scroll can land at: towers, inhibitors and
    /// the nexus.
    pub fn scroll_destinations(&self, team: u8) -> Vec<(usize, Vec3)> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                unit.team == team
                    && unit.health > 0.0
                    && matches!(
                        unit.kind,
                        UnitKind::Tower | UnitKind::Inhibitor | UnitKind::Nexus
                    )
            })
            .map(|(index, unit)| (index, unit.pos))
            .collect()
    }

    /// Start `unit`'s scroll toward the allied structure nearest `aim` (a
    /// point under the cursor or on the minimap). The hero stops where they
    /// are; the arrival is marked for both teams.
    pub fn start_scroll(
        &mut self,
        unit: usize,
        aim: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> Result<(), Refusal> {
        let Some(hero) = self.units.get(unit) else {
            return Err(Refusal::Dead);
        };
        scroll::can_start(hero.health, &hero.scroll, hero.stun_timer, &hero.cc)?;
        let team = hero.team;
        let (structure, at) =
            scroll::nearest(&self.scroll_destinations(team), aim).ok_or(Refusal::NoDestination)?;
        let reach = self.units[structure].body_radius + self.scroll_config.offset;
        let rough = scroll::arrival(at, self.team_home(team), reach);
        let dest = collision
            .nearest_walkable_world(rough.x, rough.z)
            .map(|(tx, ty)| collision.world_of_tile(tx, ty, height))
            .unwrap_or(Vec3::new(
                rough.x,
                height.height_at(rough.x, rough.z),
                rough.z,
            ));

        let config = self.scroll_config.clone();
        let hero = &mut self.units[unit];
        hero.path.clear();
        hero.path_index = 0;
        hero.moving = false;
        hero.attack_target = None;
        hero.attack_pending = false;
        hero.scroll.cooldown = config.cooldown;
        hero.scroll.channel = Some(Channel {
            remaining: config.channel,
            total: config.channel,
            dest,
            structure,
        });
        self.gfx_on(config.origin_gfx, unit, 1.0);
        self.gfx_hold_at(config.marker_gfx, dest, config.channel, 1.0);
        self.add_decal(dest, 150.0, RING[team.min(1) as usize], config.channel);
        self.scroll_pings.push((dest, team, PING_SECONDS));
        tracing::info!(
            "scroll: unit {unit} channelling to structure {structure} at tile {:.0},{:.0}",
            dest.x / 128.0,
            dest.z / 128.0
        );
        Ok(())
    }

    /// Stop `unit`'s channel because they moved or cast. The scroll stays
    /// spent, as in Dota 2.
    pub fn cancel_scroll(&mut self, unit: usize) {
        if let Some(channel) = self
            .units
            .get_mut(unit)
            .and_then(|hero| hero.scroll.channel.take())
        {
            self.clear_scroll_marker(channel.dest);
            tracing::info!("scroll: unit {unit} cancelled the channel");
        }
    }

    /// A broken or cancelled channel takes its destination marker and ring
    /// with it. Both were placed exactly at `dest`.
    fn clear_scroll_marker(&mut self, dest: Vec3) {
        let marker = self.scroll_config.marker_gfx;
        self.spot_bursts.retain(|burst| {
            !(burst.spotanim == marker && burst.follow.is_none() && burst.pos == dest)
        });
        self.decals.retain(|decal| decal.pos != dest);
    }

    /// Run every hero's scroll for one frame: cooldowns, broken channels and
    /// arrivals, and the minimap pings.
    pub(super) fn update_scrolls(&mut self, dt: f32) {
        self.scroll_pings.retain_mut(|(_, _, timer)| {
            *timer -= dt;
            *timer > 0.0
        });
        for index in 0..self.units.len() {
            let unit = &mut self.units[index];
            if !unit.is_champion {
                continue;
            }
            let dest = unit.scroll.channel.map(|channel| channel.dest);
            if unit.health <= 0.0 {
                unit.scroll.channel = None;
                if let Some(dest) = dest {
                    self.clear_scroll_marker(dest);
                }
                continue;
            }
            let (stun, cc) = (unit.stun_timer, unit.cc.clone());
            match scroll::tick(&mut unit.scroll, dt, stun, &cc) {
                Some(Tick::Running) => {
                    unit.moving = false;
                    // A slow shimmer of motes rises off the channelling hero.
                    let pos = unit.pos;
                    let tint = RING[unit.team.min(1) as usize];
                    if self.next_random() < dt * 10.0 {
                        self.emit_burst(pos + Vec3::Y * 30.0, tint, 2, 70.0, 10.0, 0.7);
                    }
                }
                Some(Tick::Broken) => {
                    if let Some(dest) = dest {
                        self.clear_scroll_marker(dest);
                    }
                    tracing::info!("scroll: unit {index}'s channel was broken");
                }
                Some(Tick::Arrive(dest)) => self.arrive_by_scroll(index, dest),
                None => {}
            }
        }
    }

    fn arrive_by_scroll(&mut self, index: usize, dest: Vec3) {
        let arrival = self.scroll_config.arrival_gfx;
        if index == self.player_index {
            self.teleport_player(dest);
        } else {
            let unit = &mut self.units[index];
            unit.pos = dest;
            unit.path.clear();
            unit.path_index = 0;
            unit.moving = false;
        }
        self.gfx_on(arrival, index, 1.0);
        self.play_sound("teleport", dest);
        tracing::info!(
            "scroll: unit {index} arrived at tile {:.0},{:.0}",
            dest.x / 128.0,
            dest.z / 128.0
        );
    }

    /// The player's scroll for the HUD: seconds of cooldown left, the full
    /// cooldown, and the channel's progress when one runs.
    pub fn player_scroll(&self) -> (f32, f32, Option<f32>) {
        let state = &self.units[self.player_index].scroll;
        (
            state.cooldown,
            self.scroll_config.cooldown,
            state.channel.map(|channel| channel.progress()),
        )
    }
}
