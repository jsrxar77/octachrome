//! Native Instruments Maschine MK3 (`17cc:1600`).
//!
//! Protocol facts come from autotel/maschine-mk3-linux `docs/protocol.md` and
//! `docs/hardware-map.md` (read off real hardware). This is an independent
//! implementation; no code from that project or from Encdr is included.

pub mod protocol;

#[cfg(feature = "mk3")]
mod display;
#[cfg(feature = "mk3")]
mod usb;
#[cfg(feature = "mk3")]
pub use usb::{reset_usb, Mk3Surface};

pub const VENDOR_ID: u16 = 0x17cc;
pub const PRODUCT_ID: u16 = 0x1600;
pub const HID_INTERFACE: u8 = 4;
pub const DISPLAY_INTERFACE: u8 = 5;
pub const DISPLAY_ENDPOINT: u8 = 0x04;
