//! Minimal 16-bit stereo WAV writer for offline renders.

use std::io::Write;

pub fn write_wav(
    path: &std::path::Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
) -> std::io::Result<()> {
    let n = left.len().min(right.len());
    let data_len = (n * 4) as u32;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&2u16.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&(sample_rate * 4).to_le_bytes())?;
    f.write_all(&4u16.to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for i in 0..n {
        for s in [left[i], right[i]] {
            f.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())?;
        }
    }
    f.flush()
}

/// One line per `window` samples: RMS and peak in dBFS of the L/R mix, so a
/// run can be checked for "does it sound, and when" without listening.
pub fn loudness_report(l: &[f32], r: &[f32], window: usize) -> String {
    let db = |x: f32| if x < 1e-6 { f32::NEG_INFINITY } else { 20.0 * x.log10() };
    let mut out = String::from("   time    rms    peak  (dBFS)\n");
    for (i, (cl, cr)) in l.chunks(window).zip(r.chunks(window)).enumerate() {
        let (mut sum, mut peak) = (0.0f64, 0.0f32);
        for (a, b) in cl.iter().zip(cr) {
            let m = (a + b) * 0.5;
            sum += (m * m) as f64;
            peak = peak.max(a.abs()).max(b.abs());
        }
        let rms = (sum / cl.len().max(1) as f64).sqrt() as f32;
        let bar = if rms.is_finite() { "#".repeat(((db(rms) + 60.0).max(0.0) / 2.0) as usize) } else { String::new() };
        out += &format!("{:6.1}s {:6.1} {:7.1}  {bar}\n", i as f32 * window as f32 / 48000.0, db(rms), db(peak));
    }
    out
}
