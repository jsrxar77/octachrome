//! `op8` command line.
//!
//! ```text
//! op8 run              play on a connected Maschine MK3 (default)
//! op8 sim [-s SECS] [-o DIR]
//!                      headless run with a simulated MK3: scripted input,
//!                      writes out.wav and screenshots of both screens
//! op8 hw-test [-s SECS]
//!                      Phase 1 bring-up: rainbow on all 16 pads, a test frame
//!                      on both screens, every input event printed
//! ```

use anyhow::Result;
use op8::{demo, script::{Action, Script}, App};
use op8_gfx::{text, Align, FontSize, FrameBuffer480x272, Rgb565};
use op8_hw::{mock::MockSurface, ControlEvent, ControlSurface, PadEvent, Screen};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("run");
    let opt = |short: &str, long: &str| {
        args.iter()
            .position(|a| a == short || a == long)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let seconds: Option<f32> = opt("-s", "--seconds").and_then(|s| s.parse().ok());
    // Screen theme (docs/design.md §2): glossy (default), matte, chrome.
    let theme = opt("-t", "--theme").or_else(|| std::env::var("OP8_THEME").ok());
    let theme_given = theme.is_some();
    if let Some(name) = theme {
        let id = op8::settings::theme_from_name(&name)
            .ok_or_else(|| anyhow::anyhow!("unknown theme {name:?} (glossy, matte, chrome)"))?;
        op8_gfx::set_theme(id);
    }
    let script = match opt("--script", "--script") {
        Some(path) => {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("reading script {path}: {e}"))?;
            Some(Script::parse(&text).map_err(|e| anyhow::anyhow!("script {path}: {e:#}"))?)
        }
        None => None,
    };
    match cmd {
        "run" => run(script, theme_given),
        "sim" => sim(
            seconds,
            PathBuf::from(opt("-o", "--out").unwrap_or_else(|| "out".into())),
            script.unwrap_or_else(|| {
                Script::parse(include_str!("../scripts/demo.op8")).expect("built-in script")
            }),
        ),
        "hw-test" => hw_test(seconds.unwrap_or(30.0)),
        "calib" => calib(seconds.unwrap_or(20.0)),
        "usb-reset" => usb_reset(),
        "-h" | "--help" | "help" => {
            println!("usage: op8 [run | sim [-s SECS] [-o DIR] | hw-test [-s SECS]] [--script FILE] [-t glossy|matte|chrome]");
            Ok(())
        }
        other => anyhow::bail!("unknown command {other:?} (try --help)"),
    }
}

/// Fixed-rate frame clock.
struct Pacer {
    start: Instant,
    frame: u64,
    period: Duration,
}

impl Pacer {
    fn new(fps: u32) -> Self {
        Self {
            start: Instant::now(),
            frame: 0,
            period: Duration::from_secs_f64(1.0 / fps as f64),
        }
    }
    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    fn wait(&mut self) {
        self.frame += 1;
        let next = self.start + self.period * self.frame as u32;
        if let Some(d) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(d);
        }
    }
    /// After a long stop (a reconnect), continue from now instead of racing
    /// through all the frames that were missed.
    fn skip_to_now(&mut self) {
        self.frame = (self.start.elapsed().as_nanos() / self.period.as_nanos()) as u64;
    }
    fn seconds(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }
}

#[cfg(feature = "mk3")]
fn open_mk3() -> Result<op8_hw::mk3::Mk3Surface> {
    let (mut s, screen_err) = op8_hw::mk3::Mk3Surface::open()?;
    if let Some(e) = screen_err {
        eprintln!("screens unavailable, continuing without them: {e}");
    }
    // `OP8_BRIGHTNESS` wins; otherwise what the supply allows: 50 on a limited
    // one (behind a hub, 480 mA, 80 resets the MK3 as soon as the music plays;
    // 60 held, 70 failed after 30 s), 80 on a full one.
    let brightness = std::env::var("OP8_BRIGHTNESS")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .unwrap_or(s.power_plan().brightness);
    let _ = s.set_brightness(brightness);
    Ok(s)
}

