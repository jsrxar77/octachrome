//! Real-time audio output through cpal: CoreAudio on macOS, ALSA on the Pi.
//!
//! The sequencer's clock runs inside the audio callback, so when the audio
//! device goes away (the MK3 resets, a cable is pulled) the sequencer stops
//! with it: PLAY lights up and nothing moves. [`Audio`] therefore owns the
//! engine and reopens the stream whenever it fails, instead of leaving the
//! machine silent until it is restarted.
//!
//! Environment:
//! * `OP3_AUDIO_DEVICE` — substring of the output device name. Default:
//!   the Maschine MK3's own interface if present, else the system default.
//! * `OP3_BUFFER` — frames per callback, default 128 (try 64 on a Pi 5).

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use op3_engine::Engine;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const SCRATCH: usize = 4096;
/// How often a missing or failed stream is tried again.
const RETRY: Duration = Duration::from_secs(1);
/// After a running stream dies the device has usually just reset; reopening it
/// at once makes it fail again, so wait this long first.
const SETTLE: Duration = Duration::from_secs(3);

pub struct AudioOut {
    pub stream: cpal::Stream,
    pub device: String,
    pub channels: u16,
    pub buffer: String,
}

impl AudioOut {
    fn describe(&self) -> String {
        format!(
            "audio: {} · {} ch · 48 kHz · buffer {}",
            self.device, self.channels, self.buffer
        )
    }
}

/// Keeps one stream alive: starts it, notices when it has failed, and starts it
/// again. The stream type is generic so the retry logic can be tested without a
/// sound card.
pub struct Keeper<S> {
    failed: Arc<AtomicBool>,
    current: Option<S>,
    last_try: Option<Instant>,
    last_error: Option<String>,
    /// No attempt before this time (set when a running stream dies).
    not_before: Option<Instant>,
}

/// What a call to [`Keeper::tick`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    /// A stream is running now (first start, or after a failure).
    Started,
    /// Starting it failed with this message (reported once per distinct message).
    Failed(String),
}

impl<S> Keeper<S> {
    pub fn new() -> Self {
        Self {
            failed: Arc::new(AtomicBool::new(false)),
            current: None,
            last_try: None,
            last_error: None,
            not_before: None,
        }
    }

    /// The flag a stream's error callback sets when it dies.
    pub fn failed_flag(&self) -> Arc<AtomicBool> {
        self.failed.clone()
    }

    pub fn current(&self) -> Option<&S> {
        self.current.as_ref()
    }

    /// Call every frame. Starts the stream if there is none, or drops and
    /// restarts it if it failed; it tries at most once per [`RETRY`].
    pub fn tick(
        &mut self,
        now: Instant,
        start: impl FnOnce(&Arc<AtomicBool>) -> Result<S>,
    ) -> Option<Change> {
        if self.failed.swap(false, Relaxed) {
            // Release the dead stream (and the device) before opening another.
            self.current = None;
            self.last_try = None;
            self.not_before = Some(now + SETTLE);
        }
        if self.current.is_some() || self.not_before.is_some_and(|t| now < t) {
            return None;
        }
        if self.last_try.is_some_and(|t| now.duration_since(t) < RETRY) {
            return None;
        }
        self.last_try = Some(now);
        match start(&self.failed) {
            Ok(s) => {
                self.current = Some(s);
                self.last_error = None;
                Some(Change::Started)
            }
            Err(e) => {
                let msg = format!("{e:#}");
                if self.last_error.as_deref() == Some(&msg) {
                    None
                } else {
                    self.last_error = Some(msg.clone());
                    Some(Change::Failed(msg))
                }
            }
        }
    }
}

impl<S> Default for Keeper<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine and its audio stream. Call [`Audio::keep_alive`] every frame.
pub struct Audio {
    engine: Arc<Mutex<Engine>>,
    keeper: Keeper<AudioOut>,
}

impl Audio {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
            keeper: Keeper::new(),
        }
    }

    /// Opens the stream, reopens it after a failure, and returns a line to
    /// print when something changed.
    pub fn keep_alive(&mut self) -> Option<String> {
        let engine = self.engine.clone();
        let change = self.keeper.tick(Instant::now(), |failed| start(engine, failed.clone()))?;
        Some(match change {
            Change::Started => self.keeper.current().map(AudioOut::describe)?,
            Change::Failed(e) => format!("audio: {e}; trying again"),
        })
    }
}

