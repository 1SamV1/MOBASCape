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
    /// How heavy the champion is to move, 0 (feather) to 100 (colossus). The
    /// player never sees it. It sets the three timings below wherever they are
    /// left out, and how much spring the champion has in its step. See
    /// docs/MOVEMENT.md.
    #[serde(default)]
    pub weight: Option<f32>,
    /// Seconds to turn half way round. Left out, it follows from `weight`;
    /// with no weight either, movement is instant, as it was before.
    #[serde(default)]
    pub turn_time: Option<f32>,
    /// Seconds from standing to top speed.
    #[serde(default)]
    pub accel_time: Option<f32>,
    /// Seconds from top speed to standing.
    #[serde(default)]
    pub stop_time: Option<f32>,
    /// World units covered in one pass of the walk animation. Absent or zero
    /// uses top speed times the walk cycle's length, which plays the walk at
    /// its authored rate at top speed.
    #[serde(default)]
    pub stride: f32,
    /// Scales the spring in the champion's step and its float at rest: 0 for
    /// none, 1 for what its weight gives. Drawing only; it never moves the
    /// champion.
    #[serde(default = "one")]
    pub bounce: f32,
}

impl ChampionConfig {
    pub fn load(name: &str) -> Result<Self> {
        let path = format!("data/champions/{name}.json");
        let data =
            std::fs::read_to_string(&path).with_context(|| format!("reading champion {}", path))?;
        Ok(serde_json::from_str(&data)?)
    }
}

fn one() -> f32 {
    1.0
}

/// How a champion handles: the timings its weight gives, unless set by hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handling {
    pub turn_time: f32,
    pub accel_time: f32,
    pub stop_time: f32,
    /// 0 for the heaviest champion to 1 for the lightest. Drives the spring
    /// in its step.
    pub lightness: f32,
}

/// The timings and lightness for a weight and any timings set by hand.
pub fn handling(
    weight: Option<f32>,
    turn_time: Option<f32>,
    accel_time: Option<f32>,
    stop_time: Option<f32>,
) -> Handling {
    let from_weight = weight.map(|w| {
        let w = w.clamp(0.0, 100.0);
        (0.06 + 0.0046 * w, 0.04 + 0.0031 * w, 0.005 + 0.0023 * w)
    });
    let (turn, accel, stop) = from_weight.unwrap_or((0.0, 0.0, 0.0));
    Handling {
        turn_time: turn_time.unwrap_or(turn).max(0.0),
        accel_time: accel_time.unwrap_or(accel).max(0.0),
        stop_time: stop_time.unwrap_or(stop).max(0.0),
        // a champion with no weight given has no spring: it moves as before
        lightness: weight.map_or(0.0, |w| 1.0 - w.clamp(0.0, 100.0) / 100.0),
    }
}

impl ChampionConfig {
    pub fn handling(&self) -> Handling {
        handling(self.weight, self.turn_time, self.accel_time, self.stop_time)
    }
}
