//! MIDI out (docs/features.md §M).
use op3::bench::Bench;
use op3_hw::Button;

// F-M01: the MIDI track emits well-formed note-on/note-off messages on one channel.
#[test]
fn f_m01_midi_messages_are_well_formed() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(3000);
    let msgs = b.midi_between(0, 3000);
    assert!(!msgs.is_empty(), "no MIDI at all");
    let channel = msgs[0][0] & 0x0f;
    let mut notes_on = 0;
    for m in &msgs {
        assert!(m.len() == 3, "message of {} bytes", m.len());
        // Notes, plus the CC74 (filter) that the bridge sends on parameter change.
        assert!(
            matches!(m[0] & 0xf0, 0x80 | 0x90 | 0xb0),
            "status {:#x} is not note or CC",
            m[0]
        );
        assert_eq!(m[0] & 0x0f, channel, "messages on more than one channel");
        assert!(m[1] < 128 && m[2] < 128, "data byte out of range: {m:?}");
        if m[0] & 0xf0 == 0xb0 {
            assert_eq!(m[1], 74, "unexpected controller {}", m[1]);
            continue;
        }
        if m[0] & 0xf0 == 0x90 && m[2] > 0 {
            notes_on += 1;
        }
    }
    assert!(notes_on > 0, "no note-on with velocity above zero");
    b.capture("f-m01-midi");
}

// F-M02: a full run of the demo loses no events.
#[test]
fn f_m02_demo_loses_no_events() {
    let mut b = Bench::demo();
    b.press(Button::Play);
    b.run_ms(7000);
    assert_eq!(b.dropped(), 0, "{} events dropped", b.dropped());
}
