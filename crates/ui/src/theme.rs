//! A theme: which image is "the panel", which font is "body text", what
//! colour "announcement" is. Whoever draws asks for roles; nobody asks for
//! an image by where it came from.

use std::collections::BTreeMap;

use crate::Colour;
use crate::atlas::{Atlas, AtlasBuilder, Image, Region};
use crate::font::{Font, Glyph};

/// An image that grows by repetition: so much of each side is kept as it
/// is, and what is between is repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panel {
    pub region: Region,
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

/// One character of a font being added to a theme.
#[derive(Debug, Clone)]
pub struct GlyphSource {
    pub character: char,
    /// Its picture: only the opacity is used, so it can take any colour.
    pub image: Option<Image>,
    pub left: i32,
    pub top: i32,
    pub advance: i32,
}

/// The colour drawn in place of one the theme does not have: loud on
/// purpose, so a hole in a theme is seen and not guessed round.
pub const MISSING: Colour = [1.0, 0.0, 1.0, 1.0];

#[derive(Debug, Clone)]
pub struct Theme {
    pub atlas: Atlas,
    sprites: BTreeMap<String, Region>,
    panels: BTreeMap<String, Panel>,
    fonts: BTreeMap<String, Font>,
    colours: BTreeMap<String, Colour>,
    missing: Region,
}

impl Theme {
    /// The image with this role; a magenta and black check if there is none.
    pub fn sprite(&self, role: &str) -> Region {
        self.sprites.get(role).copied().unwrap_or(self.missing)
    }

    pub fn has_sprite(&self, role: &str) -> bool {
        self.sprites.contains_key(role)
    }

    pub fn panel(&self, role: &str) -> Option<&Panel> {
        self.panels.get(role)
    }

    pub fn font(&self, role: &str) -> Option<&Font> {
        self.fonts.get(role)
    }

    /// The colour with this role; magenta if there is none.
    pub fn colour(&self, role: &str) -> Colour {
        self.colours.get(role).copied().unwrap_or(MISSING)
    }

    pub fn has_colour(&self, role: &str) -> bool {
        self.colours.contains_key(role)
    }

    /// Every role of each kind, in name order.
    pub fn sprite_roles(&self) -> impl Iterator<Item = &str> {
        self.sprites.keys().map(String::as_str)
    }

    pub fn panel_roles(&self) -> impl Iterator<Item = &str> {
        self.panels.keys().map(String::as_str)
    }

    pub fn font_roles(&self) -> impl Iterator<Item = &str> {
        self.fonts.keys().map(String::as_str)
    }

    pub fn colour_roles(&self) -> impl Iterator<Item = &str> {
        self.colours.keys().map(String::as_str)
    }

    /// The check drawn for a missing image.
    pub fn missing(&self) -> Region {
        self.missing
    }
}

/// Collects a theme's images, fonts and colours, then packs them.
pub struct ThemeBuilder {
    atlas: AtlasBuilder,
    missing: usize,
    sprites: Vec<(String, usize)>,
    panels: Vec<(String, usize, [u32; 4])>,
    fonts: Vec<(String, Vec<(char, Option<usize>, i32, i32, i32)>, i32, i32)>,
    colours: BTreeMap<String, Colour>,
}

impl Default for ThemeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeBuilder {
    pub fn new() -> Self {
        let mut atlas = AtlasBuilder::new();
        let check = (0..64)
            .map(|i| {
                if (i % 8 / 4 + i / 8 / 4) % 2 == 0 {
                    [255, 0, 255, 255]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect();
        let missing = atlas.add(Image::new(8, 8, check));
        Self {
            atlas,
            missing,
            sprites: Vec::new(),
            panels: Vec::new(),
            fonts: Vec::new(),
            colours: BTreeMap::new(),
        }
    }

    pub fn sprite(&mut self, role: &str, image: Image) {
        let id = self.atlas.add(image);
        self.sprites.push((role.to_string(), id));
    }

    /// A panel from one image. `insets` is how much of its left, top, right
    /// and bottom is kept as corner and edge; the rest is what repeats.
    pub fn panel(&mut self, role: &str, image: Image, insets: [u32; 4]) -> Result<(), String> {
        let [left, top, right, bottom] = insets;
        if (left + right) as usize >= image.width || (top + bottom) as usize >= image.height {
            return Err(format!(
                "panel {role}: insets {insets:?} leave nothing to repeat in an image {} by {}",
                image.width, image.height
            ));
        }
        let id = self.atlas.add(image);
        self.panels.push((role.to_string(), id, insets));
        Ok(())
    }

    /// A font. `ascent` is from the top of a line to its baseline and
    /// `line_height` from one baseline to the next, at the font's own size.
    pub fn font(&mut self, role: &str, glyphs: Vec<GlyphSource>, ascent: i32, line_height: i32) {
        let glyphs = glyphs
            .into_iter()
            .map(|glyph| {
                // kept as opacity over white, so text takes whatever colour
                // it is drawn in
                let id = glyph
                    .image
                    .filter(|i| i.width > 0 && i.height > 0)
                    .map(|image| {
                        let pixels = image.pixels.iter().map(|p| [255, 255, 255, p[3]]).collect();
                        self.atlas
                            .add(Image::new(image.width, image.height, pixels))
                    });
                (glyph.character, id, glyph.left, glyph.top, glyph.advance)
            })
            .collect();
        self.fonts
            .push((role.to_string(), glyphs, ascent, line_height));
    }

    pub fn colour(&mut self, role: &str, colour: Colour) {
        self.colours.insert(role.to_string(), colour);
    }

    pub fn build(self) -> Theme {
        let (atlas, regions) = self.atlas.build();
        let fonts = self
            .fonts
            .into_iter()
            .map(|(role, glyphs, ascent, line_height)| {
                let glyphs = glyphs
                    .into_iter()
                    .map(|(character, id, left, top, advance)| {
                        let glyph = Glyph {
                            region: id.map(|id| regions[id]),
                            left,
                            top,
                            advance,
                        };
                        (character, glyph)
                    })
                    .collect();
                (role, Font::new(glyphs, ascent, line_height))
            })
            .collect();
        Theme {
            sprites: self
                .sprites
                .into_iter()
                .map(|(role, id)| (role, regions[id]))
                .collect(),
            panels: self
                .panels
                .into_iter()
                .map(|(role, id, [left, top, right, bottom])| {
                    let panel = Panel {
                        region: regions[id],
                        left,
                        top,
                        right,
                        bottom,
                    };
                    (role, panel)
                })
                .collect(),
            fonts,
            colours: self.colours,
            missing: regions[self.missing],
            atlas,
        }
    }
}
