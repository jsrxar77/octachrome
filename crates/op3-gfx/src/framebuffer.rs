use crate::Rgb565;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor, Pixel,
};

pub const WIDTH: usize = 480;
pub const HEIGHT: usize = 272;

/// An axis-aligned rectangle in screen pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub const FULL: Rect = Rect {
        x: 0,
        y: 0,
        w: WIDTH as u16,
        h: HEIGHT as u16,
    };

    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }
    pub const fn area(&self) -> u32 {
        self.w as u32 * self.h as u32
    }
    pub const fn right(&self) -> u16 {
        self.x + self.w
    }
    pub const fn bottom(&self) -> u16 {
        self.y + self.h
    }
}

/// One MK3 screen: 480×272 pixels of [`Rgb565`], row-major.
///
/// Heap-allocated once (≈255 KB) and reused for the lifetime of the app.
#[derive(Clone)]
pub struct FrameBuffer480x272 {
    pixels: Box<[u16]>,
}

impl Default for FrameBuffer480x272 {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FrameBuffer480x272 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameBuffer480x272").finish_non_exhaustive()
    }
}

impl FrameBuffer480x272 {
    pub const WIDTH: usize = WIDTH;
    pub const HEIGHT: usize = HEIGHT;

    pub fn new() -> Self {
        Self {
            pixels: vec![0u16; WIDTH * HEIGHT].into_boxed_slice(),
        }
    }

    pub fn pixels(&self) -> &[u16] {
        &self.pixels
    }
    pub fn pixels_mut(&mut self) -> &mut [u16] {
        &mut self.pixels
    }

    pub fn clear(&mut self, c: Rgb565) {
        self.pixels.fill(c.0);
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Rgb565 {
        Rgb565(self.pixels[y * WIDTH + x])
    }

    /// Set one pixel; silently ignores coordinates off-screen.
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, c: Rgb565) {
        if x >= 0 && y >= 0 && (x as usize) < WIDTH && (y as usize) < HEIGHT {
            self.pixels[y as usize * WIDTH + x as usize] = c.0;
        }
    }

    /// Fill a rectangle, clipped to the screen.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb565) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = ((x + w).max(0) as usize).min(WIDTH);
        let y1 = ((y + h).max(0) as usize).min(HEIGHT);
        if x0 >= x1 {
            return;
        }
        for row in y0..y1 {
            self.pixels[row * WIDTH + x0..row * WIDTH + x1].fill(c.0);
        }
    }

    /// Copy out a rectangle, row-major, for a partial transfer.
    pub fn copy_rect(&self, r: Rect, out: &mut Vec<u16>) {
        out.clear();
        for row in r.y as usize..r.bottom() as usize {
            let s = row * WIDTH + r.x as usize;
            out.extend_from_slice(&self.pixels[s..s + r.w as usize]);
        }
    }

    /// Binary PPM (P6) dump, used by the simulator for screenshots.
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
        out.reserve(WIDTH * HEIGHT * 3);
        for &p in self.pixels.iter() {
            let (r, g, b) = Rgb565(p).to_rgb888();
            out.extend_from_slice(&[r, g, b]);
        }
        out
    }
}

impl OriginDimensions for FrameBuffer480x272 {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for FrameBuffer480x272 {
    type Color = pixelcolor::Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(p, c) in pixels {
            self.put(p.x, p.y, c.into());
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        FrameBuffer480x272::clear(self, color.into());
        Ok(())
    }
}
