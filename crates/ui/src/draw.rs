//! What is to be drawn this frame: quads over the atlas, each clipped to a
//! rectangle and kept in layer order.
//!
//! Every position is a whole pixel of the window, and every image is drawn
//! at a whole multiple of its own size, the same across and down. Nothing
//! here can produce a stretched sprite or a half pixel.

use crate::Colour;
use crate::atlas::{Atlas, Region};
use crate::font::Font;
use crate::theme::Panel;

/// A rectangle of window pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    /// What two rectangles share; empty, with no size, if nothing.
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = (self.right().min(other.right()) - x).max(0);
        let h = (self.bottom().min(other.bottom()) - y).max(0);
        Rect { x, y, w, h }
    }
}

/// One corner of a quad, as the graphics card is given it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Window pixels from the top left.
    pub pos: [f32; 2],
    /// Atlas pixels from its top left.
    pub uv: [f32; 2],
    /// Multiplies what the atlas holds there.
    pub colour: Colour,
}

#[derive(Debug, Clone, Copy)]
struct Quad {
    layer: i32,
    dest: Rect,
    /// Left, top, right, bottom in the atlas.
    uv: [f32; 4],
    colour: Colour,
}

const WHITE: Colour = [1.0, 1.0, 1.0, 1.0];

/// The frame's drawing, in the order it was asked for.
#[derive(Debug, Clone)]
pub struct DrawList {
    screen: Rect,
    scale: i32,
    white: Region,
    clips: Vec<Rect>,
    layer: i32,
    quads: Vec<Quad>,
}

impl DrawList {
    /// A list for a window of this size, drawing at this whole scale.
    pub fn new(window: (u32, u32), scale: u32, atlas: &Atlas) -> Self {
        Self {
            screen: Rect::new(0, 0, window.0 as i32, window.1 as i32),
            scale: scale.max(1) as i32,
            white: atlas.white,
            clips: Vec::new(),
            layer: 0,
            quads: Vec::new(),
        }
    }

    /// The scale: how many window pixels one pixel of an image covers.
    pub fn scale(&self) -> i32 {
        self.scale
    }

    /// The whole window.
    pub fn screen(&self) -> Rect {
        self.screen
    }

