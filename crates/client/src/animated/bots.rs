//! Bot heroes: the nine champions the draft picked, played on both teams.
//!
//! Each bot has a [`BotBrain`] that rethinks about once a second and moves
//! between a few jobs, like a human player's evening:
//!
//! * **Lane**: walk the assigned lane (top, mid or bottom) toward the enemy
//!   nexus, last-hitting minions, trading with enemy heroes who are weaker,
//!   and only hitting a tower, inhibitor or the nexus while friendly minions
//!   tank it.
//! * **Jungle**: clear the camps on their own side of the river for gold
//!   and experience. The jungler lives here; laners take short stints.
//! * **Boss**: from level 5, a team sends its jungler and one laner to the
//!   region boss on its side for the team buff.
//! * **Gank**: now and then two bots of a team leave their jobs together to
//!   hunt the weakest enemy hero, then go back.
//! * **Retreat**: below 30% health, walk home, heal at the fountain and buy
//!   the next item of their class's recommended build.
//!
//! Bots level, earn gold (passive income plus last hits, camps and kills)
//! and gear up exactly like the player does. A laner leaving the fountain
//! reads its teleport scroll to its lane's front tower when that is a long
//! walk away.

use glam::Vec3;
use rift_cache::Cache;

use super::{AnimatedScene, UnitKind, planar_delta, turn_toward};
use crate::champion::ChampionConfig;
use crate::items::ItemConfig;
use crate::scene::{HeightGrid, WorldCollision};

/// Seconds between a bot's decisions.
const THINK_INTERVAL: f32 = 1.0;
/// Health fraction under which a bot walks home.
const RETREAT_AT: f32 = 0.30;
/// How far a laning bot looks for something to hit.
const LANE_SIGHT: f32 = 750.0;
/// Friendly minions that must stand near an enemy structure before a bot
/// joins the push (so bots do not dive towers alone).
const PUSH_MINIONS: usize = 2;
/// Seconds of laning before a laner takes a jungle stint, and its jitter.
const LANE_STINT: f32 = 90.0;
/// A gank or boss attempt gives up after this long.
const SORTIE_TIMEOUT: f32 = 35.0;
/// Seconds between a team's ganks (first one after `GANK_START`).
const GANK_EVERY: f32 = 80.0;
const GANK_START: f32 = 120.0;
/// Seconds between a team's boss attempts once someone is level 5.
const BOSS_EVERY: f32 = 90.0;
const BOSS_LEVEL: u32 = 5;
/// Fountain: heal radius and rate (fraction of max health per second).
const HOME_RADIUS: f32 = 900.0;
const HOME_HEAL: f32 = 0.06;
/// Passive gold per second, the same as the player's.
const PASSIVE_GOLD: f32 = 8.0;
/// Gold beyond the next item's price before a bot walks home to buy it.
const RECALL_SPARE: u32 = 400;
/// A hero this close to its own fountain is in its base: no gank goes in.
const BASE_SAFE: f32 = 3200.0;
/// Items a bot carries.
const MAX_ITEMS: usize = 6;
/// Recompute a travel path at most this often (seconds).
const REPATH: f32 = 1.0;
/// A lane's front tower farther than this from the fountain is worth a
/// teleport scroll instead of the walk.
const SCROLL_WALK: f32 = 4500.0;

/// One drafted bot: its champion, team, role and lane call.
pub type BotPick = (
    ChampionConfig,
    u8,
    BotRole,
    Option<crate::hero_select::Lane>,
);

/// What a bot is assigned to in the draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotRole {
    /// Lane index into the map's lanes (0 top, 1 mid, 2 bottom).
    Lane(usize),
    Jungle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotMode {
    Lane,
    Jungle,
    /// The region boss camp (unit index).
    Boss(usize),
    /// Hunting an enemy hero (unit index) with a partner.
    Gank(usize),
    Retreat,
}

impl BotMode {
    pub fn label(self) -> &'static str {
        match self {
            BotMode::Lane => "laning",
            BotMode::Jungle => "jungling",
            BotMode::Boss(_) => "boss",
            BotMode::Gank(_) => "ganking",
            BotMode::Retreat => "going home",
        }
    }
}