#[cfg(feature = "mk3")]
fn run(mut script: Option<Script>, theme_given: bool) -> Result<()> {
    use std::sync::{atomic::AtomicBool, Arc};
    let (engine, handle, midi_rx) = demo::rig();
    let stop = Arc::new(AtomicBool::new(false));
    // Diagnostic: OP8_MIDI=off sends no MIDI to the MK3's port.
    let midi_off = std::env::var("OP8_MIDI").is_ok_and(|v| v == "off");
    let port = if midi_off {
        Err(anyhow::anyhow!("turned off by OP8_MIDI=off"))
    } else {
        op8::midi::open_default()
    }
    .unwrap_or_else(|e| {
        eprintln!("midi: {e}; MidiOutBridge output is discarded");
        Box::new(op8::midi::NullPort)
    });
    println!("midi out: {}", port.name());
    let _midi = op8::midi::spawn(midi_rx, port, stop.clone());

    // The engine lives in `Audio`, which reopens the stream if the device goes
    // away: the sequencer's clock runs in the audio callback, so without audio
    // PLAY would light up and nothing would move.
    #[cfg(feature = "audio")]
    let mut audio = op8::audio::Audio::new(engine);
    #[cfg(not(feature = "audio"))]
    let _engine = {
        eprintln!("built without the `audio` feature: no sound");
        engine
    };

    let surface = open_mk3()?;
    println!("control surface: {}", surface.name());
    println!("{}", surface.power_note());
    let mut app = App::new(surface, handle);
    // The theme: `-t` / `OP8_THEME` win; otherwise the one saved from the
    // settings view; otherwise glossy. Later changes are saved to the same file.
    if let Some(path) = op8::settings::default_path() {
        if !theme_given {
            op8_gfx::set_theme(op8::settings::Settings::load(&path).theme);
        }
        app.settings_path = Some(path);
    }
    app.log_events = std::env::var_os("OP8_LOG_EVENTS").is_some();
    app.no_leds = std::env::var_os("OP8_NO_LEDS").is_some();
    if let Some(dir) = std::env::var_os("OP8_DUMP_DIR") {
        println!("dumping screens to {}/left.png and right.png", PathBuf::from(&dir).display());
        app.dump_dir = Some(PathBuf::from(dir));
    }
    let mut pacer = Pacer::new(60);
    let mut backoff = op8::backoff::Backoff::new();
    // The theme's light (backlight, LED levels), applied again whenever the
    // theme or the surface changes.
    let mut lit: Option<op8_gfx::ThemeId> = None;
    loop {
        if lit != Some(op8_gfx::theme_id()) {
            let _ = app.surface.set_look(op8_gfx::theme().light);
            lit = Some(op8_gfx::theme_id());
        }
        #[cfg(feature = "audio")]
        if let Some(line) = audio.keep_alive() {
            println!("[{:.1}s] {line}", pacer.seconds());
        }
        if let Some(sc) = script.as_mut() {
            for action in sc.due(pacer.now_ms()) {
                if let Action::Event(e) = action {
                    app.inject(e);
                }
            }
            if sc.is_done() {
                println!("script done; the instrument is yours");
                script = None;
            }
        }
        if let Err(e) = app.frame(pacer.now_ms()) {
            // Let go of the screens first: while the old surface holds
            // interface 5, a new one can't claim it and would come up blind.
            app.surface.close_display();
            // A MK3 that has just reset needs a few seconds. Opening it at
            // once makes it fail again, which showed up as the screens and the
            // sound switching on and off in a loop, so wait, and wait longer
            // while the failures keep coming.
            let wait = backoff.failed(Instant::now());
            eprintln!(
                "[{:.1}s] MK3: {e}; reconnecting in {} s (if it keeps failing: close op8, run `op8 usb-reset`, or replug the cable)",
                pacer.seconds(),
                wait.as_secs()
            );
            std::thread::sleep(wait);
            loop {
                if let Ok(s) = open_mk3() {
                    if s.has_screens() {
                        app.surface = s;
                        app.invalidate();
                        lit = None;
                        println!("[{:.1}s] MK3: back", pacer.seconds());
                        break;
                    }
                }
                std::thread::sleep(Duration::from_secs(2)); // still coming back
            }
            pacer.skip_to_now();
        }
        pacer.wait();
    }
}

