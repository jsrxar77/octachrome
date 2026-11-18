//! Reading a WAV file into mono samples at the engine's rate. Not real-time: it runs when an
//! instrument is created or a sample is chosen, never on the audio thread.

use octachrome_cartridge::SAMPLE_RATE;

/// The longest sample kept: ten seconds.
pub const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 10;

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// A WAV file (PCM 8, 16, 24 or 32 bit, or 32-bit float; any number of channels) as mono samples
/// at 48 kHz in -1..1, or `None` if it is not one this can read. Longer than ten seconds is cut.
pub fn parse(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let (mut fmt, mut data) = (None, None);
    let mut i = 12;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32_at(bytes, i + 4)? as usize;
        let body = bytes.get(i + 8..(i + 8 + len).min(bytes.len()))?;
        match id {
            b"fmt " => fmt = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        i += 8 + len + (len & 1);
    }
    let (fmt, data) = (fmt?, data?);
    let mut tag = u16_at(fmt, 0)?;
    let channels = u16_at(fmt, 2)? as usize;
    let rate = u32_at(fmt, 4)?;
    let bits = u16_at(fmt, 14)? as usize;
    if tag == 0xFFFE {
        tag = u16_at(fmt, 24)?; // extensible: the real format is the start of the sub-format
    }
    if channels == 0 || rate == 0 || !matches!((tag, bits), (1, 8 | 16 | 24 | 32) | (3, 32)) {
        return None;
    }
    let bytes_per = bits / 8;
    let frame = bytes_per * channels;
    let frames = data.len() / frame;
    let sample = |at: usize| -> f32 {
        let b = &data[at..at + bytes_per];
        match (tag, bits) {
            (1, 8) => (b[0] as f32 - 128.0) / 128.0,
            (1, 16) => i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0,
            (1, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
            (1, _) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0,
            _ => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
        }
    };
    let mono: Vec<f32> = (0..frames)
        .map(|f| (0..channels).map(|c| sample(f * frame + c * bytes_per)).sum::<f32>() / channels as f32)
        .map(|s| if s.is_finite() { s.clamp(-1.0, 1.0) } else { 0.0 })
        .collect();
    if mono.is_empty() {
        return None;
    }
    // to the engine's rate, by linear interpolation
    let ratio = rate as f64 / SAMPLE_RATE as f64;
    let n = ((mono.len() as f64 / ratio) as usize).min(MAX_SAMPLES);
    Some(
        (0..n)
            .map(|k| {
                let p = k as f64 * ratio;
                let (i0, f) = (p as usize, (p - p.floor()) as f32);
                let a = mono[i0.min(mono.len() - 1)];
                let b = mono[(i0 + 1).min(mono.len() - 1)];
                a + (b - a) * f
            })
            .collect(),
    )
}

/// A 16-bit mono WAV file of `samples` at `rate`, for tests and tools.
pub fn encode_16(samples: &[f32], rate: u32) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(&data);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    // F-A16
    #[test]
    fn a_wav_is_read_and_brought_to_the_engines_rate() {
        let tone: Vec<f32> = (0..2400).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        let at_24k = parse(&encode_16(&tone, 24_000)).expect("reads");
        assert_eq!(at_24k.len(), 4800, "half the rate, twice the samples");
        assert!((at_24k[100] - tone[50]).abs() < 0.01);
        let same = parse(&encode_16(&tone, 48_000)).expect("reads");
        assert_eq!(same.len(), 2400);
    }

    // F-A16
    #[test]
    fn what_is_not_a_wav_is_refused() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"RIFF....WAVE").is_none());
        let good = encode_16(&[0.1; 100], 48_000);
        assert!(parse(&good[..40]).is_none(), "a cut file");
        let mut bad = good.clone();
        bad[20] = 9; // a format this does not know
        assert!(parse(&bad).is_none());
        assert!(parse(&[0u8; 200]).is_none());
    }
}
