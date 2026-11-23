//! Paper balance: every champion's numbers at level 1 (all rank 1) and at
//! level 11 (every ability at its top rank), worked out with the same
//! formulas the game uses, so outliers show before anyone plays.
//!
//! `rift-client --balance-report docs/BALANCE.md` writes the report.
//!
//! What it models:
//! - health +40 and attack damage +2 per level (`level_up_player`);
//! - effective health against physical damage, `hp * (1 + armor / 100)`
//!   (`armor_damage_multiplier`), shields and heals ignored;
//! - basic-attack damage per second, with the ranged-level scale on ranged
//!   autos;
//! - ability damage from the authored rank tables, times the power the
//!   runtime applies: 5% per level for every hero (`kit::scaled_damage`),
//!   and the ranged scale on the Ranger's abilities;
//! - burst (every damaging ability once) and sustained ability damage per
//!   second (damage / cooldown);
//! - shields and heals from abilities labelled "Shield" or "Heal".
//!
//! What it does not: hit rates, crowd control, summons' damage, passives,
//! items, or damage that comes from percentages.

use std::fmt::Write as _;

use crate::champion::{ChampionAbility, ChampionConfig};

/// A champion is flagged when a number is this far above or below the
/// median of its group (melee or ranged).
pub const HIGH: f32 = 1.35;
pub const LOW: f32 = 0.70;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage {
    pub level: u32,
    pub hp: f32,
    pub ehp: f32,
    pub basic_dps: f32,
    pub burst: f32,
    pub ability_dps: f32,
    pub protection: f32,
}

impl Stage {
    pub fn total_dps(&self) -> f32 {
        self.basic_dps + self.ability_dps
    }
}

#[derive(Debug, Clone)]
pub struct ChampionBalance {
    pub id: String,
    pub name: String,
    pub ranged: bool,
    /// Mana for one cast of each ability, and casts of the full rotation a
    /// full bar pays for at level 1.
    pub combo_cost: f32,
    pub combos: f32,
    pub early: Stage,
    pub late: Stage,
}

/// Mana costs of the hand-written kits in `animated.rs`.
fn hand_written_costs(id: &str) -> [f32; 4] {
    match id {
        "monkey" => [40.0, 30.0, 40.0, 100.0],
        "ranger" => [30.0, 50.0, 25.0, 80.0],
        _ => [25.0, 30.0, 40.0, 80.0],
    }
}

/// What the runtime multiplies authored ability damage by at `level`.
fn power(config: &ChampionConfig, level: u32) -> f32 {
    let base = crate::kit::scaled_damage(1.0, 0.0, level);
    if config.kit.is_none() && config.id == "ranger" {
        base * (1.0 + config.ranged / 100.0)
    } else {
        base
    }
}

fn label(ability: &ChampionAbility) -> &str {
    ability.damage_label.as_deref().unwrap_or("Damage")
}

fn stage(config: &ChampionConfig, level: u32, top_rank: bool) -> Stage {
    let gained = level.saturating_sub(1) as f32;
    let hp = config.health + 40.0 * gained;
    let damage = config.damage + 2.0 * gained;
    let ranged_scale = if config.ranged_auto {
        1.0 + config.ranged / 100.0
    } else {
        1.0
    };
    let power = power(config, level);
    let mut burst = 0.0;
    let mut ability_dps = 0.0;
    let mut protection = 0.0;
    for ability in &config.abilities {
        let Some(rank) = (if top_rank {
            ability.ranks.last()
        } else {
            ability.ranks.first()
        }) else {
            continue;
        };
        let Some(amount) = rank.damage else {
            continue;
        };
        let amount = amount * power;
        match label(ability) {
            "Damage" | "Bonus damage" => {
                burst += amount;
                ability_dps += amount / rank.cooldown.max(0.1);
            }
            "Shield" | "Heal" => protection += amount,
            // Summon hits ("Guard hit", "Spiderling hit") are per summon
            // and per swing; left out of burst.
            _ => {}
        }
    }
    Stage {
        level,
        hp,
        ehp: hp * (1.0 + config.armor.max(0.0) / 100.0),
        basic_dps: damage * ranged_scale / config.attack_interval.max(0.1),
        burst,
        ability_dps,
        protection,
    }
}

pub fn analyse(config: &ChampionConfig) -> ChampionBalance {
    let costs = match &config.kit {
        Some(kit) => std::array::from_fn(|slot| kit.abilities[slot].cost),
        None => hand_written_costs(&config.id),
    };
    let combo_cost: f32 = costs.iter().sum();
    ChampionBalance {
        id: config.id.clone(),
        name: config.name.clone(),
        ranged: config.ranged_auto,
        combo_cost,
        combos: config.mana / combo_cost.max(1.0),
        early: stage(config, 1, false),
        late: stage(config, 11, true),
    }
}

fn median(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) * 0.5
    } else {
        values[mid]
    }
}

/// One flagged number: champion, stage, what, and its ratio to the median.
#[derive(Debug, Clone, PartialEq)]
pub struct Flag {
    pub id: String,
    pub late: bool,
    pub metric: &'static str,
    pub ratio: f32,
}

