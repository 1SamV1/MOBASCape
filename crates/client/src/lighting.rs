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
    /// Rim light on units: a sky-tinted edge glow on their silhouettes
    /// that lifts heroes off the ground (0 = off).
    pub unit_rim: f32,
    /// Sun shadow blur radius in shadow-map texels (soft penumbra).
    pub shadow_softness: f32,
    /// Ambient occlusion strength (0 = off, 1 = full).
    pub ssao_strength: f32,
    /// How far ambient occlusion reaches, in world units (a tile is 128).
    pub ssao_radius: f32,
    /// Ambient occlusion contrast (above 1 deepens the creases).
    pub ssao_power: f32,
}

impl Default for Lighting {
    /// The second reference retune of 2026-11-24 (see docs/ROADMAPPLAN.md):
    /// darker ambient, a stronger key light and deeper shadows for the
    /// reference's contrast, greener grass, a darker but still coloured fog
    /// of war.
    fn default() -> Self {
        Self {
            terrain_ambient: 1.12,
            unit_ambient: 1.40,
            sun_strength: 1.02,
            shadow_floor: 0.26,
            hidden_tint: [0.28, 0.30, 0.34],
            hidden_saturation: 0.60,
            grass_saturation: 1.24,
            grass_brightness: 0.58,
            grass_mottle: 0.30,
            unit_rim: 0.45,
            shadow_softness: 3.0,
            ssao_strength: 0.85,
            ssao_radius: 150.0,
            ssao_power: 1.6,
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
        check("unit_rim", self.unit_rim, 0.0, 2.0)?;
        check("shadow_softness", self.shadow_softness, 0.0, 8.0)?;
        check("ssao_strength", self.ssao_strength, 0.0, 1.0)?;
        check("ssao_radius", self.ssao_radius, 8.0, 512.0)?;
        check("ssao_power", self.ssao_power, 0.25, 4.0)?;
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
                self.unit_rim,
            ],
        ]
    }

    /// Ambient occlusion tuning for [`crate::ssao::Ssao`].
    pub fn ssao(&self) -> crate::ssao::SsaoSettings {
        crate::ssao::SsaoSettings {
            radius: self.ssao_radius,
            strength: self.ssao_strength,
            power: self.ssao_power,
        }
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
        // Distinct values, so a swapped field shows (the tuning itself is
        // checked against the shipped file above).
        let lighting = Lighting {
            terrain_ambient: 1.0,
            unit_ambient: 2.0,
            sun_strength: 3.0,
            shadow_floor: 0.4,
            hidden_tint: [0.5, 0.6, 0.7],
            hidden_saturation: 0.8,
            grass_saturation: 9.0,
            grass_brightness: 10.0,
            grass_mottle: 11.0,
            unit_rim: 12.0,
            ..Lighting::default()
        };
        assert_eq!(
            lighting.uniforms(),
            [
                [1.0, 2.0, 3.0, 0.4],
                [0.5, 0.6, 0.7, 0.8],
                [9.0, 10.0, 11.0, 12.0]
            ]
        );
    }
}
