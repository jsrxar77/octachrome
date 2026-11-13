//! Text scripts that play the part of a hand on the MK3.
//!
//! One action per line, `<seconds> <command> …`; `#` starts a comment. Lines
//! at the same time run in file order.
//!
//! ```text
//! 0.1  btn play
//! 1.0  btn group b            # A..H
//! 1.0  touch 5 on             # knobs are 0..7, left to right
//! 1.0  knob 5 150             # turn by 150 units (1000 per turn)
//! 2.0  pad 14 strike 0.7      # pads are 0..15; strike [vel] | press | pressure v | release
//! 3.0  btn screen 6           # the buttons above the displays, 1..8
//! 3.5  encoder -2
//! 4.0  strip 0.25             # Smart Strip position, or `strip off`
//! 5.0  shot macro             # sim only: save both screens as macro-left/right.ppm
//! ```
//!
//! `btn NAME` presses and releases; `btn NAME down` / `btn NAME up` hold it
//! (for combos like `mute` + `group a`).

use anyhow::{bail, Context, Result};
use op3_hw::{Button, ControlEvent, PadEvent};

#[derive(Clone, Debug)]
pub enum Action {
    Event(ControlEvent),
    /// Save both screens (only meaningful in `sim`).
    Shot(String),
}

#[derive(Default)]
pub struct Script {
    actions: Vec<(u64, Action)>,
    next: usize,
}

impl Script {
    pub fn parse(text: &str) -> Result<Self> {
        let mut actions = Vec::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let t: Vec<&str> = line.split_whitespace().collect();
            let at: f32 = t[0]
                .parse()
                .with_context(|| format!("line {}: expected seconds, got {:?}", n + 1, t[0]))?;
            let ms = (at * 1000.0).round() as u64;
            let parsed = parse_command(&t[1..]).with_context(|| format!("line {}: {line}", n + 1))?;
            actions.extend(parsed.into_iter().map(|a| (ms, a)));
        }
        actions.sort_by_key(|(ms, _)| *ms); // stable: file order within a time
        Ok(Self { actions, next: 0 })
    }

    /// Actions whose time has come, in order.
    pub fn due(&mut self, now_ms: u64) -> Vec<Action> {
        let mut out = Vec::new();
        while let Some((ms, a)) = self.actions.get(self.next) {
            if *ms > now_ms {
                break;
            }
            out.push(a.clone());
            self.next += 1;
        }
        out
    }

    pub fn is_done(&self) -> bool {
        self.next >= self.actions.len()
    }

    /// Time of the last action.
    pub fn end_ms(&self) -> u64 {
        self.actions.last().map_or(0, |(ms, _)| *ms)
    }
}

fn parse_command(t: &[&str]) -> Result<Vec<Action>> {
    let Some(&cmd) = t.first() else {
        bail!("missing command");
    };
    let arg = |i: usize| -> Result<&str> {
        t.get(i).copied().with_context(|| format!("{cmd}: missing argument {i}"))
    };
    let num = |i: usize| -> Result<f32> {
        arg(i)?.parse().with_context(|| format!("{cmd}: {:?} is not a number", t[i]))
    };
    let ev = |e| Ok(vec![Action::Event(e)]);
    match cmd {
        "btn" => {
            let (button, used) = parse_button(&t[1..])?;
            let button_event = |pressed| Action::Event(ControlEvent::Button { button, pressed });
            match t.get(1 + used).copied() {
                None => Ok(vec![button_event(true), button_event(false)]),
                Some("down") => Ok(vec![button_event(true)]),
                Some("up") => Ok(vec![button_event(false)]),
                Some(other) => bail!("btn: expected down or up, got {other:?}"),
            }
        }
        "pad" => {
            let pad = arg(1)? .parse::<u8>().context("pad: number 0..15")?;
            if pad > 15 {
                bail!("pad: {pad} is not in 0..15");
            }
            let event = match arg(2)? {
                "strike" => PadEvent::Strike(if t.len() > 3 { num(3)? } else { 0.8 }),
                "press" => PadEvent::Press,
                "pressure" => PadEvent::Pressure(num(3)?),
                "release" => PadEvent::Release,
                other => bail!("pad: unknown action {other:?} (strike, press, pressure, release)"),
            };
            ev(ControlEvent::Pad { pad, event })
        }
        "knob" => {
            let index = arg(1)?.parse::<u8>().context("knob: number 0..7")?;
            if index > 7 {
                bail!("knob: {index} is not in 0..7");
            }
            ev(ControlEvent::Knob { index, delta: num(2)? as i32 })
        }
        "touch" => {
            let index = arg(1)?.parse::<u8>().context("touch: number 0..7")?;
            if index > 7 {
                bail!("touch: {index} is not in 0..7");
            }
            let touched = match arg(2)? {
                "on" => true,
                "off" => false,
                other => bail!("touch: expected on or off, got {other:?}"),
            };
            ev(ControlEvent::KnobTouch { index, touched })
        }
        "encoder" => ev(ControlEvent::Encoder { delta: num(1)? as i32 }),
        "strip" => match arg(1)? {
            "off" => ev(ControlEvent::Strip { position: None }),
            _ => ev(ControlEvent::Strip { position: Some(num(1)?.clamp(0.0, 1.0)) }),
        },
        "shot" => Ok(vec![Action::Shot(arg(1)?.to_string())]),
        other => bail!("unknown command {other:?} (btn, pad, knob, touch, encoder, strip, shot)"),
    }
}

