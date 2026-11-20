//! One picture holding every image the interface draws, so that the whole
//! interface is one draw.

/// A plain image: red, green, blue and opacity for each pixel, row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

impl Image {
    pub fn new(width: usize, height: usize, pixels: Vec<[u8; 4]>) -> Self {
        assert_eq!(pixels.len(), width * height, "an image has all its pixels");
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn filled(width: usize, height: usize, colour: [u8; 4]) -> Self {
        Self::new(width, height, vec![colour; width * height])
    }

    /// A part of this image as an image of its own.
    pub fn part(&self, x: usize, y: usize, width: usize, height: usize) -> Image {
        let mut pixels = Vec::with_capacity(width * height);
        for row in y..y + height {
            for col in x..x + width {
                pixels.push(self.pixels[row * self.width + col]);
            }
        }
        Image::new(width, height, pixels)
    }
}

/// Where an image sits in the atlas, in its pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Region {
    /// A part of this region, given from its own top left corner.
    pub fn part(&self, x: u32, y: u32, w: u32, h: u32) -> Region {
        debug_assert!(x + w <= self.w && y + h <= self.h, "inside the region");
        Region {
            x: self.x + x,
            y: self.y + y,
            w,
            h,
        }
    }
}

/// Every image packed into one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    pub width: u32,
    pub height: u32,
    /// Four bytes a pixel, row by row: what the graphics card is given.
    pub pixels: Vec<u8>,
    /// One solid white pixel, for drawing plain rectangles.
    pub white: Region,
}

/// Collects images, then packs them.
#[derive(Default)]
pub struct AtlasBuilder {
    images: Vec<Image>,
}

impl AtlasBuilder {
    pub fn new() -> Self {
        // the first image is always the white pixel
        Self {
            images: vec![Image::filled(1, 1, [255, 255, 255, 255])],
        }
    }

    /// Add an image; the number returned finds its region after packing.
    pub fn add(&mut self, image: Image) -> usize {
        self.images.push(image);
        self.images.len() - 1
    }

    /// Pack everything, tallest first, in rows. Returns the atlas and the
    /// region of every image in the order they were added.
    pub fn build(&self) -> (Atlas, Vec<Region>) {
        let widest = self.images.iter().map(|i| i.width).max().unwrap_or(1);
        let area: usize = self.images.iter().map(|i| i.width * i.height).sum();
        // a power of two wide: at least the widest image, and about square
        let mut width = 64usize;
        while width < widest || width * width < area + area / 4 {
            width *= 2;
        }

        let mut order: Vec<usize> = (0..self.images.len()).collect();
        order.sort_by(|&a, &b| {
            self.images[b]
                .height
                .cmp(&self.images[a].height)
                .then(a.cmp(&b))
        });
        let mut regions = vec![Region::default(); self.images.len()];
        let (mut x, mut y, mut row) = (0usize, 0usize, 0usize);
        for &i in &order {
            let image = &self.images[i];
            if x + image.width > width {
                x = 0;
                y += row;
                row = 0;
            }
            regions[i] = Region {
                x: x as u32,
                y: y as u32,
                w: image.width as u32,
                h: image.height as u32,
            };
            x += image.width;
            row = row.max(image.height);
        }
        let height = (y + row).max(1).next_power_of_two();

        let mut pixels = vec![0u8; width * height * 4];
        for (image, region) in self.images.iter().zip(&regions) {
            for r in 0..image.height {
                for c in 0..image.width {
                    let at = ((region.y as usize + r) * width + region.x as usize + c) * 4;
                    pixels[at..at + 4].copy_from_slice(&image.pixels[r * image.width + c]);
                }
            }
        }
        let atlas = Atlas {
            width: width as u32,
            height: height as u32,
            pixels,
            white: regions[0],
        };
        (atlas, regions)
    }
}

impl Atlas {
    /// The pixel at a place in the atlas; clear if outside it.
    pub fn pixel(&self, x: i64, y: i64) -> [u8; 4] {
        if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
            return [0; 4];
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.pixels[at],
            self.pixels[at + 1],
            self.pixels[at + 2],
            self.pixels[at + 3],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_image_is_in_the_atlas_whole_and_none_overlap() {
        let mut builder = AtlasBuilder::new();
        let sizes = [
            (5, 3),
            (40, 40),
            (1, 90),
            (64, 2),
            (17, 17),
            (130, 9),
            (3, 3),
        ];
        let ids: Vec<usize> = sizes
            .iter()
            .enumerate()
            .map(|(k, &(w, h))| builder.add(Image::filled(w, h, [k as u8 + 1, 0, 0, 255])))
            .collect();
        let (atlas, regions) = builder.build();
        assert!(atlas.width.is_power_of_two() && atlas.height.is_power_of_two());
        let mut owner = vec![0u8; (atlas.width * atlas.height) as usize];
        for (k, &id) in ids.iter().enumerate() {
            let region = regions[id];
            assert_eq!((region.w as usize, region.h as usize), sizes[k]);
            assert!(region.x + region.w <= atlas.width && region.y + region.h <= atlas.height);
            for y in region.y..region.y + region.h {
                for x in region.x..region.x + region.w {
                    assert_eq!(atlas.pixel(x as i64, y as i64), [k as u8 + 1, 0, 0, 255]);
                    let at = (y * atlas.width + x) as usize;
                    assert_eq!(owner[at], 0, "two images share a pixel");
                    owner[at] = k as u8 + 1;
                }
            }
        }
        assert_eq!(
            atlas.pixel(atlas.white.x as i64, atlas.white.y as i64),
            [255; 4]
        );
        assert_eq!((atlas.white.w, atlas.white.h), (1, 1));
    }

    #[test]
    fn packing_is_the_same_every_time() {
        let make = || {
            let mut builder = AtlasBuilder::new();
            for k in 0..30usize {
                builder.add(Image::filled(k % 7 + 1, k % 5 + 1, [k as u8; 4]));
            }
            builder.build()
        };
        assert_eq!(make(), make());
    }

    #[test]
    fn a_part_of_an_image_is_those_pixels() {
        let image = Image::new(3, 2, (0..6).map(|k| [k as u8; 4]).collect());
        let part = image.part(1, 0, 2, 2);
        assert_eq!(part.pixels, vec![[1; 4], [2; 4], [4; 4], [5; 4]]);
    }
}
