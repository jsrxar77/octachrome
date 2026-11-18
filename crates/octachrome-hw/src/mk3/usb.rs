//! Real MK3 transport.
//!
//! * Screens: claim vendor interface 5 with `nusb` (pure Rust on Linux and
//!   macOS) and write bulk transfers to endpoint `0x04`. Linux binds no kernel
//!   driver to it. On macOS NI's `NIHardwareAgent` may hold it: quit NI
//!   services (or unload the agent) before running Octachrome.
//! * HID: on Linux, `/dev/hidrawN` read/written directly (no C library). On
//!   macOS, through IOKit via `hidapi`, because the OS keeps its own driver
//!   on HID interfaces.

use super::{display::DisplayLink, power, protocol::*, DISPLAY_INTERFACE, PRODUCT_ID, VENDOR_ID};
use crate::{
    ControlEvent, ControlSurface, FrameBuffer480x272, HwError, HwResult, LedFrame, Rect, Screen,
};

fn err(context: &str, e: impl std::fmt::Display) -> HwError {
    HwError(format!("{context}: {e}"))
}

pub struct Mk3Surface {
    hid: hid::HidPort,
    /// The screens, on a thread of their own (see `display.rs`).
    display: Option<DisplayLink>,
    prev: ControlsState,
    have_prev: bool,
    pads: PadTracker,
    last_leds: Option<LedFrame>,
    read_buf: [u8; 128],
    /// What the host's supply allows: backlight and LED limits (see `power.rs`).
    power: power::Plan,
    /// What the supply allows before the theme asks for less.
    base_power: power::Plan,
    power_note: String,
}

impl Mk3Surface {
    /// Open the first MK3 found. Screens are optional: if interface 5 can't
    /// be claimed the surface still works for input and LEDs and reports why.
    pub fn open() -> HwResult<(Self, Option<HwError>)> {
        let hid = hid::HidPort::open()?;
        let (display, display_err) = match open_display() {
            Ok(iface) => (Some(DisplayLink::start(iface)), None),
            Err(e) => (None, Some(e)),
        };
        // Read at every open, so plugging the MK3 into another port is noticed.
        let (power, power_note) = power::choose();
        let s = Self {
            hid,
            display,
            prev: ControlsState::default(),
            have_prev: false,
            pads: PadTracker::default(),
            last_leds: None,
            read_buf: [0; 128],
            power,
            base_power: power,
            power_note,
        };
        Ok((s, display_err))
    }

    /// The limits for the supply the host gives this MK3.
    pub fn power_plan(&self) -> power::Plan {
        self.power
    }

    /// Apply a theme's look: backlight and LED levels as a share of what the
    /// supply allows. `OCTACHROME_BRIGHTNESS` still wins for the backlight.
    pub fn set_look(&mut self, look: octachrome_gfx::Light) -> HwResult<()> {
        self.power = self.base_power.with_look(look);
        let brightness = std::env::var("OCTACHROME_BRIGHTNESS")
            .ok()
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(self.power.brightness);
        self.last_leds = None; // rewrite the LEDs under the new limits
        self.set_brightness(brightness)
    }

    /// One line saying what was detected and what it means.
    pub fn power_note(&self) -> &str {
        &self.power_note
    }

    pub fn has_screens(&self) -> bool {
        self.display.is_some()
    }

    /// Stop the display thread and release interface 5, so a new surface can
    /// claim it (used before reconnecting).
    pub fn close_display(&mut self) {
        self.display = None;
    }

    /// Set both backlights (0..=100).
    pub fn set_brightness(&mut self, brightness: u8) -> HwResult<()> {
        for s in [Screen::Left, Screen::Right] {
            self.hid.send_feature(&display_feature(s, brightness, 50))?;
        }
        Ok(())
    }
}

