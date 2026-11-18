//! How much current the host gives the MK3, and what the app does about it.
//!
//! A bright backlight resets a MK3 that is fed too little: behind a USB 2.0 hub
//! (480 mA) it reset within seconds of the music starting at brightness 80, and
//! held at 60 (measured 2026-11-13, docs/roadmap.md). So on a limited supply the
//! screens are dimmed and the LEDs capped; on a full one they are left alone.
//!
//! macOS reports what it granted the device as `UsbPowerSinkAllocation` in the
//! I/O registry. The threshold between the two is provisional: it is the value
//! seen behind the hub (480) against what a device asks for (500), and has to be
//! confirmed with the MK3 plugged straight into the Mac.

use crate::{
    leds::{button_is_colour, button_slot, LedFrame, LED_SLOTS, PAD_BASE},
    Button,
};

/// Allocations below this are treated as a limited supply.
pub const FULL_POWER_MA: u32 = 500;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    /// True when the supply is limited (or unknown).
    pub limited: bool,
    /// Backlight, 0..=100.
    pub brightness: u8,
    /// Highest level a colour LED may be asked for (0..=3).
    pub colour_level_max: u8,
    /// Multiplier for the single-colour LEDs and the strip.
    pub mono_scale: f32,
}

pub const LIMITED: Plan = Plan {
    limited: true,
    brightness: 50,
    colour_level_max: 2,
    mono_scale: 0.6,
};

pub const FULL: Plan = Plan {
    limited: false,
    brightness: 80,
    colour_level_max: 3,
    mono_scale: 1.0,
};

/// The plan for a supply of `ma` milliamps; unknown counts as limited, because
/// guessing high resets the device and guessing low only dims it.
pub fn plan_for(ma: Option<u32>) -> Plan {
    match ma {
        Some(m) if m >= FULL_POWER_MA => FULL,
        _ => LIMITED,
    }
}

