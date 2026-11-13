//! The OP-3 application: wires the engine, the cartridges, the UI and a
//! control surface together.
//!
//! The library half exists so the controller and the frame loop can be tested
//! with a mock surface; `src/main.rs` is a thin CLI on top.

pub mod app;
pub mod backoff;
pub mod bench;
#[cfg(feature = "audio")]
pub mod audio;
pub mod controller;
pub mod demo;
pub mod midi;
pub mod script;
pub mod settings;
pub mod wav;

#[doc(hidden)]
pub use op3_engine;

pub use app::{App, FrameStats};
pub use controller::Controller;
