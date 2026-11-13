//! Byte-level MK3 protocol: pure functions, no I/O.

use crate::{
    controls::{Button, ControlEvent, PadEvent},
    leds::{hid_to_pad, LedFrame, LED_SLOTS},
    FrameBuffer480x272, Rect, Screen,
};

pub const CONTROLS_REPORT: u8 = 0x01;
pub const PADS_REPORT: u8 = 0x02;
pub const LED_REPORT_0: u8 = 0x80;
pub const LED_REPORT_1: u8 = 0x81;
pub const LED_BANK_0: usize = 62;
pub const LED_BANK_1: usize = 41;

const KNOB_MODULUS: i32 = 1000;
const STRIP_MAX: f32 = 1024.0;
/// Hard strikes land around 1950 of 4095 (measured), so scale against 2000.
const VELOCITY_MAX: f32 = 2000.0;

/// Button for each bit of the 80-bit field (`None` = unused or a touch sensor
/// handled separately).
pub fn button_for_bit(bit: usize) -> Option<Button> {
    use Button::*;
    Some(match bit {
        0 => JogPush,
        2 => JogUp,
        3 => JogRight,
        4 => JogDown,
        5 => JogLeft,
        6 => Shift,
        7 => Screen(8),
        8..=15 => Group((bit - 8) as u8),
        16 => Notes,
        17 => Volume,
        18 => Swing,
        19 => Tempo,
        20 => NoteRepeat,
        21 => Lock,
        24 => PadMode,
        25 => Keyboard,
        26 => Chords,
        27 => Step,
        28 => FixedVel,
        29 => Scene,
        30 => Pattern,
        31 => Events,
        33 => Variation,
        34 => Duplicate,
        35 => Select,
        36 => Solo,
        37 => Mute,
        38 => Pitch,
        39 => Mod,
        40 => Perform,
        41 => Restart,
        42 => Erase,
        43 => Tap,
        44 => Follow,
        45 => Play,
        46 => Rec,
        47 => Stop,
        48 => Macro,
        49 => Settings,
        50 => ArrowRight,
        51 => Sampling,
        52 => Mixer,
        53 => PlugIn,
        56 => ChannelMidi,
        57 => Arranger,
        58 => Browser,
        59 => ArrowLeft,
        60 => File,
        61 => Auto,
        64..=70 => Screen((bit - 63) as u8),
        71 => JogTouch,
        _ => return None,
    })
}

/// Decoded input report `0x01`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlsState {
    pub buttons: [u8; 10],
    pub encoder: u8,
    pub knobs: [u16; 8],
    pub strip: u16,
}

impl ControlsState {
    pub fn bit(&self, i: usize) -> bool {
        self.buttons[i / 8] & (1 << (i % 8)) != 0
    }

    /// `report` includes the report id.
    pub fn parse(report: &[u8]) -> Option<Self> {
        if report.first() != Some(&CONTROLS_REPORT) || report.len() < 42 {
            return None;
        }
        let p = &report[1..];
        let u16le = |o: usize| u16::from_le_bytes([p[o], p[o + 1]]);
        let mut s = ControlsState::default();
        s.buttons.copy_from_slice(&p[0..10]);
        s.encoder = p[10] & 0x0f;
        for (k, v) in s.knobs.iter_mut().enumerate() {
            *v = u16le(11 + k * 2);
        }
        // Analog field 1 (offset 29) is the strip; field 0 is a free-running
        // counter and must be ignored.
        s.strip = u16le(29);
        Some(s)
    }
}

/// Shortest signed distance between two readings of a wrapping counter.
pub fn wrap_delta(prev: i32, now: i32, modulus: i32) -> i32 {
    let d = (now - prev).rem_euclid(modulus);
    if d > modulus / 2 {
        d - modulus
    } else {
        d
    }
}

