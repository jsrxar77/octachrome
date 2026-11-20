//! What the demo build lets through (`--features demo`, docs/architecture.md "Demo"): choosing and listening to a song, PLAY and STOP, and
//! turning the knobs of the instrument of a track. Everything else does nothing, and its light is dark (the rule of the lights: a button
//! is lit if and only if it has a function in the context).

use octachrome_hw::{Button, ControlEvent};
use octachrome_ui::{Main, UiState};

/// Does the demo build give `b` a function in the current context?
pub fn button(ui: &UiState, b: Button) -> bool {
    match b {
        // the transport, FILE (to choose a song), CHANNEL (to come back to the instrument) and the tracks A-H
        Button::Play | Button::Stop | Button::File | Button::ChannelMidi | Button::Group(_) => true,
        // the pages of the instrument (sound, shape, fx, mod: the screen buttons 5 to 8), only while composing
        Button::Screen(n) => ui.main == Main::Instrument && n >= 5,
        _ => false,
    }
}

/// Does the demo build let this event through?
pub fn allows(ui: &UiState, ev: &ControlEvent) -> bool {
    match ev {
        ControlEvent::Button { button, .. } => button_event(ui, *button),
        // the pads only choose a song (in FILE)
        ControlEvent::Pad { .. } => ui.main == Main::File,
        // the knobs under the right screen: the instrument
        ControlEvent::Knob { index, .. } | ControlEvent::KnobTouch { index, .. } => ui.main == Main::Instrument && *index >= 4,
        // the 4D encoder and the Smart Strip
        _ => false,
    }
}

fn button_event(ui: &UiState, b: Button) -> bool {
    button(ui, b)
}
