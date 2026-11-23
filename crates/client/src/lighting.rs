//! World light and fog-of-war tuning from `data/lighting.json`.
//!
//! The numbers that set the map's mood (ambient strength, key light, shadow
//! depth, how dark unseen ground falls, grass saturation) live in data so
//! they can be tuned in a running client: edit the file and press F5, which
//! reloads the scene shader and this file together.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// The default location, relative to the working directory.
pub const LIGHTING_PATH: &str = "data/lighting.json";

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Lighting {
    /// Hemispheric ambient multiplier on terrain and scenery.
    pub terrain_ambient: f32,
    /// Ambient multiplier on units (kept a little higher so they read).
    pub unit_ambient: f32,
    /// Key light (sun) strength.
    pub sun_strength: f32,
    /// Fraction of the sun left inside cast shadows (0 = black).
    pub shadow_floor: f32,
    /// Colour multiplier on ground no ally sees (linear RGB).
    pub hidden_tint: [f32; 3],
    /// Saturation kept on unseen ground (0 = grey, 1 = full colour).
    pub hidden_saturation: f32,
    /// Grass saturation boost (1 = the texture's own).
    pub grass_saturation: f32,
    /// Grass base brightness before the mottle adds its variation.
    pub grass_brightness: f32,
    /// How much the slow noise mottle brightens grass patches.
    pub grass_mottle: f32,
}

impl Default for Lighting {
    /// The phase 1 retune of 2026-11-23 (see docs/ROADMAPPLAN.md).
    fn default() -> Self {
        Self {
            terrain_ambient: 1.30,
            unit_ambient: 1.48,
            sun_strength: 0.90,
            shadow_floor: 0.36,
            hidden_tint: [0.40, 0.42, 0.47],
            hidden_saturation: 0.45,
            grass_saturation: 1.12,
            grass_brightness: 0.58,
            grass_mottle: 0.30,
        }
    }
}

impl Lighting {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let lighting: Lighting =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        lighting.validate()?;
        Ok(lighting)
    }

    /// Load the default file, falling back to the built-in values.
    pub fn load_default() -> Self {
        Self::load(LIGHTING_PATH).unwrap_or_else(|err| {
            tracing::warn!("lighting: {err:#}; using built-in values");
            Self::default()
        })
    }

    /// Reject values that would black out or blow out the scene.
    pub fn validate(&self) -> Result<()> {
        let check = |name: &str, value: f32, low: f32, high: f32| {
            if value.is_finite() && (low..=high).contains(&value) {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "{name} = {value} is outside {low}..={high}"
                ))
            }
        };
        check("terrain_ambient", self.terrain_ambient, 0.0, 4.0)?;
        check("unit_ambient", self.unit_ambient, 0.0, 4.0)?;
        check("sun_strength", self.sun_strength, 0.0, 4.0)?;
        check("shadow_floor", self.shadow_floor, 0.0, 1.0)?;
        check("hidden_saturation", self.hidden_saturation, 0.0, 1.0)?;
        check("grass_saturation", self.grass_saturation, 0.0, 3.0)?;
        check("grass_brightness", self.grass_brightness, 0.0, 2.0)?;
        check("grass_mottle", self.grass_mottle, 0.0, 2.0)?;
        for (i, &c) in self.hidden_tint.iter().enumerate() {
            check(&format!("hidden_tint[{i}]"), c, 0.0, 1.0)?;
        }
        Ok(())
    }

    /// The three shader uniform blocks: light, unseen ground, grass.
    pub fn uniforms(&self) -> [[f32; 4]; 3] {
        let [r, g, b] = self.hidden_tint;
        [
            [
                self.terrain_ambient,
                self.unit_ambient,
                self.sun_strength,
                self.shadow_floor,
            ],
            [r, g, b, self.hidden_saturation],
            [
                self.grass_saturation,
                self.grass_brightness,
                self.grass_mottle,
                0.0,
            ],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_matches_the_built_in_defaults() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/lighting.json");
        assert_eq!(Lighting::load(path).unwrap(), Lighting::default());
    }

    #[test]
    fn missing_keys_fall_back_and_unknown_keys_are_rejected() {
        let partial: Lighting = serde_json::from_str(r#"{ "sun_strength": 1.2 }"#).unwrap();
        assert_eq!(partial.sun_strength, 1.2);
        assert_eq!(partial.terrain_ambient, Lighting::default().terrain_ambient);
        assert!(serde_json::from_str::<Lighting>(r#"{ "sun_strenght": 1.2 }"#).is_err());
    }

    #[test]
    fn out_of_range_values_are_refused() {
        let bad = Lighting {
            shadow_floor: 1.5,
            ..Lighting::default()
        };
        assert!(bad.validate().is_err());
        let nan = Lighting {
            sun_strength: f32::NAN,
            ..Lighting::default()
        };
        assert!(nan.validate().is_err());
    }

    #[test]
    fn uniforms_pack_in_shader_order() {
        let u = Lighting::default().uniforms();
        assert_eq!(u[0], [1.30, 1.48, 0.90, 0.36]);
        assert_eq!(u[1][3], 0.45);
        assert_eq!(u[2][0], 1.12);
    }
}
