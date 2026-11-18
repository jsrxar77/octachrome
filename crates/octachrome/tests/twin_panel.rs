//! The twin (twin/octachrome-twin.html) draws the front panel of the unit where the unit has it, from a copy of the positions (UNIT_PANEL) in
//! `guide/mk3.js` (the one source of every picture of the unit, docs/design.md §2.4). This test checks that the copy is the same;
//! `OCTACHROME_WRITE_TWIN=1 cargo test -p octachrome --test twin_panel` writes it. It needs `node`, which every machine that builds
//! the guide already has.
use std::path::Path;
use std::process::Command;

const BEGIN: &str = "// <panel>\n";
const END: &str = "// </panel>\n";

/// The positions of mk3.js, rounded to a tenth, as JSON: the body, the screens, the knobs, the pads, the encoder, the strip and every button.
const DUMP: &str = "global.window = {}; require(process.argv[1]); const M = window.MK3, r = a => a.map(v => Math.round(v * 10) / 10);
console.log(JSON.stringify({ body: r(M.body), screens: M.screens.map(r), knobs: M.knobs.map(r), pads: M.pads.map(r), encoder: r(M.encoder), strip: r(M.strip), btn: M.BTN.map(b => [b[0], ...r(b.slice(1, 5))]) }));";

fn block(root: &Path) -> String {
    let out = Command::new("node").arg("-e").arg(DUMP).arg(root.join("guide/mk3.js")).output().expect("node is needed to read guide/mk3.js");
    assert!(out.status.success(), "node failed: {}", String::from_utf8_lossy(&out.stderr));
    format!("{BEGIN}const UNIT_PANEL = {};\n{END}", String::from_utf8(out.stdout).unwrap().trim())
}

// F-F21: the twin places every part of the unit where guide/mk3.js has it (the manual's map is drawn from the same file), so the
// twin and the manual show the same unit.
#[test]
fn the_twin_has_the_panel_of_the_manual() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("twin/octachrome-twin.html");
    let twin = std::fs::read_to_string(&path).unwrap();
    let (a, b) = (twin.find(BEGIN).expect("the twin has a panel block"), twin.find(END).expect("and its end"));
    let have = &twin[a..b + END.len()];
    let want = block(&root);
    if std::env::var_os("OCTACHROME_WRITE_TWIN").is_some() && have != want {
        std::fs::write(&path, format!("{}{}{}", &twin[..a], want, &twin[b + END.len()..])).unwrap();
        return;
    }
    assert!(have == want, "the twin's panel is not the one of guide/mk3.js: run OCTACHROME_WRITE_TWIN=1 cargo test -p octachrome --test twin_panel");
}
