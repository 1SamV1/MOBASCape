//! Battle spells on the scene (docs/SPELLS.md, Part C): casting a magic
//! hero's D or F spell, for the player and the bots alike, the bolts'
//! landing, and Vengeance firing. The rules themselves are pure, in
//! `crate::spells`.

use glam::Vec3;

use super::{AnimatedScene, HitsplatKind, Projectile, UnitKind, planar_delta};
use crate::fx;
use crate::kit::{self, Gfx, Status};
use crate::scene::{HeightGrid, WorldCollision};
use crate::spells::{self, Effect, Refusal, Vengeance};

/// What a battle spell bolt carries to its target.
#[derive(Debug, Clone)]
pub struct SpellHit {
    pub statuses: Vec<Status>,
    pub gfx: Gfx,
    /// Where it was cast from (knockbacks and fears push away from here).
    pub from: Vec3,
}

/// A battle spell slot for the HUD.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellView {
    pub id: String,
    pub name: String,
    pub cooldown: f32,
    pub max: f32,
}

impl AnimatedScene {
    /// Give `unit` its battle spells: `ids` for a magic hero of `class`,
    /// none for anyone else.
    pub fn set_spells<S: AsRef<str>>(&mut self, unit: usize, class: &str, ids: &[S]) {
        let picks = self.spell_book.picks(ids);
        let loadout = spells::Loadout::for_class(class, picks);
        let names: Vec<&str> = loadout
            .picks
            .iter()
            .flatten()
            .map(|&n| self.spell_book.spells[n].id.as_str())
            .collect();
        tracing::info!("spells: unit {unit} ({class}) carries {names:?}");
        if let Some(hero) = self.units.get_mut(unit) {
            hero.spells = loadout;
        }
    }

