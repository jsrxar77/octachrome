# OP-8

- **To continue the work, read `docs/architecture.md` first** (what OP-8 is, how it is built, decisions taken, how to build, run and test), then `docs/roadmap.md` (status and next steps).
- The product owner, Javi, writes in Rioplatense Spanish: **answer him in that language.** Everything in the repository (code, comments, documents) is in English.
- `docs/` holds exactly six documents: `architecture.md`, `design.md`, `buttons.md`, `instruments.md`, `features.md` and `roadmap.md`. Do not add a seventh. Keep them current with the `op8-docs` skill (`.claude/skills/op8-docs/`).
- Everything seen on screen or handled with the MK3 follows `docs/design.md` (UX/UI treatise). Read it before touching `crates/op8-ui`, `crates/op8/src/controller.rs` or `twin/op8-twin.html`.
- A UI change is tried first in `twin/op8-twin.html`, and `docs/design.md` is updated in the same PR.
- What each button does: `docs/buttons.md`. What the system must do and how each behaviour is tested: `docs/features.md`.
- No GPL dependencies and no GPL code (Encdr, autotel/maschine-mk3-linux and Deckard are GPL): the MK3 protocol is implemented separately, from facts only.
- Instruments are the centre of the product: follow `docs/instruments.md` (at most 8 parameters, a drawing that explains the sound).
- Mutable Instruments code (STM32, MIT) can be used, but without its module names. Surge XT and Vital are GPL: do not include without a decision from Javi.
- The brand mark (eight rainbow buttons forming an 8 on its side, in the order red, orange, yellow, green, blue, indigo, violet, white) is defined in `docs/design.md` §2.3, with its colours in each theme (`Theme.brand`). Respect it: always eight, always that order, in every theme.
- Both MK3 screens animate. Never assume either one is still.
