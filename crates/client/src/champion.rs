use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

/// One ability slot, shown in the hero-select detail panel.
#[derive(Debug, Clone, Deserialize)]
pub struct ChampionAbility {
    /// Hotkey badge: Q, W, E or R.
    pub key: String,
    pub name: String,
    pub description: String,
    /// Hand-authored numbers for each rank, rank 1 first. Q, W and E have
    /// four ranks and R three; the ability bar tooltip lists them and the
    /// runtime applies them (see `AnimatedScene::rank_*`).
    #[serde(default)]
    pub ranks: Vec<AbilityRank>,
    /// What the headline number means: "Damage" unless set (for example
    /// "Shield" or "Heal" on a defensive ability).
    #[serde(default)]
    pub damage_label: Option<String>,
    /// Shown instead of a number when the ranks carry no `damage`, e.g.
    /// "25% of missing health".
    #[serde(default)]
    pub damage_note: Option<String>,
}

/// One rank of an ability. Damage scales every damage, heal and shield the
/// ability deals by `damage / rank_1_damage`; range scales its reach (shot
/// length, cast range, dash length) by `range / rank_1_range`; the cooldown
/// replaces the kit's.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct AbilityRank {
    #[serde(default)]
    pub damage: Option<f32>,
    /// World units; 0 means the ability is centred on the caster.
    pub range: f32,
    /// Seconds.
    pub cooldown: f32,
}

/// One named sound: a cache sound id plus volume and playback rate.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct SoundSpec {
    pub id: u32,
    #[serde(default = "one")]
    pub volume: f32,
    #[serde(default = "one")]
    pub speed: f32,
}

/// Shared combat and interface sounds every unit falls back to
/// (`data/sounds.json`). A champion's own `sounds` entry with the same name
/// overrides one for that champion.
pub type SoundTable = std::collections::BTreeMap<String, SoundSpec>;

/// Load `data/sounds.json`. A missing or broken file leaves the game silent
/// apart from champion-specific sounds rather than failing the boot.
pub fn load_default_sounds() -> SoundTable {
    load_sound_table("data/sounds.json").unwrap_or_else(|err| {
        tracing::warn!("sounds: {err:#}; shared combat sounds are off");
        SoundTable::new()
    })
}

