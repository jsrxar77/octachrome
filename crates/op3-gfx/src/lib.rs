//! Pixel layer shared by the UI, the cartridges and the hardware drivers.
//!
//! * [`Rgb565`] — the native pixel format of both MK3 screens.
//! * [`FrameBuffer480x272`] — one screen's worth of pixels. It implements
//!   `embedded_graphics::DrawTarget`, so every primitive and font from
//!   `embedded-graphics` draws straight into it.
//! * [`DirtyTracker`] — compares a freshly drawn frame against the last one
//!   that was sent to the device and returns the small set of rectangles that
//!   actually changed. Sending only those is what keeps a 60 FPS UI cheap on a
//!   bus that only manages ~13 full frames per second per screen.
//!
//! Nothing in here knows about USB.

mod color;
mod dirty;
mod draw;
mod font;
mod framebuffer;
pub mod paint;
pub mod theme;

pub use color::Rgb565;
pub use dirty::{DirtyTracker, DirtyUpdate};
pub use draw::*;
pub use framebuffer::{FrameBuffer480x272, Rect, HEIGHT, WIDTH};
pub use theme::{knob, set_theme, theme, theme_id, Theme, ThemeId};

/// Re-export so cartridges can draw without adding their own dependency.
pub use embedded_graphics as eg;
