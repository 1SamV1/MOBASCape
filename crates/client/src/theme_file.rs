//! Reads a theme file and makes a theme of it from the game cache.
//!
//! This is the only place that knows both sides: the file names roles and
//! says which cache sprite or font each one is; the drawing layer is handed
//! plain images under those roles and never sees an id. A theme made of
//! other art needs another loader like this one and nothing else changed.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use rift_cache::font::{FONT_INDEX, char_of};
use rift_cache::sprite::SPRITE_INDEX;
use rift_cache::{Cache, Font, SpriteSheet};
use rift_ui::Theme;
use rift_ui::atlas::Image;
use rift_ui::theme::{GlyphSource, ThemeBuilder};
use serde::Deserialize;

#[derive(Deserialize)]
struct ThemeFile {
    #[serde(default)]
    sprites: BTreeMap<String, SpriteRef>,
    #[serde(default)]
    panels: BTreeMap<String, PanelRef>,
    #[serde(default)]
    fonts: BTreeMap<String, FontRef>,
    /// "#rrggbb" or "#rrggbbaa".
    #[serde(default)]
    colours: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct SpriteRef {
    sprite: u32,
    #[serde(default)]
    frame: usize,
}

/// A panel is either cut from one cache sprite or a flat colour. Flat
/// panels are for pieces the reference draws as plain see-through
/// rectangles, which have no sprite to cut.
#[derive(Deserialize)]
#[serde(untagged)]
enum PanelRef {
    Sprite {
        sprite: u32,
        #[serde(default)]
        frame: usize,
        /// How much of the left, top, right and bottom is kept as it is.
        insets: [u32; 4],
    },
    Solid {
        /// "#rrggbb" or "#rrggbbaa".
        solid: String,
    },
}

#[derive(Deserialize)]
struct FontRef {
    font: u32,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn load(cache: &Cache, path: &Path) -> Result<Theme> {
    load_with_items(cache, path, &[])
}

/// Load the theme and bake the OSRS inventory icon for every item in `items`.
/// The role is `icon.item.<key>`; a failed bake leaves the role out, and the
/// component falls back to its colour swatch instead of a missing image.
pub fn load_with_items(cache: &Cache, path: &Path, items: &[(String, u32)]) -> Result<Theme> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading theme {}", path.display()))?;
    let file: ThemeFile =
        serde_json::from_str(&text).with_context(|| format!("parsing theme {}", path.display()))?;