/// Reset the MK3 on the USB bus, which is what unplugging and replugging it
/// does. After a failed transfer the device can answer every HID write with an
/// error until it is reset; it re-enumerates a few seconds later.
pub fn reset_usb() -> HwResult<()> {
    let info = nusb::list_devices()
        .map_err(|e| err("listing USB devices", e))?
        .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
        .ok_or_else(|| HwError("no Maschine MK3 on USB".into()))?;
    let dev = info.open().map_err(|e| err("opening MK3 USB device", e))?;
    dev.reset().map_err(|e| err("resetting the MK3", e))
}

fn open_display() -> HwResult<nusb::Interface> {
    // Diagnostic: OCTACHROME_NO_SCREENS=1 leaves the screens alone (the app then runs without them).
    if std::env::var_os("OCTACHROME_NO_SCREENS").is_some() {
        return Err(HwError("turned off by OCTACHROME_NO_SCREENS".into()));
    }
    let info = nusb::list_devices()
        .map_err(|e| err("listing USB devices", e))?
        .find(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
        .ok_or_else(|| HwError("no Maschine MK3 on USB".into()))?;
    let dev = info.open().map_err(|e| err("opening MK3 USB device", e))?;
    dev.claim_interface(DISPLAY_INTERFACE).map_err(|e| {
        err(
            "claiming display interface 5 (on macOS, quit NIHardwareAgent first)",
            e,
        )
    })
}

impl ControlSurface for Mk3Surface {
    fn name(&self) -> &str {
        "Maschine MK3"
    }

    fn poll(&mut self, out: &mut Vec<ControlEvent>) -> HwResult<()> {
        // Drain everything queued; the kernel buffer drops the oldest reports
        // when full, so reading promptly matters more than reading carefully.
        for _ in 0..64 {
            let n = self.hid.read(&mut self.read_buf)?;
            if n == 0 {
                break;
            }
            let report = &self.read_buf[..n];
            match report[0] {
                CONTROLS_REPORT => {
                    if let Some(now) = ControlsState::parse(report) {
                        if self.have_prev {
                            diff_controls(&self.prev, &now, out);
                        }
                        self.prev = now;
                        self.have_prev = true;
                    }
                }
                PADS_REPORT => self.pads.parse(report, out),
                _ => {}
            }
        }
        Ok(())
    }

    fn write_leds(&mut self, leds: &LedFrame) -> HwResult<()> {
        // On a limited supply the LEDs are capped before they are encoded.
        let leds = &self.power.limit(leds);
        let (a, b) = encode_leds(leds);
        let prev = self.last_leds.map(|p| encode_leds(&p));
        if prev.is_none_or(|(pa, _)| pa != a) {
            self.hid.write(&a)?;
        }
        if prev.is_none_or(|(_, pb)| pb != b) {
            self.hid.write(&b)?;
        }
        self.last_leds = Some(*leds);
        Ok(())
    }

    /// Hands the whole frame to the display thread, which works out which rows
    /// changed on its own; `_rects` (what the caller thinks changed) is not
    /// needed. Returns at once, whatever the USB is doing.
    fn write_screen(
        &mut self,
        screen: Screen,
        fb: &FrameBuffer480x272,
        _rects: &[Rect],
    ) -> HwResult<()> {
        match &self.display {
            Some(display) => display.submit(screen, fb),
            None => Ok(()),
        }
    }
}

#[cfg(target_os = "linux")]
mod hid {
    use super::{err, PRODUCT_ID, VENDOR_ID};
    use crate::{HwError, HwResult};
    use std::{
        fs::{File, OpenOptions},
        io::{ErrorKind, Read, Write},
        os::unix::{fs::OpenOptionsExt, io::AsRawFd},
        path::PathBuf,
    };

    pub struct HidPort {
        file: File,
    }

    impl HidPort {
        pub fn open() -> HwResult<Self> {
            let path = find()?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&path)
                .map_err(|e| {
                    err(
                        &format!(
                            "opening {} (install deploy/99-maschine-mk3.rules and replug)",
                            path.display()
                        ),
                        e,
                    )
                })?;
            Ok(Self { file })
        }

        /// Non-blocking; Ok(0) when nothing is queued.
        pub fn read(&mut self, buf: &mut [u8]) -> HwResult<usize> {
            match self.file.read(buf) {
                Ok(n) => Ok(n),
                Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(0),
                Err(e) => Err(err("reading hidraw", e)),
            }
        }

        pub fn write(&mut self, report: &[u8]) -> HwResult<()> {
            self.file
                .write_all(report)
                .map_err(|e| err("writing hidraw", e))
        }

        pub fn send_feature(&mut self, report: &[u8]) -> HwResult<()> {
            // HIDIOCSFEATURE(len) = _IOC(_IOC_WRITE|_IOC_READ, 'H', 0x06, len)
            let req = (3u64 << 30) | ((report.len() as u64) << 16) | ((b'H' as u64) << 8) | 0x06;
            // SAFETY: the buffer outlives the call and its length is encoded in the request.
            let rc = unsafe { libc::ioctl(self.file.as_raw_fd(), req as _, report.as_ptr()) };
            if rc < 0 {
                return Err(err("HIDIOCSFEATURE", std::io::Error::last_os_error()));
            }
            Ok(())
        }
    }

    /// Find the hidraw node whose HID_ID matches the MK3.
    fn find() -> HwResult<PathBuf> {
        let want = format!("HID_ID=0003:{:08X}:{:08X}", VENDOR_ID, PRODUCT_ID);
        let dir = std::fs::read_dir("/sys/class/hidraw")
            .map_err(|e| err("listing /sys/class/hidraw", e))?;
        for entry in dir.flatten() {
            let uevent =
                std::fs::read_to_string(entry.path().join("device/uevent")).unwrap_or_default();
            if uevent.lines().any(|l| l.eq_ignore_ascii_case(&want)) {
                return Ok(PathBuf::from("/dev").join(entry.file_name()));
            }
        }
        Err(HwError(
            "no hidraw node for the Maschine MK3 (is it plugged in?)".into(),
        ))
    }
}