    /// Living heroes hostile to `team`, as a bolt's candidates. A hero
    /// casting for the player's team needs to see them.
    fn spell_enemy_heroes(&self, team: u8) -> Vec<(usize, Vec3)> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                unit.kind.is_champion()
                    && unit.team != team
                    && unit.team < 2
                    && unit.health > 0.0
                    && unit.spawn_delay <= 0.0
                    && (team != 0 || unit.visible)
            })
            .map(|(index, unit)| (index, unit.pos))
            .collect()
    }

    /// Cast `caster`'s battle spell in `slot` (0 for D, 1 for F) toward
    /// `aim`. Nothing is spent when it is refused.
    pub fn cast_battle_spell(
        &mut self,
        caster: usize,
        slot: usize,
        aim: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> Result<(), Refusal> {
        let hero = self.units.get(caster).ok_or(Refusal::Dead)?;
        let index = spells::can_cast(&hero.spells, slot, hero.health, hero.stun_timer, &hero.cc)?;
        let spell = self.spell_book.spells[index].clone();
        let (origin, team, yaw) = (hero.pos, hero.team, hero.yaw);
        let (power, level) = (hero.ability_power, hero.level);
        let facing = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let size = spell.gfx.scale * self.gfx_size(caster);

        // Check what can still refuse before anything is spent or played.
        let bolt_target = match &spell.effect {
            Effect::Blink { .. } if !spells::can_blink(&self.units[caster].cc) => {
                return Err(Refusal::Blocked);
            }
            Effect::Bolt { range, .. } => Some(
                spells::bolt_target(&self.spell_enemy_heroes(team), origin, aim, *range)
                    .ok_or(Refusal::NoTarget)?,
            ),
            _ => None,
        };

        let sound = match &spell.effect {
            Effect::Blink { range } => {
                let rough = spells::blink_target(origin, aim, facing, *range);
                let dest = collision
                    .nearest_walkable_world(rough.x, rough.z)
                    .map(|(tx, ty)| collision.world_of_tile(tx, ty, height))
                    .unwrap_or(Vec3::new(
                        rough.x,
                        height.height_at(rough.x, rough.z),
                        rough.z,
                    ));
                if let Some(id) = spell.gfx.cast {
                    self.gfx_at(id, origin, size);
                }
                self.place_unit(caster, dest);
                if let Some(id) = spell.gfx.land {
                    self.gfx_on(id, caster, size);
                }
                "teleport"
            }
            Effect::HealGroup {
                own,
                allies,
                radius,
            } => {
                if let Some(id) = spell.gfx.cast {
                    self.gfx_on(id, caster, size);
                }
                self.spell_heal(caster, *own);
                let friends: Vec<usize> = (0..self.units.len())
                    .filter(|&i| {
                        let unit = &self.units[i];
                        i != caster
                            && unit.kind.is_champion()
                            && unit.team == team
                            && unit.health > 0.0
                            && planar_delta(origin, unit.pos).length() <= *radius
                    })
                    .collect();
                for friend in friends {
                    self.spell_heal(friend, *allies);
                    if let Some(id) = spell.gfx.hit {
                        let size = spell.gfx.scale * self.gfx_size(friend);
                        self.gfx_on(id, friend, size);
                    }
                }
                "kit_heal"
            }
            Effect::Vengeance {
                reflect,
                duration,
                say,
            } => {
                if let Some(id) = spell.gfx.cast {
                    self.gfx_on(id, caster, size);
                }
                self.units[caster].vengeance = Some(Vengeance {
                    reflect: *reflect,
                    timer: *duration,
                    say: say.clone(),
                });
                "kit_shield"
            }
            Effect::Bolt {
                damage,
                speed,
                visual,
                on_hit,
                ..
            } => {
                let target = bolt_target.expect("checked above");
                if let Some(id) = spell.gfx.cast {
                    self.gfx_on(id, caster, size);
                }
                let to = planar_delta(origin, self.units[target].pos);
                if to.length_squared() > 1.0 {
                    self.units[caster].yaw = to.x.atan2(to.z);
                }
                let from = origin + Vec3::Y * (self.units[caster].bar_height * 0.6).max(60.0);
                let mut shot = Projectile::homing(
                    from,
                    target,
                    self.unit_chest(target),
                    *speed,
                    kit::scaled_damage(*damage, power, level),
                    team,
                    Some(caster),
                    fx::look_by_name(visual),
                );
                shot.spell = Some(Box::new(SpellHit {
                    statuses: on_hit.clone(),
                    gfx: spell.gfx.clone(),
                    from: origin,
                }));
                self.projectiles.push(shot);
                "kit_projectile"
            }
            Effect::Nova {
                radius,
                damage,
                on_hit,
            } => {
                if let Some(id) = spell.gfx.cast {
                    self.gfx_on(id, caster, size);
                }
                let damage = kit::scaled_damage(*damage, power, level);
                let struck: Vec<usize> = (0..self.units.len())
                    .filter(|&i| {
                        let unit = &self.units[i];
                        unit.team != team
                            && unit.health > 0.0
                            && unit.spawn_delay <= 0.0
                            && !unit.kind.is_structure()
                            && !matches!(
                                unit.kind,
                                UnitKind::DeliveryGuard | UnitKind::StorageChest
                            )
                            && planar_delta(origin, unit.pos).length()
                                <= radius + unit.hit_radius.max(40.0) * 0.5
                    })
                    .collect();
                for target in struck {
                    let splashed = damage > 0.0 && self.units[target].shield > 0.0;
                    if damage > 0.0 {
                        self.damage_unit_by(target, damage, team, Some(caster));
                    }
                    self.land_statuses(target, on_hit, origin, &spell.gfx, splashed, caster);
                }
                self.add_decal(origin, *radius, [70, 150, 60], 0.8);
                "kit_area"
            }
        };

        let hero = &mut self.units[caster];
        hero.spells.cooldowns[slot] = spell.cooldown;
        // A cast interrupts a basic attack that has not landed yet, and
        // cancels a scroll being read.
        hero.attack_pending = false;
        hero.attack_windup_timer = 0.0;
        self.cancel_scroll(caster);
        self.start_action(caster, true, 0.0);
        self.play_sound(sound, origin);
        tracing::info!("spell: unit {caster} cast {}", spell.name);
        Ok(())
    }

    /// Move a unit straight to `pos` (a blink), dropping its walk.
    fn place_unit(&mut self, index: usize, pos: Vec3) {
        if index == self.player_index {
            self.teleport_player(pos);
        } else {
            let unit = &mut self.units[index];
            unit.pos = pos;
            unit.path.clear();
            unit.path_index = 0;
            unit.moving = false;
            self.bot_forget_path(index);
        }
    }

    /// Heal `index` by `share` of its max health (halved under heal cut),
    /// with a heal hitmark.
    fn spell_heal(&mut self, index: usize, share: f32) {
        let unit = &mut self.units[index];
        let cut = if unit.cc.heal_cut_timer > 0.0 {
            0.5
        } else {
            1.0
        };
        let before = unit.health;
        unit.health = (unit.health + unit.max_health * share * cut).min(unit.max_health);
        let healed = unit.health - before;
        if healed > 0.5 {
            self.push_hitsplat(index, healed, HitsplatKind::Heal, None);
        }
    }

    /// A battle spell bolt reached `index`: its statuses and graphics.
    pub(super) fn land_spell(
        &mut self,
        index: usize,
        hit: &SpellHit,
        splashed: bool,
        caster: usize,
    ) {
        self.land_statuses(index, &hit.statuses, hit.from, &hit.gfx, splashed, caster);
    }

    /// Damage just landed on `index` from `aggressor`. A waiting Vengeance
    /// sends part of it back, once, at an enemy hero.
    pub(super) fn vengeance_check(&mut self, index: usize, aggressor: Option<usize>, amount: f32) {
        let Some(from) = aggressor else {
            return;
        };
        if from == index || from >= self.units.len() || self.units[index].vengeance.is_none() {
            return;
        }
        let enemy_hero = self.units[from].kind.is_champion()
            && self.units[from].team != self.units[index].team
            && self.units[from].health > 0.0;
        let Some((reflect, say)) =
            spells::trigger_vengeance(&mut self.units[index].vengeance, amount, enemy_hero)
        else {
            return;
        };
        self.say(index, &say);
        let team = self.units[index].team;
        // Vengeance's own damage never sets off another Vengeance.
        self.reflecting = true;
        self.damage_unit_mixed(from, 0.0, reflect, team, Some(index));
        self.reflecting = false;
        tracing::info!("spell: unit {index}'s vengeance sent {reflect:.0} back at unit {from}");
    }

    /// Cooldowns and waiting Vengeance, every frame.
    pub(super) fn update_spells(&mut self, dt: f32) {
        for unit in &mut self.units {
            if unit.health <= 0.0 {
                unit.vengeance = None;
            }
            if unit.spells.has_spells() {
                unit.spells.tick(dt);
            }
            spells::tick_vengeance(&mut unit.vengeance, dt);
        }
    }

    /// The player's D and F for the HUD; `None` for an empty slot.
    pub fn player_spells(&self) -> [Option<SpellView>; 2] {
        let loadout = &self.units[self.player_index].spells;
        std::array::from_fn(|slot| {
            let spell = self.spell_book.get(loadout.picks[slot]?)?;
            Some(SpellView {
                id: spell.id.clone(),
                name: spell.name.clone(),
                cooldown: loadout.cooldowns[slot],
                max: spell.cooldown,
            })
        })
    }

    /// The living enemy hero nearest `unit`, for bots and demos.
    pub fn nearest_enemy_hero(&self, unit: usize) -> Option<usize> {
        let me = self.units.get(unit)?;
        self.units
            .iter()
            .enumerate()
            .filter(|(i, other)| {
                *i != unit
                    && other.kind.is_champion()
                    && other.team != me.team
                    && other.team < 2
                    && other.health > 0.0
            })
            .min_by(|a, b| {
                let da = planar_delta(me.pos, a.1.pos).length();
                let db = planar_delta(me.pos, b.1.pos).length();
                da.total_cmp(&db)
            })
            .map(|(i, _)| i)
    }

    /// What bot `index` sees for `spell`'s rules.
    fn bot_view(&self, index: usize, spell: &spells::Spell) -> spells::BotView {
        let me = &self.units[index];
        let reach = match &spell.effect {
            Effect::Nova { radius, .. } => *radius,
            Effect::HealGroup { radius, .. } => *radius,
            _ => 0.0,
        };
        let distance = |pos: Vec3| planar_delta(me.pos, pos).length();
        let enemy = self.nearest_enemy_hero(index).and_then(|e| {
            let other = &self.units[e];
            (me.team != 0 || other.visible).then(|| {
                (
                    distance(other.pos),
                    other.health / other.max_health.max(1.0),
                )
            })
        });
        let mut view = spells::BotView {
            health: me.health / me.max_health.max(1.0),
            enemy,
            ..Default::default()
        };
        for (i, other) in self.units.iter().enumerate() {
            if i == index || other.health <= 0.0 || other.spawn_delay > 0.0 {
                continue;
            }
            let near = distance(other.pos) <= reach + other.hit_radius.max(40.0) * 0.5;
            if !near {
                continue;
            }
            if other.team == me.team {
                if other.kind.is_champion() && other.health < other.max_health * 0.6 {
                    view.hurt_allies += 1;
                }
            } else if !other.kind.is_structure() {
                view.enemies_near += 1;
                if other.kind.is_champion() {
                    view.heroes_near += 1;
                }
            }
        }
        view
    }

    /// A bot's battle spells, weighed on its think tick: at most one goes
    /// off.
    pub(super) fn bot_battle_spells(
        &mut self,
        index: usize,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) {
        let loadout = self.units[index].spells.clone();
        if !loadout.has_spells() || self.units[index].health <= 0.0 {
            return;
        }
        for slot in 0..2 {
            let Some(n) = loadout.picks[slot] else {
                continue;
            };
            if loadout.cooldowns[slot] > 0.0 {
                continue;
            }
            let spell = self.spell_book.spells[n].clone();
            let Some(aim) = spells::bot_aim(&spell, &self.bot_view(index, &spell)) else {
                continue;
            };
            let me = &self.units[index];
            let target = match aim {
                spells::BotAim::Home => me.home,
                spells::BotAim::Here => me.pos,
                spells::BotAim::Enemy => match self.nearest_enemy_hero(index) {
                    Some(enemy) => self.units[enemy].pos,
                    None => continue,
                },
            };
            if self
                .cast_battle_spell(index, slot, target, collision, height)
                .is_ok()
            {
                return;
            }
        }
    }

    /// The player casts D (0) or F (1).
    pub fn cast_player_spell(
        &mut self,
        slot: usize,
        aim: Vec3,
        collision: &WorldCollision,
        height: &HeightGrid,
    ) -> Result<(), Refusal> {
        if self.kit_controls(self.player_index) {
            return Err(Refusal::Disabled);
        }
        let pi = self.player_index;
        self.cast_battle_spell(pi, slot, aim, collision, height)
    }
}