#[cfg(not(feature = "mk3"))]
fn run(_: Option<Script>, _: bool) -> Result<()> {
    anyhow::bail!("built without the `mk3` feature; use `op8 sim` or rebuild with default features")
}

/// Headless end-to-end run with a scripted mock MK3.
fn sim(seconds: Option<f32>, out: PathBuf, mut script: Script) -> Result<()> {
    // Default length: the script plus a second of tail.
    let seconds = seconds.unwrap_or_else(|| (script.end_ms() as f32 / 1000.0 + 1.2).max(8.0));
    std::fs::create_dir_all(&out)?;
    let (mut engine, handle, mut midi_rx) = demo::rig();
    let mut app = App::new(MockSurface::new(), handle);
    const FPS: u64 = 60;
    let spf = (op8_cartridge::SAMPLE_RATE as u64 / FPS) as usize; // 800 samples per frame
    let frames = (seconds * FPS as f32) as u64;
    let (mut wav_l, mut wav_r) = (Vec::new(), Vec::new());
    let (mut l, mut r) = (vec![0.0f32; 128], vec![0.0f32; 128]);
    let (mut midi_msgs, mut pixels) = (0usize, 0u64);
    let started = Instant::now();
    let shot = |app: &App<MockSurface>, name: &str| -> Result<()> {
        std::fs::write(
            out.join(format!("{name}-left.ppm")),
            app.surface.screens[0].to_ppm(),
        )?;
        std::fs::write(
            out.join(format!("{name}-right.ppm")),
            app.surface.screens[1].to_ppm(),
        )?;
        Ok(())
    };
    for f in 0..frames {
        for action in script.due(f * 1000 / FPS) {
            match action {
                Action::Event(e) => app.inject(e),
                Action::Shot(name) => shot(&app, &name)?,
            }
        }
        // ---- one frame of audio, then one UI frame -------------------------
        for _ in 0..spf / 100 {
            engine.render(&mut l[..100], &mut r[..100]);
            wav_l.extend_from_slice(&l[..100]);
            wav_r.extend_from_slice(&r[..100]);
        }
        while midi_rx.pop().is_ok() {
            midi_msgs += 1;
        }
        let s = app.frame(f * 1000 / FPS)?;
        pixels += (s.left_pixels + s.right_pixels) as u64;
    }
    shot(&app, "5-final")?;
    op8::wav::write_wav(
        &out.join("out.wav"),
        &wav_l,
        &wav_r,
        op8_cartridge::SAMPLE_RATE as u32,
    )?;
    let full = 2 * 480 * 272 * frames;
    println!(
        "sim: {frames} frames, {:.1}% of full-frame pixels sent, {midi_msgs} MIDI messages, dropped events {}, {:.1} s rendered in {} ms",
        100.0 * pixels as f64 / full as f64,
        app.handle.status.dropped_events.load(std::sync::atomic::Ordering::Relaxed),
        seconds,
        started.elapsed().as_millis(),
    );
    println!("wrote {}/out.wav and screenshots", out.display());
    print!("{}", op8::wav::loudness_report(&wav_l, &wav_r, op8_cartridge::SAMPLE_RATE as usize / 2));
    Ok(())
}

/// Phase 1 bring-up on real hardware.
/// TEMPORARY calibration: a red bar 40 px wide on the left edge, a white bar at
/// the centre and a green bar 40 px wide on the right edge, on both screens.
/// Shows how many pixels the device shifts the image by, and in which direction.
#[cfg(feature = "mk3")]
fn calib(seconds: f32) -> Result<()> {
    use op8_hw::{Rect, Screen};
    let mut s = open_mk3()?;
    let mut fb = FrameBuffer480x272::new();
    fb.fill_rect(0, 0, 40, 272, Rgb565::from_hex(0xff0000));
    fb.fill_rect(220, 0, 40, 272, Rgb565::from_hex(0xffffff));
    fb.fill_rect(440, 0, 40, 272, Rgb565::from_hex(0x00ff00));
    for screen in [Screen::Left, Screen::Right] {
        s.write_screen(screen, &fb, &[Rect::FULL])?;
    }
    println!("calibration: red 0..40, white 220..260, green 440..480; {seconds} s");
    std::thread::sleep(Duration::from_secs_f32(seconds));
    Ok(())
}