/// Emit events for everything that changed between two control reports.
pub fn diff_controls(prev: &ControlsState, now: &ControlsState, out: &mut Vec<ControlEvent>) {
    for bit in 0..80 {
        let (a, b) = (prev.bit(bit), now.bit(bit));
        if a == b {
            continue;
        }
        if (72..80).contains(&bit) {
            // Touch sensors run backwards: bit 72 = knob 8, bit 79 = knob 1.
            out.push(ControlEvent::KnobTouch {
                index: (79 - bit) as u8,
                touched: b,
            });
        } else if let Some(button) = button_for_bit(bit) {
            out.push(ControlEvent::Button { button, pressed: b });
        }
    }
    let d = wrap_delta(prev.encoder as i32, now.encoder as i32, 16);
    if d != 0 {
        out.push(ControlEvent::Encoder { delta: d });
    }
    for k in 0..8 {
        let d = wrap_delta(prev.knobs[k] as i32, now.knobs[k] as i32, KNOB_MODULUS);
        if d != 0 {
            out.push(ControlEvent::Knob {
                index: k as u8,
                delta: d,
            });
        }
    }
    if prev.strip != now.strip {
        let position = (now.strip != 0).then(|| (now.strip as f32 / STRIP_MAX).clamp(0.0, 1.0));
        out.push(ControlEvent::Strip { position });
    }
}

/// Per-pad state to clean up the device's event ordering quirks.
#[derive(Clone, Copy, Debug, Default)]
pub struct PadTracker {
    down: [bool; 16],
}

impl PadTracker {
    /// Decode input report `0x02` (`report` includes the report id).
    ///
    /// * `Press`/`Strike` both mean "down"; `Press` is only sent once.
    /// * Pressure is dropped for pads that are not down (it arrives early).
    /// * `Release` is sent once, on whichever of press-off/note-off comes first.
    pub fn parse(&mut self, report: &[u8], out: &mut Vec<ControlEvent>) {
        if report.first() != Some(&PADS_REPORT) {
            return;
        }
        for (i, slot) in report[1..].chunks_exact(3).enumerate() {
            if i > 0 && slot == [0, 0, 0] {
                break;
            }
            let hid = slot[0];
            let Some(pad) = hid_to_pad(hid) else { continue };
            let value = (((slot[1] & 0x0f) as u16) << 8) | slot[2] as u16;
            let p = pad as usize;
            let event = match slot[1] & 0xf0 {
                0x00 if !self.down[p] => {
                    self.down[p] = true;
                    PadEvent::Press
                }
                0x10 => {
                    self.down[p] = true;
                    PadEvent::Strike((value as f32 / VELOCITY_MAX).clamp(0.05, 1.0))
                }
                0x20 | 0x30 if self.down[p] => {
                    self.down[p] = false;
                    PadEvent::Release
                }
                0x40 if self.down[p] && value > 0 => PadEvent::Pressure(soft_pressure(value)),
                _ => continue,
            };
            out.push(ControlEvent::Pad { pad, event });
        }
    }
}

/// Two-thirds of a real press sits below 500 of 4095; a square-root curve
/// spreads that range across the useful part of 0..1.
fn soft_pressure(v: u16) -> f32 {
    (v as f32 / 4095.0).sqrt().clamp(0.0, 1.0)
}

/// The two LED output reports, with report ids.
pub fn encode_leds(leds: &LedFrame) -> ([u8; LED_BANK_0 + 1], [u8; LED_BANK_1 + 1]) {
    let mut a = [0u8; LED_BANK_0 + 1];
    let mut b = [0u8; LED_BANK_1 + 1];
    a[0] = LED_REPORT_0;
    b[0] = LED_REPORT_1;
    a[1..].copy_from_slice(&leds.slots[..LED_BANK_0]);
    b[1..].copy_from_slice(&leds.slots[LED_BANK_0..LED_SLOTS]);
    for v in a[1..].iter_mut().chain(b[1..].iter_mut()) {
        *v &= 0x7f;
    }
    (a, b)
}

