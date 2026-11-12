//! Dirty-rectangle extraction by diffing against the last frame sent.
//!
//! Diffing (rather than asking every drawing routine to report what it
//! touched) means the UI can simply redraw everything each frame and still
//! only push the pixels that changed. Comparing two 130k-pixel buffers is a
//! few tens of microseconds on a Pi 5; the USB transfer it saves is
//! milliseconds.
//!
//! The screen is cut into tiles of `TILE_W`×`TILE_H`. Changed tiles in one
//! tile-row are joined into horizontal runs, runs with the same span in
//! consecutive tile-rows are merged vertically, and if the result is too
//! fragmented or covers most of the screen a single full-screen rect is sent
//! instead. Every rect has an even pixel count, which the MK3 requires (its
//! length field counts 32-bit words, i.e. pixel pairs).

use crate::{FrameBuffer480x272, Rect, HEIGHT, WIDTH};

pub const TILE_W: usize = 32;
pub const TILE_H: usize = 16;
const TILES_X: usize = WIDTH / TILE_W; // 15
const TILES_Y: usize = HEIGHT / TILE_H; // 17

/// Above this many rects, per-transfer overhead outweighs the savings.
const MAX_RECTS: usize = 12;
/// Above this fraction of the screen, just send all of it.
const FULL_FRAME_RATIO: f32 = 0.6;

/// What to send for one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirtyUpdate {
    /// Nothing changed.
    None,
    /// Send these rectangles.
    Rects(Vec<Rect>),
}

impl DirtyUpdate {
    pub fn rects(&self) -> &[Rect] {
        match self {
            DirtyUpdate::None => &[],
            DirtyUpdate::Rects(r) => r,
        }
    }
    pub fn pixel_count(&self) -> u32 {
        self.rects().iter().map(Rect::area).sum()
    }
}

/// Remembers what the device is currently showing.
pub struct DirtyTracker {
    shown: FrameBuffer480x272,
    force_full: bool,
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DirtyTracker {
    /// The first call to [`DirtyTracker::diff`] always returns the full screen.
    pub fn new() -> Self {
        Self {
            shown: FrameBuffer480x272::new(),
            force_full: true,
        }
    }

    /// Force the next update to be a full frame (e.g. after a reconnect).
    pub fn invalidate(&mut self) {
        self.force_full = true;
    }

    /// Compare `next` with what is on screen, record `next` as shown, and
    /// return what has to be transferred.
    pub fn diff(&mut self, next: &FrameBuffer480x272) -> DirtyUpdate {
        if self.force_full {
            self.force_full = false;
            self.shown.pixels_mut().copy_from_slice(next.pixels());
            return DirtyUpdate::Rects(vec![Rect::FULL]);
        }

        let mut dirty = [[false; TILES_X]; TILES_Y];
        let mut any = false;
        let (a, b) = (self.shown.pixels(), next.pixels());
        for (ty, row) in dirty.iter_mut().enumerate() {
            for y in ty * TILE_H..(ty + 1) * TILE_H {
                let line = y * WIDTH;
                for (tx, cell) in row.iter_mut().enumerate() {
                    if *cell {
                        continue;
                    }
                    let s = line + tx * TILE_W;
                    if a[s..s + TILE_W] != b[s..s + TILE_W] {
                        *cell = true;
                        any = true;
                    }
                }
            }
        }
        if !any {
            return DirtyUpdate::None;
        }
        self.shown.pixels_mut().copy_from_slice(next.pixels());

        // Horizontal runs per tile row: (tx0, tx1_exclusive, ty0, ty1_exclusive)
        let mut rects: Vec<(usize, usize, usize, usize)> = Vec::new();
        for (ty, row) in dirty.iter().enumerate() {
            let mut tx = 0;
            while tx < TILES_X {
                if !row[tx] {
                    tx += 1;
                    continue;
                }
                let start = tx;
                while tx < TILES_X && row[tx] {
                    tx += 1;
                }
                // Merge with a rect directly above that has the same span.
                if let Some(r) = rects
                    .iter_mut()
                    .find(|r| r.0 == start && r.1 == tx && r.3 == ty)
                {
                    r.3 = ty + 1;
                } else {
                    rects.push((start, tx, ty, ty + 1));
                }
            }
        }

        let to_px = |r: &(usize, usize, usize, usize)| {
            Rect::new(
                (r.0 * TILE_W) as u16,
                (r.2 * TILE_H) as u16,
                ((r.1 - r.0) * TILE_W) as u16,
                ((r.3 - r.2) * TILE_H) as u16,
            )
        };
        let rects: Vec<Rect> = rects.iter().map(to_px).collect();
        let area: u32 = rects.iter().map(Rect::area).sum();
        if rects.len() > MAX_RECTS || area as f32 > FULL_FRAME_RATIO * Rect::FULL.area() as f32 {
            return DirtyUpdate::Rects(vec![Rect::FULL]);
        }
        DirtyUpdate::Rects(rects)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rgb565;

    #[test]
    fn first_frame_is_full_then_nothing() {
        let mut t = DirtyTracker::new();
        let fb = FrameBuffer480x272::new();
        assert_eq!(t.diff(&fb), DirtyUpdate::Rects(vec![Rect::FULL]));
        assert_eq!(t.diff(&fb), DirtyUpdate::None);
    }

    #[test]
    fn single_pixel_change_is_one_tile() {
        let mut t = DirtyTracker::new();
        let mut fb = FrameBuffer480x272::new();
        t.diff(&fb);
        fb.put(100, 40, Rgb565::WHITE);
        let u = t.diff(&fb);
        assert_eq!(u.rects(), &[Rect::new(96, 32, 32, 16)]);
        assert_eq!(t.diff(&fb), DirtyUpdate::None);
    }

    #[test]
    fn vertical_merge_of_aligned_runs() {
        let mut t = DirtyTracker::new();
        let mut fb = FrameBuffer480x272::new();
        t.diff(&fb);
        fb.fill_rect(0, 0, 64, 40, Rgb565::WHITE); // tiles x0..2, y0..3
        assert_eq!(t.diff(&fb).rects(), &[Rect::new(0, 0, 64, 48)]);
    }

    #[test]
    fn large_change_becomes_full_frame() {
        let mut t = DirtyTracker::new();
        let mut fb = FrameBuffer480x272::new();
        t.diff(&fb);
        fb.fill_rect(0, 0, 480, 200, Rgb565::WHITE);
        assert_eq!(t.diff(&fb).rects(), &[Rect::FULL]);
    }

    #[test]
    fn every_rect_has_even_pixel_count() {
        let mut t = DirtyTracker::new();
        let mut fb = FrameBuffer480x272::new();
        t.diff(&fb);
        for i in 0..10 {
            fb.put(i * 47, i * 26, Rgb565::WHITE);
        }
        for r in t.diff(&fb).rects() {
            assert_eq!(r.area() % 2, 0);
        }
    }

    #[test]
    fn small_change_is_much_cheaper_than_full_frame() {
        // The playhead moving one step on a 16-step grid.
        let mut t = DirtyTracker::new();
        let mut fb = FrameBuffer480x272::new();
        t.diff(&fb);
        fb.fill_rect(30, 120, 24, 4, Rgb565::WHITE);
        let px = t.diff(&fb).pixel_count();
        assert!(px * 20 < Rect::FULL.area(), "{px} pixels");
    }
}
