//! Settings: the main mode that takes both screens (design.md §4), in four pages chosen with the buttons 1 to 4 over the left screen:
//! `theme`, `audio`, `midi` and `light`.
//!
//! **theme.** Left: one card per theme, each drawn in that theme's own colours (its eight brand keys, the backlight it asks for and its
//! LED level), the active one outlined. Right: the eight encoder colours as dials and the track colours, in the active theme. The theme
//! applies live, so the screens are their own preview.
//!
//! **audio.** Left: the audio outputs, one to a row (the first row is `automatic`); the chosen one has a dot and the one that is open is
//! tagged `in use`. Right: the audio inputs the same way, then the size of the audio block, and what is open.
//!
//! **midi.** Left: the MIDI output ports the same way. Right: the MIDI inputs (`off` first), and under them four buttons: one that plays a
//! note, one that silences everything, one that turns the MIDI clock out on and off and one that chooses what a note that comes in plays.
//!
//! **light.** Left: the step of the screen backlight and the highest level of the pad LEDs, which are shares of what the supply allows,
//! never more. Right: what the supply is and what that means.
//!
//! A control that does nothing here shows nothing (rule 11).

use crate::nav::Page;
use crate::theme::{button_strip, dial, track_color};
use op8_engine::EngineStatus;
use op8_gfx::{
    knob,
    paint::{fill_circle, fill_convex, fill_round_rect, stroke_round_rect},
    text, text_v, theme, theme_id, theme_of, Align, FontSize, FrameBuffer480x272, ThemeId, VAlign,
};
use std::sync::atomic::Ordering::Relaxed;

const CARD_W: f32 = 136.0;
const CARD_H: f32 = 160.0;
const CARD_GAP: f32 = 18.0;
const CARD_Y: f32 = 62.0;

/// What the pages of settings show and the controller keeps: the lists of devices (given by whoever can see the hardware), which is chosen
/// and which is open, and the steps of the light.
#[derive(Clone, Debug)]
pub struct SettingsView {
    /// The open page: 0 theme, 1 audio, 2 midi, 3 light.
    pub page: u8,
    pub outs: Vec<String>,
    pub ins: Vec<String>,
    pub ports: Vec<String>,
    /// The MIDI inputs there are.
    pub midi_ins: Vec<String>,
    /// The chosen entry of each list: 0 is `automatic`, `n` is the `n`-th name.
    pub out_sel: usize,
    pub in_sel: usize,
    pub port_sel: usize,
    pub midi_in_sel: usize,
    /// What is open now, as the device says it.
    pub out_now: String,
    pub in_now: String,
    pub port_now: String,
    pub midi_in_now: String,
    /// What a note that comes in plays: the selected track (`false`) or the track of its MIDI channel.
    pub midi_by_channel: bool,
    /// The audio block chosen (64, 128 or 256) and what the stream says it has.
    pub buffer: u32,
    pub stream_now: String,
    /// The backlight step (1..=4) and the pad level (1..=3).
    pub screen: u8,
    pub pads: u8,
    /// MIDI clock out is on.
    pub midi_clock: bool,
    /// What the supply allows, in a line.
    pub supply: String,
    /// What just happened.
    pub note: String,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            page: 0,
            outs: Vec::new(),
            ins: Vec::new(),
            ports: Vec::new(),
            midi_ins: Vec::new(),
            out_sel: 0,
            in_sel: 0,
            port_sel: 0,
            midi_in_sel: 0,
            out_now: String::new(),
            in_now: String::new(),
            port_now: String::new(),
            midi_in_now: String::new(),
            midi_by_channel: false,
            buffer: 128,
            stream_now: String::new(),
            screen: 4,
            pads: 3,
            midi_clock: true,
            supply: String::new(),
            note: String::new(),
        }
    }
}

impl SettingsView {
    pub fn page_kind(&self) -> Page {
        match self.page {
            1 => Page::Audio,
            2 => Page::Midi,
            3 => Page::Light,
            _ => Page::Theme,
        }
    }
}

/// The title of the page that is open.
fn page_name(page: u8) -> &'static str {
    ["theme", "audio", "midi", "light"][(page as usize).min(3)]
}