#[cfg(target_os = "macos")]
mod hid {
    use super::{err, PRODUCT_ID, VENDOR_ID};
    use crate::{HwError, HwResult};

    pub struct HidPort {
        dev: hidapi::HidDevice,
    }

    // SAFETY: the device handle is only ever used from the thread that owns
    // the surface; hidapi's macOS backend has no thread affinity.
    unsafe impl Send for HidPort {}

    impl HidPort {
        pub fn open() -> HwResult<Self> {
            let api = hidapi::HidApi::new().map_err(|e| err("starting hidapi", e))?;
            let info = api
                .device_list()
                .find(|d| {
                    d.vendor_id() == VENDOR_ID
                        && d.product_id() == PRODUCT_ID
                        && (d.interface_number() == super::super::HID_INTERFACE as i32
                            || d.usage_page() == 0xff01
                            || d.interface_number() < 0)
                })
                .ok_or_else(|| HwError("no Maschine MK3 HID interface found".into()))?;
            let dev = info
                .open_device(&api)
                .map_err(|e| err("opening MK3 HID", e))?;
            dev.set_blocking_mode(false)
                .map_err(|e| err("hid non-blocking", e))?;
            Ok(Self { dev })
        }

        pub fn read(&mut self, buf: &mut [u8]) -> HwResult<usize> {
            self.dev.read(buf).map_err(|e| err("reading HID", e))
        }

        pub fn write(&mut self, report: &[u8]) -> HwResult<()> {
            self.dev
                .write(report)
                .map(|_| ())
                .map_err(|e| err("writing HID", e))
        }

        pub fn send_feature(&mut self, report: &[u8]) -> HwResult<()> {
            self.dev
                .send_feature_report(report)
                .map_err(|e| err("HID feature report", e))
        }
    }
}
