//! Real-time audio output through cpal: CoreAudio on macOS, ALSA on the Pi.
//!
//! Environment:
//! * `OP3_AUDIO_DEVICE` — substring of the output device name. Default:
//!   the Maschine MK3's own interface if present, else the system default.
//! * `OP3_BUFFER` — frames per callback, default 128 (try 64 on a Pi 5).

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use op3_engine::Engine;

const SCRATCH: usize = 4096;

pub struct AudioOut {
    pub stream: cpal::Stream,
    pub device: String,
    pub channels: u16,
    pub buffer: String,
}

pub fn start(engine: Engine) -> Result<AudioOut> {
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
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, engine),
        cpal::SampleFormat::I32 => build_stream::<i32>(&device, &config, engine),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, engine),
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
    mut engine: Engine,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let ch = config.channels as usize;
    let mut l = vec![0.0f32; SCRATCH];
    let mut r = vec![0.0f32; SCRATCH];
    let cb = move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
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
        .build_output_stream(config, cb, |e| eprintln!("audio stream error: {e}"), None)
        .context("building audio stream")
}