fn load_sound_table(path: &str) -> Result<SoundTable> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {path}"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChampionConfig {
    /// File stem used by the hero-select screen and portrait atlas.
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Short role label shown under the name on the hero-select card.
    #[serde(default)]
    pub role: String,
    /// Combat class: MELEE, RANGED or MAGIC. Drives the hero-select filters.
    #[serde(default)]
    pub class: String,
    /// Ability list shown in the hero-select detail panel.
    #[serde(default)]
    pub abilities: Vec<ChampionAbility>,
    /// Cache sprite IDs for the Q, W, E and R icons in the live ability bar.
    #[serde(default)]
    pub ability_icons: Vec<u32>,
    /// Data-driven passive and abilities. Champions without one keep their
    /// hand-written kit in `animated.rs`.
    #[serde(default)]
    pub kit: Option<crate::kit::Kit>,
    /// Cosmetic weapon (an item id from `data/items/items.json`) a humanoid
    /// hero holds while no weapon is in the inventory. It grants no stats.
    #[serde(default)]
    pub weapon: Option<String>,
    /// Named cache sound effects the kit plays (`"q_throw"`, ...). Ids are
    /// cache index-4 sounds; audition them with `rift-tools sound ID --out`.
    #[serde(default)]
    pub sounds: std::collections::BTreeMap<String, SoundSpec>,
    /// One-line flavour text shown on the hero-select card.
    #[serde(default)]
    pub tagline: String,
    pub model: u32,
    /// Extra OSRS models rendered with the same pose. Multi-model NPCs (for
    /// example King Awowogei's crown) are authored as separate meshes.
    #[serde(default)]
    pub attachments: Vec<u32>,
    pub idle: u32,
    pub walk: u32,
    pub scale: f32,
    /// Extra yaw for a model whose forward axis breaks the cache convention.
    /// The renderer already turns every OSRS model to face its movement.
    #[serde(default)]
    pub model_yaw: f32,
    /// Multiplier for sideways rotation in the walk clip. Some cache walk
    /// animations (notably King Awowogei) twist the torso/arms far more than
    /// reads well at MOBA scale; 1.0 keeps the clip untouched.
    #[serde(default = "default_walk_sway")]
    pub walk_sway: f32,
    /// Multiplier for the walk clip's arm-group transforms. 0 freezes the
    /// arms in their rest pose while the torso and legs animate.
    #[serde(default = "one")]
    pub walk_arm: f32,
    /// Seconds from committing to an attack to the hit landing. Zero keeps
    /// instant attacks.
    #[serde(default)]
    pub attack_windup: f32,
    /// Cache sequence played for each basic attack. The clip is time scaled
    /// so its `attack_hit` moment lands when the wind-up ends.
    #[serde(default)]
    pub attack_anim: Option<u32>,
    /// Fraction of the attack clip at which the blow connects.
    #[serde(default = "half")]
    pub attack_hit: f32,
    /// Cache sequence played when an ability is cast. Defaults to the
    /// attack clip.
    #[serde(default)]
    pub cast_anim: Option<u32>,
    /// Basic-attack projectile look from `fx::LOOKS` (ranged champions).
    /// Defaults by class: darts for RANGED, an arcane orb for MAGIC.
    #[serde(default)]
    pub projectile: Option<String>,
    pub health: f32,
    pub mana: f32,
    pub damage: f32,
    /// Starting armor. Physical damage is scaled by 100 / (100 + armor).
    #[serde(default)]
    pub armor: f32,
    pub attack_range: f32,
    pub attack_interval: f32,
    pub speed: f32,
    pub ranged_auto: bool,
    /// Ranged level, shown in the OSRS-style stats panel and used to scale
    /// ranged damage.
    #[serde(default)]
    pub ranged: f32,
    #[serde(default)]
    pub q_damage: f32,
    #[serde(default)]
    pub r_damage: f32,
    #[serde(default)]
    pub e_heal: f32,
    #[serde(default)]
    pub e_shield: f32,
    #[serde(default)]
    pub w_distance: f32,
    /// How heavy the champion is to move, 0 (feather) to 100 (colossus).
    /// Drives turn, acceleration and stopping time unless they are set by hand.
    #[serde(default)]
    pub weight: Option<f32>,
    /// Seconds to turn half way round. Defaults from `weight`.
    #[serde(default)]
    pub turn_time: Option<f32>,
    /// Seconds from standing to top speed.
    #[serde(default)]
    pub accel_time: Option<f32>,
    /// Seconds from top speed to a stop.
    #[serde(default)]
    pub stop_time: Option<f32>,
    /// World units covered in one pass of the walk animation. Zero derives it
    /// from top speed and the animation's own loop length.
    #[serde(default)]
    pub stride: f32,
    /// Scales the drawing-only spring in the champion's step.
    #[serde(default = "one")]
    pub bounce: f32,
}

fn default_walk_sway() -> f32 {
    1.0
}

fn one() -> f32 {
    1.0
}

fn half() -> f32 {
    0.5
}

/// Movement timings derived from a champion's weight, with per-file overrides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handling {
    pub turn_time: f32,
    pub accel_time: f32,
    pub stop_time: f32,
    /// 1 for the lightest champion, 0 for the heaviest.
    pub lightness: f32,
}

impl ChampionConfig {
    pub fn handling(&self) -> Handling {
        let from_weight = self.weight.map(|weight| {
            let weight = weight.clamp(0.0, 100.0);
            (
                0.06 + 0.0046 * weight,
                0.04 + 0.0031 * weight,
                0.005 + 0.0023 * weight,
            )
        });
        let (turn, accel, stop) = from_weight.unwrap_or((0.0, 0.0, 0.0));
        Handling {
            turn_time: self.turn_time.unwrap_or(turn).max(0.0),
            accel_time: self.accel_time.unwrap_or(accel).max(0.0),
            stop_time: self.stop_time.unwrap_or(stop).max(0.0),
            lightness: self
                .weight
                .map_or(0.0, |weight| 1.0 - weight.clamp(0.0, 100.0) / 100.0),
        }
    }
}

