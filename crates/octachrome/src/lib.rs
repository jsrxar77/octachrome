//! The Octachrome application: wires the engine, the cartridges, the UI and a
//! control surface together.
//!
//! The library half exists so the controller and the frame loop can be tested
//! with a mock surface; `src/main.rs` is a thin CLI on top.

pub mod app;
pub mod backoff;
pub mod bench;
#[cfg(feature = "audio")]
pub mod audio;
pub mod clockin;
pub mod controller;
pub mod demo;
pub mod factory;
pub mod lights;
pub mod limits;
pub mod midi;
pub mod projectfile;
pub mod script;
pub mod settings;
pub mod wav;

#[doc(hidden)]
pub use octachrome_engine;

pub use app::{App, FrameStats};

/// True in the demo build (`--features demo`): everything plays and can be edited, but nothing is saved (projects, presets, recordings of
/// SAMPLING) and the screens carry the word "demo".
pub const DEMO_BUILD: bool = cfg!(feature = "demo");

/// What a refused save says on the screen.
pub const DEMO_NOTE: &str = "demo: saving is in the full version";

/// The word "demo" in a corner of a screen, in the build that is one.
pub fn demo_mark(fb: &mut octachrome_gfx::FrameBuffer480x272) {
    if DEMO_BUILD {
        let t = octachrome_gfx::theme();
        octachrome_gfx::text(fb, "demo", 190, 31, octachrome_gfx::FontSize::Small, t.dim, octachrome_gfx::Align::Center);
    }
}
pub use controller::Controller;
