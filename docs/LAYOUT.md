# Slate-To-UCS — planned layout

Status: draft, 2026-11-12. Companion to [PLAN.md](PLAN.md), which covers architecture, CI/CD,
packaging and phases. This document is the file-level view: what exists, where it goes, and what
each piece is for.

Nothing described here exists yet except `README.md`, `LICENSE`, `.gitignore`, `AGENTS.md`,
`CHANGELOG.md` and `docs/`. The tree is the target for phases 1–8; the `Phase` column says when a
file is expected to appear.

---

## 1. How to read this

- **The tree is a contract, not a suggestion.** A file that does not fit a row below needs a new
  row and a reason, not a new directory.
- **`Phase`** refers to the phase numbers in `PLAN.md` §15. Phase 0 is a PR to `ttucs` and adds
  nothing here.
- **`Kind`** is one of: `entry` (a process or bundle entry point), `lib` (a library), `gen`
  (generated, never hand-edited), `data` (a resource or fixture), `doc`, `cfg` (build or tooling
  configuration), `ci` (a workflow).
- Paths are relative to the repository root.

---

## 2. Top-level tree

```
Slate-To-UCS/
├─ README.md                    doc    pitch, install, 60-second first run, installer size
├─ LICENSE                      doc    MIT, Copyright (c) 2026 Jon Evans
├─ CHANGELOG.md                 doc    Keep a Changelog 1.1.0, assembled at release
├─ AGENTS.md                    doc    rulebook for contributors, human or AI
├─ CONTRIBUTING.md              doc    setup, checks, PR shape                    phase 1
├─ SECURITY.md                  doc    disclosure, signing fingerprints, verify   phase 7
├─ CODEOWNERS                   cfg    review routing                            phase 1
├─ version.toml                 cfg    single source of the product version      phase 1
├─ Cargo.toml                   cfg    workspace root, shared lints              phase 1
├─ rust-toolchain.toml          cfg    pinned toolchain                          phase 1
├─ package.json                 cfg    pnpm workspace root                       phase 1
├─ pnpm-workspace.yaml          cfg    workspace globs                           phase 1
├─ justfile                     cfg    task entry points                         phase 1
├─ deny.toml                    cfg    cargo-deny licences and advisories        phase 1
├─ .gitignore                   cfg    ignores weights, SDK, build output        exists
├─ .editorconfig                cfg    line endings, indent                      phase 1
├─ .github/                     ci     workflows, templates                      phase 1
├─ changelog.d/                 doc    one fragment per PR                      phase 1
├─ crates/                      lib    Rust libraries and the CLI                phase 1
├─ apps/                        entry  desktop app and REAPER integration          phase 1
├─ packages/contract/           gen    OpenAPI and generated clients             phase 1
├─ models/                      data   gitignored; manifests and pins only       phase 1
├─ fixtures/                    data   tiny audio and transcripts for tests      phase 1
├─ docs/                        doc    this document and the rest                exists
└─ tools/xtask/                 entry  build, bump, changelog, sign, verify       phase 1
```

---

## 3. Root files

