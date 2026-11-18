//! The twin (twin/octachrome-twin.html) plays the eight pieces from a copy of them, written here from `factory.rs`: the steps of every pattern,
//! the scenes, the song, what each track carries and how each piece voices its instruments. This test checks that the copy is the
//! same; `OCTACHROME_WRITE_TWIN=1 cargo test -p octachrome --test twin_pieces` writes it. The twin has no parameter locks: they are left out.
use octachrome::factory;
use octachrome_seq::{Brain, Command, Project, Step, StepComponent};

const BEGIN: &str = "// <pieces>\n";
const END: &str = "// </pieces>\n";

fn num(x: f32) -> String {
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-" { "0".into() } else { s }
}

fn brain(b: &Brain) -> String {
    let kinds = ["triad", "seventh", "sus2", "sus4"];
    let prog: Vec<String> = b.progression.iter().map(|c| format!("[{},\"{}\",{}]", c.degree, kinds[c.kind as usize], c.length)).collect();
    format!("[{},{},{},{},[{}]]", b.key, b.scale as u8, b.mode as u8, b.len, prog.join(","))
}

fn step(i: usize, s: &Step) -> String {
    let comps: Vec<String> = s.components.iter().filter_map(|c| match *c {
        StepComponent::Ratchet(n) => Some(format!("[\"ratchet\",{n}]")),
        StepComponent::Chance(n) => Some(format!("[\"chance\",{n}]")),
        StepComponent::Every { n, of } => Some(format!("[\"every\",[{n},{of}]]")),
        StepComponent::Velocity(n) => Some(format!("[\"vel\",{n}]")),
        StepComponent::Transpose(n) => Some(format!("[\"trans\",{n}]")),
        StepComponent::Octave(n) => Some(format!("[\"oct\",{n}]")),
        StepComponent::Hold(n) => Some(format!("[\"hold\",{n}]")),
        StepComponent::Delay(n) => Some(format!("[\"delay\",{n}]")),
        StepComponent::RandomNote(n) => Some(format!("[\"rand\",{n}]")),
        StepComponent::ParamLock { .. } | StepComponent::None => None,
    }).collect();
    format!("[{i},{},{},{},{},[{}]]", s.note, s.velocity, s.gate, s.lanes, comps.join(","))
}

/// The twin's order of the divisions: 1/32, 1/16T, 1/16, 1/8, 1/4.
fn division(d: octachrome_seq::Division) -> u8 {
    [0, 2, 1, 3, 4][d as usize]
}

fn piece(i: usize) -> String {
    let p0 = factory::project(i);
    let mut tracks = Vec::new();
    for t in 0..8 {
        let tr = &p0.tracks[t];
        let mut pats = Vec::new();
        for k in 0..16u8 {
            let mut p: Project = p0.clone();
            p.apply(&Command::SetPattern { track: t as u8, index: k, now: false });
            let x = &p.tracks[t];
            let steps: Vec<String> = x.steps.iter().enumerate().filter(|(_, s)| s.active || s.lanes != 0).map(|(i, s)| step(i, s)).collect();
            if k > 0 && steps.is_empty() {
                pats.push("null".to_string());
            } else {
                pats.push(format!("{{len:{},div:{},kit:{},s:[{}]}}", x.length, division(x.division), x.kit, steps.join(",")));
            }
        }
        while pats.last().map(|s| s == "null").unwrap_or(false) {
            pats.pop();
        }
        tracks.push(format!(
            "{{link:{},pl:[{},{},{},{}],fx:[{},{},{},{}],lfo:[{},{},{},{}],level:{},pan:{},sa:{},sb:{},pats:[{}]}}",
            tr.brain_link, tr.players.mode as u8, tr.players.chord as u8, tr.players.speed, tr.players.octaves,
            tr.fx.kind as u8, tr.fx.a, tr.fx.b, tr.fx.c, tr.lfo.shape as u8, tr.lfo.rate, tr.lfo.depth, tr.lfo.target,
            tr.level, tr.pan, tr.send_a, tr.send_b, pats.join(",")
        ));
    }
    let scenes: Vec<String> = p0.scenes.iter().map(|s| if s.used { format!("{{p:{:?},m:{},b:{}}}", s.pattern, s.mute, brain(&s.brain)) } else { "null".into() }).collect();
    let song: Vec<String> = p0.song.steps[..p0.song.len as usize].iter().map(|s| format!("[{},{}]", s.scene, s.bars)).collect();
    format!(
        "{{name:\"{}\",bpm:{},swing:{},brain:{},tracks:[{}],scenes:[{}],song:{{on:{},steps:[{}]}}}}",
        factory::NAMES[i], num(p0.bpm), num(p0.swing), brain(&p0.brain), tracks.join(","), scenes.join(","), p0.song.on, song.join(",")
    )
}

fn voicing(i: usize) -> (String, String, String) {
    let (names, gain, knobs, kit) = factory::voicing(i);
    let presets = format!("[{:?},{}]", names, num(gain));
    let knobs: Vec<String> = knobs.iter().map(|(t, ks)| format!("{t}:[{}]", ks.iter().map(|(k, v)| format!("[{k},{}]", num(*v))).collect::<Vec<_>>().join(","))).collect();
    let kit: Vec<String> = kit.iter().map(|(s, v)| format!("{s}:[{}]", v.iter().map(|x| num(*x)).collect::<Vec<_>>().join(","))).collect();
    (presets, format!("{{{}}}", knobs.join(",")), format!("{{{}}}", kit.join(",")))
}

fn block() -> String {
    let mut out = String::from(BEGIN);
    out.push_str("const PIECES = [\n");
    for i in 0..8 {
        out.push_str(&format!("  {},\n", piece(i)));
    }
    out.push_str("];\n");
    let v: Vec<_> = (0..8).map(voicing).collect();
    out.push_str(&format!("const PIECE_PRESETS = [{}];\n", v.iter().map(|x| x.0.clone()).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("const PIECE_KNOBS = [{}];\n", v.iter().map(|x| x.1.clone()).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("const PIECE_KIT = [{}];\n", v.iter().map(|x| x.2.clone()).collect::<Vec<_>>().join(", ")));
    out.push_str(END);
    out
}

// F-F12: the twin plays the eight pieces from a copy of them that is the same as the unit's (the steps of every pattern, the scenes,
// the song, what each track carries, the presets, knobs and kit of each piece; not the parameter locks, which the twin does not have).
#[test]
fn the_twin_carries_the_same_pieces() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../twin/octachrome-twin.html");
    let twin = std::fs::read_to_string(&path).unwrap();
    let (a, b) = (twin.find(BEGIN).expect("the twin has a pieces block"), twin.find(END).expect("and its end"));
    let have = &twin[a..b + END.len()];
    let want = block();
    if std::env::var_os("OCTACHROME_WRITE_TWIN").is_some() && have != want {
        std::fs::write(&path, format!("{}{}{}", &twin[..a], want, &twin[b + END.len()..])).unwrap();
        return;
    }
    assert!(have == want, "the twin's pieces are not the unit's: run OCTACHROME_WRITE_TWIN=1 cargo test -p octachrome --test twin_pieces");
}