fn start(engine: Arc<Mutex<Engine>>, failed: Arc<AtomicBool>) -> Result<AudioOut> {
    let host = cpal::default_host();
    let want = std::env::var("OP3_AUDIO_DEVICE").ok();
    let mut devices: Vec<cpal::Device> = host
        .output_devices()
        .context("listing audio outputs")?
        .collect();
    let pick = |name: &str| {
        devices
            .iter()
            .position(|d| d.name().is_ok_and(|n| n.contains(name)))
    };
    let device = match want.as_deref().and_then(pick).or_else(|| pick("Maschine")) {
        Some(i) => devices.swap_remove(i),
        None => host
            .default_output_device()
            .ok_or_else(|| anyhow!("no audio output device"))?,
    };
    let name = device.name().unwrap_or_else(|_| "?".into());
    let default = device
        .default_output_config()
        .context("querying output config")?;
    let channels = default.channels().max(2);
    let frames: u32 = std::env::var("OP3_BUFFER")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(128);
    let mut config = cpal::StreamConfig {
        channels,
        sample_rate: cpal::SampleRate(op3_cartridge::SAMPLE_RATE as u32),
        buffer_size: cpal::BufferSize::Fixed(frames),
    };
    // Ask for a fixed small buffer only if the device says it can do it.
    let fixed_ok = device
        .supported_output_configs()
        .map(|mut it| {
            it.any(|c| {
                c.channels() == channels
                    && c.min_sample_rate().0 <= 48_000
                    && c.max_sample_rate().0 >= 48_000
                    && matches!(c.buffer_size(), cpal::SupportedBufferSize::Range { min, max } if *min <= frames && frames <= *max)
            })
        })
        .unwrap_or(false);
    if !fixed_ok {
        eprintln!("audio: {name} can't do a fixed {frames}-frame buffer; using its default");
        config.buffer_size = cpal::BufferSize::Default;
    }
    let stream = match default.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, engine, failed),
        cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, engine, failed),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, engine, failed),
        other => Err(anyhow!("unsupported sample format {other:?}")),
    }?;
    stream.play().context("starting audio stream")?;
    let buffer = match config.buffer_size {
        cpal::BufferSize::Fixed(n) => format!("{n} frames"),
        cpal::BufferSize::Default => "default".into(),
    };
    Ok(AudioOut {
        stream,
        device: name,
        channels,
        buffer,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    engine: Arc<Mutex<Engine>>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let ch = config.channels as usize;
    let mut l = vec![0.0f32; SCRATCH];
    let mut r = vec![0.0f32; SCRATCH];
    let cb = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
        // Only this callback ever locks the engine while a stream is alive, so
        // the lock is never contended; if it somehow is, play silence.
        let Ok(mut engine) = engine.try_lock() else {
            data.fill(T::from_sample(0.0));
            return;
        };
        for chunk in data.chunks_mut(SCRATCH * ch) {
            let frames = chunk.len() / ch;
            let (cl, cr) = (&mut l[..frames], &mut r[..frames]);
            engine.render(cl, cr);
            for (f, frame) in chunk.chunks_mut(ch).enumerate() {
                for (c, s) in frame.iter_mut().enumerate() {
                    // Main L/R, mirrored to headphones (ch 3/4) on the MK3.
                    let v = if c % 2 == 0 { cl[f] } else { cr[f] };
                    *s = T::from_sample(v);
                }
            }
        }
    };
    device
        .build_output_stream(
            config,
            cb,
            move |e| {
                eprintln!("audio stream error: {e}");
                failed.store(true, Relaxed);
            },
            None,
        )
        .context("building audio stream")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    // F-A06 (docs/features.md)
    #[test]
    fn starts_once_and_then_leaves_it_alone() {
        let (mut k, t0) = (Keeper::<u32>::new(), Instant::now());
        assert_eq!(k.tick(t0, |_| Ok(1)), Some(Change::Started));
        for ms in [10, 2000, 60_000] {
            assert_eq!(k.tick(at(t0, ms), |_| panic!("must not restart")), None);
        }
        assert_eq!(k.current(), Some(&1));
    }

    // F-A06
    #[test]
    fn restarts_after_the_stream_fails() {
        let (mut k, t0) = (Keeper::<u32>::new(), Instant::now());
        k.tick(t0, |_| Ok(1));
        k.failed_flag().store(true, Relaxed); // the error callback of the dead stream
        assert_eq!(k.tick(at(t0, 100), |_| panic!("opened too soon")), None, "noticed, settling");
        assert_eq!(k.tick(at(t0, 3200), |_| Ok(2)), Some(Change::Started));
        assert_eq!(k.current(), Some(&2), "the dead stream was replaced");
    }

    // F-A06
    #[test]
    fn waits_for_the_device_to_settle_before_reopening_it() {
        let (mut k, t0) = (Keeper::<u32>::new(), Instant::now());
        k.tick(t0, |_| Ok(1));
        k.failed_flag().store(true, Relaxed);
        // Right after the failure, and for the settle time, it does not touch the device.
        for ms in [0, 100, 1000, 2900] {
            assert_eq!(k.tick(at(t0, 100 + ms), |_| panic!("opened too soon")), None, "{ms} ms");
        }
        assert_eq!(k.current(), None);
        assert_eq!(k.tick(at(t0, 3200), |_| Ok(2)), Some(Change::Started));
    }

    // F-A06
    #[test]
    fn keeps_trying_while_the_device_is_gone_but_not_every_frame() {
        let (mut k, t0) = (Keeper::<u32>::new(), Instant::now());
        k.tick(t0, |_| Ok(1));
        k.failed_flag().store(true, Relaxed);
        let mut attempts = 0;
        let mut told = 0;
        // 8 seconds at 60 fps with the device missing: after the settle time,
        // about one attempt a second.
        for frame in 0..480u64 {
            let r = k.tick(at(t0, 100 + frame * 16), |_| {
                attempts += 1;
                Err(anyhow!("no audio output device"))
            });
            if let Some(Change::Failed(_)) = r {
                told += 1;
            }
        }
        assert!((4..=6).contains(&attempts), "{attempts} attempts in 8 s");
        assert_eq!(told, 1, "the same failure is reported once");
        // The device comes back.
        assert_eq!(k.tick(at(t0, 20_000), |_| Ok(3)), Some(Change::Started));
        assert_eq!(k.current(), Some(&3));
    }

    // F-A06
    #[test]
    fn a_new_failure_message_is_reported() {
        let (mut k, t0) = (Keeper::<u32>::new(), Instant::now());
        let f = |e: &str, _: &Arc<AtomicBool>| -> Result<u32> { Err(anyhow!(e.to_string())) };
        assert!(matches!(k.tick(t0, |x| f("one", x)), Some(Change::Failed(m)) if m == "one"));
        assert!(matches!(k.tick(at(t0, 1100), |x| f("two", x)), Some(Change::Failed(m)) if m == "two"));
    }
}
