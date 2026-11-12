# Slate-To-UCS — build plan

Status: draft for review. Revision 2, 2026-11-12.

Revision 1 of this document was written without `ttucs-studio.md`, which was missing from disk.
It has since been found at `ttucs/docs/plans/ttucs-studio.md` (written 2026-11-10) and this
revision is reconciled against it. Where the two disagreed, the handover brief wins, because it
records decisions the owner had already made. Section 3 lists those decisions and what changed.

This document covers the product family — a standalone desktop application, a REAPER extension and
REAPER scripts — plus repository layout, documentation, changelog, CI/CD, Windows MSI packaging
with code signing (PKI), and the two open questions (IDE, repo split).

---

## 1. Inputs and current state

### 1.1 This repository

`github.com/SoundsLikeJonny/Slate-To-UCS` at `bd0eef2` contains exactly three tracked files:

| path | contents |
|---|---|
| `README.md` | two lines: "Turn the slates from your recordings into UCS filenames and metadata" |
| `LICENSE` | MIT, Copyright (c) 2026 Jon Evans |
| `.gitignore` | the standard GitHub Python template |

Untracked and not ignored: `.idea/`, `.abacusai/`, and `docs/`. The first two should be added to
`.gitignore` before the first real commit; `.abacusai/` is the agent memory folder and must never
be committed.

### 1.2 The model repository (`~/PycharmProjects/ttucs`)

Read for this plan: `README.md`, `AGENTS.md`, `CHANGELOG.md`, `docs/RELEASING.md`,
`docs/plans/ttucs-studio.md`, and `docs/about/about_1` through `about_20`.

What matters here:

- `ttucs` is **text in, metadata out**. It never sees audio. Audio decoding, speech-to-text and
  slate segmentation are our problem, not ttucs's.
- The public API is small: `TTUCS.from_pretrained()`, `predict`, `predict_batch`, and a
  `Prediction` dataclass with `cat_id`, `category_full`, `confidence`, `decision`,
  `alternatives`, `categories`, `filename`, `keywords`, `slate`, `to_dict()`/`to_json()`.
- Weights resolve offline-first through `ttucs.hub`: explicit path, installed
  `ttucs-model-bow` package, cache, then a SHA-256-pinned GitHub Release asset.
  `TTUCS_OFFLINE=1` guarantees no network. We inherit this rather than inventing a second scheme.
- Published accuracy on the honest held-out set is 0.713 top-1 / 0.925 top-5 (n=80). The UI must
  make "the model is unsure" a first-class state, not an error.
- The conventions worth copying: Keep a Changelog, SemVer, Conventional Commits, Trusted
  Publishing, `AGENTS.md` as a rulebook, and a `docs/about/` guided tour.

### 1.3 The handover brief

`ttucs/docs/plans/ttucs-studio.md` is a handover brief for building two consumers of `ttucs`:
a desktop app and a C++ REAPER extension, both talking to one engine over one contract. It fixes
the engine surface, the shell, the STT choice, the Python packaging strategy, the repo split, the
offline/telemetry boundary, the endpoint table, the launcher contract, per-phase acceptance
criteria, and a set of REAPER gotchas. It does **not** cover packaging, code signing, CI/CD,
documentation structure or changelog mechanics — those are this document's job.

---

## 2. Product definition

Three surfaces, one engine, one contract.

| surface | user | job |
|---|---|---|
| **Slate-To-UCS Desktop** | recordist at their desk | point at a folder, transcribe slates, review suggestions worst-first, override, export, optionally rename |
| **REAPER extension** | recordist inside a session | tag selected items/takes, write notes and take names, set render patterns |
| **REAPER scripts (Lua)** | recordist who wants their own workflow | small composable actions over the same engine |

The rule that keeps this from becoming three products: **no surface owns inference.** Every
surface is a client of `ttucs serve`, and that process is the only place a model is loaded.

Naming, settled here because it lands in Add/Remove Programs and in the signature's `/d` field:

| thing | name |
|---|---|
| product / installer display name | `Slate-To-UCS` |
| desktop app | Slate-To-UCS (window title, About panel) |
| CLI | `slate` |
| engine | `ttucs serve` (a `ttucs` subcommand, not our binary) |
| REAPER extension binary | `slate_reaper.dll` / `.dylib` / `.so` |
| REAPER menu | `Extensions > Slate-To-UCS` |
| release tag | `vX.Y.Z` in this repo |

The brief's placeholder `ttucs Studio` is retired. Its open question about the app name is
answered by the repo name.

### 2.1 Non-goals for v1

Inherited from the brief, plus two of ours:

- No cloud sync, no multi-user, no model training from the UI.
- No Windows-only shortcuts that break on macOS.
- No third-party sound-library metadata format beyond a plain CSV.
- No audio classification. A slate that says "door creak" over a recording of a bird is still a
  door creak; that is a ttucs property we inherit and must not paper over.
- No Rust reimplementation of the classifier (see §3).

---

## 3. Decisions inherited from the brief

These are recorded as settled. They are not re-opened here; each row notes what this revision
changed in response.

| decision | choice | effect on this plan |
|---|---|---|
| Inference engine | the existing Python `ttucs` package, unchanged; the `bow` model is PyTorch and reimplementing it is not on the table | **Revision 1's Rust `slate-infer` phase is deleted.** ONNX export is noted as a post-v1 separate project, exactly as the brief says |
| Engine surface | `ttucs serve` — JSON over loopback HTTP | **Revision 1's Rust `axum` daemon is deleted.** The engine is a `ttucs` subcommand using `http.server.ThreadingHTTPServer` |
| Desktop shell | Tauri 2 + **React** + TypeScript + Vite; confirmed by the owner 2026-11-10, not open for substitution | Revision 1 said Svelte. Changed to React. If a Tauri limitation blocks a phase, raise it rather than switching shells |
| Speech-to-text | `whisper.cpp`, native binary, C API | Adopted. Shared by both consumers; no Python STT dependency |
| Python packaging | PyInstaller/Nuitka **onedir** sidecar, not onefile | Adopted. Torch unpacks slowly from onefile; onedir starts in ~1 s |
| Repos | `ttucs` + the consumer repos; the REAPER SDK must never be vendored in `ttucs` | Reconciled in §6: the constraint is that the SDK stays out of `ttucs`, which one consumer repo satisfies. See §6.1 |
| Offline | user data and model input never leave the machine; telemetry and auto-update are opt-in and carry no user content | Adopted as §10, with the brief's allowlist table |

Two consequences worth stating plainly:

1. **The engine is Python and stays Python.** The installer therefore carries a CPython tree and
   torch, and is 250–400 MB. The brief accepts this for v1 and so does this plan. Anyone who finds
   that unacceptable is asking for the ONNX project, which is not v1.
2. **Phase 0 is a PR to `ttucs`, not to this repo.** `ttucs serve` does not exist yet. Nothing in
   Slate-To-UCS can be built end to end until it lands.

---

## 4. Architecture