| file | kind | phase | function |
|---|---|---|---|
| `README.md` | doc | exists | The pitch, the install path per platform, the 60-second first run, and the honest installer size (250–400 MB, because of torch). Links to `docs/`. |
| `LICENSE` | doc | exists | MIT. Weights and the UCS list are redistributable; the REAPER SDK is not redistributed at all. |
| `CHANGELOG.md` | doc | exists | Keep a Changelog 1.1.0. `[Unreleased]` at the top, compare links at the bottom. Assembled from `changelog.d/` once that tooling lands. |
| `AGENTS.md` | doc | exists | Twelve hard rules, the data flow, the repository map, PowerShell-first commands, the install checklist, the telemetry allowlist, naming, versioning, release sequence, current state. |
| `CONTRIBUTING.md` | doc | 1 | Dev setup, the checks that must pass, PR shape, the changelog-fragment requirement, review expectations. |
| `SECURITY.md` | doc | 7 | Disclosure address, the signing certificate fingerprints, and the exact commands a user runs to verify a download. |
| `CODEOWNERS` | cfg | 1 | Routes `crates/`, `apps/desktop/`, `apps/reaper/` and `.github/` to the owner. |
| `version.toml` | cfg | 1 | `version = "X.Y.Z"`. The only place the product version is authored; `just bump` writes it everywhere else and `cargo xtask verify-versions` proves they agree. |
| `Cargo.toml` | cfg | 1 | Workspace members, shared `[workspace.dependencies]`, `[workspace.lints]` with `unsafe_code = "forbid"` outside the FFI crates. |
| `rust-toolchain.toml` | cfg | 1 | Pins the channel and components (`rustfmt`, `clippy`), so CI and the owner's machine build identically. |
| `package.json` | cfg | 1 | pnpm workspace root, scripts that delegate to `just`, and the dev dependency set for the UI. |
| `pnpm-workspace.yaml` | cfg | 1 | `apps/desktop`, `packages/contract`. |
| `justfile` | cfg | 1 | `setup`, `check`, `test`, `dev`, `build`, `bundle`, `bump`, `changelog`, `sign`, `verify`. Each recipe is one line over `cargo xtask` or a package manager, so there is one entry point per task. |
| `deny.toml` | cfg | 1 | Licence allowlist (MIT, Apache-2.0, BSD, ISC, Unicode, Zlib), advisory denial, and a ban on duplicate crate versions where it matters. |
| `.gitignore` | cfg | exists | Must gain `models/`, `target/`, `node_modules/`, `dist/`, `build/`, `*.safetensors`, `*.onnx`, `*.gguf`, `apps/reaper/extension/vendor/`, `.idea/`, `.abacusai/`, `.abacus.donotdelete`. |
| `.editorconfig` | cfg | 1 | LF for source, CRLF preserved for `.ps1`, 4 spaces for Rust and C++, 2 for TS and JSON. |

---

## 4. `crates/` — Rust libraries and the CLI

Five crates, layered so that a dependency arrow never points up.

```
slate-cli ──▶ slate-engine ──▶ (HTTP)  ttucs serve
    │
    ├────────▶ slate-audio ──▶ whisper.cpp (external binary)
    ├────────▶ slate-ucs
    └────────▶ slate-tagger ──▶ slate-ucs
```

`slate-engine` does not depend on `slate-audio`; the launcher and the decoder are independent.
`slate-tagger` depends on `slate-ucs` for the filename it writes. Nothing in `crates/` depends on
`apps/`.

### 4.1 `crates/slate-audio/` — decode, resample, VAD, segment

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | Depends on `symphonia`, `rubato`, `ort` (optional), `thiserror`. |
| `src/lib.rs` | lib | 1 | Public surface: `decode`, `resample`, `detect_speech`, `segment`, and the `AudioBuffer` type. |
| `src/audio.rs` | lib | 1 | `AudioBuffer { samples, sample_rate, channels }`, downmix to mono, peak/RMS helpers, duration. |
| `src/decode/mod.rs` | lib | 1 | Format dispatch by extension and magic bytes; returns `AudioBuffer` plus a `SourceInfo` (sample rate, bit depth, channel count, duration, container). |
| `src/decode/wav.rs` | lib | 1 | RIFF/WAVE reader. Walks chunks, skips `bext`/`iXML`/`LIST`, handles `WAVE_FORMAT_EXTENSIBLE`, 24-bit packed samples. |
| `src/decode/rf64.rs` | lib | 1 | RF64 with `ds64` 64-bit sizes. Field recorders produce these above 4 GB and `symphonia` does not read them. |
| `src/decode/symphonia.rs` | lib | 2 | FLAC, MP3, OGG/Vorbis, AAC/M4A through `symphonia`. |
| `src/decode/ffmpeg.rs` | lib | 8 | Optional sidecar for exotic containers. Feature-gated, off by default, never required. |
| `src/resample.rs` | lib | 1 | `rubato` sinc resampler to 16 kHz mono, with a fast path when the source is already 16 kHz. |
| `src/vad/mod.rs` | lib | 2 | `VoiceDetector` trait and selection between Silero and the energy fallback. |
| `src/vad/silero.rs` | lib | 2 | Silero VAD through `ort` (ONNX). Model pinned by SHA-256 in `models/manifest.toml`. |
| `src/vad/energy.rs` | lib | 2 | Adaptive energy threshold with hysteresis. Used when ONNX is unavailable, and as a cross-check. |
| `src/segment.rs` | lib | 2 | Turns speech regions into candidate slates, scoring by position in the take (head and tail first), duration, and speech density. Returns ranked `SlateRegion { start, end, score }`. |
| `src/error.rs` | lib | 1 | `AudioError` with a variant per failure a user can act on: unsupported format, truncated file, RF64 without `ds64`, decode failure at offset. |
| `tests/fixtures.rs` | lib | 1 | Round-trips every file in `fixtures/audio/` and asserts duration and sample rate. |
| `tests/rf64.rs` | lib | 1 | Asserts a >4 GB RF64 header parses without reading the payload. |