/// Build one bulk transfer that draws `rect` of `fb` on `screen`:
///
/// ```text
/// 84 00 SS 60 00 00 00 00 | x y w h (BE u16) | 02 00 00 00 | len (BE u32, pixel pairs)
/// <pixels RGB565 BE> | 02 00 00 00 | 03 00 00 00 (blit) | 40 00 00 00 (end)
/// ```
///
/// `rect.w * rect.h` must be even (the length counts pixel pairs); odd rects
/// are widened by one column.
pub fn encode_screen(screen: Screen, fb: &FrameBuffer480x272, rect: Rect, out: &mut Vec<u8>) {
    let mut r = rect;
    if r.area() % 2 == 1 {
        if r.right() < FrameBuffer480x272::WIDTH as u16 {
            r.w += 1;
        } else {
            r.x -= 1;
            r.w += 1;
        }
    }
    out.clear();
    out.reserve(32 + r.area() as usize * 2);
    out.extend_from_slice(&[0x84, 0x00, screen as u8, 0x60, 0, 0, 0, 0]);
    for v in [r.x, r.y, r.w, r.h] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    out.extend_from_slice(&[0x02, 0, 0, 0]);
    out.extend_from_slice(&(r.area() / 2).to_be_bytes());
    let px = fb.pixels();
    for y in r.y as usize..r.bottom() as usize {
        let row = &px[y * FrameBuffer480x272::WIDTH + r.x as usize
            ..y * FrameBuffer480x272::WIDTH + r.right() as usize];
        for p in row {
            out.extend_from_slice(&p.to_be_bytes());
        }
    }
    out.extend_from_slice(&[0x02, 0, 0, 0, 0x03, 0, 0, 0, 0x40, 0, 0, 0]);
}