/// One bot hero's mind.
pub struct BotBrain {
    pub unit: usize,
    pub name: String,
    pub role: BotRole,
    pub mode: BotMode,
    /// Seconds in the current mode.
    pub mode_time: f32,
    think: f32,
    /// Lane waypoint the bot is walking to.
    lane_wp: usize,
    /// Camp being cleared in jungle mode.
    camp: Option<usize>,
    camps_cleared: u32,
    path: Vec<Vec3>,
    path_goal: Vec3,
    repath: f32,
    /// Items to buy, in order, and how many are bought.
    build: Vec<ItemConfig>,
    pub bought: usize,
    gold_income: f32,
    /// Q..R read from the champion data (see `bot_cast`).
    pub spells: Vec<super::bot_cast::BotSpell>,
    /// Seconds until the next ability may be cast.
    pub cast_gap: f32,
    /// A dash in flight, landing its hit when it comes down.
    pub landing: Option<super::bot_cast::Landing>,
}

/// Team-wide timers that pick when to gank and when to take the boss.
#[derive(Debug, Clone, Copy)]
pub struct TeamPlan {
    pub gank_timer: f32,
    pub boss_timer: f32,
}

impl Default for TeamPlan {
    fn default() -> Self {
        Self {
            gank_timer: GANK_START,
            boss_timer: BOSS_EVERY * 0.5,
        }
    }
}

/// Lanes for a team's bot slots in draft order: top, mid, bottom, bottom
/// (the support), then the jungler. Blue's first slot is the player, who
/// is free to go anywhere, so blue bots skip mid.
pub fn roles_for(team: u8, count: usize) -> Vec<BotRole> {
    let order: &[BotRole] = if team == 0 {
        &[
            BotRole::Lane(0),
            BotRole::Lane(2),
            BotRole::Lane(2),
            BotRole::Jungle,
        ]
    } else {
        &[
            BotRole::Lane(0),
            BotRole::Lane(1),
            BotRole::Lane(2),
            BotRole::Lane(2),
            BotRole::Jungle,
        ]
    };
    (0..count)
        .map(|i| order.get(i).copied().unwrap_or(BotRole::Lane(i % 3)))
        .collect()
}

impl AnimatedScene {
    /// Where `team`'s heroes respawn and heal: the player's spawn for blue,
    /// the mirror of it around red's nexus for red.
    pub fn team_home(&self, team: u8) -> Vec3 {
        if team == 0 {
            self.player_spawn
        } else {
            let offset = self.player_spawn - self.nexus_positions[0];
            self.nexus_positions[1] - offset
        }
    }

    /// Spawn the draft's bots: `(champion, team, role, lane call)` each,
    /// with the item build their class buys and, for magic heroes, the
    /// lane call's battle spells. Returns how many spawned.
    pub fn spawn_bots(
        &mut self,
        cache: &Cache,
        bots: &[BotPick],
        builds: &[Vec<ItemConfig>],
        height: &HeightGrid,
    ) -> usize {
        let mut spawned = 0;
        for (n, (champion, team, role, lane)) in bots.iter().enumerate() {
            let home = self.team_home(*team);
            // A loose ring around the fountain so they do not stack.
            let angle = n as f32 * 1.3;
            let pos = home + Vec3::new(angle.cos() * 160.0, 0.0, angle.sin() * 160.0);
            let pos = Vec3::new(pos.x, height.height_at(pos.x, pos.z), pos.z);
            let index = match self.spawn_champion_unit(cache, champion, *team, pos) {
                Ok(index) => index,
                Err(err) => {
                    tracing::warn!("bots: could not spawn {}: {err:#}", champion.name);
                    continue;
                }
            };
            self.units[index].home = home;
            self.set_spells(index, &champion.class, &crate::spells::lane_defaults(*lane));
            let mode = match role {
                BotRole::Jungle => BotMode::Jungle,
                BotRole::Lane(_) => BotMode::Lane,
            };
            tracing::info!(
                "bots: {} on team {team} as {:?} (unit {index})",
                champion.name,
                role
            );
            self.bots.push(BotBrain {
                unit: index,
                name: champion.name.clone(),
                role: *role,
                mode,
                mode_time: 0.0,
                think: (n as f32 * 0.17) % THINK_INTERVAL,
                lane_wp: 0,
                camp: None,
                camps_cleared: 0,
                path: Vec::new(),
                path_goal: pos,
                repath: 0.0,
                build: builds.get(n).cloned().unwrap_or_default(),
                bought: 0,
                gold_income: 0.0,
                spells: super::bot_cast::spells_for(&champion.abilities),
                cast_gap: 0.0,
                landing: None,
            });
            spawned += 1;
        }
        spawned
    }

