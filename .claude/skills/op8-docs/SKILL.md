---
name: op8-docs
description: Maintains the six OP-8 project documents in docs/ (architecture, design, buttons, instruments, features, roadmap). Use whenever code, behaviour, a button, an instrument, a test or the plan changes and the documents must follow, or when asked to add, correct, review or reorganise documentation. It manages only those six files.
---

# OP-8 documentation

`docs/` holds **exactly six documents**, all in English. This skill keeps them true and in step with the code. It does not manage anything else: README, `CLAUDE.md`, code comments and the twin are outside its scope (it may *read* them, and it tells the user when one of them needs a change).

Chat with Javi, the product owner, is in Rioplatense Spanish. Everything written to the repository is English.

## The six documents, and what each one decides

Each fact lives in exactly one of them. If a fact is needed elsewhere, link to it; do not copy it.

| Document | Owns | Never contains |
| :-- | :-- | :-- |
| `architecture.md` | What OP-8 is, the document map, crates, frame flow, audio engine, control, screens pipeline, simulator and scripts, test bench, decisions taken and known limits, how to build/run/test, how to work | Per-button behaviour, colours, the plan |
| `design.md` | The 13 rules, colours, typeface, screen structure, components, motion. **Rules over the code** | What a button does, implementation detail |
| `buttons.md` | What each MK3 button does (Maschine → OP-XY → OP-8) and its status ✓ ◐ ○ | Test cases, the order of work |
| `instruments.md` | What an instrument is, its rules (max 8 parameters, one drawing), the extension mechanism, done instruments, candidates, the Pi budget | Engine internals |
| `features.md` | Verifiable behaviours with IDs, how each is verified, its status, the manual-review list | Design rationale |
| `roadmap.md` | What is done, hardware status and known setup limits, the numbered order of work | Behaviour specs |

## Which document a change touches

| The change | Update |
| :-- | :-- |
| A button gains, loses or changes behaviour | `buttons.md` (status), `features.md` (case), `roadmap.md` (move the item) |
| A new instrument or a change to the contract | `instruments.md`, `architecture.md` §6 if the contract changed, `features.md` |
| A visual or UX change | `design.md` (same PR; Javi approves changes to the treatise) |
| A new crate, thread, protocol or env variable | `architecture.md` |
| A new or changed test, or a behaviour a test now covers | `features.md` (row, ID, status) |
| Something verified or broken on the real MK3 | `roadmap.md` → Hardware status, and the `features.md` row if there is one |
| A decision from Javi | `architecture.md` §10 |
| Work finished or reordered | `roadmap.md` |

## Conventions

- **English, plain and short.** No Spanish in any document, including names of people's wording ("Javi" is fine).
- **Stable anchors.** Other files cite section numbers: `architecture.md` §7 (screens) and §9 (test bench), `design.md` rules 1–13 and §3–§6. Do not renumber. Add new sections at the end of a file or as a subsection.
- **Feature IDs** are `F-<area letter><two digits>` (`F-S01`). Areas: T transport, S sequencer, B Brain, P Punch-in, K knobs, D screens, L LEDs, M MIDI, A audio, G scripts, C settings, H hardware link. Never reuse or renumber an ID; a retired feature is marked, not deleted. Each ID in `features.md` that is not manual review must appear as a comment in a test (`// F-S01`), and every ID in a test must be in `features.md`.
- **Status marks.** `buttons.md` and `roadmap.md`: ✓ done, ◐ partial, ○ to do. `features.md`: `✓` passes, `✗` fails (reason in the notes), `R` manual review. A status is only changed after it has been observed (a test run, or a stated observation on the real MK3), never because it "should" hold.
- **Absolute dates** (`2026-11-13`), never "today" or "last week".
- **Links** between documents are relative (`[design.md](design.md)`), paths to code are in backticks.
- **Truth over tidiness.** If the code and a document disagree, find out which is right. `design.md` rules over the code; for every other document the code and the tests are the evidence.
- **Both screens animate.** Never write that either screen is static.

## Procedure

1. Read the document(s) the change touches, and the code or test that changed. Do not edit from memory.
2. Edit only inside `docs/` and only the six files. Do not create a seventh file there; if content has no home, put it in the closest of the six and say so.
3. Run `sh .claude/skills/op8-docs/check.sh`. It verifies: exactly six documents, no references to removed names, document links resolve, every automated feature has a test carrying its ID, every ID in a test exists in `features.md`, and the documents contain no Spanish. Fix every `FAIL`.
4. If a behaviour changed, run `cargo test --workspace` and make sure `features.md` agrees with the result.
5. Report what changed, in which document, and anything outside `docs/` that now needs an edit (README, `CLAUDE.md`, a code comment, the twin).

## Never

- Add a document to `docs/`, or put a plan, a log or a changelog there.
- Copy the same fact into two documents.
- Mark something verified that was not observed.
- Edit code, tests, the README or `CLAUDE.md` as part of a documentation task: report them instead.
