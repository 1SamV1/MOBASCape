//! The interface drawing layer (docs/INTERFACE.md, rule 3 and stage 2).
//!
//! It offers four things: a sprite, a panel that grows by repeating its
//! edges and middle, a line of text in a bitmap font, and a solid or
//! see-through rectangle. All of them clip to a rectangle and draw in layer
//! order, at one whole-number scale, on whole pixels.
//!
//! It knows nothing about RuneScape, the cache, or the graphics card. It is
//! given plain images and tables of letters, by role, in a [`Theme`]; it
//! hands back a list of triangles over one picture, the [`Atlas`]. What the
//! roles are made of is the theme's business, and a different theme is a
//! different set of images and nothing else.

pub mod atlas;
pub mod draw;
pub mod font;
pub mod hud;
pub mod outline;
pub mod raster;
pub mod scale;
pub mod scroll;
pub mod test_screen;
pub mod theme;
pub mod theme_sheet;
pub mod wrap;

pub use atlas::{Atlas, Image, Region};
pub use draw::{DrawList, Rect, Vertex};
pub use font::{Font, Glyph};
pub use theme::{Panel, Theme, ThemeBuilder};

/// Red, green, blue and opacity, each from 0 to 1.
pub type Colour = [f32; 4];