    /// Drop a bot's planned walk after it was moved (a blink), so it plans
    /// again from where it stands.
    pub(super) fn bot_forget_path(&mut self, unit: usize) {
        if let Some(bot) = self.bots.iter_mut().find(|bot| bot.unit == unit) {
            bot.path.clear();
            bot.repath = 0.0;
        }
    }

    /// The brain driving `unit`, if it is a bot.
    pub fn bot_of(&self, unit: usize) -> Option<&BotBrain> {
        self.bots.iter().find(|bot| bot.unit == unit)
    }

    /// Per-tick bookkeeping for every bot, alive or dead: income, healing,
    /// shopping, and the team plans that start ganks and boss runs.
    pub(super) fn update_bot_brains(&mut self, dt: f32) {
        if self.bots.is_empty() {
            return;
        }
        self.tick_bot_spells(dt);
        for b in 0..self.bots.len() {
            let unit = self.bots[b].unit;
            let alive = self.units[unit].health > 0.0;
            let bot = &mut self.bots[b];
            bot.mode_time += dt;
            bot.think -= dt;
            bot.repath -= dt;
            if !alive {
                bot.mode = BotMode::Retreat;
                bot.path.clear();
                bot.camp = None;
                continue;
            }
            bot.gold_income += PASSIVE_GOLD * dt;
            let gold = bot.gold_income.floor();
            bot.gold_income -= gold;
            let home = self.units[unit].home;
            let u = &mut self.units[unit];
            u.gold = u.gold.saturating_add(gold as u32);
            if u.pos.distance(home) <= HOME_RADIUS {
                u.health = (u.health + u.max_health * HOME_HEAL * dt).min(u.max_health);
                self.bot_shop(b);
            }
        }
        for team in 0..2u8 {
            self.plan_team(team, dt);
        }
        self.bot_status_timer -= dt;
        if self.bot_status_timer <= 0.0 {
            self.bot_status_timer = 30.0;
            for bot in &self.bots {
                let u = &self.units[bot.unit];
                tracing::info!(
                    "bots: status {} t{} lv{} {}g {} items {} at tile {:.0},{:.0} hp {:.0}/{:.0}",
                    bot.name,
                    u.team,
                    u.level,
                    u.gold,
                    bot.bought,
                    bot.mode.label(),
                    u.pos.x / 128.0,
                    u.pos.z / 128.0,
                    u.health,
                    u.max_health
                );
            }
        }
    }

    /// Buy every next build item the bot can afford (only at the fountain).
    fn bot_shop(&mut self, b: usize) {
        loop {
            let bot = &self.bots[b];
            if bot.bought >= MAX_ITEMS.min(bot.build.len()) {
                return;
            }
            let item = bot.build[bot.bought].clone();
            let unit = &mut self.units[bot.unit];
            if unit.gold < item.cost {
                return;
            }
            unit.gold -= item.cost;
            unit.attack_damage += item.damage;
            if item.health > 0.0 {
                unit.max_health += item.health;
                unit.health += item.health;
            }
            unit.speed += item.speed;
            unit.ability_power += item.ability;
            tracing::debug!("bots: {} bought {}", bot.name, item.name);
            self.bots[b].bought += 1;
        }
    }