/// Returns the button and how many tokens it used.
fn parse_button(t: &[&str]) -> Result<(Button, usize)> {
    let Some(&name) = t.first() else {
        bail!("btn: missing button name");
    };
    match name {
        "group" => {
            let l = t.get(1).context("btn group: missing letter a..h")?;
            let g = match l.as_bytes() {
                [c @ b'a'..=b'h'] => c - b'a',
                _ => bail!("btn group: expected a..h, got {l:?}"),
            };
            Ok((Button::Group(g), 2))
        }
        "screen" => {
            let n: u8 = t.get(1).context("btn screen: missing 1..8")?.parse().context("btn screen: 1..8")?;
            if !(1..=8).contains(&n) {
                bail!("btn screen: {n} is not in 1..8");
            }
            Ok((Button::Screen(n), 2))
        }
        _ => BUTTONS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, b)| (*b, 1))
            .with_context(|| format!("btn: unknown button {name:?}")),
    }
}

/// Names follow the silkscreen of the MK3.
const BUTTONS: &[(&str, Button)] = &[
    ("play", Button::Play),
    ("stop", Button::Stop),
    ("rec", Button::Rec),
    ("restart", Button::Restart),
    ("tap", Button::Tap),
    ("follow", Button::Follow),
    ("shift", Button::Shift),
    ("step", Button::Step),
    ("chords", Button::Chords),
    ("perform", Button::Perform),
    ("noterepeat", Button::NoteRepeat),
    ("keyboard", Button::Keyboard),
    ("padmode", Button::PadMode),
    ("fixedvel", Button::FixedVel),
    ("mute", Button::Mute),
    ("solo", Button::Solo),
    ("erase", Button::Erase),
    ("select", Button::Select),
    ("duplicate", Button::Duplicate),
    ("variation", Button::Variation),
    ("events", Button::Events),
    ("lock", Button::Lock),
    ("pattern", Button::Pattern),
    ("scene", Button::Scene),
    ("tempo", Button::Tempo),
    ("swing", Button::Swing),
    ("volume", Button::Volume),
    ("notes", Button::Notes),
    ("pitch", Button::Pitch),
    ("mod", Button::Mod),
    ("macro", Button::Macro),
    ("settings", Button::Settings),
    ("auto", Button::Auto),
    ("file", Button::File),
    ("browser", Button::Browser),
    ("arranger", Button::Arranger),
    ("mixer", Button::Mixer),
    ("sampling", Button::Sampling),
    ("plugin", Button::PlugIn),
    ("channel", Button::ChannelMidi),
    ("left", Button::ArrowLeft),
    ("right", Button::ArrowRight),
    ("jogpush", Button::JogPush),
    ("jogup", Button::JogUp),
    ("jogdown", Button::JogDown),
    ("jogleft", Button::JogLeft),
    ("jogright", Button::JogRight),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_orders_actions() {
        let mut s = Script::parse(
            "2.0 pad 3 strike 0.5\n# comment\n\n1.0 btn group c   # inline\n1.0 knob 5 -40\n3.0 shot end\n",
        )
        .unwrap();
        assert_eq!(s.end_ms(), 3000);
        assert!(s.due(999).is_empty());
        let a = s.due(1000);
        assert_eq!(a.len(), 3); // press + release + knob
        assert!(matches!(
            a[0],
            Action::Event(ControlEvent::Button { button: Button::Group(2), pressed: true })
        ));
        assert!(matches!(a[2], Action::Event(ControlEvent::Knob { index: 5, delta: -40 })));
        let b = s.due(2500);
        assert!(matches!(
            b[0],
            Action::Event(ControlEvent::Pad { pad: 3, event: PadEvent::Strike(v) }) if v == 0.5
        ));
        assert!(!s.is_done());
        assert!(matches!(&s.due(10_000)[0], Action::Shot(n) if n == "end"));
        assert!(s.is_done());
    }

    #[test]
    fn hold_and_screen_buttons() {
        let mut s = Script::parse("0 btn mute down\n0 btn group a\n0.5 btn mute up\n0 btn screen 6\n").unwrap();
        let a = s.due(0);
        assert_eq!(a.len(), 5); // mute down, group a x2, screen 6 x2
        assert!(matches!(
            a[0],
            Action::Event(ControlEvent::Button { button: Button::Mute, pressed: true })
        ));
        assert_eq!(s.due(500).len(), 1);
    }

    #[test]
    fn rejects_mistakes_with_the_line_number() {
        for bad in [
            "x btn play",
            "1 btn nonsense",
            "1 pad 16 strike",
            "1 knob 8 1",
            "1 btn group i",
            "1 btn screen 9",
            "1 dance",
            "1 touch 1 maybe",
        ] {
            let e = Script::parse(bad).err().unwrap_or_else(|| panic!("{bad:?} parsed"));
            assert!(format!("{e:#}").contains("line 1"), "{bad:?}: {e:#}");
        }
    }

    #[test]
    fn the_demo_script_parses() {
        let s = Script::parse(include_str!("../scripts/demo.op3")).unwrap();
        assert!(s.end_ms() > 6000);
    }
}
