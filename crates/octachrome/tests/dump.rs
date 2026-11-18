//! Screen dump (`OCTACHROME_DUMP_DIR` in `octachrome run`): the app writes both screens as PNG.
use octachrome::bench::Bench;

#[test]
fn dump_writes_both_screens_as_png() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("dump-test");
    let _ = std::fs::remove_dir_all(&dir);
    let mut b = Bench::demo();
    b.app.dump_dir = Some(dir.clone());
    b.run_ms(20);
    for name in ["left", "right"] {
        let bytes = std::fs::read(dir.join(format!("{name}.png")))
            .unwrap_or_else(|e| panic!("{name}.png: {e}"));
        assert_eq!(&bytes[1..4], b"PNG", "{name} is not a PNG");
        assert_eq!(&bytes[16..20], &480u32.to_be_bytes(), "{name} width");
        assert_eq!(&bytes[20..24], &272u32.to_be_bytes(), "{name} height");
    }
}
