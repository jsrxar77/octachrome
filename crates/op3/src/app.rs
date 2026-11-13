//! The control/UI loop: one call to [`App::frame`] per display frame.

use crate::{bench::encode_png, controller::Controller};
use op3_engine::EngineHandle;
use op3_gfx::{DirtyTracker, FrameBuffer480x272};
use op3_hw::{ControlEvent, ControlSurface, HwResult, LedFrame, Screen};
use std::path::PathBuf;

/// How often the dump files are rewritten, in milliseconds.
const DUMP_EVERY_MS: u64 = 250;

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    pub events: usize,
    pub left_pixels: u32,
    pub right_pixels: u32,
}

pub struct App<S: ControlSurface> {
    pub surface: S,
    pub handle: EngineHandle,
    pub ctl: Controller,
    pub screens: [FrameBuffer480x272; 2],
    dirty: [DirtyTracker; 2],
    events: Vec<ControlEvent>,
    /// Events from a script, handled with the next frame's input.
    injected: Vec<ControlEvent>,
    leds: LedFrame,
    /// When set, the screens are written there as `left.png` and `right.png`
    /// (see `OP3_DUMP_DIR`), rewritten at most every [`DUMP_EVERY_MS`].
    pub dump_dir: Option<PathBuf>,
    last_dump_ms: Option<u64>,
    /// Print every control event and every transport change (`OP3_LOG_EVENTS`).
    pub log_events: bool,
    logged_playing: bool,
    logged_main: (op3_ui::Main, op3_gfx::ThemeId),
    /// Where the settings are saved when the user changes one; `None` (the
    /// simulator and the test bench) means never touch the disk.
    pub settings_path: Option<PathBuf>,
    /// Diagnostic (`OP3_NO_LEDS`): never write the LEDs.
    pub no_leds: bool,
}

impl<S: ControlSurface> App<S> {
    pub fn new(surface: S, handle: EngineHandle) -> Self {
        Self {
            surface,
            handle,
            ctl: Controller::new(),
            screens: [FrameBuffer480x272::new(), FrameBuffer480x272::new()],
            dirty: [DirtyTracker::new(), DirtyTracker::new()],
            events: Vec::with_capacity(256),
            injected: Vec::new(),
            leds: LedFrame::default(),
            dump_dir: None,
            last_dump_ms: None,
            log_events: false,
            logged_playing: false,
            logged_main: (op3_ui::Main::Instrument, op3_gfx::ThemeId::Op1),
            settings_path: None,
            no_leds: false,
        }
    }

    /// Queue an event as if the surface had reported it.
    pub fn inject(&mut self, e: ControlEvent) {
        self.injected.push(e);
    }

    /// Input → commands → draw → send only what changed.
    pub fn frame(&mut self, now_ms: u64) -> HwResult<FrameStats> {
        let mut stats = FrameStats::default();
        self.events.clear();
        self.surface.poll(&mut self.events)?;
        self.events.append(&mut self.injected);
        stats.events = self.events.len();
        for i in 0..self.events.len() {
            let ev = self.events[i];
            if self.log_events {
                eprintln!("[{:.1}s] event: {ev:?}", now_ms as f32 / 1000.0);
            }
            self.ctl.handle(ev, &mut self.handle, now_ms);
        }
        self.ctl.tick(&self.handle, now_ms);
        if std::mem::take(&mut self.ctl.theme_changed) {
            if let Some(path) = &self.settings_path {
                let settings = crate::settings::Settings {
                    theme: op3_gfx::theme_id(),
                };
                if let Err(e) = settings.save(path) {
                    eprintln!("settings: could not save {}: {e}", path.display());
                }
            }
        }
        if self.log_events {
            // What the engine reports, to tell "the button never arrived" from
            // "it arrived but the sequencer did not start".
            let playing = self.handle.status.playing.load(std::sync::atomic::Ordering::Relaxed);
            let now = (self.ctl.ui.main, op3_gfx::theme_id());
            if now != self.logged_main {
                eprintln!(
                    "[{:.1}s] settings: main mode {:?}, theme {}",
                    now_ms as f32 / 1000.0,
                    now.0,
                    now.1.name()
                );
                self.logged_main = now;
            }
            if playing != self.logged_playing {
                eprintln!(
                    "[{:.1}s] transport: {}",
                    now_ms as f32 / 1000.0,
                    if playing { "playing" } else { "stopped" }
                );
                self.logged_playing = playing;
            }
        }

        op3_ui::animate(&mut self.ctl.ui, &self.handle.project, &self.handle.status);
        op3_ui::draw_left(
            &mut self.screens[0],
            &self.ctl.ui,
            &self.handle.project,
            &self.handle.status,
        );
        let track = self.ctl.ui.track;
        let params: [f32; 8] = std::array::from_fn(|i| self.handle.param(track, i));
        op3_ui::draw_right(
            &mut self.screens[1],
            &self.ctl.ui,
            self.handle.slots[track].as_ref(),
            &params,
        );

        for (i, screen) in [Screen::Left, Screen::Right].into_iter().enumerate() {
            let update = self.dirty[i].diff(&self.screens[i]);
            if !update.rects().is_empty() {
                self.surface
                    .write_screen(screen, &self.screens[i], update.rects())?;
            }
            let px = update.pixel_count();
            if i == 0 {
                stats.left_pixels = px;
            } else {
                stats.right_pixels = px;
            }
        }

        if let Some(dir) = &self.dump_dir {
            if self.last_dump_ms.is_none_or(|t| now_ms.saturating_sub(t) >= DUMP_EVERY_MS) {
                self.last_dump_ms = Some(now_ms);
                let _ = std::fs::create_dir_all(dir);
                for (i, name) in ["left", "right"].into_iter().enumerate() {
                    let png = encode_png(&self.screens[i]);
                    let _ = std::fs::write(dir.join(format!("{name}.png")), png);
                }
            }
        }

        self.ctl.leds(&self.handle, &mut self.leds);
        if !self.no_leds {
            self.surface.write_leds(&self.leds)?;
        }
        self.handle.collect_garbage();
        Ok(stats)
    }