    /// Draw only inside this rectangle, and inside whatever was already
    /// being kept to, until [`DrawList::pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) {
        let clip = self.clip().intersect(&rect);
        self.clips.push(clip);
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or(self.screen)
    }

    /// Everything asked for from now on is drawn over lower layers and under
    /// higher ones, whatever order it is asked for in. Within a layer, later
    /// is on top. Returns the layer it was on.
    pub fn set_layer(&mut self, layer: i32) -> i32 {
        std::mem::replace(&mut self.layer, layer)
    }

    /// A plain rectangle, solid or see-through.
    pub fn rect(&mut self, rect: Rect, colour: Colour) {
        let dest = rect.intersect(&self.clip());
        if dest.is_empty() {
            return;
        }
        let (u, v) = (self.white.x as f32 + 0.5, self.white.y as f32 + 0.5);
        self.quads.push(Quad {
            layer: self.layer,
            dest,
            uv: [u, v, u, v],
            colour,
        });
    }

    /// The edge of a rectangle, `thick` window pixels wide, inside it.
    pub fn outline(&mut self, rect: Rect, thick: i32, colour: Colour) {
        self.rect(Rect::new(rect.x, rect.y, rect.w, thick), colour);
        self.rect(
            Rect::new(rect.x, rect.bottom() - thick, rect.w, thick),
            colour,
        );
        self.rect(
            Rect::new(rect.x, rect.y + thick, thick, rect.h - 2 * thick),
            colour,
        );
        self.rect(
            Rect::new(
                rect.right() - thick,
                rect.y + thick,
                thick,
                rect.h - 2 * thick,
            ),
            colour,
        );
    }

    /// An image at the interface's scale, its top left corner at a window
    /// pixel.
    pub fn sprite(&mut self, region: Region, x: i32, y: i32) {
        self.image(region, x, y, self.scale, None, WHITE);
    }

    /// An image at a whole multiple of its own size, whatever the
    /// interface's scale, multiplied by a colour.
    pub fn sprite_times(&mut self, region: Region, x: i32, y: i32, times: u32, colour: Colour) {
        self.image(region, x, y, times.max(1) as i32, None, colour);
    }

    /// The size an image covers at the interface's scale.
    pub fn size_of(&self, region: Region) -> (i32, i32) {
        (region.w as i32 * self.scale, region.h as i32 * self.scale)
    }

    fn image(
        &mut self,
        region: Region,
        x: i32,
        y: i32,
        times: i32,
        within: Option<Rect>,
        colour: Colour,
    ) {
        let whole = Rect::new(x, y, region.w as i32 * times, region.h as i32 * times);
        let mut dest = whole.intersect(&self.clip());
        if let Some(within) = within {
            dest = dest.intersect(&within);
        }
        if dest.is_empty() {
            return;
        }
        // what is cut off the picture is cut off what is read from the atlas
        let u = |px: i32| region.x as f32 + (px - whole.x) as f32 / times as f32;
        let v = |py: i32| region.y as f32 + (py - whole.y) as f32 / times as f32;
        self.quads.push(Quad {
            layer: self.layer,
            dest,
            uv: [u(dest.x), v(dest.y), u(dest.right()), v(dest.bottom())],
            colour,
        });
    }

    /// An image repeated to fill an area, from its top left; the last one in
    /// each direction is cut off at the area's edge, not squeezed.
    pub fn tile(&mut self, region: Region, area: Rect) {
        self.tile_from(region, area.x, area.y, area);
    }

    /// An image repeated to fill an area, the repeats counted from a point
    /// that may be outside it: the first one is then cut off too.
    fn tile_from(&mut self, region: Region, from_x: i32, from_y: i32, area: Rect) {
        if region.w == 0 || region.h == 0 || area.is_empty() {
            return;
        }
        let (w, h) = self.size_of(region);
        let mut y = from_y;
        while y < area.bottom() {
            let mut x = from_x;
            while x < area.right() {
                self.image(region, x, y, self.scale, Some(area), WHITE);
                x += w;
            }
            y += h;
        }
    }

    /// A panel filling an area: its corners kept as they are, its edges and
    /// middle repeated. Nothing is stretched.
    pub fn panel(&mut self, panel: &Panel, area: Rect) {
        if area.is_empty() {
            return;
        }
        if let Some(colour) = panel.solid {
            self.rect(area, colour);
            return;
        }
        let s = self.scale;
        let r = panel.region;
        let (l, t) = (panel.left, panel.top);
        let (mid_w, mid_h) = (
            r.w - panel.left - panel.right,
            r.h - panel.top - panel.bottom,
        );
        // if the area is too small for both corners, each keeps its half
        let left = (l as i32 * s).min(area.w - area.w / 2);
        let right = (panel.right as i32 * s).min(area.w / 2);
        let top = (t as i32 * s).min(area.h - area.h / 2);
        let bottom = (panel.bottom as i32 * s).min(area.h / 2);
        let (x0, x1, x2) = (area.x, area.x + left, area.right() - right);
        let (y0, y1, y2) = (area.y, area.y + top, area.bottom() - bottom);
        let (inner_w, inner_h) = (x2 - x1, y2 - y1);

        // The far corners and edges are placed by their own outer edge, so
        // that an area too small for them cuts their inner side, not their
        // outer one.
        let far_x = area.right() - panel.right as i32 * s;
        let far_y = area.bottom() - panel.bottom as i32 * s;

        // middle, then edges, then corners
        self.tile(
            r.part(l, t, mid_w, mid_h),
            Rect::new(x1, y1, inner_w, inner_h),
        );
        self.tile(r.part(l, 0, mid_w, t), Rect::new(x1, y0, inner_w, top));
        self.tile_from(
            r.part(l, r.h - panel.bottom, mid_w, panel.bottom),
            x1,
            far_y,
            Rect::new(x1, y2, inner_w, bottom),
        );
        self.tile(r.part(0, t, l, mid_h), Rect::new(x0, y1, left, inner_h));
        self.tile_from(
            r.part(r.w - panel.right, t, panel.right, mid_h),
            far_x,
            y1,
            Rect::new(x2, y1, right, inner_h),
        );
        // each corner is kept to its share of the area
        let corners = [
            (r.part(0, 0, l, t), x0, y0, Rect::new(x0, y0, left, top)),
            (
                r.part(r.w - panel.right, 0, panel.right, t),
                far_x,
                y0,
                Rect::new(x2, y0, right, top),
            ),
            (
                r.part(0, r.h - panel.bottom, l, panel.bottom),
                x0,
                far_y,
                Rect::new(x0, y2, left, bottom),
            ),
            (
                r.part(
                    r.w - panel.right,
                    r.h - panel.bottom,
                    panel.right,
                    panel.bottom,
                ),
                far_x,
                far_y,
                Rect::new(x2, y2, right, bottom),
            ),
        ];
        for (part, x, y, share) in corners {
            self.image(part, x, y, s, Some(share), WHITE);
        }
    }

    /// A line of text, its pen starting at `x` on a baseline at `baseline`,
    /// both in window pixels. Returns how wide it came out.
    pub fn text(&mut self, font: &Font, text: &str, x: i32, baseline: i32, colour: Colour) -> i32 {
        let s = self.scale;
        let top = baseline - font.ascent * s;
        let mut pen = x;
        for c in text.chars() {
            let glyph = font.glyph(c);
            if let Some(region) = glyph.region {
                self.image(
                    region,
                    pen + glyph.left * s,
                    top + glyph.top * s,
                    s,
                    None,
                    colour,
                );
            }
            pen += glyph.advance * s;
        }
        pen - x
    }

    /// How many quads have been asked for and not clipped away.
    pub fn len(&self) -> usize {
        self.quads.len()
    }

    pub fn is_empty(&self) -> bool {
        self.quads.is_empty()
    }

    /// The triangles, lowest layer first and, within a layer, in the order
    /// asked for. Four corners and six indices a quad.
    pub fn finish(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut order: Vec<usize> = (0..self.quads.len()).collect();
        order.sort_by_key(|&i| (self.quads[i].layer, i));
        let mut vertices = Vec::with_capacity(order.len() * 4);
        let mut indices = Vec::with_capacity(order.len() * 6);
        for i in order {
            let quad = &self.quads[i];
            let (x0, y0) = (quad.dest.x as f32, quad.dest.y as f32);
            let (x1, y1) = (quad.dest.right() as f32, quad.dest.bottom() as f32);
            let [u0, v0, u1, v1] = quad.uv;
            let base = vertices.len() as u32;
            for (pos, uv) in [
                ([x0, y0], [u0, v0]),
                ([x1, y0], [u1, v0]),
                ([x0, y1], [u0, v1]),
                ([x1, y1], [u1, v1]),
            ] {
                vertices.push(Vertex {
                    pos,
                    uv,
                    colour: quad.colour,
                });
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
        }
        (vertices, indices)
    }
}