### 4.2 `crates/slate-ucs/` — taxonomy, filename, keywords

A port of `ttucs.text` and `ttucs.taxonomy`. It must agree with the Python implementation exactly;
`fixtures/golden/` is how that is proven.

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | `serde`, `csv`, `once_cell`, `thiserror`. No async. |
| `src/lib.rs` | lib | 1 | Re-exports `Taxonomy`, `CatId`, `UcsFilename`, `keywords`. |
| `src/taxonomy.rs` | lib | 1 | Loads `resources/ucs_v8.2.1.csv` once into a `HashMap<CatId, Entry>`. 753 rows, 738 usable because `ARCHIVED` is excluded. Lookup by CatID, by category, and by substring for the UI's search. |
| `src/catid.rs` | lib | 1 | `CatId` newtype: parse, validate against the taxonomy, split into category and subcategory, render `category_full` (`DOOR-Creak`). |
| `src/filename.rs` | lib | 1 | `UcsFilename` builder and validator for `CatID_FX Name_CreatorID_SourceID[_UserData]`. Rejects characters the filesystem or UCS forbids, collapses whitespace, enforces the `NONE` placeholder for an absent creator. |
| `src/keywords.rs` | lib | 1 | Keyword extraction from the description, matching `ttucs` output for the golden corpus. |
| `src/text.rs` | lib | 1 | Normalisation port: case folding, number words, gear lexicon lookups, punctuation. Only what the filename and keywords need. |
| `src/resources.rs` | lib | 1 | `include_str!` of the CSV so the crate is self-contained, with a `sha256` constant checked by a test. |
| `resources/ucs_v8.2.1.csv` | data | 1 | Vendored copy, pinned. Regenerated by `cargo xtask sync-taxonomy` from the `ttucs` resources, never hand-edited. |
| `tests/conformance.rs` | lib | 1 | Reads `fixtures/golden/predictions.jsonl` and asserts equality on `cat_id`, `filename`, `keywords`, `decision`. This is the guard against silent divergence. |
| `tests/filename.rs` | lib | 1 | Table-driven cases: empty FX name, over-long name, forbidden characters, `NONE` creator, `UserData` present. |

### 4.3 `crates/slate-tagger/` — metadata writing

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | `byteorder`, `quick-xml` (iXML), `serde_json`, `chrono`, `thiserror`. |
| `src/lib.rs` | lib | 1 | `TagPlan` (what would be written), `apply(plan)`, `probe(path)`. |
| `src/bext.rs` | lib | 4 | Reads and writes the `bext` chunk: description, originator, originator reference, origination date and time, time reference, coding history. Preserves unknown fields. |
| `src/ixml.rs` | lib | 4 | Reads and writes `iXML`: `PROJECT`, `SCENE`, `TAKE`, `TAPE`, `NOTE`, `SPEED`, plus a `UCS` block carrying CatID and category. |
| `src/sidecar.rs` | lib | 2 | Writes `<file>.slate.json`: the full `Prediction` plus provenance (model id, weights sha256, engine version, `schema_version`, timestamp). |
| `src/probe.rs` | lib | 2 | Reads whatever metadata already exists so the UI can show a before/after diff and never silently clobber a recordist's notes. |
| `src/atomic.rs` | lib | 4 | Write to a temp file in the same directory, fsync, rename over the original. Refuses across filesystems. |
| `src/rename.rs` | lib | 4 | Dry-run diff (`Vec<RenameOp>`), collision and invalid-name detection, journal write, and `undo(journal)`. Never overwrites an existing file. |
| `src/journal.rs` | lib | 4 | `slate-undo-<timestamp>.json` schema and reader, so undo works after a restart. |
| `tests/bext_roundtrip.rs` | lib | 4 | Write, re-read, compare every field, including a file that already had a `bext` chunk. |
| `tests/rename_undo.rs` | lib | 4 | Rename a temp tree, undo, assert byte-identical names and no leftovers. |

### 4.4 `crates/slate-engine/` — the launcher contract and HTTP client

