//! Hardware layer.
//!
//! Everything above this crate talks to a [`ControlSurface`] and never to USB.
//! Two implementations ship:
//!
//! * [`mock::MockSurface`] — scripted input, recorded output; used by tests
//!   and by the desktop simulator.
//! * `mk3::Mk3Surface` (feature `mk3`) — a real Maschine MK3: HID interface 4
//!   for buttons/knobs/pads/LEDs, vendor bulk interface 5 for the screens.
//!
//! The byte-level protocol ([`mk3::protocol`]) is pure functions and is always
//! compiled and unit-tested, with or without the device.

pub mod controls;
pub mod leds;
pub mod mk3;
pub mod mock;

pub use controls::{Button, ControlEvent, PadEvent};
pub use leds::{LedFrame, NiColor};
pub use op8_gfx::{FrameBuffer480x272, Rect};

/// The two screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Screen {
    Left = 0,
    Right = 1,
}

#[derive(Debug)]
pub struct HwError(pub String);

impl std::fmt::Display for HwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for HwError {}

pub type HwResult<T> = Result<T, HwError>;

/// A controller with buttons, pads, knobs, LEDs and two 480×272 screens.
pub trait ControlSurface {
    fn name(&self) -> &str;
    /// Non-blocking: append whatever input arrived since the last call.
    fn poll(&mut self, out: &mut Vec<ControlEvent>) -> HwResult<()>;
    /// Send the full LED state (implementations may skip unchanged banks).
    fn write_leds(&mut self, leds: &LedFrame) -> HwResult<()>;
    /// Send the given rectangles of `fb` to `screen`.
    fn write_screen(
        &mut self,
        screen: Screen,
        fb: &FrameBuffer480x272,
        rects: &[Rect],
    ) -> HwResult<()>;
}
