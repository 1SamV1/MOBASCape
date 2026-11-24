//! OSRS stone interface frames, put together from the cache's own pieces.
//!
//! The game draws its interface windows (bank, Grand Exchange, collection
//! log) from a background tile plus separate edge and riveted-corner
//! sprites. Neither the live theme nor the legacy HUD draws a window from
//! nine separate images, so this joins them once into a single picture:
//! 32px corners around a middle that repeats. Drawn as a repeating nine-slice
//! it gives the same frame at any size. Every pixel comes from cache index 8.

use anyhow::{Context, Result, bail};
use image::RgbaImage;
use rift_cache::sprite::SPRITE_INDEX;
use rift_cache::{Cache, SpriteSheet};

/// Corner cell size of every frame piece, and so the frame's insets.
pub const CORNER: u32 = 32;
/// The repeating middle: three edge pieces across, two down.
pub const MIDDLE: (u32, u32) = (96, 64);

/// Which cache sprites make one kind of frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSpec {
    /// Background tile, repeated under everything.
    pub tile: u32,
    /// Background brightness: 1.0 keeps the tile as it is; inner panels
    /// are a darker recess of the same stone.
    pub shade: f32,
    /// Top, left, bottom and right edge pieces.
    pub edges: [u32; 4],
    /// Top-left, top-right, bottom-left and bottom-right corners.
    pub corners: [u32; 4],
}

/// The window frame: interface background 297, the thick riveted stone
/// border 954-961 (the bank and Grand Exchange frame).
pub const OUTER: FrameSpec = FrameSpec {
    tile: 297,
    shade: 1.0,
    edges: [954, 955, 956, 957],
    corners: [958, 959, 960, 961],
};

/// An inner panel: the same stone, darker, inside the thinner border
/// 962/967 with the plain corners 972-975.
pub const INNER: FrameSpec = FrameSpec {
    tile: 297,
    shade: 0.62,
    edges: [962, 967, 962, 967],
    corners: [972, 973, 974, 975],
};

/// The full frame size: corners plus one middle.
pub fn size() -> (u32, u32) {
    (CORNER * 2 + MIDDLE.0, CORNER * 2 + MIDDLE.1)
}

/// One frame of a cache sprite, cropped to its own pixels.
pub fn sprite_image(cache: &Cache, id: u32) -> Result<RgbaImage> {
    let group = cache
        .group(SPRITE_INDEX, id)
        .with_context(|| format!("reading sprite {id}"))?;
    let sheet = SpriteSheet::decode(id, &group.data)?;
    let Some(frame) = sheet.frames.first() else {
        bail!("sprite {id} has no frames");
    };
    if frame.width == 0 || frame.height == 0 {
        bail!("sprite {id} is empty");
    }
    let raw: Vec<u8> = frame.rgba(&sheet.palette).into_iter().flatten().collect();
    RgbaImage::from_raw(frame.width as u32, frame.height as u32, raw)
        .with_context(|| format!("sprite {id} buffer"))
}

/// One frame placed at its offset inside the sheet's full size (how the
/// game lines up corner pieces).
fn sheet_image(cache: &Cache, id: u32) -> Result<RgbaImage> {
    let group = cache
        .group(SPRITE_INDEX, id)
        .with_context(|| format!("reading sprite {id}"))?;
    let sheet = SpriteSheet::decode(id, &group.data)?;
    let Some(frame) = sheet.frames.first() else {
        bail!("sprite {id} has no frames");
    };
    let mut whole = RgbaImage::new(sheet.width as u32, sheet.height as u32);
    let pixels = frame.rgba(&sheet.palette);
    for y in 0..frame.height {
        for x in 0..frame.width {
            let (px, py) = ((frame.offset_x + x) as u32, (frame.offset_y + y) as u32);
            if px < whole.width() && py < whole.height() {
                whole.put_pixel(px, py, image::Rgba(pixels[y * frame.width + x]));
            }
        }
    }
    Ok(whole)
}

/// Join a frame's pieces into one `size()` picture with `CORNER` insets.
pub fn compose(cache: &Cache, spec: &FrameSpec) -> Result<RgbaImage> {
    let tile = sprite_image(cache, spec.tile)?;
    let edges = spec
        .edges
        .iter()
        .map(|&id| sprite_image(cache, id))
        .collect::<Result<Vec<_>>>()?;
    let corners = spec
        .corners
        .iter()
        .map(|&id| sheet_image(cache, id))
        .collect::<Result<Vec<_>>>()?;
    Ok(assemble(&tile, spec.shade, &edges, &corners))
}

