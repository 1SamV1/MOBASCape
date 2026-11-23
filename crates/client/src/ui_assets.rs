use std::collections::HashMap;

use anyhow::{Context, Result};
use image::imageops::FilterType;
use rift_cache::font::FONT_INDEX;
use rift_cache::sprite::SPRITE_INDEX;
use rift_cache::{Cache, Font, SpriteSheet};

// The atlas holds cache item icons, cache UI sprites, the minimap and a
// shelf of champion portraits. 2048px gives the portrait shelf room for a
// full roster without dropping the last champion.
const ATLAS_SIZE: u32 = 2048;
/// Reserved shelf below the item/UI rows and above the font/minimap slots.
const PORTRAIT_ROW_Y: u32 = 428;
/// Roster cards are at most 118px wide, so 128px portraits stay sharp and
/// let two rows hold every champion above the minimap slot.
const PORTRAIT_SIZE: u32 = 128;
const PORTRAIT_GAP: u32 = 4;

/// Cache fonts baked into the atlas for overhead names and hitsplat numbers.
/// The ids are the font archives from index 13; their glyph sheets are the
/// sprite group with the same id in index 8. `p12_full` is OSRS's overhead
/// font, `b12_full` is the bold variant the hitsplat numbers use.
const CACHE_FONTS: [(&str, u32); 3] = [("p11", 494), ("p12", 495), ("b12", 496)];

/// OSRS combat sprites from index 8. These are the exact splats from the
/// cache: blue miss, red damage, gold-trimmed maximum hit, green poison,
/// orange disease, teal venom, the purple heal cross and the white PK skull.
const HITMARK_SPRITES: [(&str, u32); 8] = [
    ("damage", 1359),
    ("max", 3571),
    ("poison", 1360),
    ("disease", 1361),
    ("venom", 1632),
    ("heal", 1629),
    ("miss", 1358),
    ("skull", 439),
];

/// Small cache UI icons the legacy HUD uses outside the hitmark family.
const UI_SPRITES: [(&str, u32); 2] = [("icon_swords", 442), ("icon_skull", 439)];

/// One cache sprite placed in the atlas, with its size in pixels.
#[derive(Clone, Copy)]
pub struct SpriteSlot {
    pub uv: [f32; 4],
    pub width: f32,
    pub height: f32,
}

/// A small shelf-packed atlas of cache-native OSRS interface art.
///
/// Nothing here reads an image file: item icons come from the cache's item
/// models, ability icons and the score-strip icons from index 8, and the
/// fonts and hitmarks from their cache archives. Missing cache entries leave
/// the HUD's procedural fallbacks in place.
pub struct FontGlyph {
    pub uv: [f32; 4],
    pub w: f32,
    pub h: f32,
    pub left: f32,
    pub top: f32,
    pub advance: f32,
}

/// A RuneScape interface font baked into the UI atlas.
pub struct FontAtlas {
    pub size: f32,
    pub glyphs: HashMap<char, FontGlyph>,
}

impl FontAtlas {
    pub fn glyph(&self, c: char) -> Option<&FontGlyph> {
        self.glyphs.get(&c)
    }

    pub fn measure(&self, text: &str, scale: f32) -> f32 {
        text.chars()
            .map(|c| self.glyph(c).map(|g| g.advance).unwrap_or(self.size * 0.5))
            .sum::<f32>()
            * scale
    }
}

pub struct UiAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub white_uv: [f32; 4],
    item_uv: HashMap<String, [f32; 4]>,
    ui_uv: HashMap<String, [f32; 4]>,
    hero_uv: HashMap<String, [f32; 4]>,
    minimap_uv: Option<[f32; 4]>,
    /// Cache-native fonts by name (`p11`, `p12`, `b12`).
    cache_fonts: HashMap<String, FontAtlas>,
    /// Cache-native hitmark sprites by kind.
    hitmarks: HashMap<String, SpriteSlot>,
    item_cursor: (u32, u32, u32),
    ui_cursor: (u32, u32, u32),
}

