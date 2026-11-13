//! `op3` command line.
//!
//! ```text
//! op3 run              play on a connected Maschine MK3 (default)
//! op3 sim [-s SECS] [-o DIR]
//!                      headless run with a simulated MK3: scripted input,
//!                      writes out.wav and screenshots of both screens
//! op3 hw-test [-s SECS]
//!                      Phase 1 bring-up: rainbow on all 16 pads, a test frame
//!                      on both screens, every input event printed
//! ```

use anyhow::Result;
use op3::{demo, App};
use op3_gfx::{text, Align, FontSize, FrameBuffer480x272, Rgb565};
use op3_hw::{mock::MockSurface, ControlEvent, ControlSurface, PadEvent, Screen};
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
    // Screen theme (docs/DESIGN.md §2): op-1 (default), op-1-field, op-xy.
    let theme = opt("-t", "--theme").or_else(|| std::env::var("OP3_THEME").ok());
    if let Some(name) = theme {
        let id = match name.as_str() {
            "op-1" | "op1" => op3_gfx::ThemeId::Op1,
            "op-1-field" | "field" => op3_gfx::ThemeId::Op1Field,
            "op-xy" | "opxy" => op3_gfx::ThemeId::OpXy,
            other => anyhow::bail!("unknown theme {other:?} (op-1, op-1-field, op-xy)"),
        };
        op3_gfx::set_theme(id);
    }
    match cmd {
        "run" => run(),
        "sim" => sim(
            seconds.unwrap_or(8.0),
            PathBuf::from(opt("-o", "--out").unwrap_or_else(|| "out".into())),
        ),
        "hw-test" => hw_test(seconds.unwrap_or(30.0)),
        "-h" | "--help" | "help" => {
            println!("usage: op3 [run | sim [-s SECS] [-o DIR] | hw-test [-s SECS]] [-t op-1|op-1-field|op-xy]");
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
}

#[cfg(feature = "mk3")]
fn open_mk3() -> Result<op3_hw::mk3::Mk3Surface> {
    let (mut s, screen_err) = op3_hw::mk3::Mk3Surface::open()?;
    if let Some(e) = screen_err {
        eprintln!("screens unavailable, continuing without them: {e}");
    }
    let _ = s.set_brightness(80);
    Ok(s)
}

#[cfg(feature = "mk3")]
fn run() -> Result<()> {
    use std::sync::{atomic::AtomicBool, Arc};
    let (engine, handle, midi_rx) = demo::rig();
    let stop = Arc::new(AtomicBool::new(false));
    let port = op3::midi::open_default().unwrap_or_else(|e| {
        eprintln!("midi: {e}; MidiOutBridge output is discarded");
        Box::new(op3::midi::NullPort)
    });
    println!("midi out: {}", port.name());
    let _midi = op3::midi::spawn(midi_rx, port, stop.clone());

    #[cfg(feature = "audio")]
    let _audio = {
        let a = op3::audio::start(engine)?;
        println!(
            "audio: {} · {} ch · 48 kHz · buffer {}",
            a.device, a.channels, a.buffer
        );
        a
    };
    #[cfg(not(feature = "audio"))]
    let _engine = {
        eprintln!("built without the `audio` feature: no sound");
        engine
    };

    let surface = open_mk3()?;
    println!("control surface: {}", surface.name());
    let mut app = App::new(surface, handle);
    let mut pacer = Pacer::new(60);
    loop {
        if let Err(e) = app.frame(pacer.now_ms()) {
            eprintln!("MK3: {e}; reconnecting…");
            loop {
                std::thread::sleep(Duration::from_secs(1));
                if let Ok(s) = open_mk3() {
                    app.surface = s;
                    app.invalidate();
                    break;
                }
            }
        }
        pacer.wait();
    }
}

#[cfg(not(feature = "mk3"))]
fn run() -> Result<()> {
    anyhow::bail!("built without the `mk3` feature; use `op3 sim` or rebuild with default features")
}

/// Headless end-to-end run with a scripted mock MK3.
fn sim(seconds: f32, out: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&out)?;
    let (mut engine, handle, mut midi_rx) = demo::rig();
    let mut app = App::new(MockSurface::new(), handle);
    const FPS: u64 = 60;
    let spf = (op3_cartridge::SAMPLE_RATE as u64 / FPS) as usize; // 800 samples per frame
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
    let pad = |app: &mut App<MockSurface>, pad: u8, event: PadEvent| {
        app.surface.push(ControlEvent::Pad { pad, event })
    };
    let button = |app: &mut App<MockSurface>, b: op3_hw::Button| {
        app.surface.push(ControlEvent::Button {
            button: b,
            pressed: true,
        });
        app.surface.push(ControlEvent::Button {
            button: b,
            pressed: false,
        });
    };

    for f in 0..frames {
        let at = |s: f32| f == (s * FPS as f32) as u64;
        // ---- the script --------------------------------------------------
        if at(0.1) {
            button(&mut app, op3_hw::Button::Play);
        }
        if at(1.0) {
            button(&mut app, op3_hw::Button::Group(1)); // bass track
            app.surface.push(ControlEvent::KnobTouch {
                index: 5,
                touched: true,
            });
            app.surface.push(ControlEvent::Knob {
                index: 5,
                delta: 150,
            }); // more FM index
        }
        if at(1.9) {
            app.surface.push(ControlEvent::KnobTouch {
                index: 5,
                touched: false,
            });
            shot(&app, "1-step")?;
            pad(&mut app, 14, PadEvent::Strike(0.7)); // step 3 (top row, has a ratchet)
        }
        if at(2.4) {
            shot(&app, "2-inspector")?;
            pad(&mut app, 14, PadEvent::Release);
        }
        if at(3.0) {
            button(&mut app, op3_hw::Button::Chords);
        }
        if at(3.2) {
            pad(&mut app, 3, PadEvent::Strike(0.9)); // live IV chord
        }
        if at(3.8) {
            shot(&app, "3-brain")?;
            pad(&mut app, 3, PadEvent::Release);
            button(&mut app, op3_hw::Button::Perform);
            button(&mut app, op3_hw::Button::Group(0));
        }
        if at(4.0) {
            pad(&mut app, 0, PadEvent::Strike(0.4)); // LPF
        }
        for (k, p) in [(4.2, 0.5), (4.4, 0.8), (4.6, 0.95)] {
            if at(k) {
                pad(&mut app, 0, PadEvent::Pressure(p));
            }
        }
        if at(4.8) {
            shot(&app, "4-punch")?;
            pad(&mut app, 0, PadEvent::Release);
            pad(&mut app, 10, PadEvent::Strike(1.0)); // repeat 1/16
        }
        if at(5.4) {
            pad(&mut app, 10, PadEvent::Release);
            button(&mut app, op3_hw::Button::Step);
            app.surface.push(ControlEvent::Strip {
                position: Some(0.25),
            });
        }
        if at(6.0) {
            app.surface.push(ControlEvent::Strip { position: None });
            button(&mut app, op3_hw::Button::Group(3)); // macro track
            app.surface.push(ControlEvent::Knob {
                index: 6,
                delta: 60,
            }); // brighter timbre
        }
        if at(6.8) {
            shot(&app, "6-macro")?;
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
    op3::wav::write_wav(
        &out.join("out.wav"),
        &wav_l,
        &wav_r,
        op3_cartridge::SAMPLE_RATE as u32,
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
    Ok(())
}

/// Phase 1 bring-up on real hardware.
#[cfg(feature = "mk3")]
fn hw_test(seconds: f32) -> Result<()> {
    use op3_hw::{LedFrame, NiColor, Rect};
    let mut s = open_mk3()?;
    println!(
        "{}: rainbow pads + test frames; press things (Ctrl-C to quit)",
        s.name()
    );
    for (screen, label) in [(Screen::Left, "OP-3  LEFT"), (Screen::Right, "OP-3  RIGHT")] {
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
                op3_hw::Button::Group(g),
                op3::controller::TRACK_LED_COLORS[g as usize],
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
    op3_gfx::rect_outline(&mut fb, 0, 0, 480, 272, 1, Rgb565::WHITE);
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
