//! Scripts and captures (docs/features.md §G).
use octachrome::bench::Bench;
use octachrome::script::Script;

// F-G01: the built-in demo script parses and ends at 6.8 s.
#[test]
fn f_g01_demo_script_parses() {
    let s = Script::parse(include_str!("../scripts/demo.ocs")).expect("demo.ocs parses");
    assert_eq!(s.end_ms(), 6800);
}

// F-G02: a broken script names the line.
#[test]
fn f_g02_errors_name_the_line() {
    for bad in ["0.1 btn nonsense", "0.1 pad 16 strike", "0.1 knob 8 1", "0.1 btn screen 9"] {
        let text = format!("0.0 btn play\n{bad}\n");
        let err = Script::parse(&text).err().unwrap_or_else(|| panic!("{bad:?} parsed"));
        assert!(format!("{err:#}").contains("line 2"), "{bad:?}: {err:#}");
    }
}

// F-G03: `shot` writes both screens as PNG into OCTACHROME_BENCH_DIR.
#[test]
fn f_g03_shot_writes_png() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bench-g03");
    let _ = std::fs::remove_dir_all(&dir);
    // Only this test file uses the variable, and the test runs alone in its own binary's threads.
    std::env::set_var("OCTACHROME_BENCH_DIR", &dir);
    let mut b = Bench::demo();
    b.load_script("0.1 btn play\n0.5 shot probe\n");
    b.run_script();
    for side in ["left", "right"] {
        let p = dir.join(format!("probe-{side}.png"));
        let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert_eq!(&bytes[1..4], b"PNG");
    }
    std::env::remove_var("OCTACHROME_BENCH_DIR");
}