impl UiAtlas {
    /// An empty atlas with only the white texel every untextured HUD quad
    /// uses. `load_with_cache` fills the visual roles.
    pub fn load() -> Result<Self> {
        let mut atlas = Self {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            pixels: vec![0; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize],
            white_uv: [0.0; 4],
            item_uv: HashMap::new(),
            ui_uv: HashMap::new(),
            hero_uv: HashMap::new(),
            minimap_uv: None,
            cache_fonts: HashMap::new(),
            hitmarks: HashMap::new(),
            item_cursor: (2, 2, 0),
            ui_cursor: (2, 110, 0),
        };
        atlas.pixels[0..4].copy_from_slice(&[255, 255, 255, 255]);
        atlas.white_uv = [
            0.5 / ATLAS_SIZE as f32,
            0.5 / ATLAS_SIZE as f32,
            0.5 / ATLAS_SIZE as f32,
            0.5 / ATLAS_SIZE as f32,
        ];
        Ok(atlas)
    }

    /// Load the empty atlas, then bake the OSRS cache's fonts, hitmarks and
    /// shared interface icons. A cache without those archives degrades to the
    /// built-in pixel font and procedural hitsplats, so this never blocks a
    /// match.
    pub fn load_with_cache(cache: &Cache) -> Result<Self> {
        let mut atlas = Self::load()?;
        atlas.add_cache_assets(cache);
        Ok(atlas)
    }

    /// Bake the cache-native overhead, hitmark and score-strip assets.
    ///
    /// Fonts get one shelf each in the lower half of the atlas; the hitmark
    /// row sits below them. Item and UI icons pack from the top.
    pub fn add_cache_assets(&mut self, cache: &Cache) {
        for (index, (name, id)) in CACHE_FONTS.iter().enumerate() {
            let y = 1300 + index as u32 * 140;
            if let Err(error) = self.add_cache_font(cache, *id, name, 2, y) {
                tracing::warn!("cache font {name} ({id}): {error:#}");
            }
        }
        let mut x = 2;
        for (key, id) in HITMARK_SPRITES {
            match self.add_cache_sprite(cache, id, x, 1740) {
                Ok(slot) => {
                    x += slot.width as u32 + 4;
                    self.hitmarks.insert(key.to_string(), slot);
                }
                Err(error) => tracing::warn!("cache hitmark {key} ({id}): {error:#}"),
            }
        }
        for (key, id) in UI_SPRITES {
            if let Err(error) = self.add_cache_ui_sprite(cache, id, key) {
                tracing::warn!("cache UI sprite {key} ({id}): {error:#}");
            }
        }
        tracing::info!(
            "cache UI: {} fonts, {} hitmarks, {} UI sprites",
            self.cache_fonts.len(),
            self.hitmarks.len(),
            self.ui_uv.len()
        );
    }

    /// Decode one cache font into a shelf of tintable glyphs.
    fn add_cache_font(
        &mut self,
        cache: &Cache,
        id: u32,
        name: &str,
        origin_x: u32,
        origin_y: u32,
    ) -> Result<()> {
        let measurements = cache.group(FONT_INDEX, id)?.data;
        let letters = SpriteSheet::decode(id, &cache.group(SPRITE_INDEX, id)?.data)?;
        let font = Font::decode(id, &measurements, &letters)?;

        let mut cursor_x = origin_x;
        let mut cursor_y = origin_y;
        let mut row_height = 0u32;
        let mut glyphs = HashMap::new();
        for code in 32u8..=126 {
            let glyph = font.glyphs[code as usize];
            let char = rift_cache::font::char_of(code);
            let Some(frame) = letters.frames.get(code as usize) else {
                continue;
            };
            if frame.width == 0 || frame.height == 0 {
                // Spaces and other blank frames still advance the pen.
                glyphs.insert(
                    char,
                    FontGlyph {
                        uv: [0.0; 4],
                        w: 0.0,
                        h: 0.0,
                        left: glyph.left as f32,
                        top: glyph.top as f32,
                        advance: glyph.advance as f32,
                    },
                );
                continue;
            }
            let pixels = frame.rgba(&letters.palette);
            let mut raw = Vec::with_capacity(pixels.len() * 4);
            for px in pixels {
                raw.extend_from_slice(&px);
            }
            let image = image::RgbaImage::from_raw(frame.width as u32, frame.height as u32, raw)
                .with_context(|| format!("font {id} glyph {code}"))?;
            if cursor_x + frame.width as u32 + 1 > ATLAS_SIZE {
                cursor_x = origin_x;
                cursor_y += row_height + 1;
                row_height = 0;
            }
            let uv = self.copy(&image, cursor_x, cursor_y);
            glyphs.insert(
                char,
                FontGlyph {
                    uv,
                    w: frame.width as f32,
                    h: frame.height as f32,
                    left: glyph.left as f32,
                    top: glyph.top as f32,
                    advance: glyph.advance as f32,
                },
            );
            cursor_x += frame.width as u32 + 1;
            row_height = row_height.max(frame.height as u32);
        }
        tracing::debug!(
            "cache font {name}: {} glyphs, drops to y {}",
            glyphs.len(),
            cursor_y + row_height
        );
        self.cache_fonts.insert(
            name.to_string(),
            FontAtlas {
                size: font.ascent as f32,
                glyphs,
            },
        );
        Ok(())
    }

