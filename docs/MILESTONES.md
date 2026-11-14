# Slate-To-UCS — milestones

Status: draft, 2026-11-12. Companion to [PLAN.md](PLAN.md) (architecture, CI/CD, packaging, phases
0–8) and [LAYOUT.md](LAYOUT.md) (file-level view). This document turns those phases into dated
goals for the next eight weeks.

All dates and durations are **estimates** for one developer working part-time, not commitments.
Each milestone has an exit criterion that is either met or not met; there is no "mostly done".

---

## 1. How to read this

- **Weeks run Monday to Sunday.** Week 1 starts 2026-11-16. The current week (2026-11-12 to
  2026-11-15) is M0.
- **A milestone is done when its exit criterion passes on a machine that is not the dev machine**,
  or when the criterion is explicitly a local check. Anything else is "in progress".
- **Phase numbers** refer to `PLAN.md` §15. Milestones group phases; they do not renumber them.
- **Slip rule:** if a milestone's exit criterion is not met by its end date, the next milestone does
  not start. Scope is cut from the milestone in progress, not borrowed from the next one.
- Estimates assume the owner reviews PRs within a day. A PR that sits for three days moves every
  downstream date by three days.

---

## 2. Assumptions and capacity

| assumption | value | if wrong |
|---|---|---|
| Developer time | ~20 h/week, evenings and one weekend day | halve the scope per milestone, not the exit criteria |
| Owner review latency | ≤ 1 day per PR | dates shift 1:1 |
| `ttucs serve` lands on schedule | M1, week 2 | everything from M2 onward slips 1:1; M0 and the scaffold still proceed |
| No new platform targets | Windows x64 is the only signed target until M5 | adding macOS signing early costs ~1 week |
| Azure Artifact Signing eligibility | confirmed by 2026-12-11 | M5 falls back to a cloud signing service, +1 week |
| Holiday availability | 2026-12-21 to 2027-01-03 at ~50% capacity | M3 stretches into M4's window |

Two of these are decisions only the owner can make, and both have deadlines in §10.

---

## 3. Milestone overview

| # | name | dates | weeks | phases | exit criterion, one line |
|---|---|---|---|---|---|
| M0 | Groundwork | 2026-11-12 → 11-15 | 0 | – | repo hygiene done, plan/layout/agents/changelog landed, phase 0 PR open |
| M1 | Contract | 2026-11-16 → 11-29 | 1–2 | 0, 1 | `ttucs serve` merged and released; this repo scaffolds and builds on 3 OSes |
| M2 | First light | 2026-11-30 → 12-13 | 3–4 | 1, 2 | a folder of real slates is transcribed, predicted and exported from the app |
| M3 | Trustworthy | 2026-12-14 → 12-27 | 5–6 | 3, 4 | transcript edits re-predict instantly; rename dry-run and undo are proven |
| M4 | In the DAW | 2026-12-28 → 2027-01-10 | 7–8 | 5, 6 | 20 items tagged in REAPER as one undo step; scripts install via ReaPack |
| M5 | Shippable | 2027-01-11 → 01-24 | 9–10 | 7, 8 | a signed MSI installs, upgrades and uninstalls cleanly on a clean VM |

M5 is outside the eight-week window but is listed so the packaging work is not a surprise. It is
the first milestone where an artefact reaches a user who is not the owner.

---

## 4. M0 — Groundwork (2026-11-12 → 11-15)

The current week. No code. Everything here unblocks M1 and removes the two things most likely to
bite later: an unignored `.idea/` and an undocumented contract.

### Goals

1. `.gitignore` covers `.idea/`, `.abacusai/`, `.abacus.donotdelete`, `models/`, `target/`,
   `node_modules/`, `dist/`, `build/`, `*.safetensors`, `*.onnx`, `*.gguf`, and
   `apps/reaper/extension/vendor/`.
2. `docs/PLAN.md`, `docs/LAYOUT.md`, `AGENTS.md` and `CHANGELOG.md` are committed on a branch and
   opened as one PR.
3. The phase 0 PR against `ttucs` (`feat/serve-api`) is open, with the contract from `PLAN.md` §5
   implemented and `tests/test_serve.py` green.
4. The four open decisions in §10 have owners and dates, even if the answers are still pending.

### Exit criterion

`git status` is clean apart from ignored paths; the docs PR is open; the `ttucs` PR is open and CI
is green on it.