    /// Start a gank or a boss run for `team` when its timers come up.
    fn plan_team(&mut self, team: u8, dt: f32) {
        let mut plan = self.team_plans[team as usize];
        plan.gank_timer -= dt;
        plan.boss_timer -= dt;
        if plan.gank_timer <= 0.0 {
            plan.gank_timer = GANK_EVERY + (self.bots.len() as f32 * 3.7) % 20.0;
            self.start_gank(team);
        }
        if plan.boss_timer <= 0.0 {
            plan.boss_timer = BOSS_EVERY;
            self.start_boss(team);
        }
        self.team_plans[team as usize] = plan;
    }

    /// Bots of `team` free to leave what they are doing (alive, healthy,
    /// not already on a sortie).
    fn free_bots(&self, team: u8) -> Vec<usize> {
        (0..self.bots.len())
            .filter(|&b| {
                let bot = &self.bots[b];
                let unit = &self.units[bot.unit];
                unit.team == team
                    && unit.health > unit.max_health * 0.6
                    && matches!(bot.mode, BotMode::Lane | BotMode::Jungle)
            })
            .collect()
    }

    /// Two bots hunt the weakest living enemy hero.
    fn start_gank(&mut self, team: u8) {
        let target = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                u.team != team
                    && u.team < 2
                    && u.health > 0.0
                    && matches!(u.kind, UnitKind::Player | UnitKind::BotChampion)
                    // Never into the enemy's base: the fountain and the
                    // towers around it would kill the gankers.
                    && !(self.own_side(u.team, u.pos)
                        && u.pos.distance(self.team_home(u.team)) < BASE_SAFE)
            })
            .min_by(|a, b| {
                let ra = a.1.health / a.1.max_health.max(1.0);
                let rb = b.1.health / b.1.max_health.max(1.0);
                ra.total_cmp(&rb)
            })
            .map(|(i, _)| i);
        let Some(target) = target else {
            return;
        };
        let target_pos = self.units[target].pos;
        let mut free = self.free_bots(team);
        free.sort_by(|&a, &b| {
            let da = self.units[self.bots[a].unit].pos.distance(target_pos);
            let db = self.units[self.bots[b].unit].pos.distance(target_pos);
            da.total_cmp(&db)
        });
        if free.len() < 2 {
            return;
        }
        for &b in &free[..2] {
            self.set_bot_mode(b, BotMode::Gank(target));
        }
        tracing::info!(
            "bots: team {team} gank on unit {target} by {} and {}",
            self.bots[free[0]].name,
            self.bots[free[1]].name
        );
    }

    /// The jungler and the nearest laner take the region boss on their side.
    fn start_boss(&mut self, team: u8) {
        let ready = self.bots.iter().any(|bot| {
            self.units[bot.unit].team == team && self.units[bot.unit].level >= BOSS_LEVEL
        });
        if !ready {
            return;
        }
        let Some(boss) = self.region_boss(team) else {
            return;
        };
        let boss_pos = self.units[boss].pos;
        let mut free = self.free_bots(team);
        free.sort_by(|&a, &b| {
            let jungle_first = |x: usize| self.bots[x].role != BotRole::Jungle;
            jungle_first(a).cmp(&jungle_first(b)).then_with(|| {
                let da = self.units[self.bots[a].unit].pos.distance(boss_pos);
                let db = self.units[self.bots[b].unit].pos.distance(boss_pos);
                da.total_cmp(&db)
            })
        });
        for &b in free.iter().take(2) {
            self.set_bot_mode(b, BotMode::Boss(boss));
        }
        if let Some(&first) = free.first() {
            tracing::info!(
                "bots: team {team} takes the region boss (unit {boss}) with {}",
                self.bots[first].name
            );
        }
    }

    /// The living region boss on `team`'s side of the river.
    fn region_boss(&self, team: u8) -> Option<usize> {
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.kind == UnitKind::Camp && u.camp_boss && u.health > 0.0)
            .filter(|(_, u)| self.own_side(team, u.pos))
            .map(|(i, _)| i)
            .next()
    }

    /// True when `pos` is nearer `team`'s nexus than the enemy's.
    fn own_side(&self, team: u8, pos: Vec3) -> bool {
        let own = self.nexus_positions[team as usize];
        let enemy = self.nexus_positions[1 - team as usize];
        pos.distance(own) <= pos.distance(enemy)
    }

    fn set_bot_mode(&mut self, b: usize, mode: BotMode) {
        if mode == BotMode::Lane && self.bots[b].mode != BotMode::Lane {
            // Join the lane at the waypoint nearest the bot (its start when
            // leaving the fountain), then walk it forward.
            let lane = self.bot_lane_path(b);
            let pos = self.units[self.bots[b].unit].pos;
            self.bots[b].lane_wp = (0..lane.len())
                .min_by(|&a, &c| pos.distance(lane[a]).total_cmp(&pos.distance(lane[c])))
                .unwrap_or(0);
        }
        let level = self.units[self.bots[b].unit].level;
        let bot = &mut self.bots[b];
        if bot.mode != mode {
            tracing::debug!(
                "bots: {} (lv {level}, {} items) {} -> {}",
                bot.name,
                bot.bought,
                bot.mode.label(),
                mode.label()
            );
            bot.mode = mode;
            bot.mode_time = 0.0;
            bot.path.clear();
            bot.camp = None;
        }
    }

    /// Experience and gold for a bot's last hit on a `kind`, with level-ups
    /// on the player's table (+40 health, +2 damage a level).
    pub(super) fn bot_reward(&mut self, unit: usize, kind: UnitKind) {
        let (xp, gold) = super::reward_for(kind);
        if xp <= 0.0 {
            return;
        }
        self.units[unit].gold = self.units[unit].gold.saturating_add(gold);
        self.bot_xp(unit, xp);
    }

    /// Experience for a bot, with the player's level-up stat growth.
    pub(super) fn bot_xp(&mut self, unit: usize, xp: f32) {
        self.units[unit].xp += xp;
        loop {
            let u = &mut self.units[unit];
            if u.level >= super::MAX_LEVEL || u.xp < super::xp_for_level(u.level) {
                break;
            }
            u.xp -= super::xp_for_level(u.level);
            super::level_up_stats(u);
            let level = u.level;
            if let Some(tier) = crate::talents::TALENT_LEVELS
                .iter()
                .position(|&l| l == level)
            {
                // Bots split their picks between the two sides.
                self.apply_talent(unit, tier, ((unit + tier) % 2) as u8);
            }
        }
    }

    /// One bot's decision and action for this tick (alive only).
    pub(super) fn update_bot(
        &mut self,
        index: usize,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) {
        let Some(b) = self.bots.iter().position(|bot| bot.unit == index) else {
            return;
        };
        // A dash or channel from its kit moves the bot for it.
        if self.kit_controls(index) {
            return;
        }
        // A bot channelling its teleport scroll stands still until it lands.
        if self.units[index].scroll.channel.is_some() {
            self.units[index].moving = false;
            return;
        }
        if self.bots[b].think <= 0.0 {
            self.bots[b].think += THINK_INTERVAL;
            self.bot_think(b);
            self.bot_battle_spells(index, collision, height);
        }
        match self.bots[b].mode {
            BotMode::Retreat => {
                let home = self.units[index].home;
                self.units[index].attack_target = None;
                if self.bot_try_escape(b, collision) {
                    return;
                }
                self.bot_travel(b, home, dt, height, collision);
            }
            BotMode::Lane => self.bot_lane(b, dt, height, collision),
            BotMode::Jungle => self.bot_jungle(b, dt, height, collision),
            BotMode::Boss(camp) => {
                if self.units[camp].health <= 0.0 {
                    self.set_bot_mode(b, self.default_mode(b));
                } else {
                    self.bot_engage(b, camp, dt, height, collision);
                }
            }
            BotMode::Gank(target) => {
                let alive = self.units[target].health > 0.0;
                if !alive {
                    self.set_bot_mode(b, self.default_mode(b));
                } else {
                    // Clear the minions in the way only when they block the
                    // last stretch; otherwise run straight at the target.
                    self.bot_engage(b, target, dt, height, collision);
                }
            }
        }
    }

    /// What a bot goes back to after a sortie.
    fn default_mode(&self, b: usize) -> BotMode {
        match self.bots[b].role {
            BotRole::Jungle => BotMode::Jungle,
            BotRole::Lane(_) => BotMode::Lane,
        }
    }

    /// The once-a-second decision.
    fn bot_think(&mut self, b: usize) {
        let unit = &self.units[self.bots[b].unit];
        let ratio = unit.health / unit.max_health.max(1.0);
        let at_home = unit.pos.distance(unit.home) <= HOME_RADIUS * 0.5;
        let mode = self.bots[b].mode;
        let mode_time = self.bots[b].mode_time;
        match mode {
            BotMode::Retreat => {
                if at_home && ratio >= 0.95 {
                    self.set_bot_mode(b, self.default_mode(b));
                }
            }
            _ if ratio < RETREAT_AT => self.set_bot_mode(b, BotMode::Retreat),
            // Rich enough for the next item and not on a sortie: go buy it.
            BotMode::Lane | BotMode::Jungle if self.bot_wants_shop(b) => {
                self.set_bot_mode(b, BotMode::Retreat);
            }
            BotMode::Boss(_) | BotMode::Gank(_) if mode_time > SORTIE_TIMEOUT => {
                self.set_bot_mode(b, self.default_mode(b));
            }
            BotMode::Lane => {
                // Laners take a jungle stint now and then for gold and xp.
                let jitter = (self.bots[b].unit as f32 * 7.3) % 25.0;
                if mode_time > LANE_STINT + jitter {
                    self.set_bot_mode(b, BotMode::Jungle);
                }
            }
            BotMode::Jungle => {
                let laner = matches!(self.bots[b].role, BotRole::Lane(_));
                if laner && (self.bots[b].camps_cleared >= 1 || mode_time > 40.0) {
                    self.bots[b].camps_cleared = 0;
                    self.set_bot_mode(b, BotMode::Lane);
                }
            }
            _ => {}
        }
    }

    /// True when the bot can afford its next item with change to spare.
    fn bot_wants_shop(&self, b: usize) -> bool {
        let bot = &self.bots[b];
        let gold = self.units[bot.unit].gold;
        bot.build
            .get(bot.bought)
            .is_some_and(|item| bot.bought < MAX_ITEMS && gold >= item.cost + RECALL_SPARE)
    }

    /// Lane: push along the assigned lane, fighting what is in reach.
    fn bot_lane(&mut self, b: usize, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        let index = self.bots[b].unit;
        if self.bot_scroll_to_lane(b, height, collision) {
            return;
        }
        if let Some(target) = self.bot_lane_target(index) {
            self.bot_engage(b, target, dt, height, collision);
            return;
        }
        let lane = self.bot_lane_path(b);
        if lane.is_empty() {
            self.units[index].moving = false;
            return;
        }
        let pos = self.units[index].pos;
        let wp = self.bots[b].lane_wp.min(lane.len() - 1);
        let goal = lane[wp];
        if planar_delta(pos, goal).length() < 160.0 && wp + 1 < lane.len() {
            self.bots[b].lane_wp = wp + 1;
            return;
        }
        self.bots[b].lane_wp = wp;
        // Hold short of an enemy tower the minions are not tanking.
        if self.unsafe_step(index, goal) {
            self.units[index].moving = false;
            return;
        }
        self.bot_travel(b, goal, dt, height, collision);
    }

    /// Leaving the fountain for a lane whose front tower is a long walk
    /// away: read the teleport scroll there instead (Dota 2 style).
    fn bot_scroll_to_lane(
        &mut self,
        b: usize,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) -> bool {
        let BotRole::Lane(lane) = self.bots[b].role else {
            return false;
        };
        let index = self.bots[b].unit;
        let unit = &self.units[index];
        if unit.scroll.cooldown > 0.0 || unit.pos.distance(unit.home) > HOME_RADIUS * 1.5 {
            return false;
        }
        let (team, home, pos) = (unit.team, unit.home, unit.pos);
        let front = self
            .units
            .iter()
            .filter(|s| {
                s.team == team
                    && s.health > 0.0
                    && s.kind == UnitKind::Tower
                    && s.structure_lane == Some(lane)
            })
            .map(|s| s.pos)
            .max_by(|a, c| a.distance(home).total_cmp(&c.distance(home)));
        let Some(front) = front else {
            return false;
        };
        if pos.distance(front) < SCROLL_WALK {
            return false;
        }
        self.start_scroll(index, front, collision, height).is_ok()
    }

    /// The bot's lane, oriented from its own nexus to the enemy's.
    fn bot_lane_path(&self, b: usize) -> Vec<Vec3> {
        let lane = match self.bots[b].role {
            BotRole::Lane(lane) => lane,
            BotRole::Jungle => 1,
        };
        let Some(path) = self.lanes.get(lane) else {
            return Vec::new();
        };
        let mut path = path.clone();
        if self.units[self.bots[b].unit].team == 1 {
            path.reverse();
        }
        path
    }

    /// True when walking to `goal` would put the bot inside an enemy
    /// structure's reach that friendly minions are not tanking.
    fn unsafe_step(&self, index: usize, goal: Vec3) -> bool {
        let team = self.units[index].team;
        self.units.iter().any(|s| {
            s.team != team
                && s.team < 2
                && s.health > 0.0
                && s.kind == UnitKind::Tower
                && goal.distance(s.pos) < s.attack_range + 120.0
                && self.minions_near(team, s.pos, 700.0) < PUSH_MINIONS
        })
    }

    fn minions_near(&self, team: u8, pos: Vec3, radius: f32) -> usize {
        self.units
            .iter()
            .filter(|u| u.is_minion_of(team) && u.health > 0.0 && u.pos.distance(pos) <= radius)
            .count()
    }

    /// What a laning bot should hit: a weaker enemy hero close by, else the
    /// nearest minion, else a structure its minions are tanking.
    fn bot_lane_target(&self, index: usize) -> Option<usize> {
        let me = &self.units[index];
        let team = me.team;
        let my_ratio = me.health / me.max_health.max(1.0);
        let mut hero: Option<(usize, f32)> = None;
        let mut minion: Option<(usize, f32)> = None;
        let mut building: Option<(usize, f32)> = None;
        for (i, u) in self.units.iter().enumerate() {
            if i == index || u.team == team || u.team >= 2 || u.health <= 0.0 {
                continue;
            }
            if (i == self.player_index && self.player_untargetable())
                || self.kit_hidden_from(i, me.pos)
            {
                continue;
            }
            let d = me.pos.distance(u.pos);
            if d > LANE_SIGHT + u.hit_radius {
                continue;
            }
            let better = |slot: &Option<(usize, f32)>| slot.is_none_or(|(_, bd)| d < bd);
            match u.kind {
                UnitKind::Player | UnitKind::BotChampion => {
                    let ratio = u.health / u.max_health.max(1.0);
                    let attacking_me = u.attack_target == Some(index);
                    if (ratio <= my_ratio + 0.1 || attacking_me) && better(&hero) {
                        hero = Some((i, d));
                    }
                }
                UnitKind::MeleeMinion | UnitKind::RangedMinion => {
                    if better(&minion) {
                        minion = Some((i, d));
                    }
                }
                UnitKind::Tower | UnitKind::Inhibitor | UnitKind::Nexus
                    if self.minions_near(team, u.pos, 700.0) >= PUSH_MINIONS
                        && better(&building) =>
                {
                    building = Some((i, d));
                }
                _ => {}
            }
        }
        hero.or(minion).or(building).map(|(i, _)| i)
    }

    /// Jungle: clear the nearest living camp on the bot's own side.
    fn bot_jungle(&mut self, b: usize, dt: f32, height: &HeightGrid, collision: &WorldCollision) {
        let index = self.bots[b].unit;
        let team = self.units[index].team;
        let current = self.bots[b].camp.filter(|&c| self.units[c].health > 0.0);
        if current.is_none() && self.bots[b].camp.is_some() {
            self.bots[b].camps_cleared += 1;
        }
        let camp = current.or_else(|| {
            let pos = self.units[index].pos;
            self.units
                .iter()
                .enumerate()
                .filter(|(_, u)| {
                    u.kind == UnitKind::Camp
                        && !u.camp_boss
                        && u.health > 0.0
                        && self.own_side(team, u.home)
                })
                .min_by(|a, c| pos.distance(a.1.pos).total_cmp(&pos.distance(c.1.pos)))
                .map(|(i, _)| i)
        });
        self.bots[b].camp = camp;
        match camp {
            Some(camp) => self.bot_engage(b, camp, dt, height, collision),
            None => {
                // Every camp is down: help the lane meanwhile.
                self.bot_lane(b, dt, height, collision);
            }
        }
    }

    /// Fight `target`: strike when in reach, else close in.
    fn bot_engage(
        &mut self,
        b: usize,
        target: usize,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) {
        let index = self.bots[b].unit;
        let target_pos = self.units[target].pos;
        let (in_range, ready, damage, ranged) = {
            let unit = &self.units[index];
            (
                self.in_reach(index, target, unit.attack_range),
                unit.attack_cooldown <= 0.0 && unit.strike.is_none(),
                unit.attack_damage,
                unit.ranged_auto,
            )
        };
        self.units[index].attack_target = Some(target);
        if self.bot_try_cast(b, target, collision) {
            self.units[index].moving = false;
            return;
        }
        if in_range {
            let unit = &mut self.units[index];
            unit.moving = false;
            let dir = target_pos - unit.pos;
            if dir.length_squared() > 1e-6 {
                turn_toward(unit, dir.x.atan2(dir.z), dt);
            }
            if ready {
                self.begin_strike(index, target, damage, ranged);
            }
        } else {
            self.bot_travel(b, target_pos, dt, height, collision);
        }
    }

    /// Walk toward `goal`, around walls: straight when close, along a
    /// collision path (recomputed at most once a second) when far.
    fn bot_travel(
        &mut self,
        b: usize,
        goal: Vec3,
        dt: f32,
        height: &HeightGrid,
        collision: &WorldCollision,
    ) {
        let index = self.bots[b].unit;
        let pos = self.units[index].pos;
        let speed_mult = self.team_speed_mult(self.units[index].team) * self.kit_move_mult(index);
        if planar_delta(pos, goal).length() < 600.0 {
            self.bots[b].path.clear();
            self.move_unit_toward(index, goal, dt, height, speed_mult);
            return;
        }
        let stale = self.bots[b].path.is_empty() || self.bots[b].path_goal.distance(goal) > 256.0;
        if stale && self.bots[b].repath <= 0.0 {
            self.bots[b].repath = REPATH;
            let mut path = self
                .kit_passage(index, collision)
                .path_world(pos, goal, height);
            if path.is_empty() {
                path.push(goal);
            }
            self.bots[b].path = path;
            self.bots[b].path_goal = goal;
        }
        let next = loop {
            let Some(&next) = self.bots[b].path.first() else {
                break goal;
            };
            if planar_delta(pos, next).length() < 48.0 {
                self.bots[b].path.remove(0);
            } else {
                break next;
            }
        };
        self.move_unit_toward(index, next, dt, height, speed_mult);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_team_covers_every_lane_and_has_a_jungler() {
        let red = roles_for(1, 5);
        for lane in 0..3 {
            assert!(red.contains(&BotRole::Lane(lane)), "red lane {lane}");
        }
        assert!(red.contains(&BotRole::Jungle));
        let blue = roles_for(0, 4);
        assert!(blue.contains(&BotRole::Lane(0)) && blue.contains(&BotRole::Lane(2)));
        assert!(blue.contains(&BotRole::Jungle));
        assert!(!blue.contains(&BotRole::Lane(1)), "mid is the player's");
    }
}