    /// Decode one cache sprite (the first frame) and place it in the atlas.
    fn add_cache_sprite(&mut self, cache: &Cache, id: u32, x: u32, y: u32) -> Result<SpriteSlot> {
        let sheet = SpriteSheet::decode(id, &cache.group(SPRITE_INDEX, id)?.data)?;
        let frame = sheet
            .frames
            .first()
            .with_context(|| format!("sprite {id} has no frames"))?;
        let pixels = frame.rgba(&sheet.palette);
        let mut raw = Vec::with_capacity(pixels.len() * 4);
        for px in pixels {
            raw.extend_from_slice(&px);
        }
        let image = image::RgbaImage::from_raw(frame.width as u32, frame.height as u32, raw)
            .with_context(|| format!("sprite {id} frame buffer"))?;
        let uv = self.copy(&image, x, y);
        Ok(SpriteSlot {
            uv,
            width: frame.width as f32,
            height: frame.height as f32,
        })
    }

    /// Pack a cache sprite into the legacy HUD's UI shelf under `key`.
    /// Champion ability icons use this with their `icon.ability.*` roles.
    pub fn add_cache_ui_sprite(&mut self, cache: &Cache, id: u32, key: &str) -> Result<()> {
        let (x, y, row_height) = self.ui_cursor;
        let slot = self.add_cache_sprite(cache, id, x, y)?;
        let width = slot.width.ceil() as u32;
        let height = slot.height.ceil() as u32;
        let (next_x, next_y, next_row) = if x + width + 1 > ATLAS_SIZE {
            (2, y + row_height + 1, height)
        } else {
            (x + width + 1, y, row_height.max(height))
        };
        self.ui_cursor = (next_x, next_y, next_row);
        self.ui_uv.insert(key.to_string(), slot.uv);
        Ok(())
    }

    /// Pack a cache-rendered item icon (32x32 inventory render) under the
    /// item id the legacy shop already asks for.
    pub fn add_item_icon(&mut self, key: &str, image: &image::RgbaImage) {
        let (x, y, row_height) = self.item_cursor;
        let width = image.width();
        let height = image.height();
        if width == 0 || height == 0 {
            return;
        }
        let (copy_x, copy_y, row_height) = if x + width + 1 > ATLAS_SIZE {
            (2, y + row_height + 1, 0)
        } else {
            (x, y, row_height)
        };
        let uv = self.copy(image, copy_x, copy_y);
        self.item_uv.insert(key.to_string(), uv);
        self.item_cursor = (copy_x + width + 1, copy_y, row_height.max(height));
    }

    /// The interface's default font: OSRS's plain 12 (the overhead and
    /// interface font) straight from the cache.
    pub fn font(&self) -> Option<&FontAtlas> {
        self.cache_fonts.get("p12")
    }

    /// A named cache font, falling back to the default so text never
    /// disappears on a cache without the font archives.
    pub fn font_named(&self, name: &str) -> Option<&FontAtlas> {
        self.cache_fonts.get(name).or_else(|| self.font())
    }

    /// The exact cache hitmark sprite for a damage kind.
    pub fn hitmark(&self, kind: &str) -> Option<SpriteSlot> {
        self.hitmarks.get(kind).copied()
    }