The single implementation of `PLAN.md` §5.4. The C++ extension reimplements the same three steps;
this crate is the reference.

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | `reqwest` (blocking, rustls), `serde`, `serde_json`, `directories`, `thiserror`. |
| `src/lib.rs` | lib | 1 | `Engine::connect_or_spawn()`, `Engine::health()`, `Engine::predict()`, `Engine::shutdown_if_owned()`. |
| `src/endpoint.rs` | lib | 1 | Resolves `$TTUCS_HOME` (falling back to the platform cache dir, same rule as `ttucs.hub.cache_root()`), reads and writes `serve.json`, and treats a dead `pid` as "not running". |
| `src/launcher.rs` | lib | 1 | Spawns `ttucs serve --port 0 --token-file <path>` detached, polls the file for 30 s, then polls `/health` until `warm` within a 60 s first-run budget. Records whether this process owns the server. |
| `src/token.rs` | lib | 1 | Reads the token, sets `X-Ttucs-Token`, and refuses to log it. |
| `src/client.rs` | lib | 1 | Typed calls for `/health`, `/predict`, `/parse`, `/taxonomy/search`, `/taxonomy/{cat_id}`, `/shutdown`. Enforces the 1 MiB and 512-text caps client-side too. |
| `src/schema.rs` | lib | 1 | `schema_version` gate: accepts 1, refuses anything else with a message the UI can display verbatim. |
| `src/types.rs` | lib | 1 | `Prediction`, `SlateFields`, `Alternative`, `CategoryScore`, `Health`. Unknown JSON keys are ignored, never fatal. |
| `src/error.rs` | lib | 1 | `EngineError`: not installed, failed to start, unhealthy, unauthorised, version mismatch, transport. Each maps to one UI message. |
| `tests/launcher.rs` | lib | 2 | Starts a stub server on port 0, asserts attach, spawn, warm polling and shutdown-only-if-owned. |

### 4.5 `crates/slate-cli/` — the `slate` binary

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | `clap`, plus the four crates above. |
| `src/main.rs` | entry | 1 | Argument parsing, exit codes (`0` ok, `1` user error, `2` engine unavailable, `3` partial batch), and top-level error rendering. |
| `src/cmd/mod.rs` | lib | 1 | Subcommand registration. |
| `src/cmd/predict.rs` | lib | 1 | Text in, `Prediction` out. `--json` always emits a list, matching the engine. |
| `src/cmd/transcribe.rs` | lib | 2 | Audio in, transcript and segments out. `--json`, `--model`, `--language`. |
| `src/cmd/tag.rs` | lib | 4 | Folder in, `TagPlan` out. `--dry-run` is the default; `--apply` is required to write. `--undo <journal>`. |
| `src/cmd/doctor.rs` | lib | 1 | Prints engine reachability, `ttucs` version, model source and manifest state, whisper model presence, REAPER resource paths, and whether offline mode is on. This is what the install smoke test runs. |
| `src/output.rs` | lib | 1 | Table and JSON renderers, so every command has one machine and one human form. |
| `tests/cli.rs` | lib | 1 | Asserts exit codes and JSON shapes against a stub engine. |

---

## 5. `apps/desktop/` — the Tauri application

```
apps/desktop/
├─ index.html
├─ vite.config.ts
├─ tsconfig.json
├─ src/                    React + TypeScript
└─ src-tauri/              Rust shell
```

### 5.1 `apps/desktop/src/` — UI