impl ChampionConfig {
    pub fn load(name: &str) -> Result<Self> {
        let path = format!("data/champions/{name}.json");
        let data =
            std::fs::read_to_string(&path).with_context(|| format!("reading champion {}", path))?;
        let mut config: Self = serde_json::from_str(&data)?;
        config.fill_defaults(name);
        Ok(config)
    }

    /// Load every champion in `data/champions` so the roster stays data-driven.
    pub fn load_all() -> Result<Vec<Self>> {
        let dir = Path::new("data/champions");
        let mut paths = std::fs::read_dir(dir)
            .with_context(|| format!("reading champion directory {}", dir.display()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect::<Vec<_>>();
        paths.sort();

        let mut champions = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            champions.push(Self::load(stem)?);
        }
        if champions.is_empty() {
            anyhow::bail!("no champions found in {}", dir.display());
        }
        Ok(champions)
    }

    fn fill_defaults(&mut self, name: &str) {
        if self.id.is_empty() {
            self.id = name.to_string();
        }
        if self.role.is_empty() {
            self.role = if self.ranged_auto {
                "RANGED SKIRMISHER"
            } else if self.health >= 650.0 {
                "TANK MAGE"
            } else {
                "BALANCED FIGHTER"
            }
            .to_string();
        }
        if self.tagline.is_empty() {
            self.tagline = "A champion of the Karamja rift.".to_string();
        }
        if self.class.is_empty() {
            self.class = if self.ranged_auto { "RANGED" } else { "MELEE" }.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_sound_table_names_every_sound_the_scene_asks_for() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/sounds.json");
        let table = load_sound_table(path.to_str().unwrap()).expect("data/sounds.json");
        for name in [
            "melee_hit",
            "ranged_hit",
            "blocked_hit",
            "player_hurt",
            "ranged_release",
            "magic_release",
            "cast",
            "shop_buy",
            "shop_sell",
            "skill_point",
            "frame_sound",
            "structure_fall",
        ] {
            let spec = table.get(name).unwrap_or_else(|| panic!("missing {name}"));
            assert!(spec.volume > 0.0 && spec.volume <= 1.0, "{name} volume");
            assert!(spec.speed > 0.25 && spec.speed < 4.0, "{name} speed");
        }
    }

    #[test]
    fn every_champion_file_parses_and_kits_are_complete() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/champions");
        let mut kits = 0;
        for entry in std::fs::read_dir(&dir).expect("champion dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read");
            let config: ChampionConfig = serde_json::from_str(&text)
                .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
            assert_eq!(config.abilities.len(), 4, "{}", path.display());
            assert_eq!(config.ability_icons.len(), 4, "{}", path.display());
            if let Some(kit) = &config.kit {
                kits += 1;
                for ability in &kit.abilities {
                    assert!(!ability.actions.is_empty(), "{}", path.display());
                    assert!(ability.cooldown > 0.0 && ability.cost >= 0.0);
                }
            }
        }
        assert!(
            kits >= 12,
            "expected the twelve kit champions, found {kits}"
        );
    }