    /// Upload a baked champion portrait into the reserved portrait shelf.
    /// Re-uploading a known champion reuses its slot instead of burning a new
    /// one.
    pub fn add_hero_portrait(&mut self, key: &str, image: &image::RgbaImage) {
        let fitted = if image.width() == PORTRAIT_SIZE && image.height() == PORTRAIT_SIZE {
            image.clone()
        } else {
            image::imageops::resize(image, PORTRAIT_SIZE, PORTRAIT_SIZE, FilterType::Lanczos3)
        };
        if let Some(&uv) = self.hero_uv.get(key) {
            let x = (uv[0] * self.width as f32).round() as u32;
            let y = (uv[1] * self.height as f32).round() as u32;
            self.copy(&fitted, x, y);
            return;
        }
        let index = self.hero_uv.len() as u32;
        let per_row = (ATLAS_SIZE - 2) / (PORTRAIT_SIZE + PORTRAIT_GAP);
        let x = 2 + (index % per_row) * (PORTRAIT_SIZE + PORTRAIT_GAP);
        let y = PORTRAIT_ROW_Y + (index / per_row) * (PORTRAIT_SIZE + PORTRAIT_GAP);
        // Two rows end above the minimap slot at y = 760.
        if y + PORTRAIT_SIZE > 756 {
            tracing::warn!("hero portrait {key} does not fit in the UI atlas");
            return;
        }
        let uv = self.copy(&fitted, x, y);
        self.hero_uv.insert(key.to_string(), uv);
    }

    /// Upload the baked terrain minimap into a reserved atlas slot.
    pub fn add_minimap(&mut self, image: &image::RgbaImage) {
        let uv = self.copy(image, 2, 760);
        self.minimap_uv = Some(uv);
    }

    pub fn minimap_uv(&self) -> Option<[f32; 4]> {
        self.minimap_uv
    }

    fn copy(&mut self, image: &image::RgbaImage, x: u32, y: u32) -> [f32; 4] {
        // Clamp to the atlas so a mis-sized upload can never panic.
        let w = image.width().min(self.width.saturating_sub(x));
        let h = image.height().min(self.height.saturating_sub(y));
        if w == 0 || h == 0 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let source_width = image.width() as usize;
        for py in 0..h {
            for px in 0..w {
                let src = (py as usize * source_width + px as usize) * 4;
                let dst = (((y + py) * self.width + (x + px)) * 4) as usize;
                self.pixels[dst..dst + 4].copy_from_slice(&image.as_raw()[src..src + 4]);
            }
        }
        let aw = self.width as f32;
        let ah = self.height as f32;
        [
            x as f32 / aw,
            y as f32 / ah,
            (x + w) as f32 / aw,
            (y + h) as f32 / ah,
        ]
    }

    pub fn item_uv(&self, id: &str) -> Option<[f32; 4]> {
        self.item_uv.get(id).copied()
    }

    pub fn ui_uv(&self, name: &str) -> Option<[f32; 4]> {
        self.ui_uv.get(name).copied()
    }

    pub fn hero_uv(&self, id: &str) -> Option<[f32; 4]> {
        self.hero_uv.get(id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn pinned_cache() -> Option<Cache> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cache = root.join(".cache/osrs-235/cache");
        if !cache.join("main_file_cache.dat2").exists() {
            return None;
        }
        Some(Cache::open(&cache).expect("the pinned cache opens"))
    }

    #[test]
    fn the_pinned_cache_bakes_osrs_fonts_sprites_and_hitmarks() {
        let Some(cache) = pinned_cache() else {
            return;
        };
        let atlas = UiAtlas::load_with_cache(&cache).expect("the UI atlas with cache assets");
        for font in ["p11", "p12", "b12"] {
            assert!(
                atlas.font_named(font).is_some(),
                "cache font {font} is missing"
            );
        }
        for key in ["icon_swords", "icon_skull"] {
            assert!(
                atlas.ui_uv(key).is_some(),
                "cache UI sprite {key} is missing"
            );
        }
        for (kind, size) in [
            ("damage", (24.0, 23.0)),
            ("max", (24.0, 24.0)),
            ("poison", (24.0, 23.0)),
            ("disease", (24.0, 23.0)),
            ("venom", (24.0, 23.0)),
            ("heal", (24.0, 23.0)),
            ("miss", (24.0, 23.0)),
            ("skull", (25.0, 25.0)),
        ] {
            let slot = atlas
                .hitmark(kind)
                .unwrap_or_else(|| panic!("cache hitmark {kind} is missing"));
            assert_eq!((slot.width, slot.height), size);
        }
    }

    #[test]
    fn item_icons_pack_and_lookup_by_key() {
        let mut atlas = UiAtlas::load().expect("an empty atlas");
        let icon = image::RgbaImage::from_pixel(32, 32, image::Rgba([1, 2, 3, 255]));
        atlas.add_item_icon("bronze_sword", &icon);
        let uv = atlas
            .item_uv("bronze_sword")
            .expect("the item icon is packed");
        assert!(uv[2] > uv[0] && uv[3] > uv[1]);
    }
}
