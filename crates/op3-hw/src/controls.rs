//! Input events, in controller-neutral terms.

/// Every button on the MK3, named after its silkscreen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    JogPush,
    JogUp,
    JogRight,
    JogDown,
    JogLeft,
    JogTouch,
    Shift,
    Screen(u8), // 1..=8, the buttons above the displays
    Group(u8),  // 0..=7 = A..H
    Notes,
    Volume,
    Swing,
    Tempo,
    NoteRepeat,
    Lock,
    PadMode,
    Keyboard,
    Chords,
    Step,
    FixedVel,
    Scene,
    Pattern,
    Events,
    Variation,
    Duplicate,
    Select,
    Solo,
    Mute,
    Pitch,
    Mod,
    Perform,
    Restart,
    Erase,
    Tap,
    Follow,
    Play,
    Rec,
    Stop,
    Macro,
    Settings,
    ArrowRight,
    Sampling,
    Mixer,
    PlugIn,
    ChannelMidi,
    Arranger,
    Browser,
    ArrowLeft,
    File,
    Auto,
}

/// A pad, numbered 0..16 from the bottom-left (silkscreen number − 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PadEvent {
    /// Finger resting (the MK3 reports this before or without a strike).
    Press,
    /// Struck, velocity 0..=1.
    Strike(f32),
    /// Pressure 0..=1 while held.
    Pressure(f32),
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlEvent {
    Button {
        button: Button,
        pressed: bool,
    },
    Pad {
        pad: u8,
        event: PadEvent,
    },
    /// Endless knob 0..8 (0 = leftmost). `delta` in device units, 1000/rev.
    Knob {
        index: u8,
        delta: i32,
    },
    /// Capacitive touch on knob 0..8.
    KnobTouch {
        index: u8,
        touched: bool,
    },
    /// The 4-D encoder's rotation, in detents.
    Encoder {
        delta: i32,
    },
    /// Smart Strip finger position 0..=1, `None` on release.
    Strip {
        position: Option<f32>,
    },
}