    #[test]
    fn every_ability_has_hand_authored_ranks_that_only_improve() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/champions");
        for entry in std::fs::read_dir(&dir).expect("champion dir") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read");
            let config: ChampionConfig = serde_json::from_str(&text).expect("parse");
            for (slot, ability) in config.abilities.iter().enumerate() {
                let at = format!("{} {}", path.display(), ability.key);
                let want = if slot == 3 { 3 } else { 4 };
                assert_eq!(ability.ranks.len(), want, "{at}");
                if let Some(kit) = &config.kit {
                    assert_eq!(
                        ability.ranks[0].cooldown, kit.abilities[slot].cooldown,
                        "{at}: rank 1 keeps the kit's cooldown"
                    );
                }
                for pair in ability.ranks.windows(2) {
                    assert!(pair[1].cooldown <= pair[0].cooldown, "{at}: cooldown");
                    assert!(pair[1].cooldown > 0.0, "{at}: cooldown");
                    assert!(pair[1].range >= pair[0].range, "{at}: range");
                    assert_eq!(pair[0].damage.is_some(), pair[1].damage.is_some(), "{at}");
                    if let (Some(a), Some(b)) = (pair[0].damage, pair[1].damage) {
                        assert!(b > a, "{at}: damage");
                    }
                }
                if ability.ranks[0].damage.is_none() {
                    assert!(
                        ability.damage_note.is_some(),
                        "{at}: no number needs a note"
                    );
                }
            }
        }
    }

    #[test]
    fn blowpipe_ranger_kit_is_data_driven() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/champions/ranger.json");
        let data = std::fs::read_to_string(&path).expect("ranger.json");
        let config: ChampionConfig = serde_json::from_str(&data).expect("ranger.json parses");
        assert!(
            config.ranged_auto,
            "the ranger must auto-attack with a blowpipe"
        );
        assert_eq!(config.ranged, 99.0, "ranged level drives ranged damage");
        assert_eq!(config.model, 314, "OSRS ranger torso model");
        assert!(
            config.attachments.contains(&176),
            "the blowpipe must be merged into the ranger"
        );
        assert!(
            config.attachments.len() >= 4,
            "the ranger needs its full body"
        );
        assert!(
            config.attack_range >= 512.0,
            "blowpipe range should be long"
        );
    }

    /// Every shipped hero, animated through idle and walk on the pinned
    /// cache: no part may drift off the body (the old merge froze
    /// attachments, so heads, feet and hands came apart).
    #[test]
    fn every_hero_part_stays_on_the_body() {
        use rift_cache::skeleton::{Binding, DRIFT_LIMIT, assemble};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cache_dir = root.join(".cache/osrs-235/cache");
        if !cache_dir.join("main_file_cache.dat2").exists() {
            return;
        }
        let cache = rift_cache::Cache::open(&cache_dir).expect("cache");
        let mut checked = 0;
        for entry in std::fs::read_dir(root.join("data/champions")).expect("champions") {
            let path = entry.expect("entry").path();
            let config: ChampionConfig =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("read"))
                    .expect("parse");
            if config.attachments.is_empty() {
                continue;
            }
            let parts: Vec<rift_cache::Model> = std::iter::once(config.model)
                .chain(config.attachments.iter().copied())
                .map(|id| {
                    rift_cache::Model::load(id, &cache.group(7, id).expect("model").data)
                        .expect("decode")
                })
                .collect();
            let clips: Vec<rift_cache::AnimationClip> = [config.idle, config.walk]
                .iter()
                .map(|&seq| rift_cache::AnimationClip::load(&cache, seq).expect("clip"))
                .collect();
            let refs: Vec<&rift_cache::AnimationClip> = clips.iter().collect();
            let (_, reports) = assemble(&parts, &refs);
            for r in reports.iter().skip(1) {
                let chosen = match r.binding {
                    Binding::Native => r.native_drift,
                    Binding::Rigid => r.rigid_drift,
                    _ => r.follow_drift,
                };
                assert!(
                    chosen <= DRIFT_LIMIT,
                    "{} part {} (model {}) drifts {chosen:.3} of its height",
                    config.name,
                    r.index,
                    r.model_id
                );
                // Allow integer rounding in the transforms (0.5% of height).
                assert!(
                    chosen <= r.static_drift + 0.005,
                    "{} part {} got worse",
                    config.name,
                    r.index
                );
            }
            checked += 1;
        }
        assert!(checked >= 10, "multi-part heroes checked: {checked}");
    }
}