type Metric = (&'static str, fn(&Stage) -> f32);

const METRICS: [Metric; 5] = [
    ("effective health", |s| s.ehp),
    ("basic DPS", |s| s.basic_dps),
    ("burst", |s| s.burst),
    ("ability DPS", |s| s.ability_dps),
    ("total DPS", |s| s.total_dps()),
];

/// Numbers at least [`HIGH`] times or at most [`LOW`] times their group's
/// median.
pub fn flags(all: &[ChampionBalance]) -> Vec<Flag> {
    let mut out = Vec::new();
    for ranged in [false, true] {
        let group: Vec<&ChampionBalance> = all.iter().filter(|c| c.ranged == ranged).collect();
        for late in [false, true] {
            let pick = |c: &ChampionBalance| if late { c.late } else { c.early };
            for (metric, get) in METRICS {
                let mid = median(group.iter().map(|c| get(&pick(c))).collect());
                if mid <= 0.0 {
                    continue;
                }
                for champion in &group {
                    let ratio = get(&pick(champion)) / mid;
                    if ratio >= HIGH || ratio <= LOW {
                        out.push(Flag {
                            id: champion.id.clone(),
                            late,
                            metric,
                            ratio,
                        });
                    }
                }
            }
        }
    }
    out
}

/// The whole report as Markdown.
pub fn report(champions: &[ChampionConfig]) -> String {
    let mut all: Vec<ChampionBalance> = champions.iter().map(analyse).collect();
    all.sort_by(|a, b| a.ranged.cmp(&b.ranged).then(a.name.cmp(&b.name)));
    let flagged = flags(&all);
    let mut md = String::new();
    let _ = writeln!(md, "# Balance report\n");
    let _ = writeln!(
        md,
        "Generated by `rift-client --balance-report docs/BALANCE.md` from \
         `data/champions`. Paper numbers only: see `crates/client/src/balance.rs` \
         for what is modelled. A number is flagged when it is at least {HIGH}x or \
         at most {LOW}x the median of its group (melee or ranged).\n"
    );
    for (title, late) in [
        ("Level 1, every ability rank 1", false),
        ("Level 11, every ability at its top rank", true),
    ] {
        let _ = writeln!(md, "## {title}\n");
        let _ = writeln!(
            md,
            "| Champion | Group | Eff. HP | Basic DPS | Burst | Ability DPS | Total DPS | Shield/heal | Combo mana | Combos per bar |"
        );
        let _ = writeln!(md, "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|");
        for c in &all {
            let s = if late { c.late } else { c.early };
            let mark = |metric: &str, value: f32| match flagged
                .iter()
                .find(|f| f.id == c.id && f.late == late && f.metric == metric)
            {
                Some(f) if f.ratio >= HIGH => format!("**{value:.0}** ▲"),
                Some(_) => format!("**{value:.0}** ▼"),
                None => format!("{value:.0}"),
            };
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} | {} | {} | {:.0} | {:.0} | {:.1} |",
                c.name,
                if c.ranged { "ranged" } else { "melee" },
                mark("effective health", s.ehp),
                mark("basic DPS", s.basic_dps),
                mark("burst", s.burst),
                mark("ability DPS", s.ability_dps),
                mark("total DPS", s.total_dps()),
                s.protection,
                c.combo_cost,
                c.combos,
            );
        }
        let _ = writeln!(md);
    }
    let _ = writeln!(md, "## Flags\n");
    if flagged.is_empty() {
        let _ = writeln!(md, "Nothing is outside the band.\n");
    }
    for f in &flagged {
        let name = all
            .iter()
            .find(|c| c.id == f.id)
            .map_or(f.id.as_str(), |c| c.name.as_str());
        let _ = writeln!(
            md,
            "- {} ({}): {} is {:.2}x the group median{}",
            name,
            if f.late { "level 11" } else { "level 1" },
            f.metric,
            f.ratio,
            if f.ratio >= HIGH { " ▲" } else { " ▼" }
        );
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster() -> Vec<ChampionConfig> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/champions");
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("champion dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let mut config: ChampionConfig =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            config.id = path.file_stem().unwrap().to_string_lossy().into_owned();
            out.push(config);
        }
        out
    }

    #[test]
    fn level_eleven_is_stronger_than_level_one_for_everyone() {
        for champion in roster().iter().map(analyse) {
            assert!(champion.late.hp > champion.early.hp, "{}", champion.id);
            assert!(
                champion.late.total_dps() > champion.early.total_dps(),
                "{}",
                champion.id
            );
        }
    }

    #[test]
    fn kit_ability_damage_grows_with_level_and_rank() {
        let frog = roster()
            .into_iter()
            .find(|c| c.id == "big_frog")
            .expect("big frog");
        let b = analyse(&frog);
        // Rank 1 burst: Q 45 + W 40 + R 210 (E is a shield) at level 1.
        assert!((b.early.burst - 295.0).abs() < 0.01, "{}", b.early.burst);
        // Top ranks 105 + 100 + 390, times 1.5 at level 11.
        assert!(
            (b.late.burst - 595.0 * 1.5).abs() < 0.01,
            "{}",
            b.late.burst
        );
        assert!((b.early.protection - 90.0).abs() < 0.01);
    }

    #[test]
    fn the_median_and_flags_follow_the_band() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(vec![4.0, 1.0, 2.0, 3.0]), 2.5);
        let report = report(&roster());
        assert!(report.contains("| Big Frog | melee |"));
        assert!(report.contains("## Flags"));
    }
}