```
┌──────────────────────────────┐  ┌────────────────────────────┐  ┌──────────────────────┐
│ Slate-To-UCS Desktop         │  │ REAPER extension (C++17)   │  │ REAPER scripts (Lua) │
│  Tauri 2 shell (Rust)        │  │  worker thread + timer     │  │  call API_Slate_*    │
│  React + TS + Vite UI        │  │  WinHTTP / libcurl         │  │  or the slate CLI    │
│  zod-validated engine client │  │                            │  │                      │
└──────────────┬───────────────┘  └─────────────┬──────────────┘  └──────────┬───────────┘
               │                                │                            │
               │      HTTP/JSON on 127.0.0.1, X-TTucs-Token header           │
               └────────────────────────────────┼────────────────────────────┘
                                                ▼
                              ┌─────────────────────────────────────┐
                              │ ttucs serve  (Python, stdlib)       │
                              │  ThreadingHTTPServer, ephemeral port│
                              │  one TTUCS instance, one lock       │
                              │  /health /predict /parse            │
                              │  /taxonomy/search /taxonomy/{id}    │
                              │  /shutdown                          │
                              └──────────────────┬──────────────────┘
                                                 │ TTUCS.from_pretrained()
                                                 ▼
                              ┌─────────────────────────────────────┐
                              │ ttucs-model-bow (installed wheel)   │
                              │  fp16 safetensors, manifest-verified│
                              └─────────────────────────────────────┘

  audio side, outside the engine:
  audio file ─▶ decode/resample to 16 kHz mono ─▶ whisper.cpp ─▶ transcript ─▶ /predict
```

Why a shared server rather than a library per surface:

1. **One model load.** Loading costs ~1–2 s and a few hundred MB; sharing it is what makes
   per-item tagging in REAPER interactive.
2. **Crash isolation.** A model or decoder fault must not take down REAPER mid-session.
3. **Language freedom.** TypeScript and C++ reach the same engine with no bindings.
4. **Testability.** The whole inference path is exercisable with `curl` and contract tests.

`ttucs predict --json` stays supported for scripting and one-shot use, but is not the integration
path: 2–4 s of process start per call.

### 4.1 Audio path (ours, not the engine's)

whisper.cpp wants 16 kHz mono PCM. Field recordings are 96 kHz/24-bit BWF, often RF64.

1. **Decode** WAV/BWF/RF64 natively, plus FLAC/MP3/OGG/AAC via `symphonia` in the Tauri shell and
   a small C++ decoder path for the extension. RF64 needs a dedicated RIFF reader; it is the
   format most likely to appear and the one most likely to fail.
2. **Resample** to 16 kHz mono (`rubato` in Rust; a small polyphase resampler in C++).
3. **Segment** with Silero VAD (ONNX) where available, energy-threshold fallback otherwise.
   Slates are short and usually at the head or tail of a take, so regions are also scored by
   position and duration.
4. **Transcribe** with the bundled whisper.cpp binary. Default `base.en` (~150 MB), optional
   `small.en`. Models are SHA-256 pinned and verified, with an offline refusal, mirroring
   `ttucs.hub`.
5. **Predict** by POSTing the transcript to `/predict`.

Word timings are kept. They are not needed for v1 labelling, but they are what makes
slate-to-region alignment possible later, and discarding them would mean re-transcribing.

### 4.2 Metadata writing

| target | what we write |
|---|---|
| WAV `bext` | description, originator, originator reference, origination date/time, coding history |
| WAV `iXML` | project, scene, take, tape, note, speed, plus a `UCS` block with CatID and category |
| sidecar `.slate.json` | the full `Prediction` plus provenance (model id, sha256, engine version, timestamp) |
| filename | `CatID_FX Name_CreatorID_SourceID[_UserData]`, via the ported UCS rules |
| REAPER item | `P_NOTES` (preserving existing text), optional take name, optional render pattern |

Writes are atomic (temp file + rename), always preceded by a dry-run diff, and never destructive
without an explicit apply. Renames are journalled so a batch can be undone.

---

## 5. The contract

Phase 0 lands this in `ttucs`. It is reproduced here because every surface depends on it.

### 5.1 Command

```
ttucs serve [-m MODEL] [--host 127.0.0.1] [--port 0] [--token-file PATH]
            [--no-token] [--top-k 5] [--warm/--no-warm] [--log-level info]
```

- `--port 0` picks a free port; the bound port is written to the token file.
- `--host` defaults to `127.0.0.1` and refuses a non-loopback address. `--allow-remote` is not
  added in v1.
- `http.server.ThreadingHTTPServer` from the standard library. **No new core dependency**; a
  `[serve]` extra is not needed and would violate the small-install goal.
- `ttucs.pipeline` is imported lazily inside the command, matching the existing CLI style.
- Warm up with one `predict("test tone")` at startup; report `warm` in `/health`.

### 5.2 Endpoints

Every response is an envelope with `schema_version` (integer, starts at 1). Consumers refuse an
unknown major version and say so in the UI rather than guessing.

| method | path | body | returns |
|---|---|---|---|
| GET | `/health` | – | engine + model identity |
| POST | `/predict` | `{texts: [str], top_k?, creator_id?, source_id?}` | `{predictions: [Prediction]}` |
| POST | `/parse` | `{texts: [str]}` | `{slates: [SlateFields]}` |
| GET | `/taxonomy/search?q=&limit=` | – | `{entries: [{cat_id, category, subcategory, cat_short, explanation}]}` |
| GET | `/taxonomy/{cat_id}` | – | `{entry: {...}}` |
| POST | `/shutdown` | – | `{ok: true}` (token required) |

`/health`:

```json
{
  "schema_version": 1,
  "status": "ok",
  "warm": true,
  "ttucs_version": "0.3.0",
  "ucs_version": "8.2.1",
  "model": {"name": "bow", "version": "0.2.0", "source": "package",
            "manifest_verified": true, "path": "..."},
  "offline": true,
  "device": "cpu",
  "unknown_threshold": 0.12,
  "category_threshold": 0.5
}
```

`/predict` **always returns an array**, even for one input. This deliberately differs from
`ttucs predict --json`, which returns a bare object for a single text and an array for many — a
wart that has already bitten scripting. Phase 0 fixes that in the same PR: `cmd_predict` always
emits a list, noted in `CHANGELOG.md` under **Changed** as breaking for anyone parsing
single-input output.

`Prediction` is exactly `Prediction.to_dict()` from `src/ttucs/pipeline.py:76`:

```json
{
  "text": "...", "description": "...",
  "cat_id": "DOORCreak", "category": "DOOR", "subcategory": "Creak",
  "cat_short": "Creak", "category_full": "DOOR-Creak",
  "confidence": 0.71, "category_confidence": 0.88,
  "decision": "confident",
  "alternatives": [{"cat_id": "...", "category": "...", "subcategory": "...",
                    "cat_short": "...", "confidence": 0.71}],
  "categories": [{"category": "DOOR", "confidence": 0.88}],
  "slate": {"raw_text": "...", "description": "...", "project": null, "location": "...",
            "microphones": [], "mic_types": [], "mic_setup": [], "recorder": null,
            "sample_rate": 96000, "bit_depth": "24", "take": "3", "scene": null,
            "slate": null, "perspective": [], "date": null, "creator": null,
            "notes": null, "unparsed": []},
  "fx_name": "Wooden Door Creak Slow",
  "keywords": ["door", "creak"],
  "filename": "DOORCreak_Wooden Door Creak Slow_NONE_ThickAsThieves_MKH416",
  "filename_parts": {"cat_id": "...", "fx_name": "...", "creator_id": "...",
                     "source_id": "...", "user_category": null, "user_data": null},
  "is_unknown": false
}
```