**Met 2026-11-14.** PRs #1, #2 and #3 merged; `ttucs` PR #14 merged as `0cee598` and verified live
(28 serve tests pass; `/health`, `/predict`, `/parse`, `/taxonomy/*`, 401, 413 and `/shutdown` all
behave as specified with `TTUCS_OFFLINE=1` and an empty `TTUCS_HOME`).

### Risks

- The `ttucs` PR is larger than it looks: it touches `cli.py`, adds a module, adds tests and changes
  the `predict --json` shape. Budget the weekend for it, not an evening.

---

## 5. M1 — Contract (2026-11-16 → 11-29)

Phase 0 lands in `ttucs`; phase 1 scaffolds this repo. This is the milestone that decides whether
the rest of the schedule is real.

### Week 1 (11-16 → 11-22): `ttucs serve`

| goal | detail |
|---|---|
| `serve` subcommand | `ThreadingHTTPServer`, loopback only, `--port 0`, `--token-file`, `--no-token`, `--top-k`, `--warm/--no-warm` |
| routes | `/health`, `/predict`, `/parse`, `/taxonomy/search`, `/taxonomy/{cat_id}`, `/shutdown` |
| guards | `X-Ttucs-Token` via `hmac.compare_digest`, no CORS, 1 MiB body cap, 512-text cap, one lock around `predict_batch` |
| endpoint file | `$TTUCS_HOME/serve.json`, mode `0600`, removed on clean shutdown, dead `pid` means not running |
| `predict --json` | always a list; **Breaking:** entry in the `ttucs` changelog |
| docs | `ttucs/docs/serve-api.md`, including the three launcher steps |
| tests | `tests/test_serve.py` covering shape, 401, array-for-one, parse-without-model, taxonomy search, size cap, shutdown |

**Exit:** `ruff check` and `pytest -q` green on 3.10–3.12; a `curl` round-trip on Windows
PowerShell with `TTUCS_OFFLINE=1` and an empty `TTUCS_HOME`.

### Week 2 (11-23 → 11-29): scaffold

| goal | detail |
|---|---|
| workspace | `version.toml`, `Cargo.toml`, `rust-toolchain.toml`, `package.json`, `pnpm-workspace.yaml`, `justfile`, `deny.toml` |
| `xtask` | `bump`, `verify-versions`, `changelog`, `docs-check`, `gen-golden`, `sync-taxonomy` |
| contract | `packages/contract/openapi.yaml` plus `gen/ts` and `gen/rust`, with the drift check wired into CI |
| first crates | `slate-ucs` with `tests/conformance.rs` against `fixtures/golden/predictions.jsonl`; `slate-engine` with `endpoint.rs`, `launcher.rs`, `client.rs` |
| CLI | `slate predict` and `slate doctor` over a real `ttucs serve` |
| CI | `ci.yml` on `windows-2022`, `macos-latest`, `ubuntu-22.04`; `nightly.yml`; `docs.yml` |
| docs | `CONTRIBUTING.md`, `CODEOWNERS`, `docs/index.md`, ADRs 0001–0006 |

**Exit:** from a clean checkout on all three OSes, `just check && just test` passes and
`slate predict "<a full slate>"` returns a `Prediction` matching the golden corpus. Contract drift
check active and failing on a deliberate edit.

### Status (2026-11-14)

The scaffold landed ahead of the window, in one PR. Verified on Windows only:

| check | result |
|---|---|
| `cargo test --workspace` | 181 tests pass |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo xtask verify-versions` / `docs-check` / `changelog --check` | pass |
| `cargo xtask gen-golden --check` | 412 rows, current |

Still open against the exit criterion: the three-OS CI matrix has not run, `just` is not installed
on the development machine so the `just check && just test` path is unexercised, and `slate predict`
has not been driven against a live `ttucs serve`. The contract drift check is wired into `ci.yml`
but has not been observed failing on a deliberate edit.

### Risks

- `slate-ucs` conformance is the first place the Rust port can silently diverge. If it does not
  match on the first run, stop and fix the port rather than loosening the test.
- `progenitor` output can be noisy; if it fights the hand-written types, generate only the schema
  and keep `types.rs` hand-written. Record the choice in ADR-0004.

---

## 6. M2 — First light (2026-11-30 → 12-13)

Phases 1 and 2. The app becomes useful for the owner's own work, which is the point at which real
feedback starts.

### Week 3 (11-30 → 12-06): shell and audio

| goal | detail |
|---|---|
| Tauri shell | window, single-instance guard, capabilities, `sidecar.rs` spawning the onedir tree with `TTUCS_OFFLINE=1` |
| UI skeleton | `App.tsx`, layout, theme tokens, `lib/engine.ts`, `lib/schema.ts` with zod validation |
| audio | `slate-audio` decode for WAV/BWF/RF64, resample to 16 kHz, `fixtures/audio/` round-trip tests |
| whisper | `fetch-whisper.ps1`, `fetch-model.ps1`, SHA-256 verification, `slate transcribe` |
| health | About panel showing `/health`, `ttucs_version`, model source, `manifest_verified` |

**Exit:** the app opens, shows a healthy engine, and `slate transcribe` produces a transcript with
timings from a 96 kHz/24-bit BWF file and from an RF64 header.

### Week 4 (12-07 → 12-13): review and export

| goal | detail |
|---|---|
| queue | drop zone, virtualised table, sorted by confidence ascending, filter chips |
| batch | transcribe with per-file progress and cancel, predict in batches of 32 |
| inspector | transcript, top-5 alternatives, decision badge, editable filename with live validation, keywords |
| override | `/taxonomy/search`, keyboard-first, row marked `overridden`, original guess retained |
| export | CSV, JSONL, column preview |
| VAD | Silero ONNX with energy fallback; segmentation scoring by position and duration |

**Exit:** 200 real files dropped, transcribed, predicted, reviewed and exported. With the network
disabled and `TTUCS_OFFLINE=1`, a proxy log or firewall rule shows **zero outbound connections**.

### Risks

- STT quality will dominate perceived accuracy. If transcripts are poor on real slates, raise the
  whisper model size before touching the classifier.
- WebView2 and WKWebView differ. Test both before calling the week done; keep CSS conservative.

---

## 7. M3 — Trustworthy (2026-12-14 → 12-27)

Phases 3 and 4. Two weeks at reduced capacity because of the holidays; scope is already trimmed to
match.

### Week 5 (12-14 → 12-20): edit

| goal | detail |
|---|---|
| transcript editor | fix an STT error, re-predict that row in under 200 ms |
| cancellation | STT cancel leaves no orphan `whisper` process; assert with a process listing |
| keyboard | `j`/`k`, `Enter`, `e`, `c`, `o`, `Space`, `Cmd/Ctrl+E`, all also mouse-reachable |
| settings | model, thresholds, default creator and source IDs, STT model, port override |
| telemetry | switch, "what is sent" panel, install ID, `DO_NOT_TRACK`, `SLATE_TELEMETRY=0` |

**Exit:** editing a transcript re-predicts in under 200 ms on a 200-row queue; the telemetry payload
test asserts no transcript, filename, path or CatID appears in any field.

### Week 6 (12-21 → 12-27): rename

| goal | detail |
|---|---|
| dry run | two-column old → new diff, collisions and invalid names flagged before anything moves |
| safety | never overwrite, refuse subfolders unless opted in, atomic temp-and-rename |
| journal | `slate-undo-<timestamp>.json`, readable after a restart |
| undo | restores every name byte-for-byte |
| CLI | `slate tag --dry-run` default, `--apply`, `--undo <journal>` |

**Exit:** a dry run on a copy of a real folder, then apply, then undo, leaves every filename
byte-identical to the start and no leftover temp files.

### Risks

- This is the only destructive feature in the product. If undo is not proven, rename does not ship
  in v1 and moves to v1.1. That is an acceptable outcome; a broken undo is not.

---

## 8. M4 — In the DAW (2026-12-28 → 2027-01-10)

Phases 5 and 6. The milestone that makes the product worth having for a recordist mid-session.

### Week 7 (12-28 → 01-03): extension

| goal | detail |
|---|---|
| SDK | `cargo xtask fetch-reaper-sdk`, pinned, gitignored, licence file present |
| build | `CMakeLists.txt` producing `slate_reaper.dll` on Windows, `.dylib`/`.so` configured |
| entry | `ReaperPluginEntry`, action registration, `Extensions > Slate-To-UCS` menu |
| threading | worker thread for HTTP, lock-free queue drained from a `plugin_register("timer")` callback at ~30 Hz |
| launcher | the three `PLAN.md` §5.4 steps in C++, 30 s poll, modal "starting engine" state |
| actions | tag selected, tag project, apply render pattern, open app |
| writes | `P_NOTES` read-modify-write preserving existing text, optional take name, one undo block per batch |
| failure | engine absent shows a clear message and a link, never crashes the host |

**Exit:** 20 selected items tagged in a real session; notes correct and pre-existing text preserved;
one undo step reverts the batch; no frame stall over ~50 ms measured with REAPER's own timer.

### Week 8 (01-04 → 01-10): scripts

| goal | detail |
|---|---|
| `API_Slate_*` | exported so Lua reaches the same code paths without HTTP |
| scripts | tag selected, tag project, render pattern, open app |
| fallback | `lib/slate_api.lua` uses `reaper.ExecProcess("slate ...")` when the extension is absent |
| ReaPack | `index.xml.tmpl`, `reapack.yml` pushing to the `reapack` branch |
| install | `reaper_install.rs` detects `%APPDATA%\REAPER` and portable installs via `reaper.ini`, copies, writes a reversible manifest |

**Exit:** scripts install via ReaPack and work with the extension present and absent; the app's
**Install REAPER integration** action is reversible.

### Risks

- The REAPER SDK licence review must happen before week 7, not during it. If it blocks, M4 slips and
  M5 proceeds without the extension in the installer.
- Main-thread stalls are the failure mode that gets an extension uninstalled. Measure, do not assume.

---

## 9. M5 — Shippable (2027-01-11 → 01-24)

Phases 7 and 8, outside the eight-week window, listed for planning only.

| week | goal |
|---|---|
| 9 | WiX MSI and NSIS exe, Azure Artifact Signing via `signCommand`, `sign-verify.yml`, `SECURITY.md` |
| 10 | install smoke tests on clean runners, upgrade and uninstall paths, DMG and notarisation, deb/rpm/AppImage |

**Exit:** `signtool verify /pa` passes on a downloaded MSI; silent install, upgrade over the previous
version and uninstall all pass on a clean VM with no residue.

---

## 10. Decision deadlines

Each of these blocks a milestone. The date is the last day the answer can arrive without moving a
date.

| decision | needed by | blocks | default if unanswered |
|---|---|---|---|
| Azure Artifact Signing eligibility (individual US/CA or organisation) | 2026-12-11 | M5 | fall back to a cloud signing service, +1 week |
| Minimum REAPER version (6.x or 7.x) | 2026-12-18 | M4 week 7 | target 7.x only, note it in the README |
| Rename in v1 or v1.1 | 2026-12-14 | M3 week 6 | ship in v1; if undo is unproven, drop to v1.1 |
| `slate` CLI in the MSI | 2027-01-08 | M5 | include it; it is one binary and the scripting path |
| Windows arm64 in or out | 2027-01-08 | M5 | out; revisit after 1.0 |

---

## 11. Weekly cadence

| when | what |
|---|---|
| Monday | pick the week's goals from this document; open the branch |
| daily | commit small, Conventional Commits, `just check` before pushing |
| Thursday | open the PR even if incomplete, so review starts early |
| Friday | update `CHANGELOG.md` under `[Unreleased]`; update `AGENTS.md` §11 current state |
| Sunday | milestone check: exit criterion met or not; if not, cut scope, do not extend |

`AGENTS.md` §11 is the running record. If this document and §11 disagree, §11 is what actually
happened.

---

## 12. Slip triggers

Re-plan when any of these is true, rather than pushing dates silently.

1. `ttucs serve` is not merged by 2026-11-29. Everything from M2 slips 1:1; M0 and the scaffold
   still proceed, and `slate-engine` is developed against a stub server.
2. `slate-ucs` conformance does not match the golden corpus after two attempts. Stop and diagnose
   the port; do not loosen the test.
3. Rename undo cannot be proven by 2026-12-27. Drop rename to v1.1 and keep M4 on schedule.
4. The REAPER SDK licence review is unresolved by 2026-12-28. Ship M5 without the extension in the
   installer and distribute it separately.
5. Azure eligibility is unresolved by 2026-12-11. Switch to a cloud signing service and add a week
   to M5.
6. Any milestone's exit criterion fails twice for the same reason. Stop, write down what was tried
   and what could not be verified, and ask the owner rather than attempting a third fix.

---

## 13. Tracking

Update the status column at each Sunday check. `not started` / `in progress` / `done` / `cut`.

| # | milestone | status | exit criterion met | notes |
|---|---|---|---|---|
| M0 | Groundwork | done | yes | docs, `SECURITY.md`, `CODE_STYLE.md` and `.gitignore` merged in #1, #2 and #3; phase 0 verified 2026-11-14 (28 serve tests pass, live round-trip clean) |
| M1 | Contract | in progress | no | phase 0 landed in `ttucs` PR #14; scaffold is the remaining work |
| M2 | First light | not started | no | blocked on M1 |
| M3 | Trustworthy | not started | no | blocked on M2 |
| M4 | In the DAW | not started | no | blocked on M3 and the SDK licence review |
| M5 | Shippable | not started | no | blocked on M4 and Azure eligibility |
