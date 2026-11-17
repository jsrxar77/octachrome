//! The real-time engine.
//!
//! ```text
//!  control thread                         audio thread (cpal callback)
//!  ──────────────                         ────────────────────────────
//!  EngineHandle ── EngineCommand ──rtrb──▶ Engine::render()
//!   └ mirror Project (for the UI)           ├ Sequencer::process  → events
//!                                           ├ cartridge.process   × 8 tracks
//!  EngineStatus ◀──────── atomics ─────────  ├ mix + Punch-in FX + master filter
//!                                           └ publish playheads, chord, levels
//! ```
//!
//! The project lives in two copies — one in the engine, one mirrored in the
//! handle for drawing — kept identical by applying the same `Command`s to
//! both. Nothing on the audio path allocates or takes a contended lock.

pub mod comp;
mod engine;
pub mod fx;
mod status;
mod track_fx;

pub use engine::{Engine, EngineCommand, EngineHandle, RecSource, SharedCartridge, SyncMessage, MAX_RECORD};
pub use fx::{PunchFx, PUNCH_SLOTS};
pub use status::EngineStatus;

pub use op8_seq as seq;