`slate` keys are the `SlateFields` dataclass fields (`src/ttucs/slate/parser.py:34`). Unknown keys
are forward-compatible additions; never fail on them.

### 5.3 Security and lifecycle

- Bind loopback only. Require `X-TTucs-Token` on every route except `/health`; compare with
  `hmac.compare_digest`. `--no-token` exists for tests and logs a warning.
- Write the endpoint file to `$TTUCS_HOME/serve.json` (falling back to the platform cache dir,
  same resolution as `hub.cache_root()`), mode `0600`:

  ```json
  {"schema_version": 1, "host": "127.0.0.1", "port": 8731, "token": "...",
   "pid": 12345, "started": "2026-11-10T12:00:00Z", "ttucs_version": "0.3.0"}
  ```

  Delete it on clean shutdown; a stale file whose `pid` is dead means "not running".
- No CORS headers. A browser page must not reach the engine even with the token.
- One `threading.Lock` around `predict_batch`; the model is not safe to call concurrently.
  `ThreadingHTTPServer` handles accept concurrency, the lock serialises inference.
- Cap request size at 1 MB and `len(texts)` at 512 so a runaway client cannot OOM the app.
- Log to stderr only; never log tokens or full transcripts at info level.

### 5.4 Launcher contract (both consumers implement this)

1. Read `$TTUCS_HOME/serve.json`. If present and `GET /health` succeeds with the token, use it.
2. Otherwise spawn `ttucs serve --port 0 --token-file <path>` detached, poll the file for up to
   30 s, then `GET /health` until `warm` is true (60 s budget on first run).
3. On app exit, call `/shutdown` only if this process started the server.

Documented in `ttucs/docs/serve-api.md` so the C++ side implements the same steps.

### 5.5 Phase 0 acceptance

- `tests/test_serve.py`: server on port 0 in a fixture; `/health` shape; token rejection (401);
  `/predict` array shape for one and many texts; `/parse` without a model; taxonomy search;
  request-size cap; `/shutdown`.
- `tests/test_cli.py`: `predict --json` always emits a list.
- `ruff check src tests scripts` and `pytest -q` green; CI green on 3.10–3.12.
- Manual: `ttucs serve`, then `curl` `/health` and `/predict` on Windows PowerShell with
  `TTUCS_OFFLINE=1` and an empty `TTUCS_HOME`.
- `CHANGELOG.md` **Added** for `ttucs serve`, **Changed** for the `--json` shape, plus
  `docs/serve-api.md`. No weights change, so no `ttucs-model-bow` bump.

### 5.6 Machine-readable form

`packages/contract/openapi.yaml` (OpenAPI 3.1) expresses the table above so clients can be
generated and drift can be checked. It is a documentation and codegen artefact, not a runtime
dependency, so it does not conflict with the no-new-dependency rule.

| consumer | generation |
|---|---|
| TypeScript | `openapi-typescript` → `packages/contract/gen/ts`, wrapped by zod schemas |
| Rust (Tauri shell) | `progenitor` → `src-tauri/src/engine/` |
| C++ | hand-written thin client, checked by a schema test against a recorded fixture |
| Lua | none; calls the extension's `API_Slate_*` functions |

CI regenerates and fails on `git diff --exit-code`. The brief's `schema_version` integer remains
the runtime compatibility gate; OpenAPI is the build-time gate.

---

## 6. Repository layout

### 6.1 One consumer repo, not two

The brief's decision table says `ttucs` + `ttucs-studio` + `ttucs-reaper`, with the rationale
"keeps the core install small; the REAPER SDK must never be vendored here". That rationale is
about `ttucs`, and it is fully satisfied by Slate-To-UCS being a separate repository. The
constraint is honoured; the count of consumer repos is not the constraint.

Slate-To-UCS therefore holds both consumers:

- the desktop app and the REAPER extension share one wire contract, one launcher implementation
  and one UCS filename algorithm;
- a cross-surface change is one PR, one CI run, one changelog entry;
- version skew between an installed extension and a newer app is impossible by construction,
  which is the failure mode that makes DAW integrations painful.

Split only if one of these becomes true:

- a surface needs a release cadence an order of magnitude faster than the other;
- a third party needs `slate-ucs` or `slate-audio` independently — publish to crates.io from this
  repo, which does not require a new repo;
- the REAPER SDK licence review (phase 5) forbids co-location. It should not: the SDK is fetched
  at build time and never committed.

`ttucs` stays separate: published PyPI package, own weights distribution, own evaluation
discipline, own release tags. We consume it by version range.

### 6.2 Tree

```
Slate-To-UCS/
├─ README.md                     pitch, screenshots, install, quick start, installer size
├─ LICENSE                       MIT
├─ CHANGELOG.md                  assembled from changelog.d/ at release time
├─ AGENTS.md                     rulebook for contributors (human or AI)
├─ CONTRIBUTING.md               setup, checks, PR expectations
├─ SECURITY.md                   disclosure policy, signing fingerprints, verify steps
├─ CODEOWNERS
├─ version.toml                  single source of the product version
├─ Cargo.toml                    workspace (Tauri shell + audio crates)
├─ rust-toolchain.toml           pinned toolchain
├─ package.json                  pnpm workspace root
├─ pnpm-workspace.yaml
├─ justfile                      task entry points
├─ deny.toml                     cargo-deny policy
├─ .github/
│  ├─ workflows/                 ci, build, release, sign, reapack, docs, nightly
│  ├─ ISSUE_TEMPLATE/
│  └─ pull_request_template.md
├─ changelog.d/                  one fragment per PR
├─ crates/
│  ├─ slate-audio/               decode (WAV/BWF/RF64/FLAC/MP3/OGG/AAC), resample, VAD, segment
│  ├─ slate-ucs/                 UCS taxonomy, filename, keywords (port of ttucs.text/taxonomy)
│  ├─ slate-tagger/              bext, iXML, sidecar, rename journal
│  ├─ slate-engine/              launcher contract, token file, health probe, typed client
│  └─ slate-cli/                 the `slate` binary
├─ apps/
│  ├─ desktop/                   Tauri 2 app
│  │  ├─ src/                    React + TS
│  │  │  ├─ app/                 routing, layout, theme
│  │  │  ├─ features/queue/      file list, transcription progress, sorting, filtering
│  │  │  ├─ features/inspector/  slate fields, alternatives, filename editor, keywords
│  │  │  ├─ features/export/     CSV / JSONL / rename
│  │  │  ├─ features/settings/   model, thresholds, defaults, STT model, telemetry, updates
│  │  │  ├─ lib/engine.ts        typed client for the contract (single source of truth)
│  │  │  └─ lib/schema.ts        zod schemas, validated on every response
│  │  ├─ src-tauri/              Rust shell: window, drag-drop, dialogs, sidecar spawn
│  │  └─ sidecar/                PyInstaller spec + whisper.cpp fetch script
│  └─ reaper/
│     ├─ extension/              C++17 shim; SDK fetched at build time, never committed
│     └─ scripts/                Lua ReaScripts + ReaPack index template
├─ packages/
│  └─ contract/                  openapi.yaml + generated clients
├─ models/                       gitignored; manifests and pins only
├─ fixtures/                     tiny audio + transcripts for tests (< 2 MB total)
├─ docs/
│  ├─ index.md                   the map
│  ├─ about/                     guided tour, ttucs style
│  ├─ architecture/              overview, process model, contract, data model
│  ├─ adr/                       one file per decision
│  ├─ guides/                    install, desktop, reaper, cli, models, offline, telemetry
│  ├─ reference/                 generated HTTP API, CLI, UCS mapping
│  └─ release/                   versioning, signing, runbook
└─ tools/
   └─ xtask/                     build, bump, changelog, sign, verify
```

