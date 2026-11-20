# Contributing to Octachrome

Thank you for looking. Octachrome is a groovebox in Rust for a Maschine MK3 ([README](README.md)); it is built by a small team and a lot of its design is decided on purpose, so a short read of this page saves everyone time.

## Build and test

You need a stable Rust (`rustup`) and a C++ compiler (the macro instrument compiles the Plaits code): `xcode-select --install` on macOS, `sudo apt install build-essential libasound2-dev` on Linux or the Pi.

```sh
cargo test --workspace            # everything; no hardware needed
cargo run --release -- sim -s 8   # a simulated MK3: writes out/out.wav and captures of both screens
open twin/octachrome-twin.html    # the interactive twin, in a browser
```

The tests are the contract: [docs/features.md](docs/features.md) lists every verifiable behaviour with its ID (`F-S01`…) and the test that checks it, and a test carries the ID of the behaviour it checks. A change that adds or changes behaviour adds or changes its row and its test.

## Where to start

* **What it is and how it is built:** [docs/architecture.md](docs/architecture.md), then [docs/roadmap.md](docs/roadmap.md) for what is done and what is next.
* **Adding an instrument:** a type that implements `InstrumentCartridge` (`crates/octachrome-cartridge`), loaded into a slot; the rules (at most eight parameters, a drawing that explains the sound) are in [docs/instruments.md](docs/instruments.md).
* **A change to a screen or a button:** read [docs/design.md](docs/design.md) first; try it in `twin/octachrome-twin.html` before the Rust, and update the design treatise in the same pull request. What each button does is in [docs/buttons.md](docs/buttons.md).
* **A new song or a change to one of the eight:** they are in `crates/octachrome/src/factory.rs`, and the twin plays a copy of them that a test writes (`OCTACHROME_WRITE_TWIN=1 cargo test -p octachrome --test twin_pieces`). Voice a song by rendering each track alone and measuring it, as [docs/instruments.md §9](docs/instruments.md) describes.

## The rules of the project

1. **Everything in the repository is in English** (code, comments, documents, commit messages).
2. **`docs/` holds exactly six documents** (`architecture`, `design`, `buttons`, `instruments`, `features`, `roadmap`) and they are kept current with the change, not after it.
3. **No GPL code and no GPL dependencies.** The MK3 protocol is implemented from facts only; Encdr, autotel/maschine-mk3-linux and Deckard are GPL and are not to be copied or depended on. Surge XT and Vital are GPL too. Mutable Instruments code (MIT) can be used, without their module names.
4. **A button is lit if and only if it has a function in the current context.** A new function brings its light in the same change.
5. **The audio thread does not allocate, lock or block.** The sequencer is deterministic: the same project gives the same output, to the sample.
6. **Both MK3 screens animate**; never assume one is still.

## Pull requests

* One idea per pull request, with its tests and its documents.
* `cargo test --workspace` passes, and `sh .claude/skills/octachrome-docs/check.sh` (the six documents, their links and their test IDs) too.
* Say what you could not test on a real MK3: part of the hardware is still unverified ([roadmap](docs/roadmap.md#hardware-status)).

The repository is private and contributions are by invitation. By contributing you agree that your work belongs to the Owner of Octachrome and is licensed as the rest of the code is (see [LICENSE](LICENSE)), including its use in the programs that are sold.