/// Display feature report (`0xf8` left, `0xf9` right) setting backlight.
pub fn display_feature(screen: Screen, brightness: u8, contrast: u8) -> [u8; 11] {
    let id = match screen {
        Screen::Left => 0xf8,
        Screen::Right => 0xf9,
    };
    let [w0, w1] = 480u16.to_le_bytes();
    let [h0, h1] = 272u16.to_le_bytes();
    [
        id,
        w0,
        w1,
        h0,
        h1,
        0x10,
        0x60,
        0x01,
        brightness.min(100),
        contrast.min(100),
        0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leds::NiColor;
    use op3_gfx::Rgb565;

    fn controls(f: impl FnOnce(&mut [u8; 42])) -> ControlsState {
        let mut r = [0u8; 42];
        r[0] = CONTROLS_REPORT;
        f(&mut r);
        ControlsState::parse(&r).unwrap()
    }

    #[test]
    fn buttons_and_touch_sensors() {
        let a = controls(|_| {});
        let b = controls(|r| {
            r[1 + 5] |= 1 << 5; // play
            r[1 + 1] |= 1 << 2; // group C
            r[1 + 9] |= 1 << 7; // bit 79 = knob 1 touch
        });
        let mut ev = Vec::new();
        diff_controls(&a, &b, &mut ev);
        assert!(ev.contains(&ControlEvent::Button {
            button: Button::Play,
            pressed: true
        }));
        assert!(ev.contains(&ControlEvent::Button {
            button: Button::Group(2),
            pressed: true
        }));
        assert!(ev.contains(&ControlEvent::KnobTouch {
            index: 0,
            touched: true
        }));
        assert_eq!(ev.len(), 3);
    }

    #[test]
    fn knobs_wrap_the_short_way() {
        assert_eq!(wrap_delta(998, 3, 1000), 5);
        assert_eq!(wrap_delta(3, 998, 1000), -5);
        assert_eq!(wrap_delta(15, 0, 16), 1);
        let a = controls(|r| r[1 + 11..1 + 13].copy_from_slice(&995u16.to_le_bytes()));
        let b = controls(|r| r[1 + 11..1 + 13].copy_from_slice(&2u16.to_le_bytes()));
        let mut ev = Vec::new();
        diff_controls(&a, &b, &mut ev);
        assert_eq!(ev, [ControlEvent::Knob { index: 0, delta: 7 }]);
    }

    #[test]
    fn strip_ignores_the_free_running_counter() {
        let a = controls(|r| r[1 + 27] = 10);
        let b = controls(|r| r[1 + 27] = 26);
        let mut ev = Vec::new();
        diff_controls(&a, &b, &mut ev);
        assert!(ev.is_empty());
        let c = controls(|r| r[1 + 29..1 + 31].copy_from_slice(&512u16.to_le_bytes()));
        diff_controls(&a, &c, &mut ev);
        assert_eq!(
            ev,
            [ControlEvent::Strip {
                position: Some(0.5)
            }]
        );
    }

    #[test]
    fn pad_quirks_are_cleaned_up() {
        let mut t = PadTracker::default();
        let mut ev = Vec::new();
        // aftertouch before press (dropped), press-on, note-on, aftertouch,
        // note-off, press-off (dropped), trailing aftertouch 0 (dropped)
        let report = [
            PADS_REPORT,
            0,
            0x40,
            0x20, // HID pad 0, aftertouch
            0,
            0x00,
            0x10, // press on
            0,
            0x17,
            0xd0, // note on, velocity 0x7d0 = 2000
            0,
            0x42,
            0x00, // aftertouch 0x200
            0,
            0x30,
            0x00, // note off
            0,
            0x20,
            0x00, // press off
            0,
            0x40,
            0x00, // aftertouch 0
            0,
            0,
            0,
            5,
            0x10,
            0x10, // padding stops decoding
        ];
        t.parse(&report, &mut ev);
        let pad = 12; // HID 0 = top-left = silkscreen 13
        assert_eq!(
            ev,
            [
                ControlEvent::Pad {
                    pad,
                    event: PadEvent::Press
                },
                ControlEvent::Pad {
                    pad,
                    event: PadEvent::Strike(1.0)
                },
                ControlEvent::Pad {
                    pad,
                    event: PadEvent::Pressure(soft_pressure(0x200))
                },
                ControlEvent::Pad {
                    pad,
                    event: PadEvent::Release
                },
            ]
        );
    }

    #[test]
    fn led_reports_have_the_right_shape() {
        let mut f = LedFrame::default();
        f.set_pad(0, NiColor::Red, 3);
        f.set_button(Button::Stop, 1.0);
        let (a, b) = encode_leds(&f);
        assert_eq!(a.len(), 63);
        assert_eq!(b.len(), 42);
        assert_eq!((a[0], b[0]), (0x80, 0x81));
        assert_eq!(a[1 + 43], 127); // stop = slot 43
        assert_eq!(b[1 + 25 + 12], NiColor::Red.byte(3)); // pad 0 = HID 12 = slot 99
    }

    #[test]
    fn screen_transfer_layout() {
        let mut fb = FrameBuffer480x272::new();
        fb.put(10, 20, Rgb565(0xABCD));
        let mut out = Vec::new();
        encode_screen(Screen::Right, &fb, Rect::new(10, 20, 2, 1), &mut out);
        assert_eq!(&out[..8], &[0x84, 0, 1, 0x60, 0, 0, 0, 0]);
        assert_eq!(&out[8..16], &[0, 10, 0, 20, 0, 2, 0, 1]);
        assert_eq!(&out[16..24], &[2, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(&out[24..28], &[0xAB, 0xCD, 0, 0]);
        assert_eq!(&out[28..], &[2, 0, 0, 0, 3, 0, 0, 0, 0x40, 0, 0, 0]);
        // a full frame is 261,120 bytes of pixels
        encode_screen(Screen::Left, &fb, Rect::FULL, &mut out);
        assert_eq!(out.len(), 24 + 261_120 + 12);
        // odd rects are widened
        encode_screen(Screen::Left, &fb, Rect::new(479, 0, 1, 1), &mut out);
        assert_eq!(&out[8..16], &[1, 222, 0, 0, 0, 2, 0, 1]);
    }

    #[test]
    fn display_feature_echoes_geometry() {
        assert_eq!(
            display_feature(Screen::Left, 80, 50),
            [0xf8, 0xe0, 0x01, 0x10, 0x01, 0x10, 0x60, 0x01, 80, 50, 0]
        );
    }
}
