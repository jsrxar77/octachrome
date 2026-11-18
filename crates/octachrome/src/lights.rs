//! The light of every button, in one table (docs/design.md §2.6).
//!
//! A button that does nothing in the current context is dark. One that does something is a very
//! faint white. One that is pressed, held, or that has the focus (the mode that is active, the page
//! that is open) is a strong white. The buttons whose LED has a colour (the groups, PLAY) keep it,
//! with the same three states.

use octachrome_hw::Button;
use octachrome_ui::{screen_buttons, Main, Mode, UiState};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    /// Nothing to do here.
    Off,
    /// Something to do here: a very faint white.
    Dim,
    /// Pressed, held, or focused: a strong white.
    Full,
}

/// What the lights depend on besides the screen state.
pub struct Context<'a> {
    pub playing: bool,
    pub shift: bool,
    pub mute: bool,
    pub erase: bool,
    /// Buttons physically down right now.
    pub down: &'a HashSet<Button>,
}

/// Every button that can have a function. Buttons not listed here have none yet, and stay dark.
pub fn all() -> Vec<Button> {
    use Button::*;
    let mut v = vec![
        Shift, Rec, Tap, Tempo, Swing, Volume, FixedVel, Step, Chords, Perform, PadMode, Keyboard, NoteRepeat, ChannelMidi, Settings, File, PlugIn, Mixer, Browser, Sampling, Pattern, Scene, Arranger, Mute, Erase, Play, Stop, Restart, Select, JogLeft,
        JogRight, JogUp, JogDown,
    ];
    v.extend((1..=8).map(Screen));
    v
}

/// The light of `b` now. (Groups A–H are lit by track, with their own rule, in `Controller::leds`.)
pub fn light(ui: &UiState, c: &Context, b: Button) -> Light {
    use Button::*;
    let composing = ui.main == Main::Instrument;
    let nav = !octachrome_ui::nav_items(ui).is_empty();
    let state = |on: bool| if on { Light::Full } else { Light::Dim };
    let l = match b {
        Shift => state(c.shift),
        Play => state(c.playing),
        // STOP has the focus while stopped, PLAY while playing
        Stop => state(!c.playing),
        Restart => Light::Dim,
        // REC arms the recording: strong while armed, and only while composing
        Tap => if composing { state(ui.metronome) } else if ui.main == Main::Sampling { state(ui.smp.count_in) } else { Light::Off },
        Tempo | Swing | Volume => if composing { Light::Dim } else { Light::Off },
        FixedVel => if composing { state(ui.fixed_vel) } else { Light::Off },
        Rec => if composing { state(ui.rec) } else if ui.main == Main::Sampling { state(ui.smp.recording || ui.smp.armed || ui.smp.counting > 0) } else { Light::Off },
        // Going back to composing, or into settings
        ChannelMidi => state(composing),
        Settings => state(ui.main == Main::Settings),
        Mixer => state(ui.main == Main::Mixer),
        File => state(ui.main == Main::File),
        PlugIn => state(ui.main == Main::Plugin),
        Pattern => state(ui.main == Main::Pattern),
        Scene => state(ui.main == Main::Scene),
        Arranger => state(ui.main == Main::Arranger),
        Sampling => state(ui.main == Main::Sampling),
        Browser => state(ui.main == Main::Browser),
        // The pad-mode buttons all work from settings too (they go back to composing).
        Step => state(composing && ui.mode == Mode::Step),
        Chords => state(composing && ui.mode == Mode::Brain),
        Perform | NoteRepeat => state(composing && ui.mode == Mode::Punch),
        PadMode => state(composing && ui.mode == Mode::Pads),
        Keyboard => state(composing && ui.mode == Mode::Keys),
        // Only while composing: nothing happens in settings
        Mute => if composing { state(c.mute) } else { Light::Off },
        Erase => if composing { state(c.erase) } else { Light::Off },
        // The 4D encoder's sides: the theme in settings, the octave on the keyboard
        // (the same contexts as `Controller::button`: the octave on the keyboard, the theme in settings,
        // the cursor of a name, and whatever the 4D can move through on the pads)
        JogLeft | JogRight => {
            if ui.main == Main::Settings || ui.main == Main::Name || (composing && ui.mode == Mode::Keys && ui.lane.is_none()) || nav {
                Light::Dim
            } else {
                Light::Off
            }
        }
        JogUp | JogDown => if nav { Light::Dim } else { Light::Off },
        // (the push of the 4D has no LED on the MK3)
        // SELECT held on a pad names it: where there are things to name
        Select => if matches!(ui.main, Main::File | Main::Browser | Main::Sampling) { Light::Dim } else { Light::Off },
        Screen(n) => match screen_buttons(ui).get(n as usize - 1).copied().flatten() {
            None => Light::Off,
            Some((_, true)) => Light::Full,
            Some((_, false)) => Light::Dim,
        },
        _ => Light::Off,
    };
    // Whatever is physically held has the focus, if it has a function at all.
    if l != Light::Off && c.down.contains(&b) {
        Light::Full
    } else {
        l
    }
}