/// Title with a dot, at the top left of a screen, at height `cy`.
fn bar(fb: &mut FrameBuffer480x272, title: &str, cy: f32) {
    let t = theme();
    fill_circle(fb, 16.0, cy, 6.0, t.fg);
    text_v(fb, title, 30, cy as i32, FontSize::Large, t.fg, Align::Left, VAlign::Middle);
}

fn transport(fb: &mut FrameBuffer480x272, st: &EngineStatus, cy: f32) {
    let t = theme();
    // Transport stays visible: PLAY keeps working here.
    if st.playing.load(Relaxed) {
        fill_convex(fb, &[(460.0, cy - 7.0), (470.0, cy - 1.0), (460.0, cy + 5.0)], t.fg);
    } else {
        fill_round_rect(fb, 460.0, cy - 6.0, 10.0, 10.0, 1.5, t.dim);
    }
}

fn ring(fb: &mut FrameBuffer480x272, ui: &crate::UiState, screen: u8) {
    if let Some(it) = crate::nav::focused(ui).filter(|it| it.screen == screen) {
        crate::nav::draw_ring(fb, &it, ui);
    }
}

pub fn draw_left(fb: &mut FrameBuffer480x272, ui: &crate::UiState, st: &EngineStatus) {
    let t = theme();
    let s = &ui.settings;
    fb.clear(t.bg);
    button_strip(fb, &crate::strip_labels(ui, 0));
    bar(fb, if s.page == 0 { "settings" } else { page_name(s.page) }, crate::theme::HEADER_CY);
    transport(fb, st, crate::theme::HEADER_CY);
    match s.page_kind() {
        Page::Theme => theme_cards(fb),
        Page::Audio => rows(fb, s, "automatic", &s.outs, s.out_sel, &s.out_now, 0),
        Page::Midi => rows(fb, s, "automatic", &s.ports, s.port_sel, &s.port_now, 0),
        Page::Light => light_left(fb, s),
    }
    ring(fb, ui, 0);
}