| file | kind | phase | function |
|---|---|---|---|
| `main.tsx` | entry | 1 | Mounts React, installs the error boundary and the query client. |
| `App.tsx` | entry | 1 | Shell layout: queue, inspector, status bar. |
| `app/router.tsx` | lib | 1 | Two routes: the workspace and settings. |
| `app/layout.tsx` | lib | 1 | Split pane, resizable, remembers its width. |
| `app/theme.css` | lib | 1 | Tokens only. Conservative CSS because WebView2, WKWebView and WebKitGTK differ. |
| `app/ErrorBoundary.tsx` | lib | 1 | Catches render errors, offers a copyable report with paths stripped. |
| `features/queue/QueueView.tsx` | lib | 2 | The default screen: drop zone, table, progress, filter chips. |
| `features/queue/QueueTable.tsx` | lib | 2 | Virtualised rows. Columns: file, duration, transcript, CatID, confidence, decision, filename. Sorted by confidence ascending. |
| `features/queue/useQueue.ts` | lib | 2 | Import, transcribe, predict in batches of 32, cancel, retry. Owns the per-file state machine. |
| `features/queue/FilterChips.tsx` | lib | 2 | all / confident / fallback / unknown / overridden. |
| `features/inspector/Inspector.tsx` | lib | 2 | Right pane: transcript, alternatives, filename, keywords, copy actions. |
| `features/inspector/Alternatives.tsx` | lib | 2 | Top-5 as clickable bars; clicking overrides. |
| `features/inspector/DecisionBadge.tsx` | lib | 2 | `confident` / `category_fallback` / `unknown`, each with a one-line explanation. |
| `features/inspector/FilenameEditor.tsx` | lib | 2 | Editable UCS filename with live validation from `slate-ucs` rules. |
| `features/inspector/TranscriptEditor.tsx` | lib | 3 | Fix an STT error and re-predict that row in under 200 ms. The highest-value accuracy lever in the app. |
| `features/rename/RenameDiff.tsx` | lib | 4 | Two-column old → new diff, collisions and invalid names flagged before anything moves. |
| `features/rename/useRename.ts` | lib | 4 | Dry run, apply, undo. Refuses a folder with subfolders unless the user opts in. |
| `features/export/ExportDialog.tsx` | lib | 2 | CSV, JSONL, and rename, with a column preview. |
| `features/export/csv.ts` | lib | 2 | CSV writer: file, cat_id, category, subcategory, confidence, decision, filename, keywords, plus slate fields. |
| `features/export/jsonl.ts` | lib | 2 | Raw `Prediction` objects, one per line. |
| `features/settings/SettingsView.tsx` | lib | 2 | Model, thresholds, default creator and source IDs, STT model, port override. |
| `features/settings/TelemetryPanel.tsx` | lib | 7 | The switch, the "what is sent" payload view, and the install-ID reset. |
| `features/settings/UpdatePanel.tsx` | lib | 7 | Channel (`stable` / `beta`), last check, and the manual check button. |
| `lib/engine.ts` | lib | 1 | Typed client for the contract. The single source of truth for how the UI talks to the engine. |
| `lib/schema.ts` | lib | 1 | zod schemas mirroring the contract, validated on every response. A shape mismatch surfaces as a visible error, not a blank cell. |
| `lib/telemetry.ts` | lib | 7 | Fixed allowlist payload builder, batching, `DO_NOT_TRACK` and `SLATE_TELEMETRY=0` handling. |
| `lib/updater.ts` | lib | 7 | Signed-manifest check, banner state, never blocks startup. |
| `lib/keys.ts` | lib | 2 | `j`/`k`, `Enter`, `e`, `c`, `o`, `Space`, `Cmd/Ctrl+E`. Every action also reachable by mouse. |
| `stores/queue.ts` | lib | 2 | Zustand store: rows, selection, filters, batch progress. |
| `stores/settings.ts` | lib | 2 | Persisted settings, with the engine's offline flag locked on. |