/// The allocation of the MK3 in the text of `ioreg -p IOUSB -l -w0`.
pub fn parse_ioreg(text: &str) -> Option<u32> {
    let mut lines = text.lines();
    lines.find(|l| l.contains("Maschine MK3@"))?;
    // Its properties follow, up to the next node of the tree.
    for line in lines {
        let t = line.trim_start_matches([' ', '|']);
        if t.starts_with("+-o ") {
            return None;
        }
        if let Some(rest) = t.strip_prefix("\"UsbPowerSinkAllocation\" = ") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// What the host granted the MK3, in mA. Only known on macOS.
pub fn detect() -> Option<u32> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = std::process::Command::new("ioreg")
        .args(["-p", "IOUSB", "-l", "-w0"])
        .output()
        .ok()?;
    parse_ioreg(&String::from_utf8_lossy(&out.stdout))
}

/// The plan to use and a line saying why. `OCTACHROME_POWER=full` or `limited` forces it.
pub fn choose() -> (Plan, String) {
    let forced = std::env::var("OCTACHROME_POWER").ok();
    let ma = detect();
    let (plan, why) = match (forced.as_deref(), ma) {
        (Some("full"), _) => (FULL, "forced by OCTACHROME_POWER=full".to_string()),
        (Some("limited"), _) => (LIMITED, "forced by OCTACHROME_POWER=limited".to_string()),
        (_, Some(m)) => (plan_for(Some(m)), format!("the host gives the MK3 {m} mA")),
        (_, None) => (LIMITED, "the supply is unknown".to_string()),
    };
    let what = if plan.limited {
        format!(
            "limited: screens at {}, LEDs capped",
            plan.brightness
        )
    } else {
        format!("full: screens at {}, LEDs at full", plan.brightness)
    };
    (plan, format!("power: {why} -> {what}"))
}

impl Plan {
    /// What the theme gets out of this supply: glossy asks for all of it, matte
    /// for less, and no theme ever gets more than the supply allows.
    pub fn with_look(&self, look: octachrome_gfx::Light) -> Plan {
        Plan {
            limited: self.limited,
            brightness: ((self.brightness as f32 * look.backlight.clamp(0.0, 1.0)).round() as u8).max(10),
            colour_level_max: self.colour_level_max.min(look.led_level.clamp(1, 3)),
            mono_scale: self.mono_scale * look.mono.clamp(0.0, 1.0),
        }
    }

    /// The frame to write: colour LEDs capped at `colour_level_max`, the rest
    /// scaled by `mono_scale` (a lit LED never rounds down to off).
    pub fn limit(&self, frame: &LedFrame) -> LedFrame {
        if self.colour_level_max >= 3 && self.mono_scale >= 1.0 {
            return *frame;
        }
        let mut colour = [false; LED_SLOTS];
        colour[PAD_BASE..PAD_BASE + 16].fill(true);
        let colour_buttons = [Button::Sampling, Button::Play, Button::Rec, Button::JogUp, Button::JogLeft, Button::JogRight, Button::JogDown]
            .into_iter()
            .chain((0..8).map(Button::Group));
        for b in colour_buttons.filter(|b| button_is_colour(*b)) {
            if let Some(s) = button_slot(b) {
                colour[s] = true;
            }
        }
        let mut out = *frame;
        for (i, v) in out.slots.iter_mut().enumerate() {
            if *v == 0 {
                continue;
            }
            *v = if colour[i] {
                let level = (*v & 3).min(self.colour_level_max);
                (*v & !3) | level
            } else {
                ((*v as f32 * self.mono_scale) as u8).max(1)
            };
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leds::NiColor;

    /// The shape of the real output: a hub, the MK3 under it, and a device after
    /// it with an allocation of its own that must not be read by mistake.
    const IOREG: &str = "\
+-o Root  <class IORegistryEntry, id 0x100000100, retain 32>
  +-o AppleT8132USBXHCI@01000000  <class AppleT8132USBXHCI, id 0x1000004c2, registered>
    +-o USB2.0 HUB@01100000  <class IOUSBHostDevice, id 0x10000b957>
      | {
      |   \"UsbPowerSinkAllocation\" = 100
      | }
      |
      +-o Maschine MK3@01120000  <class IOUSBHostDevice, id 0x100011e92>
            {
            \"Device Speed\" = 2
            \"kUSBCurrentConfiguration\" = 1
            \"USB Address\" = 1
            \"UsbPowerSinkAllocation\" = 480
            }

    +-o Other Device@01130000  <class IOUSBHostDevice, id 0x100011e93>
          {
          \"UsbPowerSinkAllocation\" = 900
          }
";

    // F-H04 (docs/features.md)
    #[test]
    fn reads_the_allocation_of_the_mk3_and_no_other_device() {
        assert_eq!(parse_ioreg(IOREG), Some(480));
        assert_eq!(parse_ioreg(&IOREG.replace("= 480", "= 900")), Some(900));
    }

    // F-H04
    #[test]
    fn no_mk3_or_no_property_means_unknown() {
        assert_eq!(parse_ioreg(""), None);
        assert_eq!(parse_ioreg(&IOREG.replace("Maschine MK3@", "Something@")), None);
        // Its own property is missing: the next device's value is not used.
        assert_eq!(parse_ioreg(&IOREG.replace("\"UsbPowerSinkAllocation\" = 480", "")), None);
    }

    // F-H04
    #[test]
    fn only_enough_current_gets_the_full_plan() {
        assert!(plan_for(Some(480)).limited, "behind the hub");
        assert!(plan_for(None).limited, "unknown is limited");
        assert!(!plan_for(Some(500)).limited);
        assert!(!plan_for(Some(900)).limited);
        assert!(LIMITED.brightness < FULL.brightness);
    }

    // F-H07
    #[test]
    fn the_theme_asks_for_light_within_what_the_supply_allows() {
        use octachrome_gfx::{theme_of, ThemeId};
        for base in [LIMITED, FULL] {
            let g = base.with_look(theme_of(ThemeId::Glossy).light);
            let c = base.with_look(theme_of(ThemeId::Chrome).light);
            let m = base.with_look(theme_of(ThemeId::Matte).light);
            assert_eq!(g.brightness, base.brightness, "glossy takes all the supply allows");
            assert!(g.brightness >= c.brightness && c.brightness > m.brightness, "glossy > chrome > matte");
            assert!(m.colour_level_max < g.colour_level_max.max(base.colour_level_max).min(3) || base.limited);
            assert!(m.mono_scale < g.mono_scale, "matte is softer on the other LEDs");
            for p in [g, c, m] {
                assert!(p.brightness <= base.brightness && p.colour_level_max <= base.colour_level_max && p.mono_scale <= base.mono_scale);
            }
        }
        // glossy on a full supply leaves the LEDs untouched; matte caps them
        let mut f = LedFrame::default();
        f.set_pad(0, NiColor::Red, 3);
        assert_eq!(FULL.with_look(theme_of(ThemeId::Glossy).light).limit(&f).pad(0) & 3, 3);
        assert_eq!(FULL.with_look(theme_of(ThemeId::Matte).light).limit(&f).pad(0) & 3, 2);
    }

    // F-H05 (docs/features.md)
    #[test]
    fn a_limited_supply_caps_the_pads_and_scales_the_rest() {
        let mut f = LedFrame::default();
        f.set_pad(0, NiColor::Red, 3);
        f.set_pad(1, NiColor::Green, 1);
        f.set_button_color(Button::Play, NiColor::Green, 3);
        f.set_button(Button::Shift, 1.0);
        f.set_button(Button::Mute, 0.01); // dimmer than one step
        let g = LIMITED.limit(&f);

        assert_eq!(g.pad(0) & 3, 2, "bright pad is capped");
        assert_eq!(g.pad(0) >> 2, NiColor::Red as u8, "and keeps its colour");
        assert_eq!(g.pad(1), f.pad(1), "a dim pad is left alone");
        let play = button_slot(Button::Play).unwrap();
        assert_eq!(g.slots[play] & 3, 2, "colour buttons are capped too");
        let shift = button_slot(Button::Shift).unwrap();
        assert_eq!(g.slots[shift], 76, "single-colour LEDs are scaled to 60 %");
        let mute = button_slot(Button::Mute).unwrap();
        assert_eq!(g.slots[mute], 1, "a lit LED never rounds down to off");
        let off = button_slot(Button::Solo).unwrap();
        assert_eq!(g.slots[off], 0, "an unlit LED stays unlit");
    }

    // F-H05
    #[test]
    fn a_full_supply_changes_nothing() {
        let mut f = LedFrame::default();
        f.set_pad(5, NiColor::Blue, 3);
        f.set_button(Button::Shift, 1.0);
        assert_eq!(FULL.limit(&f), f);
    }
}