pub fn draw_right(fb: &mut FrameBuffer480x272, ui: &crate::UiState) {
    let t = theme();
    let s = &ui.settings;
    fb.clear(t.bg);
    match s.page_kind() {
        Page::Theme => {
            bar(fb, "theme", 15.0);
            theme_dials(fb);
        }
        Page::Audio => {
            bar(fb, "input", 15.0);
            rows(fb, s, "automatic", &s.ins, s.in_sel, &s.in_now, 1);
            text_v(fb, "audio block", 16, 160, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
            for (k, b) in crate::nav::BUFFER_CHOICES.into_iter().enumerate() {
                let (x, y, w, h) = crate::nav::pick_rect(Page::Audio, 16 + k).1;
                let on = s.buffer == b;
                button(fb, (x, y, w, h), &format!("{b}"), on);
            }
            text_v(fb, &s.stream_now, 16, 226, FontSize::Medium, t.dim, Align::Left, VAlign::Middle);
        }
        Page::Midi => {
            bar(fb, "midi in", 15.0);
            rows(fb, s, "off", &s.midi_ins, s.midi_in_sel, &s.midi_in_now, 1);
            for (k, label) in ["play a note", "silence all"].into_iter().enumerate() {
                button(fb, crate::nav::pick_rect(Page::Midi, 12 + k).1, label, false);
            }
            button(fb, crate::nav::pick_rect(Page::Midi, 14).1, if s.midi_clock { "clock out: on" } else { "clock out: off" }, s.midi_clock);
            button(fb, crate::nav::pick_rect(Page::Midi, 15).1, if s.midi_by_channel { "in: by channel" } else { "in: selected" }, s.midi_by_channel);
            let hint = if s.midi_by_channel { "channels 1 to 8 play the tracks A to H" } else { "notes play the selected track" };
            text_v(fb, hint, 16, 208, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
        }
        Page::Light => {
            bar(fb, "light", 15.0);
            let mut y = 70;
            for line in wrap(&s.supply, 38) {
                text_v(fb, &line, 16, y, FontSize::Medium, t.fg, Align::Left, VAlign::Middle);
                y += 22;
            }
            text_v(fb, "the screen and the pads are never", 16, 160, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
            text_v(fb, "brighter than that: a step is a share of it", 16, 178, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
        }
    }
    if !s.note.is_empty() {
        text_v(fb, &fit(&s.note, 44), 16, 254, FontSize::Medium, t.fg, Align::Left, VAlign::Middle);
    }
    ring(fb, ui, 1);
}

/// `text` cut to `n` characters (with a dot at the end if it was cut).
fn fit(text: &str, n: usize) -> String {
    if text.chars().count() <= n {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(n.saturating_sub(1)).collect::<String>())
    }
}

/// `text` in lines of at most `n` characters, broken at spaces.
fn wrap(text: &str, n: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + word.chars().count() <= n => {
                l.push(' ');
                l.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// A button of the page: a rounded box with its label, filled when it is the one in use.
fn button(fb: &mut FrameBuffer480x272, (x, y, w, h): (f32, f32, f32, f32), label: &str, on: bool) {
    let t = theme();
    if on {
        fill_round_rect(fb, x, y, w, h, 8.0, t.panel);
    }
    stroke_round_rect(fb, x, y, w, h, 8.0, if on { 2.0 } else { 1.0 }, if on { t.fg } else { t.faint });
    text_v(fb, label, (x + w / 2.0) as i32, (y + h / 2.0) as i32, FontSize::Medium, if on { t.fg } else { t.dim }, Align::Center, VAlign::Middle);
}

/// The rows of a list of devices: `automatic` first, then the names; the chosen one has a dot and the one that is open is tagged.
fn rows(fb: &mut FrameBuffer480x272, s: &SettingsView, auto: &str, names: &[String], chosen: usize, now: &str, screen: u8) {
    let t = theme();
    let page = s.page_kind();
    let shown = crate::nav::list_len(names.len(), page, screen);
    for i in 0..shown {
        let (x, y, w, h) = crate::nav::pick_rect(page, if screen == 0 { i } else { 8 + i }).1;
        let name = if i == 0 { auto.to_string() } else { names[i - 1].clone() };
        let on = i == chosen;
        let open = i > 0 && !now.is_empty() && name == now || (i == 0 && chosen == 0 && !now.is_empty());
        fill_round_rect(fb, x, y, w, h, 6.0, if on { t.panel } else { t.bg });
        stroke_round_rect(fb, x, y, w, h, 6.0, 1.0, if on { t.dim } else { t.faint });
        if on {
            fill_circle(fb, x + 14.0, y + h / 2.0, 4.0, t.fg);
        }
        text_v(fb, &fit(&name, 40), (x + 28.0) as i32, (y + h / 2.0) as i32, FontSize::Medium, if on { t.fg } else { t.dim }, Align::Left, VAlign::Middle);
        if open {
            let tag = if i == 0 { fit(now, 14) } else { "in use".to_string() };
            text_v(fb, &tag, (x + w - 10.0) as i32, (y + h / 2.0) as i32, FontSize::Small, t.dim, Align::Right, VAlign::Middle);
        }
    }
    if names.is_empty() {
        let (x, y, ..) = crate::nav::pick_rect(page, if screen == 0 { 1 } else { 9 }).1;
        text_v(fb, "none found", (x + 28.0) as i32, (y + 12.0) as i32, FontSize::Small, t.faint, Align::Left, VAlign::Middle);
    }
}

/// The left of `light`: four steps of backlight and three levels of pad LED.
fn light_left(fb: &mut FrameBuffer480x272, s: &SettingsView) {
    let t = theme();
    text_v(fb, "screen", 16, 60, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
    for k in 0..4usize {
        let r = crate::nav::pick_rect(Page::Light, k).1;
        let on = s.screen as usize == k + 1;
        stroke_round_rect(fb, r.0, r.1, r.2, r.3, 8.0, if on { 2.0 } else { 1.0 }, if on { t.fg } else { t.faint });
        // the step as a bar that fills up: a share of the whole
        let bar_w = r.2 - 24.0;
        fill_round_rect(fb, r.0 + 12.0, r.1 + 18.0, bar_w, 6.0, 3.0, t.faint);
        fill_round_rect(fb, r.0 + 12.0, r.1 + 18.0, bar_w * (k + 1) as f32 / 4.0, 6.0, 3.0, if on { t.fg } else { t.dim });
        text_v(fb, &format!("{}%", 25 * (k + 1)), (r.0 + r.2 / 2.0) as i32, (r.1 + 42.0) as i32, FontSize::Medium, if on { t.fg } else { t.dim }, Align::Center, VAlign::Middle);
    }
    text_v(fb, "pads", 16, 160, FontSize::Small, t.dim, Align::Left, VAlign::Middle);
    for k in 0..3usize {
        let r = crate::nav::pick_rect(Page::Light, 4 + k).1;
        let on = s.pads as usize == k + 1;
        stroke_round_rect(fb, r.0, r.1, r.2, r.3, 8.0, if on { 2.0 } else { 1.0 }, if on { t.fg } else { t.faint });
        // the level as the squares of the card: lit up to the level
        for l in 0..3usize {
            fill_round_rect(fb, r.0 + 36.0 + l as f32 * 24.0, r.1 + 12.0, 16.0, 16.0, 4.0, if l <= k { if on { t.fg } else { t.dim } } else { t.faint });
        }
        text_v(fb, &format!("{}", k + 1), (r.0 + r.2 / 2.0) as i32, (r.1 + 52.0) as i32, FontSize::Medium, if on { t.fg } else { t.dim }, Align::Center, VAlign::Middle);
    }
}

fn theme_cards(fb: &mut FrameBuffer480x272) {
    let t = theme();
    let x0 = (480.0 - (3.0 * CARD_W + 2.0 * CARD_GAP)) / 2.0;
    let active = theme_id();
    for (n, id) in ThemeId::ALL.into_iter().enumerate() {
        let c = theme_of(id);
        let on = id == active;
        let x = x0 + n as f32 * (CARD_W + CARD_GAP);
        fill_round_rect(fb, x, CARD_Y, CARD_W, CARD_H, 8.0, c.panel);
        let (w, col) = if on { (2.0, t.fg) } else { (1.0, t.faint) };
        stroke_round_rect(fb, x, CARD_Y, CARD_W, CARD_H, 8.0, w, col);
        // The theme's eight colours as the brand keys, four and four.
        for (i, k) in c.brand.iter().enumerate() {
            let (kx, ky) = (x + 14.0 + (i % 4) as f32 * 28.0, CARD_Y + 20.0 + (i / 4) as f32 * 18.0);
            fill_round_rect(fb, kx, ky, 24.0, 13.0, 3.0, *k);
        }
        // Its light: the backlight it asks for, then the level of the pad LEDs.
        let track_w = CARD_W - 28.0;
        fill_round_rect(fb, x + 14.0, CARD_Y + 72.0, track_w, 5.0, 2.0, c.faint);
        fill_round_rect(fb, x + 14.0, CARD_Y + 72.0, (track_w * c.light.backlight).max(4.0), 5.0, 2.0, c.fg);
        for lvl in 0..3u8 {
            let col = if lvl < c.light.led_level { c.fg } else { c.faint };
            fill_round_rect(fb, x + 14.0 + lvl as f32 * 16.0, CARD_Y + 90.0, 12.0, 12.0, 3.0, col);
        }
        text(
            fb,
            c.name,
            (x + CARD_W / 2.0) as i32,
            (CARD_Y + 128.0) as i32,
            FontSize::Medium,
            if on { t.fg } else { t.dim },
            Align::Center,
        );
    }
}

fn theme_dials(fb: &mut FrameBuffer480x272) {
    // The eight encoders in this theme's colours (values only suggest a dial).
    const V: [f32; 8] = [0.35, 0.7, 0.5, 0.85, 0.45, 0.75, 0.3, 0.6];
    for (i, v) in V.into_iter().enumerate() {
        let (cx, cy) = (60.0 + (i % 4) as f32 * 120.0, 96.0 + (i / 4) as f32 * 66.0);
        dial(fb, cx, cy, 20.0, v, knob(i));
    }
    // The track colours of this theme.
    for i in 0..8 {
        fill_circle(fb, 240.0 + (i as f32 - 3.5) * 40.0, 224.0, 8.0, track_color(i));
    }
}
