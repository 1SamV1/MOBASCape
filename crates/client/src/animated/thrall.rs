//! Thrall scrolls: consumable shop items any hero can read to raise an
//! Arceuus thrall (docs/SPELLS.md). The thrall fights at the hero's heel
//! for its duration; reading another replaces it.

use glam::Vec3;

use super::{AnimatedScene, DEATH_SINK_TIME, UnitKind, build_unit};
use crate::fx;
use crate::items::ThrallSpec;

/// The Arceuus teleport swirl where a thrall rises.
pub(super) const THRALL_RISE: u32 = 1290;

impl AnimatedScene {
    /// Raise the player's thrall from a scroll. Refused while dead or when
    /// the thrall's NPC did not load.
    pub fn summon_thrall(&mut self, spec: &ThrallSpec) -> Result<(), &'static str> {
        let pi = self.player_index;
        if self.units[pi].health <= 0.0 {
            return Err("dead");
        }
        // Thrall Master items (the Book of the dead) raise a stronger one.
        let spec = &self.gear_thrall(pi, spec);
        let Some(asset) = self.thrall_assets.get(&spec.npc) else {
            return Err("thrall not loaded");
        };
        let (model, idle, walk) = (asset.model.clone(), asset.idle.clone(), asset.walk.clone());
        self.dismiss_thrall();
        let yaw = self.units[pi].yaw;
        // Beside the hero, a little behind its shoulder.
        let side = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
        let back = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
        let pos = self.units[pi].pos + side * 140.0 + back * 60.0;
        let ranged = spec.look.is_some();
        let kind = if ranged {
            UnitKind::RangedMinion
        } else {
            UnitKind::MeleeMinion
        };
        let mut unit = build_unit(
            model,
            idle,
            walk,
            pos,
            spec.scale,
            kind,
            0,
            spec.health,
            spec.range,
            spec.damage,
            spec.interval,
            spec.speed,
            60.0,
            170.0 * spec.scale.max(0.6),
            Vec::new(),
        );
        unit.yaw = yaw;
        unit.cc.summoned = true;
        unit.cc.life_timer = spec.duration;
        if let Some(look) = &spec.look {
            unit.look = fx::look_by_name(look);
        }
        let index = self.units.len();
        self.units.push(unit);
        self.thrall = Some(index);
        self.gfx_at(THRALL_RISE, pos, 1.0);
        self.emit_burst(pos + Vec3::Y * 60.0, [170, 150, 220], 16, 200.0, 12.0, 0.6);
        self.play_shared_sound("cast", pos);
        Ok(())
    }

    /// Lay the current thrall to rest (a new scroll replaces it).
    fn dismiss_thrall(&mut self) {
        if let Some(old) = self.thrall.take()
            && let Some(unit) = self.units.get_mut(old)
            && unit.health > 0.0
        {
            unit.health = 0.0;
            unit.death_anim = DEATH_SINK_TIME;
        }
    }
}