### 5.2 `apps/desktop/src-tauri/` — the Rust shell

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | `tauri`, `tauri-plugin-dialog`, `-fs`, `-updater`, `-shell`, plus `slate-engine`, `slate-audio`, `slate-tagger`. |
| `build.rs` | cfg | 1 | `tauri-build`, capability and icon generation. |
| `tauri.conf.json` | cfg | 1 | Window, bundle identifiers, `bundle.windows.signCommand`, updater endpoints and pubkey, macOS notarisation keys. |
| `capabilities/default.json` | cfg | 1 | Least-privilege allowlist: dialogs, fs scoped to user-chosen paths, shell for the sidecar only. |
| `src/main.rs` | entry | 1 | Thin: calls `lib::run()`. |
| `src/lib.rs` | lib | 1 | Builder, plugin registration, command registration, single-instance guard. |
| `src/commands/scan.rs` | lib | 2 | Folder walk, extension filter, duration probe, subfolder refusal. |
| `src/commands/transcribe.rs` | lib | 2 | Per-file whisper.cpp invocation with progress events and cancellation that leaves no orphan process. |
| `src/commands/predict.rs` | lib | 2 | Batched `/predict` calls through `slate-engine`. |
| `src/commands/tag.rs` | lib | 4 | `TagPlan` build and apply, dry run by default. |
| `src/commands/rename.rs` | lib | 4 | Dry-run diff, apply, undo. |
| `src/commands/reaper_install.rs` | lib | 5 | Detects `%APPDATA%\REAPER` and portable installs by scanning for `reaper.ini`, copies the extension and scripts, writes a reversible manifest. |
| `src/commands/settings.rs` | lib | 2 | Read and write the app config, including the telemetry switch and install ID. |
| `src/sidecar.rs` | lib | 1 | Spawns the PyInstaller onedir tree with `TTUCS_OFFLINE=1`, supervises it, and shuts it down on exit only if it started it. |
| `src/updater.rs` | lib | 7 | Launch and 24 h checks on a background thread, signature verification, banner events. |
| `src/paths.rs` | lib | 1 | `%LOCALAPPDATA%\SlateToUCS`, XDG equivalent, log location, config location. |
| `icons/` | data | 1 | Platform icon set, generated from one source. |
| `sidecar/slate-sidecar.spec` | cfg | 1 | PyInstaller onedir spec: `ttucs`, `ttucs-model-bow`, torch CPU, no onefile. |
| `sidecar/fetch-whisper.ps1` | cfg | 1 | Downloads the pinned whisper.cpp binary and verifies its SHA-256. |
| `sidecar/fetch-model.ps1` | cfg | 1 | Fetches the `base.en` GGML model, verifies SHA-256, caches it. |

---

## 6. `apps/reaper/` — extension and scripts

### 6.1 `apps/reaper/extension/` — C++17

| file | kind | phase | function |
|---|---|---|---|
| `CMakeLists.txt` | cfg | 5 | Builds `slate_reaper` as a shared library. Links `libcurl`. Fails loudly if the SDK is absent. |
| `src/plugin.cpp` | entry | 5 | `ReaperPluginEntry`: registers actions, hooks, the timer, and the `API_Slate_*` table. |
| `src/actions.cpp` | lib | 5 | Action table and command IDs: tag selected, tag project, apply render pattern, open app. |
| `src/api_slate.cpp` | lib | 6 | `API_Slate_*` exports so Lua scripts reach the same code paths without HTTP. |
| `src/engine_client.cpp` | lib | 5 | libcurl calls to `/health` and `/predict`. One code path on every platform. |
| `src/launcher.cpp` | lib | 5 | The `PLAN.md` §5.4 steps in C++: read `serve.json`, probe, spawn, poll 30 s, poll `warm`. |
| `src/marshal.cpp` | lib | 5 | Lock-free queue drained from a `plugin_register("timer")` callback at ~30 Hz. The only place worker results touch the main thread. |
| `src/undo.cpp` | lib | 5 | `Undo_BeginBlock2` / `Undo_EndBlock2` wrappers so a batch tag is one undo step. |
| `src/notes.cpp` | lib | 5 | `P_NOTES` read-modify-write that preserves existing text, and take-name writes via `GetSetMediaItemTakeInfo_String`. |
| `src/transcript.cpp` | lib | 5 | Transcript source order: item notes, adjacent `.txt`, then whisper.cpp on the source file. |
| `src/log.cpp` | lib | 5 | Rotating log under the app data dir. Never logs tokens or full transcripts. |
| `include/*.h` | lib | 5 | Public headers for the above. |
| `vendor/` | data | 5 | Gitignored. Populated by `cargo xtask fetch-reaper-sdk` with a pinned version and its licence file. Never committed. |
| `tests/smoke.md` | doc | 5 | The manual acceptance script: tag 20 items, one undo step, no frame stall over ~50 ms, engine-absent path reports cleanly. |

### 6.2 `apps/reaper/scripts/` — Lua

| file | kind | phase | function |
|---|---|---|---|
| `slate_tag_selected.lua` | entry | 6 | Tags selected items. Calls `API_Slate_*` when the extension is present. |
| `slate_tag_project.lua` | entry | 6 | Tags every item, with a progress dialog and cancel. |
| `slate_render_pattern.lua` | entry | 6 | Sets the project render filename pattern from the UCS fields. |
| `slate_open_app.lua` | entry | 6 | Launches Slate-To-UCS. |
| `lib/slate_api.lua` | lib | 6 | Shared helper: extension call when available, `reaper.ExecProcess("slate ...")` fallback otherwise. One code path per script. |
| `index.xml.tmpl` | cfg | 6 | ReaPack index template, filled by `reapack.yml` and pushed to the `reapack` branch. |