Rules that keep the tree honest:

- `crates/` never depends on `apps/`. `apps/` may depend on `crates/`.
- `slate-engine` never depends on `slate-audio`; the launcher and the decoder are independent.
- `packages/contract` is generated-from, never generated-into, except `gen/`.
- Nothing under `models/` is committed. Weights and ASR models travel as release assets with
  pinned SHA-256, exactly like `ttucs-model-bow`.
- The REAPER SDK is fetched by `xtask fetch-reaper-sdk` into a gitignored directory, with a
  pinned version and its licence file.

---

## 7. Technology choices

| layer | choice | why | rejected |
|---|---|---|---|
| desktop shell | Tauri 2 | ~10–20 MB shell, native drag-drop and dialogs, real web UI quality, MSI/NSIS/DMG/deb bundling, signed updater | Electron (size, two runtimes), Qt (licensing, C++ UI cost) — **settled in the brief** |
| UI | React + TypeScript + Vite | settled in the brief; large ecosystem for a table-and-diff review UI | Svelte (revision 1's pick, overridden), plain DOM |
| shell language | Rust | Tauri's native side; shares `slate-audio`/`slate-ucs` | C++ (no Tauri support) |
| engine | Python `ttucs serve`, stdlib `http.server` | settled in the brief; no new dependency; one model load | Rust daemon (revision 1, deleted), FastAPI (new dependency) |
| ASR | whisper.cpp native binary | settled in the brief; C/C++ so the extension uses the same binary | faster-whisper (Python + CTranslate2), cloud ASR (violates offline) |
| VAD | Silero ONNX, energy fallback | robust on quiet field recordings | webrtcvd (poor on non-speech noise), energy-only (fragile) |
| REAPER ext | C++17 + REAPER SDK | SDK-native, tiny surface, zero dependency risk | `rea-rs` 1.0 (stable Oct 2026; revisit if the shim passes ~1500 lines) |
| HTTP in C++ | libcurl everywhere | one code path across platforms | WinHTTP on Windows + libcurl elsewhere (two paths) |
| scripts | Lua | ReaScript's best-supported language, no build step | EEL2 (poor string/HTTP handling), Python ReaScript (fragile env) |
| build orchestration | `cargo xtask` + `just` | one entry point per task, no Make-on-Windows pain | Make (POSIX-shaped), npm scripts only (can't drive cargo well) |
| docs | Markdown + mdBook | reads on GitHub, builds a site in CI | Docusaurus (Node weight for docs only) |
| changelog | Keep a Changelog + `changelog.d/` fragments | no merge conflicts in a multi-surface repo | single `CHANGELOG.md` edited by every PR (conflicts) |

---

## 8. Desktop application

### 8.1 Screens

1. **Queue** (default). Drop audio files or paste transcripts. Columns: file, duration, transcript
   (truncated), CatID, confidence, decision badge, suggested filename. Sorted by confidence
   ascending by default — the review order is "worst first". Filter chips: all / confident /
   fallback / unknown / overridden.
2. **Inspector** (right pane, always visible). Transcript with parsed slate spans highlighted;
   CatID with top-5 alternatives as clickable bars; category confidence; decision badge with a
   one-line explanation of `category_fallback` and `unknown`; editable UCS filename with live
   validation against the `UCSFilename` rules; keyword chips; copy filename / copy JSON.
3. **Override**. CatID search via `/taxonomy/search`, keyboard-first, `/` focuses it. Overriding
   marks the row `overridden` and keeps the model's original guess visible for later analysis.
4. **Export**. CSV (file, cat_id, category, subcategory, confidence, decision, filename, keywords,
   plus slate fields), JSONL (raw `Prediction` objects), and rename on disk.
5. **Settings**. Model path or name, `unknown_threshold`, `category_threshold`, default creator and
   source IDs, STT model size, server port override. Engine offline mode is displayed as locked
   on; telemetry and update checks are separate switches with their own state.
6. **Transcript editor**. Fix STT errors and re-predict that row instantly. This is the
   highest-value accuracy lever in the app: real slates are mis-transcribed far more often than
   they are mis-classified.

### 8.2 Workflow and keyboard

Drop folder → transcribe (whisper.cpp, per-file progress, cancellable) → predict in batches of 32
→ review worst-first → accept or override → export.

`j`/`k` next/prev, `Enter` accept, `e` edit transcript, `c` copy filename, `o` override, `Space`
play the file, `Cmd/Ctrl+E` export. Every action is reachable by mouse too.

### 8.3 Rename safety (the only destructive feature)

- Dry run by default: a two-column old → new diff, with collisions and invalid names flagged
  before anything moves.
- Never overwrite an existing file; skip and report.
- Write `slate-undo-<timestamp>.json` next to the folder and offer one-click undo.
- Refuse to run on a folder containing subfolders unless the user opts in.

### 8.4 Packaging

- Tauri bundle per platform; the sidecar is a PyInstaller onedir tree in `resources/`.
- Bundle the `ttucs-model-bow` wheel contents so the app works with no network and no separate
  pip install; set `TTUCS_OFFLINE=1` in the sidecar environment.
- Installer size expectation: **250–400 MB** because of torch. State this in the README rather
  than pretending otherwise.
- The whisper model is not bundled by default; it is downloaded once, SHA-256 verified, and
  cached, or shipped in an optional offline bundle.

---

## 9. REAPER extension and scripts

### 9.1 Extension

C++17 against the REAPER extension SDK, fetched at build time. Output
`slate_reaper.dll` / `.dylib` / `.so` into `UserPlugins`. Menu under
`Extensions > Slate-To-UCS`.

| action | behaviour |
|---|---|
| Tag selected items | for each selected item, get a transcript from (in order) item notes, an adjacent `.txt`, or whisper.cpp on the source file; POST `/predict`; write `P_NOTES` with the UCS filename plus CatID, category and confidence; optionally set the take name |
| Tag all items in project | same, batched, with a progress dialog and cancel |
| Apply render pattern | set the project render filename pattern from the UCS fields |
| Open Slate-To-UCS | launch the app |

Registers `API_Slate_*` functions so Lua scripts reach the same code paths without HTTP.

Gotchas that would otherwise cost a day each:

- **Never call HTTP on the main thread.** Run requests on a worker thread and marshal results
  back with `defer` / `plugin_register("timer", ...)`.
- **Start the engine from the extension** using §5.4, including the 30 s poll. Show a modal
  "starting engine" state; do not silently hang.
- `GetSetMediaItemInfo_String(item, "P_NOTES", ...)` for notes;
  `GetSetMediaItemTakeInfo_String` for take names. Read `P_NOTES` first and preserve existing text.
- Wrap writes in `Undo_BeginBlock` / `Undo_EndBlock` so a batch tag is one undo step.
- Degrade gracefully when the engine is missing: show the error and a link to install
  instructions, never crash the host.
- Build win x64, macOS universal, linux x86_64. There is no installer framework in REAPER;
  document the copy step.

### 9.2 Scripts

- Lua ReaScripts in `apps/reaper/scripts/`, one file each with a header block.
- They call `API_Slate_*` when the extension is installed and fall back to
  `reaper.ExecProcess("slate ...")` when it is not. One code path per script.
- Distributed through ReaPack via a generated `index.xml` on a `reapack` branch. ReaPack does not
  install native extensions, so the binary ships with the app installer and the app offers
  **Install REAPER integration** (see §13.6).

---

## 10. Telemetry and auto-update

Both are wanted. Both may use the network. Neither may carry user content.

### 10.1 The line

| stays on the machine, always | may leave the machine |
|---|---|
| audio files and their paths | app version, OS and arch, locale |
| transcripts and slate text | model name/version, `schema_version` |
| CatIDs, filenames, keywords, exports | counts and timings (files processed, ms per predict) |
| project, location, creator/source IDs | decision mix (`confident` / `fallback` / `unknown`) |
| the user's overrides and corrections | crash type and stack, with paths and text stripped |

Rule of thumb: **aggregate numbers and identifiers of the build may leave; anything a recordist
typed or recorded may not.** A transcript is the most sensitive field in the app — it carries
project names, locations and client names.

### 10.2 Telemetry

- **Opt-in on first run**, one clear screen, default off until the user chooses. One switch in
  Settings, revocable at any time, with a "what is sent" panel showing the exact payload schema.
- Anonymous install ID: a random UUID in the app config, resettable from Settings. No hardware
  fingerprint, no account, no email.
- Events are a fixed, versioned allowlist (`telemetry_schema_version`). Adding a field is a schema
  bump and a changelog entry. No free-form strings, no exception messages that could embed a path
  or a transcript.
- Batch and send on a timer, not per action; drop the batch if the send fails. Never block the UI
  and never retry into a loop.
- Respect `DO_NOT_TRACK=1` and a `SLATE_TELEMETRY=0` environment override.
- Document the endpoint and retention period in the README, and give the user a delete-my-data
  route.
- The sidecar never sends anything: telemetry lives in the shell, so `TTUCS_OFFLINE=1` still means
  zero network from the engine.

### 10.3 Auto-update

- Tauri's updater plugin against a signed manifest; **signature verification is mandatory** and a
  failed check aborts the update rather than installing.
- Check on launch and every 24 h on a background thread. A failed check is silent and logged
  locally; it must never block startup or show a modal.
- Notify, do not force: a non-blocking banner with "Install and restart" / "Later". No silent
  install while a batch is running, and never mid-rename.
- Channels: `stable` by default, `beta` opt-in in Settings. The manifest points at GitHub Releases
  in this repo, separate from the `ttucs` and `ttucs-model-bow` releases.
- Weights update independently of the app: a new `ttucs-model-bow` is a sidecar payload change, so
  app version and weights version are shown separately in About.
- Offline users are unaffected: with no network the check fails quietly. Never gate a feature on a
  successful update check.

### 10.4 Acceptance

- With telemetry off and no network, the app is fully functional and makes zero outbound
  connections (verify with a proxy log).
- With telemetry on, capture one batch and assert against the allowlist that no field contains a
  transcript, filename, path or CatID from the test corpus.
- Auto-update: serve a manifest with a bad signature and confirm the update is refused with a
  clear message and the app keeps running.

---

## 11. Documentation

Four kinds, kept separate so each stays useful:

| kind | where | rule |
|---|---|---|
| **explanation** | `docs/about/` | the guided tour: what it is, how a slate becomes metadata, where the code lives. Mirrors ttucs's 20-document structure |
| **reference** | `docs/reference/` | generated where possible: HTTP API from `openapi.yaml`, CLI from `--help`, UCS mapping from the taxonomy tables. Never hand-edited |
| **how-to** | `docs/guides/` | task-shaped: install on Windows, install the REAPER integration, run fully offline, point at a folder, undo a batch, turn telemetry off |
| **decisions** | `docs/adr/` | one page per irreversible choice: context, decision, consequences, revisit trigger |

Contract documents at the root, matching ttucs:

- `README.md` — pitch, the 60-second path to a first labelled file, and the installer size.
- `AGENTS.md` — hard rules. Non-negotiable: never push `main`, never commit models or weights,
  offline-first, truthful reporting, no third-party sound-library metadata anywhere, only the
  parsed description is classified, the REAPER SDK is never committed.
- `SECURITY.md` — disclosure address, signing certificate fingerprints, verify-a-download steps.
- `CONTRIBUTING.md` — setup, checks, PR shape, changelog fragment requirement.
- `MODEL_CARD.md` / `DATA_CARD.md` — for the ASR model we ship and the classifier we consume. We
  cite ttucs's version and link; we do not restate its numbers as our own.

Docs build: mdBook to `gh-pages` on every push to `main`, with a link checker and
`cargo xtask docs-check` failing on broken relative links and undocumented public items.

ADR seed list:

| # | decision |
|---|---|
| 0001 | one consumer repo (desktop + REAPER), `ttucs` separate |
| 0002 | Tauri 2 + React for the desktop shell |
| 0003 | `ttucs serve` as the only inference owner |
| 0004 | OpenAPI as the build-time contract, `schema_version` as the runtime gate |
| 0005 | C++ REAPER shim over HTTP, no C ABI in v1 |
| 0006 | ttucs consumed as a pinned dependency, never vendored |
| 0007 | Python onedir sidecar, no Rust classifier in v1 |
| 0008 | MSI as the primary Windows package, MSIX deferred |
| 0009 | Azure Artifact Signing for CI code signing |
| 0010 | changelog fragments, single product version |
| 0011 | telemetry opt-in with a fixed allowlist |

---

## 12. Versioning and changelog

### 12.1 Versions

- **One product version** for the bundle: desktop app, REAPER extension, scripts and `slate` CLI
  ship together as `X.Y.Z`. `version.toml` is the single source; `xtask bump` propagates it.
- **Independent versions only for registry-published artefacts**, if and when they exist.
- **SemVer 2.0.0.** Pre-1.0 we still bump minor for breaking changes and say so.
- **The contract has its own version**: `schema_version` (runtime) and `info.version` in
  `openapi.yaml` (build time). A breaking contract change bumps the product major and is marked
  **Breaking:**.
- **ttucs compatibility is declared, not assumed**: `ttucs>=0.3,<0.4` in the sidecar's
  `pyproject.toml`, plus a runtime check that `/health`'s `ttucs_version` satisfies it. A mismatch
  is a startup error with a clear message, not a silent misbehaviour.

### 12.2 Changelog

Keep a Changelog 1.1.0, same as ttucs, with one structural change forced by the multi-surface
repo: **fragments, not a shared file.**

- Every PR adds `changelog.d/<pr>-<slug>.md` with a front-matter type (`added`, `changed`,
  `deprecated`, `removed`, `fixed`, `security`, `breaking`).
- CI requires a fragment for any PR touching `crates/`, `apps/`, `packages/contract/` or
  `tools/`, unless the PR carries the `skip-changelog` label.
- At release, `just changelog X.Y.Z` assembles fragments into `CHANGELOG.md` under
  `## [X.Y.Z] - YYYY-MM-DD`, in the fixed group order, deletes the consumed fragments, and
  refreshes the compare links.
- `[Unreleased]` stays at the top and is empty between releases.
- Behaviour or accuracy changes carry before/after numbers and name the test set, as ttucs does.
- Breaking changes are prefixed **Breaking:** and repeated in the release notes.

This avoids the one real problem with a single `CHANGELOG.md` here: every PR edits the same lines
at the top of the file and every merge conflicts.

Phase 0's `predict --json` shape change is a `ttucs` changelog entry, not ours, but it is called
out in our release notes because it changes what our clients receive.

---

## 13. CI/CD

### 13.1 Workflows

| workflow | trigger | jobs |
|---|---|---|
| `ci.yml` | push to `main`, every PR | fmt, clippy, cargo test (3 OS), tsc + vitest, contract drift, changelog fragment, `cargo-deny`, `cargo-audit`, `osv-scanner`, docs link check |
| `build.yml` | tag `v*`, manual | matrix build of app, CLI, REAPER extension; sign; SBOM; provenance; upload to a **draft** release |
| `release.yml` | manual approval on a draft | install smoke tests on clean runners, then publish the draft |
| `sign-verify.yml` | after `build` | `signtool verify /pa`, `codesign --verify --deep --strict`, `cosign verify-blob`, manifest check |
| `reapack.yml` | tag `v*` | build `index.xml`, push to the `reapack` branch |
| `docs.yml` | push to `main` | mdBook build to `gh-pages` |
| `nightly.yml` | cron | full matrix on `main`, unsigned, artefacts kept 7 days |

### 13.2 Matrix

| target | runner | artefacts |
|---|---|---|
| Windows x64 | `windows-2022` | `.msi`, `.exe` (NSIS), `slate.exe`, `slate_reaper.dll` |
| Windows arm64 | `windows-2022` cross | `.msi`, `.exe` (phase 8) |
| macOS arm64 | `macos-14` | `.dmg`, `slate`, `slate_reaper.dylib` |
| macOS x64 | `macos-13` | `.dmg`, `slate`, `slate_reaper.dylib` |
| Linux x64 | `ubuntu-22.04` | `.deb`, `.rpm`, `.AppImage`, `slate`, `slate_reaper.so` |

`ubuntu-22.04`, not `-latest`: it pins glibc 2.35 so the binary runs on Debian 12, Ubuntu 22.04+
and current Fedora. Building on 24.04 silently raises the floor.

### 13.3 Merge gates

1. `cargo clippy --all-targets -- -D warnings`
2. `cargo test --workspace`
3. `pnpm -r typecheck && pnpm -r test`
4. contract regeneration produces no diff
5. a changelog fragment exists
6. `cargo-deny` licences and advisories clean
7. docs links resolve

Branch protection on `main`: required checks, linear history, no force-push, no direct push, one
review. Tags `v*` protected; only the owner creates them, matching the ttucs rule.

### 13.4 Release sequence

1. PR with the version bump (`just bump X.Y.Z`), assembled changelog, and any model pin updates.
   Owner merges.
2. Owner tags the merged commit: `git tag -a vX.Y.Z -m "Slate-To-UCS X.Y.Z"` and pushes.
3. `build.yml` builds every target, signs, attests, uploads to a **draft** release.
4. `sign-verify.yml` proves each signature and each SHA-256 manifest.
5. `release.yml` runs install smoke tests on clean runners:
   - Windows: `msiexec /i Slate-To-UCS-X.Y.Z-x64.msi /qn /l*v install.log`, then
     `slate --version`, then `slate doctor`, then uninstall and check no residue.
   - macOS: mount the DMG, copy, `spctl --assess --type execute`, run `slate --version`.
   - Linux: `dpkg -i`, run `slate --version`, `dpkg -r`.
6. Owner approves; the draft is published. Release notes are the changelog section.
7. `reapack.yml` publishes the script index; `docs.yml` publishes the site.

Nothing is published automatically from a tag without the approval step. A tag is a build
request, not a release.

### 13.5 Supply chain

- `actions/attest-build-provenance` on every artefact (SLSA build provenance).
- `cosign sign-blob` keyless via GitHub OIDC for every artefact, plus `SHA256SUMS` and
  `SHA256SUMS.sig`.
- SBOM per platform: `cargo-cyclonedx` for Rust, `syft` for the bundled Python and Node trees.
- `permissions: contents: read` by default; `id-token: write` only where signing or attestation
  happens.
- No long-lived signing secrets. Windows uses OIDC federation to Azure; macOS uses an App Store
  Connect API key in an environment secret; Linux uses keyless Sigstore.
- Dependabot or Renovate for cargo, npm and actions, grouped weekly.

---

## 14. Windows MSI and PKI

### 14.1 What gets built

Tauri's WiX bundler produces the `.msi`; its NSIS bundler produces the `.exe`. Ship both:

| package | audience | notes |
|---|---|---|
| `.msi` | enterprise, GPO/Intune/SCCM, per-machine | WiX v3 under the hood; requires the Windows VBSCRIPT feature on the build machine or `light.exe` fails |
| `.exe` (NSIS) | individual users, per-user, no admin prompt | smaller, faster, no MSI servicing |

MSI is primary because it is what managed environments deploy. MSIX is deferred: its filesystem
virtualisation makes writing into `%APPDATA%\REAPER\UserPlugins` awkward and it brings Store-policy
constraints we do not need (ADR-0008).

### 14.2 MSI correctness details that bite later

- **Stable `UpgradeCode`**, generated once and never changed. Changing it breaks every future
  upgrade path.
- **`MajorUpgrade`** with `DowngradeErrorMessage`, so `X.Y.Z` cleanly replaces `X.Y.W`.
- **Product code** changes every version; upgrade code does not.
- **Install scope** decided once: per-machine to `%ProgramFiles%\Slate-To-UCS`, with the REAPER
  extension *not* written into a user profile by the MSI (§14.6).
- **Component GUIDs** stable per file path; never `Guid="*"` on a component that moves.
- **ICE validation** in CI (`light.exe -ice:all`), warnings treated as errors.
- **Silent install and uninstall** proven in CI, including the upgrade-over-previous-version case,
  not just a clean install.
- **No custom actions requiring elevation at uninstall**; MSI repair must work.

### 14.3 The PKI chain

Code signing is a public-key infrastructure with four parts: an identity validated by a
certificate authority, a private key that never leaves a FIPS-validated HSM, a signature over a
hash of the file, and a trusted timestamp that keeps the signature valid after the certificate
expires.

```
Microsoft Root CA
   └─ Microsoft ID Verified CS AOC CA / DigiCert Trusted Root
        └─ leaf: "Slate-To-UCS" code signing certificate (72 h, renewed daily)
             └─ private key in FIPS 140-2 Level 3 HSM (Microsoft-managed)
                  └─ signtool sign /fd SHA256 /tr <RFC3161> /td SHA256
                       └─ Authenticode signature in the PE / MSI
                            └─ SmartScreen, UAC publisher name, Intune trust
```

Since June 2023 the CA/Browser Forum requires publicly trusted code-signing private keys to live
on FIPS 140-2 Level 2 (or Common Criteria EAL4+) hardware. A `.pfx` on disk is no longer
acceptable for a publicly trusted certificate, which is why this plan is cloud-HSM-first.

### 14.4 Certificate options, and the recommendation

| option | how it signs in CI | cost | verdict |
|---|---|---|---|
| **Azure Artifact Signing** (formerly Azure Trusted Signing) | `azure/trusted-signing-action` with OIDC; keys in Microsoft FIPS 140-2 L3 HSM; short-lived certs | Basic $9.99/mo, 5,000 signatures, $0.005 overage; Premium $99.99/mo, 100,000 | **recommended** |
| EV certificate on a hardware token | needs a self-hosted runner with the token attached; cannot run on hosted runners | ~$300–600/yr + runner | fallback only |
| OV certificate + cloud signing service (SSL.com eSigner, DigiCert KeyLocker) | vendor CLI in CI | ~$200–500/yr | viable alternative |
| Self-signed certificate | works, but no public trust | free | internal test builds only |

Azure Artifact Signing notes that matter:

- Individual enrolment is limited to the **United States and Canada** and needs an
  "Individual"-type Azure billing account. Organisation enrolment needs verifiable business
  history. Confirm eligibility before committing to it.
- Certificates are valid ~72 hours and renew daily, so **RFC 3161 timestamping is mandatory**.
  Without `/tr`, every signature dies within three days.
- It produces an OV-level identity, not EV. SmartScreen reputation accrues per certificate and per
  file over time; EV would grant reputation immediately. Budget for the first few hundred
  downloads showing a warning, and say so in the install guide rather than pretending otherwise.
- Billing starts at account creation and is not prorated.

### 14.5 Signing order and what gets signed

Sign bottom-up, then bundle, then sign the bundle:

1. `slate.exe`, the Tauri app `.exe`, `slate_reaper.dll`, and the sidecar's launcher — each signed
   individually. SmartScreen evaluates the binary the user launches, not only the installer.
2. Build the MSI.
3. Sign the MSI itself.
4. Never rebuild after signing. The hash that was signed is the hash that ships.

Tauri supports this directly: `bundle.windows.certificateThumbprint` + `timestampUrl` for a local
certificate store, or `bundle.windows.signCommand` for an external signer. In CI we use
`signCommand`, because the certificate is never on disk.

```powershell
signtool sign /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 `
  /d "Slate-To-UCS" /a "Slate-To-UCS-1.0.0-x64.msi"
signtool verify /pa /v "Slate-To-UCS-1.0.0-x64.msi"
```

Verification the user can run, documented in `SECURITY.md`:

```powershell
Get-AuthenticodeSignature .\Slate-To-UCS-1.0.0-x64.msi | Format-List Status, SignerCertificate, TimeStamperCertificate
```

Expected: `Status = Valid`, signer `CN=Slate-To-UCS`, a timestamp present.

### 14.6 REAPER integration install path

The MSI installs `slate_reaper.dll` into the application directory, not into a user profile. The
app then offers **Install REAPER integration**, which:

1. detects REAPER resource paths (`%APPDATA%\REAPER`, plus portable installs found by scanning for
   `reaper.ini`);
2. copies the matching `.dll`/`.dylib`/`.so` into `UserPlugins`;
3. copies the Lua scripts into `Scripts\Slate-To-UCS`;
4. writes a manifest so the action can be reversed cleanly.

This avoids the classic MSI problem of a per-machine installer writing into every user's
`%APPDATA%`, and it handles portable REAPER installs, which a fixed MSI path cannot.

### 14.7 Key custody and rotation

- No signing key material in the repository, in Actions secrets, or on a developer machine.
- The Artifact Signing account is scoped to a dedicated resource group; the GitHub federated
  credential is limited to this repository and the `release` environment.
- The `release` environment requires a reviewer; only it can obtain a signing token.
- Rotation is automatic (daily short-lived certificates). The identity certificate and the
  timestamping chain are re-issued by the CA.
- Revocation: if the signing identity is compromised, disable the account, publish an advisory in
  `SECURITY.md`, and re-sign the current release with a new identity. Timestamped older releases
  remain verifiable, which is the point of timestamping.

### 14.8 macOS and Linux equivalents

| platform | signing | notarisation | verification |
|---|---|---|---|
| macOS | Developer ID Application certificate, hardened runtime, `--options runtime` | `notarytool submit --wait`, then `stapler staple` | `codesign --verify --deep --strict`, `spctl --assess` |
| Linux | no OS-level signature; GPG-sign the apt/yum repository metadata | n/a | `cosign verify-blob`, `SHA256SUMS.sig`, `rpm --checksig` |

Tauri handles macOS signing and notarisation through `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and
`APPLE_TEAM_ID`. Prefer an App Store Connect API key (`.p8`) over an app-specific password: it
does not expire with an Apple ID password change.

---

## 15. Phases

Phase 0 is a PR to `ttucs`. Phases 1–8 are this repo. Each phase ends with something installable
and a stated exit criterion; no phase is "90% done".

| phase | repo | delivers | exit criterion |
|---|---|---|---|
| **0. Contract** | `ttucs` | `ttucs serve`, `docs/serve-api.md`, `predict --json` list shape, `test_serve.py` | every endpoint covered; token rejection works; `curl` round-trip on Windows with `TTUCS_OFFLINE=1` and an empty `TTUCS_HOME` |
| **1. Shell** | this | Tauri 2 + React app, engine client, launcher contract, About panel | app opens, `/health` shown, single-text predict round-trips |
| **2. Review** | this | queue, whisper.cpp transcription, inspector, override, CSV/JSONL export | 200 files dropped, transcribed, predicted, reviewed, exported; **no user content leaves the machine** (proxy log or firewall rule) |
| **3. Edit** | this | transcript editor, instant re-predict, cancellable STT | transcript edit re-predicts in under 200 ms; STT cancel leaves no orphan processes |
| **4. Rename** | this | dry-run diff, journal, undo | dry run on a copy of a real folder; undo restores every name byte-for-byte |
| **5. REAPER extension** | this | C++ shim, actions, `API_Slate_*`, timer marshalling, undo blocks | tag 20 selected items; notes correct; one undo step; no frame stall over ~50 ms; engine-absent path reports cleanly |
| **6. Scripts** | this | Lua scripts, `index.xml`, ReaPack branch | scripts install via ReaPack and work with and without the extension present |
| **7. Packaging and signing** | this | WiX MSI, NSIS exe, Azure Artifact Signing, install smoke tests, `SECURITY.md` | `signtool verify /pa` passes on a downloaded MSI; silent install, upgrade and uninstall pass on a clean VM |
| **8. Platform polish** | this | DMG, notarisation, deb/rpm/AppImage, GPG repo, Windows arm64 | `spctl --assess` passes; `dpkg -i` and `dnf install` work from the signed repo |

Ordering note: packaging lands at phase 7 rather than phase 1 so the installer includes the
REAPER extension and the final sidecar layout. Unsigned MSIs are built from phase 1 onward in
`nightly.yml`, so packaging breakage is caught early without blocking feature work.

### 15.1 Conformance across languages

The risk in porting `ttucs.text` and `ttucs.taxonomy` to Rust and C++ is silent divergence. The
defence is a golden corpus:

1. `tools/gen_golden.py` runs the installed ttucs over `examples/dev_descriptions.jsonl` and
   `examples/realistic_slates.jsonl` and writes `fixtures/golden/predictions.jsonl` with the full
   `Prediction` dict.
2. `crates/slate-ucs/tests/conformance.rs` asserts equality on `cat_id`, `filename`, `keywords`
   and `decision`.
3. CI regenerates the corpus against the pinned ttucs version and fails on drift, so a ttucs
   upgrade that changes filenames is caught before release.

The held-out realistic slates are used here only as an equality fixture, never to tune thresholds
or to quote accuracy in the UI.

---

## 16. Risks

| risk | impact | mitigation |
|---|---|---|
| Phase 0 never lands | nothing can be built end to end | treat it as the first deliverable and block phase 1 on it; it is a small, self-contained PR |
| ttucs accuracy on real slates (0.713 top-1, n=80) | users see wrong CatIDs and lose trust | review-first UI, top-5 always visible, `decision` surfaced, one-key override, corrections logged locally |
| Torch sidecar makes the installer 250–400 MB | install friction, AV false positives | accept for v1 and say so in the README; ONNX export is the later lever, a separate project |
| RF64/BWF decode gaps | field recordings fail to open | dedicated RIFF/RF64 reader; fixtures from real recorders; optional ffmpeg sidecar, never required |
| Tauri webview differs across platforms (WebView2 / WKWebView / WebKitGTK) | UI breakage on one OS | test Windows and macOS before phase 2 sign-off; keep CSS conservative |
| STT quality dominates end-to-end accuracy | wrong CatIDs from wrong transcripts | make the transcript editable and prominent; never hide it behind a disclosure |
| Azure Artifact Signing eligibility | no public trust in CI | confirm enrolment before phase 7; fall back to a cloud signing service or a self-hosted runner with an EV token |
| SmartScreen warnings on first releases | install friction | document it, publish the fingerprint, consider EV if volume justifies it |
| REAPER SDK licensing | distribution blocked | fetch at build time, never vendor, pin the version, review before phase 5 |
| contract drift between surfaces | version-skew bugs | generated clients, drift check in CI, `schema_version` in `/health`, fixture-based contract test |
| telemetry accidentally captures user content | privacy breach | fixed allowlist, no free-form strings, a test asserting the payload against a corpus of transcripts and paths, visible "what is sent" panel |
| auto-update ships a broken build | users stranded | signed manifest, `beta` channel first, rollback path keeping the previous sidecar and app bundle |

---

## 17. Open decisions

1. **Azure Artifact Signing eligibility** — individual (US/CA only) or organisation enrolment.
   Decides §14.4 and blocks phase 7.
2. **Minimum REAPER version** — 6.x or 7.x only. Affects the SDK pin and the API surface.
3. **Rename in v1 or v1.1** — the brief leaves this open. Recommendation: v1, because the dry-run
   diff and undo journal make it safe, and it is the feature that saves the most time.
4. **Windows arm64** — phase 8 or out of scope.
5. **Whether `slate` CLI ships in the MSI** — recommendation: yes, it is the scripting path and
   costs one binary.

Answered by this revision: the app name (`Slate-To-UCS`), the repo split (§6.1), the shell
(React, per the brief), and the classifier language (Python, per the brief).

---

## 18. Answers to the two direct questions

### 18.1 Which JetBrains IDE

**IntelliJ IDEA Ultimate**, one project for the whole repo.

- Rust: the Rust plugin has feature parity with RustRover (Cargo, macro expansion, refactoring,
  LLDB/GDB debugging) and is available to IDEA Ultimate licence holders.
- C/C++: available through the CLion plugin. Note that CLion's classic engine was unbundled in
  2026.2 and CLion Nova has been the default since 2025.3, so C++ insight in IDEA Ultimate is
  good, but native debugging and profiling remain stronger in CLion itself.
- TypeScript/JS, Python, TOML, JSON Schema for `tauri.conf.json`, GitHub Actions and Docker all
  work in the same project window.
- It handles a Cargo workspace, a pnpm workspace and a Python sidecar side by side, which is
  exactly this repo.

PyCharm cannot do this: no Rust, no C/C++, so it would cover only the sidecar. Staying on PyCharm
means adding RustRover and CLion — three licences, three windows, three indexing passes over one
repo.

If cost is the constraint: RustRover is free for non-commercial use, PyCharm Community covers the
sidecar, and CLion is only needed for the REAPER shim. That works, but it is three IDEs for one
product, and the C++ surface is small enough that IDEA Ultimate is cheaper in time.

### 18.2 One repo or several

**One consumer repo**, with `ttucs` separate. Reasons in §6.1. The short version: the app and the
extension share a wire contract, a launcher and a filename algorithm, and a cross-surface change
must be one PR, one CI run and one changelog entry. The brief's actual constraint — the REAPER SDK
never enters `ttucs` — is satisfied either way.

---

## 19. Immediate next steps

1. Add `.idea/` and `.abacusai/` to `.gitignore`.
2. Land this plan, then the ADRs it seeds, as the first real PR.
3. Write `AGENTS.md` with the hard rules from §11 before any code exists.
4. Open the Phase 0 PR against `ttucs` (`feat/serve-api`) — it blocks everything else.
5. In parallel, scaffold this repo: `version.toml`, Cargo workspace, pnpm workspace, `xtask`,
   `packages/contract/openapi.yaml`, `ci.yml` on three operating systems.
6. Confirm the five open decisions in §17.
