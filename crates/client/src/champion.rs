use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ChampionConfig {
    pub name: String,
    pub model: u32,
    pub idle: u32,
    pub walk: u32,
    pub scale: f32,
    pub health: f32,
    pub mana: f32,
    pub damage: f32,
    pub attack_range: f32,
    pub attack_interval: f32,
    pub speed: f32,
    pub ranged_auto: bool,
    pub q_damage: f32,
    pub r_damage: f32,
    pub e_heal: f32,
    pub e_shield: f32,
    pub w_distance: f32,
    /// Seconds to turn half way round. See docs/MOVEMENT.md. Absent or zero
    /// keeps the old instant behaviour, for each of the four below.
    #[serde(default)]
    pub turn_time: f32,
    /// Seconds from standing to top speed.
    #[serde(default)]
    pub accel_time: f32,
    /// Seconds from top speed to standing.
    #[serde(default)]
    pub stop_time: f32,
    /// World units covered in one pass of the walk animation. Absent or zero
    /// uses top speed times the walk cycle's length, which plays the walk at
    /// its authored rate at top speed.
    #[serde(default)]
    pub stride: f32,
}

impl ChampionConfig {
    pub fn load(name: &str) -> Result<Self> {
        let path = format!("data/champions/{name}.json");
        let data =
            std::fs::read_to_string(&path).with_context(|| format!("reading champion {}", path))?;
        Ok(serde_json::from_str(&data)?)
    }
}