    let mut builder = ThemeBuilder::new();
    for (role, sprite) in &file.sprites {
        let image = frame_image(cache, sprite.sprite, sprite.frame)
            .with_context(|| format!("sprite role {role}"))?;
        builder.sprite(role, image);
    }
    for (role, panel) in &file.panels {
        match panel {
            PanelRef::Sprite {
                sprite,
                frame,
                insets,
            } => {
                let image = frame_image(cache, *sprite, *frame)
                    .with_context(|| format!("panel role {role}"))?;
                builder
                    .panel(role, image, *insets)
                    .map_err(|why| anyhow!(why))?;
            }
            PanelRef::Solid { solid } => {
                let colour = parse_colour(solid).with_context(|| format!("panel role {role}"))?;
                builder.solid_panel(role, colour);
            }
        }
    }
    for (role, font) in &file.fonts {
        add_font(cache, &mut builder, role, font.font)
            .with_context(|| format!("font role {role}"))?;
    }
    for (role, colour) in &file.colours {
        builder.colour(
            role,
            parse_colour(colour).with_context(|| format!("colour role {role}"))?,
        );
    }
    for (key, item_id) in items {
        match crate::item_icons::bake(cache, *item_id) {
            Ok(image) => builder.sprite(&format!("icon.item.{key}"), image),
            Err(err) => tracing::warn!("item icon {key} ({item_id}): {err:#}"),
        }
    }
    Ok(builder.build())
}

fn sheet(cache: &Cache, id: u32) -> Result<SpriteSheet> {
    let group = cache
        .group(SPRITE_INDEX, id)
        .with_context(|| format!("reading sprite {id}"))?;
    Ok(SpriteSheet::decode(id, &group.data)?)
}

/// One frame of a sprite as a plain image, in its place within the sheet's
/// full size, which is how the game lines a sprite up.
fn frame_image(cache: &Cache, id: u32, frame: usize) -> Result<Image> {
    let sheet = sheet(cache, id)?;
    let Some(found) = sheet.frames.get(frame) else {
        bail!(
            "sprite {id} has {} frames, not a frame {frame}",
            sheet.frames.len()
        );
    };
    if found.width == 0 || found.height == 0 {
        bail!("sprite {id} frame {frame} is empty");
    }
    let pixels = found.rgba(&sheet.palette);
    let fits = found.offset_x + found.width <= sheet.width
        && found.offset_y + found.height <= sheet.height;
    if !fits {
        return Ok(Image::new(found.width, found.height, pixels));
    }
    let mut whole = vec![[0u8; 4]; sheet.width * sheet.height];
    for y in 0..found.height {
        for x in 0..found.width {
            whole[(found.offset_y + y) * sheet.width + found.offset_x + x] =
                pixels[y * found.width + x];
        }
    }
    Ok(Image::new(sheet.width, sheet.height, whole))
}

fn add_font(cache: &Cache, builder: &mut ThemeBuilder, role: &str, id: u32) -> Result<()> {
    let measurements = cache
        .group(FONT_INDEX, id)
        .with_context(|| format!("reading font {id}"))?
        .data;
    let letters = sheet(cache, id)?;
    let font = Font::decode(id, &measurements, &letters)?;
    if let Some(note) = &font.note {
        tracing::warn!("font {id} ({role}): {note}");
    }
    let glyphs = (0..=255u8)
        .map(|code| {
            let glyph = font.glyphs[code as usize];
            let image = letters
                .frames
                .get(code as usize)
                .filter(|frame| frame.width != 0 && frame.height != 0)
                .map(|frame| Image::new(frame.width, frame.height, frame.rgba(&letters.palette)));
            GlyphSource {
                character: char_of(code),
                image,
                left: glyph.left,
                top: glyph.top,
                advance: glyph.advance,
            }
        })
        .collect();
    let line_height = font.max_ascent.max(font.ascent) + font.max_descent.max(0) + 1;
    builder.font(role, glyphs, font.ascent, line_height);
    Ok(())
}

fn parse_colour(text: &str) -> Result<rift_ui::Colour> {
    let hex = text
        .strip_prefix('#')
        .ok_or_else(|| anyhow!("colour {text:?} does not start with #"))?;
    if hex.len() != 6 && hex.len() != 8 {
        bail!("colour {text:?} is not #rrggbb or #rrggbbaa");
    }
    let part = |at: usize| -> Result<f32> {
        let byte = u8::from_str_radix(&hex[at..at + 2], 16)
            .with_context(|| format!("colour {text:?} is not hexadecimal"))?;
        Ok(byte as f32 / 255.0)
    };
    Ok([
        part(0)?,
        part(2)?,
        part(4)?,
        if hex.len() == 8 { part(6)? } else { 1.0 },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_panel_colours_parse() {
        let colour = parse_colour("#10203080").unwrap();
        let byte = |value: f32| (value * 255.0).round() as u8;
        assert_eq!(
            [
                byte(colour[0]),
                byte(colour[1]),
                byte(colour[2]),
                byte(colour[3])
            ],
            [16, 32, 48, 128]
        );
        assert!(parse_colour("#12345").is_err());
        assert!(parse_colour("102030").is_err());
        assert!(parse_colour("#zzzzzz").is_err());
    }

    /// The real theme, when the pinned cache has been unpacked; `None` in a
    /// checkout that has no cache, so the test still runs without one.
    fn pinned_theme() -> Option<(std::path::PathBuf, Theme)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cache = root.join(".cache/osrs-235/cache");
        if !cache.join("main_file_cache.dat2").exists() {
            return None;
        }
        let cache = Cache::open(&cache).expect("the pinned cache opens");
        let theme = load(&cache, &root.join("data/ui/theme.json")).expect("the theme loads");
        Some((root, theme))
    }

    #[test]
    fn the_stage_three_roles_load_from_the_pinned_cache_when_it_is_present() {
        let Some((_, theme)) = pinned_theme() else {
            return;
        };
        for role in [
            "font.small",
            "font.body",
            "font.bold",
            "font.title",
            "panel.stone",
            "panel.dark",
            "panel.parchment",
            "panel.slot",
            "button.stone",
            "button.stone.red",
            "icon.kills",
            "icon.team.blue",
            "icon.team.red",
            "icon.gold",
            "orb.ring",
        ] {
            let found =
                theme.font(role).is_some() || theme.panel(role).is_some() || theme.has_sprite(role);
            assert!(found, "role {role} is missing");
        }
        for role in [
            "text",
            "text.quiet",
            "text.dim",
            "text.title",
            "announcement",
            "bar.health",
            "shade",
            "dim",
        ] {
            assert!(theme.has_colour(role), "colour {role} is missing");
        }
        let sheet = rift_ui::theme_sheet::build(&theme, (1920, 1080), None);
        assert!(
            sheet.len() > 200,
            "the theme sheet has {} quads",
            sheet.len()
        );
    }

    #[test]
    fn every_item_with_a_cache_id_gets_a_cache_baked_icon() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cache_path = root.join(".cache/osrs-235/cache");
        if !cache_path.join("main_file_cache.dat2").exists() {
            return;
        }
        let cache = Cache::open(&cache_path).expect("the pinned cache opens");
        let text = std::fs::read_to_string(root.join("data/items/items.json")).expect("items");
        let items: Vec<crate::items::ItemConfig> = serde_json::from_str(&text).expect("item data");
        let icon_items: Vec<(String, u32)> = items
            .iter()
            .filter_map(|item| item.obj.map(|obj| (item.id.clone(), obj)))
            .collect();
        let theme = load_with_items(&cache, &root.join("data/ui/theme.json"), &icon_items)
            .expect("theme with item icons");
        for item in &items {
            if item.obj.is_some() {
                let role = format!("icon.item.{}", item.id);
                assert!(theme.has_sprite(&role), "missing {role}");
            }
        }
    }

    /// Write the theme sheet to `target/theme-sheet.ppm` for the eye, under
    /// the same conditions as the in-game sheet (step A). Ignored by
    /// default: it is evidence, not a unit test, and writes a picture.
    #[test]
    #[ignore = "writes a picture of the theme sheet for the eye"]
    fn the_theme_sheet_can_be_written_for_the_eye() {
        let Some((root, theme)) = pinned_theme() else {
            eprintln!("no pinned cache; nothing to draw");
            return;
        };
        let sheet = rift_ui::theme_sheet::build(&theme, (1920, 1080), None);
        let (vertices, indices) = sheet.finish();
        let (w, h) = (1920usize, 1080usize);
        let mut pixels = vec![[24u8, 24, 30, 255]; w * h];
        rift_ui::raster::render(&vertices, &indices, &theme.atlas, w, h, &mut pixels);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for pixel in pixels {
            ppm.extend_from_slice(&pixel[..3]);
        }
        let path = root.join("target/theme-sheet.ppm");
        std::fs::write(&path, ppm).expect("write the theme sheet picture");
        eprintln!("theme sheet written to {}", path.display());
    }

    /// The step B labelled rectangles over a plain ground, for the eye. It
    /// is ignored by default: it writes a picture, and the game's own
    /// `--ui-layout` draws the same rectangles over a live match.
    #[test]
    #[ignore = "writes a picture of the labelled layout for the eye"]
    fn the_layout_sketch_can_be_written_for_the_eye() {
        let Some((root, theme)) = pinned_theme() else {
            eprintln!("no pinned cache; nothing to draw");
            return;
        };
        let state = rift_ui::hud::sample::state();
        let placed = rift_ui::hud::place(&state, (1920, 1080), None);
        let list = rift_ui::hud::stage3::build(&theme, &placed, &state);
        let (vertices, indices) = list.finish();
        let (w, h) = (1920usize, 1080usize);
        let mut pixels = vec![[24u8, 24, 30, 255]; w * h];
        rift_ui::raster::render(&vertices, &indices, &theme.atlas, w, h, &mut pixels);
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        for pixel in pixels {
            ppm.extend_from_slice(&pixel[..3]);
        }
        let path = root.join("target/layout-1920.ppm");
        std::fs::write(&path, ppm).expect("write the layout picture");
        eprintln!(
            "layout written to {}: scale {}, usual {:.1}%, steps {:?}",
            path.display(),
            placed.scale,
            placed.usual_coverage * 100.0,
            placed.steps
        );
    }

    /// The assembled interface in the states step 15 compares: the strained
    /// whole at 1920 by 1080 and 1366 by 768, the death screen and the shop.
    /// Ignored by default because it writes pictures.
    #[test]
    #[ignore = "writes pictures of the assembled interface for the eye"]
    fn the_whole_stage_three_can_be_written_for_the_eye() {
        let Some((root, theme)) = pinned_theme() else {
            eprintln!("no pinned cache; nothing to draw");
            return;
        };
        let write = |name: &str, w: usize, h: usize, list: rift_ui::DrawList| {
            let (vertices, indices) = list.finish();
            let mut pixels = vec![[24u8, 24, 30, 255]; w * h];
            rift_ui::raster::render(&vertices, &indices, &theme.atlas, w, h, &mut pixels);
            let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
            for pixel in pixels {
                ppm.extend_from_slice(&pixel[..3]);
            }
            let path = root.join(format!("target/{name}.ppm"));
            std::fs::write(&path, ppm).expect("write the interface picture");
            path
        };

        for (name, window) in [
            ("stage3-whole-1920", (1920u32, 1080u32)),
            ("stage3-whole-1366", (1366, 768)),
        ] {
            let state = rift_ui::hud::sample::whole_for(window);
            let placed = rift_ui::hud::place(&state, window, None);
            let list = rift_ui::hud::stage3::build(&theme, &placed, &state);
            let path = write(name, window.0 as usize, window.1 as usize, list);
            eprintln!(
                "{}: scale {}, usual {:.1}%, steps {:?}",
                path.display(),
                placed.scale,
                placed.usual_coverage * 100.0,
                placed.steps
            );
        }

        let state = rift_ui::hud::sample::state();
        let placed = rift_ui::hud::place(&state, (1920, 1080), None);
        let list = rift_ui::hud::stage3::build(&theme, &placed, &state);
        let path = write("stage3-death-1920", 1920, 1080, list);
        eprintln!("{}: death state", path.display());

        let mut state = rift_ui::hud::sample::whole_for((1920, 1080));
        state.shop.open = true;
        state.tooltip = None;
        let placed = rift_ui::hud::place(&state, (1920, 1080), None);
        let list = rift_ui::hud::stage3::build(&theme, &placed, &state);
        let path = write("stage3-shop-1920", 1920, 1080, list);
        eprintln!("{}: shop state", path.display());
    }
}
