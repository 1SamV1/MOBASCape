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

#[derive(Deserialize)]
struct PanelRef {
    sprite: u32,
    #[serde(default)]
    frame: usize,
    /// How much of the left, top, right and bottom is kept as it is.
    insets: [u32; 4],
}

#[derive(Deserialize)]
struct FontRef {
    font: u32,
}

pub fn load(cache: &Cache, path: &Path) -> Result<Theme> {
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
        let image = frame_image(cache, panel.sprite, panel.frame)
            .with_context(|| format!("panel role {role}"))?;
        builder
            .panel(role, image, panel.insets)
            .map_err(|why| anyhow!(why))?;
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
