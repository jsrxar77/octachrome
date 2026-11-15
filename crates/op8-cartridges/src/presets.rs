//! The presets that come with the instruments: eight knobs' values each, read from `presets.txt` (the one text the program,
//! the tests and the twin share). BROWSER shows them in the top two rows of pads.

/// A preset: its name and the eight knobs (`None`: leave that knob where it is).
pub type Preset = (String, [Option<f32>; 8]);

const TEXT: &str = include_str!("../presets.txt");

/// The text itself, for the twin's copy to be compared with.
pub fn text() -> &'static str {
    TEXT
}

/// The presets of an instrument (by the name it gives in `info()`), in pad order, at most eight.
pub fn factory(instrument: &str) -> Vec<Preset> {
    TEXT.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let mut parts = l.split('|').map(str::trim);
            let (inst, name, values) = (parts.next()?, parts.next()?, parts.next()?);
            if inst != instrument {
                return None;
            }
            let mut knobs = [None; 8];
            for (k, v) in values.split_whitespace().take(8).enumerate() {
                knobs[k] = v.parse::<f32>().ok().map(|x| x.clamp(0.0, 1.0));
            }
            Some((name.to_string(), knobs))
        })
        .take(8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{builtin_names, create};
    use op8_cartridge::NoteEvent;

    /// Play a C3 on an instrument with a preset and return `n` samples.
    fn render(name: &str, p: &Preset, n: usize) -> Vec<f32> {
        let mut c = create(name).unwrap();
        for (i, v) in p.1.iter().enumerate() {
            if let Some(v) = v {
                c.set_parameter(i, *v);
            }
        }
        let (mut out, mut l, mut r) = (Vec::new(), vec![0.0f32; 128], vec![0.0f32; 128]);
        for b in 0..n / 128 {
            let ev = if b == 0 { vec![NoteEvent::on(0, 48, 1.0)] } else { vec![] };
            c.process(&mut l, &mut r, &ev);
            out.extend_from_slice(&l);
        }
        out
    }

    // F-A19: the twin plays the same presets, from a copy of the same text.
    #[test]
    fn the_twin_carries_the_same_text() {
        let twin = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../twin/op8-twin.html")).unwrap();
        assert!(twin.contains(super::text()), "twin/op8-twin.html has a copy of presets.txt (paste it into FACTORY_PRESETS_TEXT)");
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }
    fn crossings(x: &[f32]) -> usize {
        x.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count()
    }

    // F-A19
    #[test]
    fn every_instrument_has_eight_presets_that_play_and_differ() {
        for name in builtin_names().iter().filter(|n| **n != "MidiOutBridge") {
            let presets = factory(name);
            assert_eq!(presets.len(), 8, "{name} has eight presets");
            let mut sigs = Vec::new();
            for p in &presets {
                assert!(p.1.iter().flatten().all(|v| (0.0..=1.0).contains(v)));
                let out = render(name, p, 24_000);
                assert!(out.iter().all(|x| x.is_finite()), "{name}/{}: finite", p.0);
                let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(peak > 0.03 && peak < 1.3, "{name}/{}: peak {peak}", p.0);
                sigs.push((p.0.clone(), (rms(&out) * 1000.0) as i32, crossings(&out)));
            }
            for (i, a) in sigs.iter().enumerate() {
                for b in &sigs[i + 1..] {
                    assert!(a.1 != b.1 || a.2 != b.2, "{name}: {} and {} sound the same", a.0, b.0);
                }
            }
            let names: std::collections::HashSet<_> = presets.iter().map(|p| &p.0).collect();
            assert_eq!(names.len(), 8, "{name}: eight different names");
        }
    }

    fn by(name: &str, preset: &str) -> Preset {
        factory(name).into_iter().find(|p| p.0 == preset).unwrap()
    }

    // F-A19: the names say what they sound like.
    #[test]
    fn the_names_are_true() {
        // kick: deep is lower than click, boom lasts longer than tight
        let (deep, click) = (render("KickSyn", &by("KickSyn", "deep"), 24_000), render("KickSyn", &by("KickSyn", "click"), 24_000));
        assert!(crossings(&deep[..4800]) < crossings(&click[..4800]), "deep is lower than click");
        let tail = |x: &[f32]| rms(&x[9_600..19_200]);
        let (boom, tight) = (render("KickSyn", &by("KickSyn", "boom"), 24_000), render("KickSyn", &by("KickSyn", "tight"), 24_000));
        assert!(tail(&boom) > tail(&tight) * 2.0, "boom rings on, tight does not");
        // bass: metal is brighter than sub
        let (sub, metal) = (render("FmBass", &by("FmBass", "sub"), 24_000), render("FmBass", &by("FmBass", "metal"), 24_000));
        assert!(crossings(&metal) > crossings(&sub), "metal is brighter than sub");
        // resonator: gong rings longer than muted (the knob `damping` is the other way round: high keeps the ring)
        let (gong, muted) = (render("resonator", &by("resonator", "gong"), 48_000), render("resonator", &by("resonator", "muted"), 48_000));
        assert!(rms(&gong[24_000..48_000]) > rms(&muted[24_000..48_000]) * 2.0, "gong outlasts muted");
        // drums: boomy lasts longer than tight, on the kick it starts on
        let (boomy, tight) = (render("drums", &by("drums", "boomy"), 24_000), render("drums", &by("drums", "tight"), 24_000));
        assert!(rms(&boomy[9_600..19_200]) > rms(&tight[9_600..19_200]), "boomy lasts longer than tight");
        // sampler: the tick is short, the pad goes on
        let (tick, pad) = (render("sampler", &by("sampler", "tick"), 48_000), render("sampler", &by("sampler", "pad"), 48_000));
        assert!(rms(&tick[12_000..48_000]) < 0.002, "the tick is over");
        assert!(rms(&pad[24_000..48_000]) > 0.01, "the pad is still going");
    }
}