/// The pure part of [`compose`], so the layout is testable without a cache.
/// `edges` and `corners` are in [`FrameSpec`] order.
pub fn assemble(
    tile: &RgbaImage,
    shade: f32,
    edges: &[RgbaImage],
    corners: &[RgbaImage],
) -> RgbaImage {
    let (w, h) = size();
    let mut out = RgbaImage::new(w, h);
    if tile.width() > 0 && tile.height() > 0 {
        for y in 0..h {
            for x in 0..w {
                let p = tile.get_pixel(x % tile.width(), y % tile.height());
                let s = |c: u8| (c as f32 * shade).round().clamp(0.0, 255.0) as u8;
                out.put_pixel(x, y, image::Rgba([s(p[0]), s(p[1]), s(p[2]), 255]));
            }
        }
    }
    // Edges run flush to their side between the corners, one piece per
    // corner-sized step.
    let steps_x = MIDDLE.0.div_ceil(CORNER);
    let steps_y = MIDDLE.1.div_ceil(CORNER);
    if let [top, left, bottom, right] = edges {
        for i in 0..steps_x {
            let x = CORNER + i * CORNER;
            blit(&mut out, top, x, 0);
            blit(&mut out, bottom, x, h - bottom.height());
        }
        for i in 0..steps_y {
            let y = CORNER + i * CORNER;
            blit(&mut out, left, 0, y);
            blit(&mut out, right, w - right.width(), y);
        }
    }
    if let [tl, tr, bl, br] = corners {
        blit(&mut out, tl, 0, 0);
        blit(&mut out, tr, w - tr.width(), 0);
        blit(&mut out, bl, 0, h - bl.height());
        blit(&mut out, br, w - br.width(), h - br.height());
    }
    out
}

/// Draw `piece` over `out` at `(x, y)`, respecting its opacity.
fn blit(out: &mut RgbaImage, piece: &RgbaImage, x: u32, y: u32) {
    for py in 0..piece.height() {
        for px in 0..piece.width() {
            let (ox, oy) = (x + px, y + py);
            if ox >= out.width() || oy >= out.height() {
                continue;
            }
            let src = piece.get_pixel(px, py);
            let a = src[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let dst = out.get_pixel(ox, oy);
            let mix = |s: u8, d: u8| (s as f32 * a + d as f32 * (1.0 - a)).round() as u8;
            out.put_pixel(
                ox,
                oy,
                image::Rgba([
                    mix(src[0], dst[0]),
                    mix(src[1], dst[1]),
                    mix(src[2], dst[2]),
                    255,
                ]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, image::Rgba(c))
    }

    #[test]
    fn assemble_puts_edges_flush_and_corners_in_the_corners() {
        let tile = solid(8, 8, [100, 100, 100, 255]);
        let top = solid(32, 7, [255, 0, 0, 255]);
        let left = solid(7, 32, [0, 255, 0, 255]);
        let bottom = solid(32, 7, [0, 0, 255, 255]);
        let right = solid(7, 32, [255, 255, 0, 255]);
        let corner = solid(32, 32, [0, 0, 0, 0]);
        let mut tl = corner.clone();
        tl.put_pixel(0, 0, image::Rgba([9, 9, 9, 255]));
        let out = assemble(
            &tile,
            0.5,
            &[top, left, bottom, right],
            &[tl, corner.clone(), corner.clone(), corner],
        );
        let (w, h) = size();
        assert_eq!(out.dimensions(), (w, h));
        assert_eq!(out.get_pixel(0, 0).0, [9, 9, 9, 255]);
        assert_eq!(out.get_pixel(40, 0).0, [255, 0, 0, 255]);
        assert_eq!(out.get_pixel(40, h - 1).0, [0, 0, 255, 255]);
        assert_eq!(out.get_pixel(0, 40).0, [0, 255, 0, 255]);
        assert_eq!(out.get_pixel(w - 1, 40).0, [255, 255, 0, 255]);
        // The shaded tile fills the middle.
        assert_eq!(out.get_pixel(80, 60).0, [50, 50, 50, 255]);
    }

    #[test]
    fn the_pinned_cache_has_every_frame_piece() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = root.join(".cache/osrs-235/cache");
        if !path.join("main_file_cache.dat2").exists() {
            return;
        }
        let cache = Cache::open(&path).expect("the pinned cache opens");
        for spec in [OUTER, INNER] {
            let image = compose(&cache, &spec).expect("the frame composes");
            assert_eq!(image.dimensions(), size());
        }
    }
}