---

## 7. `packages/contract/` — the wire contract

| file | kind | phase | function |
|---|---|---|---|
| `openapi.yaml` | gen | 1 | OpenAPI 3.1 description of `/health`, `/predict`, `/parse`, `/taxonomy/*`, `/shutdown`. The build-time gate; `schema_version` remains the runtime gate. |
| `gen/ts/` | gen | 1 | `openapi-typescript` output, consumed by `lib/engine.ts`. |
| `gen/rust/` | gen | 1 | `progenitor` output, consumed by `slate-engine`. |
| `README.md` | doc | 1 | How to regenerate, and the rule that CI fails on drift. |

CI regenerates and runs `git diff --exit-code`. Nothing under `gen/` is hand-edited.

---

## 8. `tools/xtask/` — build orchestration

| file | kind | phase | function |
|---|---|---|---|
| `Cargo.toml` | cfg | 1 | A binary crate, not published. |
| `src/main.rs` | entry | 1 | Subcommand dispatch. |
| `src/bump.rs` | lib | 1 | Writes `version.toml` into `Cargo.toml`, `package.json`, `tauri.conf.json` and the extension's `version.h`. |
| `src/verify_versions.rs` | lib | 1 | Fails if any of those disagree. This is what stops a release shipping a stale version in one surface. |
| `src/changelog.rs` | lib | 1 | Assembles `changelog.d/*.md` into `CHANGELOG.md` in the fixed group order, deletes consumed fragments, refreshes compare links. |
| `src/sign.rs` | lib | 7 | Platform-appropriate signing. Refuses on a dirty tree. Windows calls `signtool` with `/fd SHA256 /tr <RFC3161> /td SHA256`. |
| `src/verify.rs` | lib | 7 | Signature, manifest, SBOM and provenance checks on built artefacts. |
| `src/fetch_reaper_sdk.rs` | lib | 5 | Downloads the pinned SDK into the gitignored vendor directory and checks its hash. |
| `src/sync_taxonomy.rs` | lib | 1 | Regenerates `crates/slate-ucs/resources/ucs_v8.2.1.csv` from `ttucs` and updates the pinned hash. |
| `src/gen_golden.rs` | lib | 1 | Runs the pinned `ttucs` over the dev and realistic sets and writes `fixtures/golden/predictions.jsonl`. |
| `src/docs_check.rs` | lib | 1 | Fails on broken relative links and undocumented public items. |

---

## 9. `fixtures/` and `models/`

| path | kind | phase | function |
|---|---|---|---|
| `fixtures/audio/*.wav` | data | 1 | A handful of short, owner-recorded or synthetic slates: 48 kHz, 96 kHz/24-bit, one RF64 header, one with an existing `bext` chunk. Total under 2 MB. |
| `fixtures/transcripts/*.txt` | data | 2 | Known-good transcripts for the audio above, so ASR and prediction can be tested separately. |
| `fixtures/golden/predictions.jsonl` | data | 1 | Full `Prediction` dicts from the pinned `ttucs`. The conformance oracle. Regenerated by `cargo xtask gen-golden`. |
| `fixtures/contract/*.json` | data | 1 | Recorded engine responses the C++ client is tested against. |
| `models/manifest.toml` | data | 1 | Pins for the whisper GGML model, the Silero ONNX model and the `ttucs-model-bow` weights: version, URL, SHA-256. |
| `models/` (contents) | data | – | Gitignored. Weights and ASR models are release assets, never committed. |

---

## 10. `docs/`

| path | kind | phase | function |
|---|---|---|---|
| `docs/index.md` | doc | 1 | The map. Entry point for the mdBook build. |
| `docs/PLAN.md` | doc | exists | Architecture, CI/CD, packaging, PKI, phases, risks, open decisions. |
| `docs/LAYOUT.md` | doc | exists | This document. |
| `docs/about/` | doc | 2 | Guided tour in the `ttucs` style: what a slate is, how audio becomes metadata, where the code lives, how a release is cut. |
| `docs/architecture/` | doc | 1 | Process model, contract, data model, threading rules for the REAPER shim. |
| `docs/adr/` | doc | 1 | One page per irreversible decision, with context, decision, consequences and a revisit trigger. Seeds listed in `PLAN.md` §11. |
| `docs/guides/` | doc | 2 | Task-shaped: install on Windows, install the REAPER integration, run fully offline, point at a folder, undo a batch, turn telemetry off. |
| `docs/reference/` | gen | 2 | Generated: HTTP API from `openapi.yaml`, CLI from `--help`, UCS mapping from the taxonomy tables. Never hand-edited. |
| `docs/release/` | doc | 7 | Versioning, signing, the release runbook, and the rollback path. |