    /// Force full redraws (after a reconnect).
    pub fn invalidate(&mut self) {
        self.dirty.iter_mut().for_each(DirtyTracker::invalidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;
    use op3_hw::{mock::MockSurface, Button, PadEvent};
    use op3_ui::Mode;

    fn app() -> (App<MockSurface>, op3_engine::Engine) {
        let (engine, handle, _midi) = demo::empty_rig();
        (App::new(MockSurface::new(), handle), engine)
    }

    fn tap(app: &mut App<MockSurface>, pad: u8, t: &mut u64) {
        app.surface.push(ControlEvent::Pad {
            pad,
            event: PadEvent::Strike(0.8),
        });
        app.frame(*t).unwrap();
        *t += 50;
        app.surface.push(ControlEvent::Pad {
            pad,
            event: PadEvent::Release,
        });
        app.frame(*t).unwrap();
        *t += 50;
    }

    #[test]
    fn tapping_a_pad_toggles_the_step() {
        let (mut app, _e) = app();
        let mut t = 0;
        tap(&mut app, 12, &mut t); // top-left pad = step 1
        assert!(app.handle.project.tracks[0].steps[0].active);
        tap(&mut app, 12, &mut t);
        assert!(!app.handle.project.tracks[0].steps[0].active);
        tap(&mut app, 3, &mut t); // bottom-right pad = step 16
        assert!(app.handle.project.tracks[0].steps[15].active);
    }

    #[test]
    fn holding_a_pad_opens_the_inspector_without_toggling() {
        let (mut app, _e) = app();
        app.surface.push(ControlEvent::Pad {
            pad: 12,
            event: PadEvent::Press,
        });
        app.frame(0).unwrap();
        app.frame(400).unwrap();
        assert_eq!(app.ctl.ui.inspect_step, Some(0));
        // knob 4 adds a component while held
        app.surface.push(ControlEvent::Knob {
            index: 3,
            delta: 40,
        });
        app.frame(420).unwrap();
        assert_eq!(app.handle.project.tracks[0].steps[0].components.len(), 1);
        app.surface.push(ControlEvent::Pad {
            pad: 12,
            event: PadEvent::Release,
        });
        app.frame(500).unwrap();
        assert_eq!(app.ctl.ui.inspect_step, None);
        assert!(
            !app.handle.project.tracks[0].steps[0].active,
            "hold must not toggle"
        );
    }

    #[test]
    fn groups_select_and_shift_links_brain() {
        let (mut app, _e) = app();
        app.surface.push(ControlEvent::Button {
            button: Button::Group(3),
            pressed: true,
        });
        app.frame(0).unwrap();
        assert_eq!(app.ctl.ui.track, 3);
        app.surface.push(ControlEvent::Button {
            button: Button::Shift,
            pressed: true,
        });
        app.surface.push(ControlEvent::Button {
            button: Button::Group(5),
            pressed: true,
        });
        app.frame(16).unwrap();
        assert!(app.handle.project.tracks[5].brain_link);
        assert_eq!(app.ctl.ui.track, 3);
    }

    #[test]
    fn modes_and_knobs() {
        let (mut app, _e) = app();
        app.surface.push(ControlEvent::Knob {
            index: 0,
            delta: -40 * 9,
        });
        app.frame(0).unwrap();
        assert_eq!(app.handle.project.tracks[0].length, 7);
        app.surface.push(ControlEvent::Button {
            button: Button::Chords,
            pressed: true,
        });
        app.surface.push(ControlEvent::Pad {
            pad: 5,
            event: PadEvent::Strike(1.0),
        });
        app.frame(16).unwrap();
        assert_eq!(app.ctl.ui.mode, Mode::Brain);
        assert_eq!(app.handle.project.brain.live.unwrap().name(), "Am");
        app.surface.push(ControlEvent::Pad {
            pad: 5,
            event: PadEvent::Release,
        });
        app.frame(32).unwrap();
        assert_eq!(app.handle.project.brain.live, None);
    }

    #[test]
    fn steady_state_frames_send_almost_nothing() {
        let (mut app, _e) = app();
        app.frame(0).unwrap();
        let s = app.frame(16).unwrap();
        assert_eq!(s.left_pixels, 0);
        assert_eq!(s.right_pixels % 2, 0);
        assert!(app.surface.led_writes >= 2);
    }
}