/// Reset the MK3 on the USB bus, as replugging it would. Run it with op8
/// closed: macOS refuses while any of the device's interfaces is in use.
#[cfg(feature = "mk3")]
fn usb_reset() -> Result<()> {
    op8_hw::mk3::reset_usb()?;
    println!("reset sent; the MK3 comes back in a few seconds");
    std::thread::sleep(Duration::from_secs(5));
    Ok(())
}

#[cfg(not(feature = "mk3"))]
fn usb_reset() -> Result<()> {
    anyhow::bail!("built without the `mk3` feature")
}

#[cfg(not(feature = "mk3"))]
fn calib(_: f32) -> Result<()> {
    anyhow::bail!("built without the `mk3` feature")
}

#[cfg(feature = "mk3")]
fn hw_test(seconds: f32) -> Result<()> {
    use op8_hw::{LedFrame, NiColor, Rect};
    let mut s = open_mk3()?;
    println!(
        "{}: rainbow pads + test frames; press things (Ctrl-C to quit)",
        s.name()
    );
    for (screen, label) in [(Screen::Left, "OP-8  LEFT"), (Screen::Right, "OP-8  RIGHT")] {
        let fb = test_frame(label, screen == Screen::Right);
        s.write_screen(screen, &fb, &[Rect::FULL])?;
    }
    let mut leds = LedFrame::default();
    let mut events = Vec::new();
    let mut held = [false; 16];
    let mut pacer = Pacer::new(30);
    while pacer.now_ms() < (seconds * 1000.0) as u64 {
        events.clear();
        s.poll(&mut events)?;
        for e in &events {
            println!("{e:?}");
            if let ControlEvent::Pad { pad, event } = e {
                held[*pad as usize] = !matches!(event, PadEvent::Release);
            }
        }
        let phase = (pacer.frame / 3) as usize;
        for p in 0..16u8 {
            if held[p as usize] {
                leds.set_pad(p, NiColor::White, 3);
            } else {
                leds.set_pad(p, NiColor::HUES[(p as usize + phase) % 16], 3);
            }
        }
        for g in 0..8u8 {
            leds.set_button_color(
                op8_hw::Button::Group(g),
                op8::controller::track_led_colors(op8_gfx::theme_id())[g as usize],
                3,
            );
        }
        s.write_leds(&leds)?;
        pacer.wait();
    }
    Ok(())
}

#[cfg(not(feature = "mk3"))]
fn hw_test(_: f32) -> Result<()> {
    anyhow::bail!("built without the `mk3` feature")
}

/// Colour bars, a 1 px white frame and a label: checks byte order (red must
/// be on the left), geometry (the frame must touch all four edges) and text.
#[allow(dead_code)]
fn test_frame(label: &str, right: bool) -> FrameBuffer480x272 {
    let mut fb = FrameBuffer480x272::new();
    let bars = [
        0xff0000, 0x00ff00, 0x0000ff, 0xffff00, 0x00ffff, 0xff00ff, 0xffffff, 0x000000,
    ];
    for (i, c) in bars.iter().enumerate() {
        fb.fill_rect(i as i32 * 60, 0, 60, 180, Rgb565::from_hex(*c));
    }
    for x in 0..480 {
        let v = (x * 255 / 479) as u8;
        let c = if right {
            Rgb565::new(v, v / 2, 255 - v)
        } else {
            Rgb565::new(v, v, v)
        };
        fb.fill_rect(x, 180, 1, 30, c);
    }
    op8_gfx::rect_outline(&mut fb, 0, 0, 480, 272, 1, Rgb565::WHITE);
    text(
        &mut fb,
        label,
        240,
        228,
        FontSize::Large,
        Rgb565::WHITE,
        Align::Center,
    );
    fb
}
