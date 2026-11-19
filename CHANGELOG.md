# Changelog

All notable changes are written here, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the versions follow [Semantic Versioning](https://semver.org/).

To publish a version: move what is under *Unreleased* to a new `## [x.y.z] - date` section, bump `version` in the root `Cargo.toml`, and push the tag `vx.y.z`; the *Release* workflow builds the binaries and uses that section as the notes of the release, published in the public repository `octachrome-releases`.

## [Unreleased]

## [0.1.0] - 2026-11-19

First public version. What the project is today, as [docs/roadmap.md](docs/roadmap.md) keeps it; the screens, the colour order and the buttons have been checked on a real MK3, the rest of the hardware side has not.

### Added
- A sequencer inspired by the OP-XY: per-track polymeter, Step Components (ratchet, chance, every, velocity, parameter locks), the Brain harmony track, scenes and a song (ARRANGER), a mixer with compressor, EQ and two sends, REC, TAP and the metronome, MIDI in and MIDI clock out.
- Eight instruments as cartridges: drums, KickSyn, FmBass, resonator, sampler, granular, a 24-engine macro oscillator (from Mutable Instruments' Plaits) and a MIDI out; presets for each.
- Eight songs that come with the project (house, techno, garage, dnb, trance, amapiano, ambient, lo-fi) and eight places for your own projects (FILE).
- The Maschine MK3 as the only control surface: pads, eight knobs, Smart Strip, 4D encoder, every light and both 480×272 screens, in three themes (matte, glossy, chrome).
- A simulator, a test bench and an interactive twin in HTML that runs in a browser.
- The manual and the home page, published with GitHub Pages from `guide/`, `twin/` and `docs/buttons.md`; the twin and the manual draw the real panel of the MK3.

### Known limits
- Not verified on hardware: the LED palette beyond red and orange, the CPU load on a Raspberry Pi 5, MIDI in on Linux.

[Unreleased]: https://github.com/jsrxar77/octachrome/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jsrxar77/octachrome-releases/releases/tag/v0.1.0
