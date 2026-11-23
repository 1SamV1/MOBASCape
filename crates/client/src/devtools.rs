//! Developer tooling for the client: batch screenshot shot lists, debug view
//! names and the hotkey reference. Nothing here affects normal play; the
//! hooks live in `main.rs` and `renderer.rs` and are opt-in or bound to
//! function keys.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Debug views understood by `scene.wgsl` (`tuning.w & 15`), in cycle order.
pub const DEBUG_VIEWS: [&str; 8] = [
    "lit", "albedo", "normal", "lighting", "fog", "lights", "shadow", "ao",
];

/// Parse a debug view name; unknown names fall back to the lit view.
pub fn debug_view_index(name: &str) -> u32 {
    DEBUG_VIEWS
        .iter()
        .position(|view| view.eq_ignore_ascii_case(name))
        .unwrap_or(0) as u32
}

pub fn debug_view_name(index: u32) -> &'static str {
    DEBUG_VIEWS
        .get(index as usize)
        .copied()
        .unwrap_or(DEBUG_VIEWS[0])
}

/// The in-match developer hotkeys, logged once at startup.
pub const HOTKEY_HELP: &str = "dev keys: F1 help | F2 next debug view | Shift+F2 previous | \
F5 reload scene.wgsl | F7 log position as a shot line | F9 screenshot to .cache/screenshots | \
F10 pause/resume the sim";

/// One camera preset in a batch screenshot run.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotSpec {
    pub name: String,
    /// Map tile the hero is teleported to; the camera follows the hero.
    pub tile: (f32, f32),
    pub pitch: Option<f32>,
    pub yaw: Option<f32>,
    pub distance: Option<f32>,
    pub view: Option<u32>,
}

impl ShotSpec {
    /// The file this shot is written to inside `dir`.
    pub fn output_path(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.png", self.name))
    }

    /// The shot as a line that `parse_shot_list` reads back.
    pub fn to_line(&self) -> String {
        let mut line = format!("{} {:.0} {:.0}", self.name, self.tile.0, self.tile.1);
        if let Some(pitch) = self.pitch {
            line.push_str(&format!(" pitch={pitch:.2}"));
        }
        if let Some(yaw) = self.yaw {
            line.push_str(&format!(" yaw={yaw:.2}"));
        }
        if let Some(distance) = self.distance {
            line.push_str(&format!(" distance={distance:.0}"));
        }
        if let Some(view) = self.view {
            line.push_str(&format!(" view={}", debug_view_name(view)));
        }
        line
    }
}

/// Parse a shot list. One shot per line:
///
/// ```text
/// # name   tile_x tile_y [pitch=R] [yaw=R] [distance=U] [view=NAME]
/// base     2730   3150
/// low      2860   3000   pitch=0.35 distance=2400
/// jungle_n 2830   3080   view=normal
/// ```
///
/// Blank lines and `#` comments are ignored. Names must be unique because
/// they become file names.
pub fn parse_shot_list(text: &str) -> Result<Vec<ShotSpec>> {
    let mut shots: Vec<ShotSpec> = Vec::new();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let context = || format!("shot list line {}: {raw:?}", number + 1);
        let mut fields = line.split_whitespace();
        let name = fields.next().with_context(context)?.to_string();
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            bail!("{}: name must be [A-Za-z0-9_-]", context());
        }
        if shots.iter().any(|shot| shot.name == name) {
            bail!("{}: duplicate shot name {name:?}", context());
        }
        let x: f32 = fields
            .next()
            .with_context(context)?
            .parse()
            .with_context(context)?;
        let y: f32 = fields
            .next()
            .with_context(context)?
            .parse()
            .with_context(context)?;
        let mut shot = ShotSpec {
            name,
            tile: (x, y),
            pitch: None,
            yaw: None,
            distance: None,
            view: None,
        };
        for option in fields {
            let (key, value) = option
                .split_once('=')
                .with_context(|| format!("{}: expected key=value, got {option:?}", context()))?;
            let number = || value.parse::<f32>().with_context(context);
            match key {
                "pitch" => shot.pitch = Some(number()?),
                "yaw" => shot.yaw = Some(number()?),
                "distance" => shot.distance = Some(number()?),
                "view" => {
                    if !DEBUG_VIEWS.iter().any(|v| v.eq_ignore_ascii_case(value)) {
                        bail!("{}: unknown view {value:?}", context());
                    }
                    shot.view = Some(debug_view_index(value));
                }
                _ => bail!("{}: unknown option {key:?}", context()),
            }
        }
        shots.push(shot);
    }
    if shots.is_empty() {
        bail!("shot list is empty");
    }
    Ok(shots)
}

/// A timestamped path for an ad-hoc F9 capture. Screenshots of cache art
/// must never be committed, so they go under the gitignored `.cache`.
pub fn hotkey_screenshot_path() -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    PathBuf::from(".cache/screenshots").join(format!("shot-{stamp}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shot_list_parses_options_comments_and_blank_lines() {
        let shots = parse_shot_list(
            "# header\n\nbase 2730 3150\nlow 2860 3000 pitch=0.35 distance=2400 # trailing\n\
             normals 2830 3080 view=NORMAL yaw=1.5\n",
        )
        .unwrap();
        assert_eq!(shots.len(), 3);
        assert_eq!(shots[0].tile, (2730.0, 3150.0));
        assert_eq!(shots[0].pitch, None);
        assert_eq!(shots[1].pitch, Some(0.35));
        assert_eq!(shots[1].distance, Some(2400.0));
        assert_eq!(shots[2].view, Some(2));
        assert_eq!(shots[2].yaw, Some(1.5));
    }

    #[test]
    fn shot_list_rejects_bad_input() {
        assert!(parse_shot_list("").is_err());
        assert!(parse_shot_list("a 1").is_err());
        assert!(parse_shot_list("a 1 x").is_err());
        assert!(parse_shot_list("a 1 2 zoom=3").is_err());
        assert!(parse_shot_list("a 1 2 view=xray").is_err());
        assert!(parse_shot_list("a 1 2\na 3 4").is_err());
        assert!(parse_shot_list("../x 1 2").is_err());
    }

    #[test]
    fn shot_lines_round_trip() {
        let text = "low 2860 3000 pitch=0.35 yaw=0.50 distance=2400 view=ao";
        let shots = parse_shot_list(text).unwrap();
        assert_eq!(shots[0].to_line(), text);
        assert_eq!(parse_shot_list(&shots[0].to_line()).unwrap(), shots);
    }

    #[test]
    fn shipped_shaders_pass_the_hot_reload_validator() {
        crate::renderer::validate_wgsl(include_str!("scene.wgsl")).unwrap();
        let broken = include_str!("scene.wgsl").replacen("fn albedo(", "fn albedo((", 1);
        let err = crate::renderer::validate_wgsl(&broken).unwrap_err();
        assert!(format!("{err:#}").contains("albedo"), "{err:#}");
    }

    #[test]
    fn debug_view_names_match_the_shader_indices() {
        assert_eq!(debug_view_index("lit"), 0);
        assert_eq!(debug_view_index("Albedo"), 1);
        assert_eq!(debug_view_index("ao"), 7);
        assert_eq!(debug_view_index("bogus"), 0);
        assert_eq!(debug_view_name(6), "shadow");
        assert_eq!(debug_view_name(99), "lit");
    }
}
