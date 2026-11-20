//! A view over content that may be taller than it, scrolled by whole
//! pixels. The shop uses it for its item list; the drawing that follows is
//! clipped to [`Scroll::view`].

use crate::draw::Rect;

/// A clipped view and where the content sits inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scroll {
    pub view: Rect,
    pub content_height: i32,
    pub offset: i32,
}

impl Scroll {
    /// A view at this offset, brought back inside the content if it is out
    /// of range. Sizes are window pixels.
    pub fn new(view: Rect, content_height: i32, offset: i32) -> Self {
        let mut scroll = Self {
            view,
            content_height,
            offset,
        };
        scroll.clamp();
        scroll
    }

    pub fn max_offset(&self) -> i32 {
        (self.content_height - self.view.h).max(0)
    }

    pub fn clamp(&mut self) {
        self.offset = self.offset.clamp(0, self.max_offset());
    }

    /// Where content `y` falls in the window.
    pub fn place(&self, content_y: i32) -> i32 {
        self.view.y + content_y - self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offset_is_kept_between_the_top_and_the_last_page() {
        let view = Rect::new(10, 20, 100, 50);
        assert_eq!(Scroll::new(view, 30, 10).offset, 0);
        assert_eq!(Scroll::new(view, 120, -5).offset, 0);
        assert_eq!(Scroll::new(view, 120, 40).offset, 40);
        assert_eq!(Scroll::new(view, 120, 999).offset, 70);
    }

    #[test]
    fn content_moves_up_by_the_offset() {
        let scroll = Scroll::new(Rect::new(10, 20, 100, 50), 120, 30);
        assert_eq!(scroll.place(0), -10);
        assert_eq!(scroll.place(30), 20);
        assert_eq!(scroll.place(50), 40);
    }

    #[test]
    fn shorter_content_never_scrolls() {
        let scroll = Scroll::new(Rect::new(0, 0, 100, 50), 20, 10);
        assert_eq!(scroll.max_offset(), 0);
        assert_eq!(scroll.offset, 0);
        assert_eq!(scroll.place(10), 10);
    }
}