---

## 11. `.github/`

| file | kind | phase | function |
|---|---|---|---|
| `workflows/ci.yml` | ci | 1 | fmt, clippy, cargo test on three OSes, tsc and vitest, contract drift, changelog fragment, `cargo-deny`, `cargo-audit`, `osv-scanner`, docs links. |
| `workflows/build.yml` | ci | 7 | Tag-triggered matrix build, signing, SBOM, provenance, upload to a draft release. |
| `workflows/release.yml` | ci | 7 | Manual approval, install smoke tests on clean runners, publish the draft. |
| `workflows/sign-verify.yml` | ci | 7 | `signtool verify /pa`, `codesign --verify --deep --strict`, `cosign verify-blob`, manifest check. |
| `workflows/reapack.yml` | ci | 6 | Builds `index.xml` and pushes it to the `reapack` branch. |
| `workflows/docs.yml` | ci | 1 | mdBook to `gh-pages` on every push to `main`. |
| `workflows/nightly.yml` | ci | 1 | Full matrix on `main`, unsigned, artefacts kept 7 days. Catches packaging breakage early. |
| `ISSUE_TEMPLATE/bug.yml` | cfg | 1 | Asks for version, OS, engine `/health` output and whether offline mode was on. Never asks for a transcript. |
| `ISSUE_TEMPLATE/feature.yml` | cfg | 1 | Problem, proposal, alternatives. |
| `pull_request_template.md` | cfg | 1 | What changed, how it was verified, changelog fragment, contract impact. |

---

## 12. `changelog.d/`

| file | kind | phase | function |
|---|---|---|---|
| `<pr>-<slug>.md` | doc | 1 | One fragment per PR, front matter `type: added\|changed\|deprecated\|removed\|fixed\|security\|breaking`, then the entry text. Consumed by `cargo xtask changelog` at release. |
| `README.md` | doc | 1 | The fragment format and the `skip-changelog` label rule. |

Until the fragment tooling lands, entries go directly under `[Unreleased]` in `CHANGELOG.md`, as
`AGENTS.md` §8 states.

---

## 13. Layering rules

These are enforced by review and, where possible, by CI.

1. `crates/` never depends on `apps/`. `apps/` may depend on `crates/`.
2. `slate-engine` never depends on `slate-audio`. The launcher and the decoder are independent.
3. `slate-tagger` may depend on `slate-ucs`; `slate-ucs` depends on nothing but `serde` and `csv`.
4. `packages/contract` is generated-from, never generated-into, except `gen/`.
5. The REAPER extension does not link any Rust crate in v1. It talks HTTP. A C ABI is a later
   optimisation for the filename fast path only.
6. Nothing under `models/` is committed. Nothing under `apps/reaper/extension/vendor/` is committed.
7. `fixtures/` stays under 2 MB total. Anything larger is generated by a script.
8. `unsafe_code` is forbidden workspace-wide except in the FFI crates, which must each carry a
   `// SAFETY:` comment on every block.

---

## 14. Where new code goes

| you are adding | it goes in |
|---|---|
| a new audio container | `crates/slate-audio/src/decode/`, plus a fixture and a round-trip test |
| a new metadata chunk | `crates/slate-tagger/src/`, plus a round-trip test and a `probe` case |
| a new engine endpoint | `packages/contract/openapi.yaml` first, then `slate-engine`, then the UI |
| a new UI screen | `apps/desktop/src/features/<name>/`, registered in `app/router.tsx` |
| a new REAPER action | `apps/reaper/extension/src/actions.cpp`, plus a Lua wrapper if it is user-facing |
| a new CLI command | `crates/slate-cli/src/cmd/`, registered in `cmd/mod.rs`, plus a `tests/cli.rs` case |
| a new build or release step | `tools/xtask/src/`, exposed as a `just` recipe |
| a new decision | `docs/adr/NNNN-<slug>.md`, and a row in `PLAN.md` §11 |

Anything that does not fit one of these rows is a design question for the owner, not a new
directory.
