# AGENTS.md

Guidance for AI coding agents (and new contributors) working on **Slate-To-UCS**. Read this before
changing anything. Human-facing docs: [README.md](README.md), [SECURITY.md](SECURITY.md) and
[docs/PLAN.md](docs/PLAN.md). `CONTRIBUTING.md` and `docs/release/` are planned and land with the
phase 1 scaffold.

## 1. What this project is and why it exists

Slate-To-UCS turns **spoken field-recording slates** into
[Universal Category System](https://universalcategorysystem.com/) (UCS v8.2.1) metadata, from
inside a recordist's own workflow rather than from a command line.

It is the consumer side of [`ttucs`](https://github.com/SoundsLikeJonny/ttucs). `ttucs` is text in,
metadata out: it never sees audio. This project supplies everything `ttucs` deliberately does not —
audio decoding, speech-to-text, slate segmentation, a review UI, and writing metadata back into
files and into REAPER.

Three surfaces, one engine, one contract:

- **Slate-To-UCS Desktop** — Tauri 2 shell, React + TypeScript UI. Point at a folder, transcribe,
  review worst-first, override, export, optionally rename.
- **REAPER extension** — C++17, tags selected items and takes, sets render patterns.
- **REAPER scripts** — Lua ReaScripts over the same engine.

All three are clients of `ttucs serve`, a loopback HTTP server that owns the only loaded model.
No surface owns inference.

**Owner's goals (keep these in mind when making trade-offs):**

- A simple, minimal, genuinely useful tool for sound designers and recordists. Prefer small, clear
  code and a small install over features.
- **User data and model input never leave the machine.** Telemetry and auto-update are opt-in and
  carry no user content.
- Everything published must be **redistributable** (MIT code, MIT weights, public UCS list, REAPER
  SDK fetched at build time and never committed).
- The main quality goal is **accuracy on real slate wording**, not on synthetic test sets. The
  published classifier scores 0.713 top-1 / 0.925 top-5 on 80 held-out realistic slates, so the UI
  must make "the model is unsure" a first-class state, never an error.
- The highest-value accuracy lever in the app is the **editable transcript**, not the classifier.
  Real slates are mis-transcribed far more often than they are mis-classified.

## 2. Hard rules

1. **Git.** Work on a feature branch and open a PR to `main`. **Never merge a PR yourself, never
   push to `main`, and never force-push `main`.** The owner reviews and merges. Tags `v*` are
   protected and only the owner creates them.
2. **No third-party sound-library metadata.** Do not add, download, train on or reference any
   commercial or third-party sound-effects library's metadata. This applies to the repo, docs,
   fixtures, screenshots, commit messages and PRs, so do not name such libraries at all. Test
   fixtures use synthetic or owner-recorded audio only.
3. **Never commit** `models/`, `target/`, `node_modules/`, `dist/`, `build/`, `*.safetensors`,
   `*.pt`, `*.onnx`, `*.gguf`, `*.bin` model files, tarballs, credentials, the REAPER SDK, or the
   agent-platform marker file `.abacus.donotdelete`. Stage files explicitly; do not use
   `git add -A` or `git add .`.
4. **Offline first.** `TTUCS_OFFLINE=1` must guarantee zero network access from the engine and the
   sidecar. The app shell may reach a telemetry endpoint and an update feed, and only under the
   rules in section 6. Nothing may touch the network at import time or during prediction.
5. **Only the parsed description is classified.** Slate fields (location, gear, take, ...) are
   shown in the UI and returned by the engine, but must never be sent into the classifier.
6. **Evaluation hygiene.** `examples/realistic_slates.jsonl` in `ttucs` is the **held-out test
   set**. Never tune thresholds, UI copy, defaults or demo numbers on it. Tune on
   `examples/dev_descriptions.jsonl`. In this repo it appears only as an equality fixture for
   conformance tests, never as a source of accuracy claims.
7. **Truthful reporting.** Never say something "works", is "verified" or is "released" unless you
   ran it and saw the result. Test the real user path: a clean install of the built artefact, the
   real binary, under no network when offline behaviour matters. State exactly what was and was not
   checked. Lead with failures and caveats.
8. **Changelog and SemVer** for every user-visible change (section 8).
9. **Secrets.** Never paste tokens into commits, issues, PRs, logs or chat. Signing keys never
   exist on disk or in Actions secrets; Windows signing uses OIDC federation to Azure, macOS uses an
   App Store Connect API key in an environment secret, Linux uses keyless Sigstore.
10. **Privacy.** Do not add the owner's personal details (address, contact info and so on) to the
    repo. Author metadata stays as it is. Never log tokens or full transcripts at info level.
11. **The contract is not negotiable at runtime.** Clients refuse an unknown `schema_version` and
    say so in the UI rather than guessing. A breaking contract change bumps the product major and is
    marked **Breaking:** in the changelog.
12. **The REAPER SDK is fetched, never vendored.** `xtask fetch-reaper-sdk` pulls a pinned version
    into a gitignored directory with its licence file. It must never enter `ttucs` either.
13. **Code style is [docs/CODE_STYLE.md](docs/CODE_STYLE.md), and it is binding.** Per-language
    naming, doc comments, comment content and formatting are specified there. Anything a formatter
    or linter can decide is not a review comment. Public API gets a doc comment; a comment explains
    why, never what. No commented-out code, no banner art, no attribution comments. SDK symbols keep
    their exact spelling and are wrapped in adapters.
14. **Tests are [docs/TESTING.md](docs/TESTING.md), and it is binding.** Every `pub` function, every
    error variant, every branch and every boundary gets a test. A bug fix ships with the test that
    would have caught it. Tests are deterministic: no network, no wall clock, no fixed port, no
    unseeded randomness, no `sleep` to fix a race, no retries to green. Coverage floors are enforced
    per crate. A conformance failure is never fixed by loosening the assertion.
15. **Assertions are for invariants, not error handling.** `debug_assert!` in Rust for internal
    invariants, `invariant()` in TypeScript, `assert()` in C++ only where `NDEBUG` cannot strip
    something needed, `assert` in Lua for argument validation, and never `assert` in Python
    production code because `python -O` removes it. Every assertion message names the invariant and
    the values, and no assertion has a side effect.

## 3. How it works (high level)

```
audio file
   │
   ▼
slate-audio (decode WAV/BWF/RF64/FLAC/MP3/OGG/AAC → 16 kHz mono → VAD → segment)
   │
   ▼
whisper.cpp (native binary, shared by both consumers)
   │
   ▼  transcript
ttucs serve  ──▶  /parse  ──▶ slate fields
   │                  └────▶ description ──▶ classifier ──▶ Prediction
   │                                          (CatID + Category heads,
   │                                           temperature calibration)
   └──▶  /predict  ──▶  cat_id, confidence, decision, alternatives,
                         filename, keywords, slate
   │
   ▼
slate-tagger (bext, iXML, sidecar .slate.json, rename journal)
   │
   ▼
REAPER P_NOTES / take name / render pattern
```

- **Engine** (`ttucs serve`, in the `ttucs` repo): `http.server.ThreadingHTTPServer` from the
  standard library, loopback only, `X-Ttucs-Token` on everything except `/health`, no CORS, 1 MiB
  and 512-text caps, one `threading.Lock` around `predict_batch`. Endpoint file at
  `$TTUCS_HOME/serve.json`, mode `0600`. Full contract in `ttucs/docs/serve-api.md`.
- **Decision logic:** `decision` is `confident`, `category_fallback` (falls back to the category's
  MISC CatID when the CatID is unsure but the category is clear) or `unknown`, driven by
  `unknown_threshold` and `category_threshold`. The UI shows all three states distinctly.
- **Launcher contract** (`crates/slate-engine`, implemented identically in C++): read
  `serve.json`; if healthy, use it; otherwise spawn `ttucs serve --port 0 --token-file <path>`
  detached, poll the file for up to 30 s, then poll `/health` until `warm` (60 s budget on first
  run); call `/shutdown` on exit only if this process started the server.
- **UCS rules** (`crates/slate-ucs`): a port of `ttucs.text` and `ttucs.taxonomy`. 753 CatIDs, 738
  used because the `ARCHIVED` workflow category is excluded. Filename shape
  `CatID_FX Name_CreatorID_SourceID[_UserData]`.
- **Conformance:** `fixtures/golden/predictions.jsonl` is generated from the pinned `ttucs` over the
  dev and realistic sets. `slate-ucs` and any future port must match on `cat_id`, `filename`,
  `keywords` and `decision`. CI regenerates and fails on drift.
- **Telemetry** lives in the shell only, never in the sidecar, so `TTUCS_OFFLINE=1` still means zero
  network from the engine. Fixed versioned allowlist, no free-form strings, opt-in, default off.

### Repository map

| path | what |
|---|---|
| `crates/slate-audio/` | decode (WAV/BWF/RF64/FLAC/MP3/OGG/AAC), resample to 16 kHz, Silero VAD with energy fallback, segmentation |
| `crates/slate-ucs/` | UCS taxonomy, filename builder, keywords; port of `ttucs.text`/`ttucs.taxonomy` |
| `crates/slate-tagger/` | `bext`, `iXML`, `.slate.json` sidecar, atomic writes, rename journal |
| `crates/slate-engine/` | launcher contract, token file, health probe, typed HTTP client |
| `crates/slate-cli/` | the `slate` binary |
| `apps/desktop/src/` | React + TS: `app/`, `features/queue`, `features/inspector`, `features/export`, `features/settings`, `lib/engine.ts`, `lib/schema.ts` (zod) |
| `apps/desktop/src-tauri/` | Rust shell: window, drag-drop, dialogs, sidecar spawn, updater |
| `apps/desktop/sidecar/` | PyInstaller onedir spec, whisper.cpp fetch script |
| `apps/reaper/extension/` | C++17 shim, `API_Slate_*`, worker thread + timer marshalling |
| `apps/reaper/scripts/` | Lua ReaScripts, ReaPack `index.xml` template |
| `packages/contract/` | `openapi.yaml` + generated clients (`gen/` only) |
| `models/` | gitignored; manifests and SHA-256 pins only |
| `fixtures/` | tiny audio and transcripts for tests, under 2 MB total |
| `docs/` | `about/`, `architecture/`, `adr/`, `guides/`, `reference/`, `release/` |
| `tools/xtask/` | build, bump, changelog, sign, verify, fetch-reaper-sdk |
| `.github/workflows/` | `ci.yml`, `build.yml`, `release.yml`, `sign-verify.yml`, `reapack.yml`, `docs.yml`, `nightly.yml` |

## 4. Environment and commands

The owner works on **Windows (PowerShell 5.1)**. When you give the owner commands, provide
PowerShell versions:

- no `&&`: put each command on its own line;
- `$env:NAME = "value"` for environment variables;
- no backslash line continuations;
- `signtool` and `msiexec` are invoked directly, not through a POSIX shim.

The owner currently uses **PyCharm**. This repo needs Rust, C/C++ and TypeScript, so
**IntelliJ IDEA Ultimate** is the recommended single project (Rust plugin, CLion plugin for C/C++,
plus TS and Python). Do not restructure the repo to suit an IDE that cannot open it.

Dev setup (bash):

```bash
just setup          # toolchain, pnpm install, python venv, pre-commit
just check          # fmt, clippy -D warnings, tsc, contract drift, changelog fragment
just test           # cargo test, vitest, contract tests
just dev            # engine + Tauri dev server with hot reload
```

Dev setup (PowerShell):

```powershell
just setup
just check
just test
```

The engine is not in this repo. Until `ttucs serve` lands, point the app at a locally installed
`ttucs` and expect `/health` to fail with a clear message rather than a crash.

| task | command |
|---|---|
| lint and test (must pass before any PR) | `just check` then `just test` |
| unit suite only, fast | `just test-unit` |
| coverage with the enforced floors | `just coverage` |
| fuzz smoke, 60 s per parser | `just fuzz` |
| build release binaries for the host | `just build` |
| build MSI/NSIS/DMG/deb for the host | `just bundle` |
| bump the product version everywhere | `just bump X.Y.Z` |
| assemble the changelog | `just changelog X.Y.Z` |
| sign an artefact | `just sign <path>` (refuses on a dirty tree) |
| verify signature, manifest, SBOM, provenance | `just verify` |
| fetch the pinned REAPER SDK | `cargo xtask fetch-reaper-sdk` |
| regenerate the golden corpus | `python tools/gen_golden.py` |

Style: [docs/CODE_STYLE.md](docs/CODE_STYLE.md) is the standard and rule 13 makes it binding. It
covers naming per language, doc comments, what a comment must and must not contain, and the
formatter and linter for each language. In short: `rustfmt` + `clippy -D warnings`, ESLint +
Prettier, `clang-format` + `clang-tidy`, `stylua` + `luacheck`, `ruff`. Line length 100.
Keep dependencies minimal; anything heavy goes behind a feature flag or into the sidecar, never into
the core install. Add tests for new behaviour.

## 5. Install and offline verification (do this before claiming a release works)

1. Build the artefacts for the host platform with `just bundle`.
2. Install into a **clean VM or a fresh user profile**, not the dev machine:
   - Windows: `msiexec /i Slate-To-UCS-X.Y.Z-x64.msi /qn /l*v install.log`, then
     `slate --version`, then `slate doctor`.
   - macOS: mount the DMG, copy, `spctl --assess --type execute`, run `slate --version`.
   - Linux: `dpkg -i`, run `slate --version`, `dpkg -r`.
3. With the network disabled, `TTUCS_OFFLINE=1` and an empty `TTUCS_HOME`:
   - `/health` must report `offline: true`, the right `ttucs_version` and
     `manifest_verified: true`;
   - a folder of real slates must transcribe, predict and export;
   - the app must make **zero outbound connections** (verify with a proxy log or a firewall rule).
4. Verify the signature the way a user would:
   `Get-AuthenticodeSignature .\Slate-To-UCS-X.Y.Z-x64.msi | Format-List Status, SignerCertificate, TimeStamperCertificate`
   must show `Status = Valid`, signer `CN=Slate-To-UCS`, and a timestamp.
5. Test the upgrade path: install the previous version, then the new one, and confirm a single
   entry in Add/Remove Programs and no orphaned files.
6. Test the uninstall and confirm no residue in `%ProgramFiles%`, `%LOCALAPPDATA%` or
   `%APPDATA%\REAPER\UserPlugins`.

## 6. Privacy, telemetry and update discipline

| stays on the machine, always | may leave the machine |
|---|---|
| audio files and their paths | app version, OS and arch, locale |
| transcripts and slate text | model name/version, `schema_version` |
| CatIDs, filenames, keywords, exports | counts and timings (files processed, ms per predict) |
| project, location, creator/source IDs | decision mix (`confident` / `fallback` / `unknown`) |
| the user's overrides and corrections | crash type and stack, with paths and text stripped |

- Rule of thumb: **aggregate numbers and identifiers of the build may leave; anything a recordist
  typed or recorded may not.** A transcript is the most sensitive field in the app.
- Telemetry is opt-in on first run, default off, one switch, revocable, with a visible "what is
  sent" panel. Respect `DO_NOT_TRACK=1` and `SLATE_TELEMETRY=0`.
- Events are a fixed, versioned allowlist. No free-form strings, no exception messages that could
  embed a path or a transcript. Adding a field is a schema bump and a changelog entry.
- A test must assert the payload against a corpus of transcripts, filenames and paths.
- Auto-update verifies a signed manifest and aborts on a bad signature. It never blocks startup,
  never installs silently mid-batch or mid-rename, and never gates a feature on a successful check.

## 7. Naming conventions

| thing | name |
|---|---|
| product / installer display name | `Slate-To-UCS` |
| desktop app | Slate-To-UCS (window title, About panel) |
| CLI | `slate` |
| engine | `ttucs serve` (a `ttucs` subcommand, not our binary) |
| REAPER extension binary | `slate_reaper.dll` / `.dylib` / `.so` |
| REAPER menu | `Extensions > Slate-To-UCS` |
| release tag (triggers the build) | `v<product version>`, e.g. `v0.1.0` |
| release assets | `Slate-To-UCS-<version>-x64.msi`, `-x64.exe`, `-arm64.dmg`, `-x64.AppImage`, `SHA256SUMS`, `SHA256SUMS.sig` |
| UCS filename produced | `CatID_FX Name_CreatorID_SourceID[_UserData]`, e.g. `DOORCreak_Wooden Door Creak Slow_NONE_ThickAsThieves_MKH416` |
| branches | `feat/...`, `fix/...`, `docs/...`, `chore/...` |
| commits | [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `test:`, `ci:`, `chore:`) |

The placeholder `ttucs Studio` from the handover brief is retired. Do not reintroduce it.

## 8. Versioning and changelog

- **SemVer 2.0.0**, **one product version** for the bundle: desktop app, REAPER extension, scripts
  and `slate` CLI ship together as `X.Y.Z`. `version.toml` is the single source; `just bump`
  propagates it into `Cargo.toml`, `package.json`, `tauri.conf.json` and the extension's
  `version.h`. `cargo xtask verify-versions` fails if they disagree.
- Independent versions only for registry-published artefacts, if and when they exist
  (`slate-ucs` on crates.io, tag prefix `core-vX.Y.Z`).
- **The contract has two versions**: `schema_version` (runtime gate, integer) and `info.version` in
  `packages/contract/openapi.yaml` (build-time gate). A breaking contract change bumps the product
  major and is marked **Breaking:**.
- **`ttucs` compatibility is declared, not assumed**: a version range in the sidecar's
  `pyproject.toml` plus a runtime check that `/health`'s `ttucs_version` satisfies it. A mismatch is
  a startup error with a clear message, not silent misbehaviour.
- **`CHANGELOG.md` follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/):**
  - Newest first, with `## [Unreleased]` at the top.
  - Group entries under `### Added`, `### Changed`, `### Deprecated`, `### Removed`, `### Fixed`,
    `### Security`, in that order, omitting empty groups. Mark breaking changes with **Breaking:**.
  - Keep the compare links at the bottom up to date:
    `[Unreleased]: .../compare/vX.Y.Z...HEAD`, `[X.Y.Z]: .../compare/vPREV...vX.Y.Z`.
  - When accuracy or behaviour changes, give concrete before/after numbers and name the test set.
- **Fragments are the rule once `changelog.d/` exists.** Every PR adds
  `changelog.d/<pr>-<slug>.md` with a front-matter type (`added`, `changed`, `deprecated`,
  `removed`, `fixed`, `security`, `breaking`). CI requires one for any PR touching `crates/`,
  `apps/`, `packages/contract/` or `tools/`, unless the PR carries the `skip-changelog` label. At
  release, `just changelog X.Y.Z` assembles fragments into `CHANGELOG.md`, deletes the consumed
  fragments and refreshes the compare links. Until that tooling lands, write entries directly under
  `[Unreleased]`.
- A `ttucs` change that alters what our clients receive (for example the `predict --json` list
  shape) is called out in our release notes even though its changelog entry lives in `ttucs`.

## 9. Release process (summary; full steps land in `docs/release/`)

1. PR with the version bump (`just bump X.Y.Z`), assembled changelog and any model pin updates.
   The owner merges it.
2. The owner tags the merged commit and pushes the tag:
   - `git tag -a vX.Y.Z -m "Slate-To-UCS X.Y.Z"`
   - `git push origin vX.Y.Z`
3. `build.yml` builds every target, signs, attests and uploads to a **draft** release. A tag is a
   build request, not a release.
4. `sign-verify.yml` proves each signature and each SHA-256 manifest.
5. `release.yml` runs the install smoke tests from section 5 on clean runners.
6. The owner approves; the draft is published with the changelog section as notes.
7. `reapack.yml` publishes the script index; `docs.yml` publishes the site.

Never publish, tag or create a release without an explicit request from the owner.

## 10. Working with the owner

- Be concise, concrete and honest. Give numbers with their n and caveats. Label estimates as
  estimates.
- Give Windows PowerShell 5.1 commands (section 4) when asking the owner to run something.
- Open a PR and stop; the owner reviews and merges. Ask before doing anything outward-facing
  (publishing, tagging, creating releases, uploading assets) unless it was explicitly requested.
- The owner is a senior technical sound designer and knows UCS, field recording, REAPER and audio
  tooling well. Do not over-explain the domain; do explain ML and packaging trade-offs briefly.
- The owner's decisions recorded in `ttucs/docs/plans/ttucs-studio.md` are settled: Tauri 2 + React,
  `ttucs serve` as the engine, whisper.cpp for STT, PyInstaller onedir, Python classifier. Raise a
  blocker with the owner rather than substituting a different choice.

## 11. Current state (update this section when it changes)

As of 2026-11-14:

- **Repository:** `github.com/SoundsLikeJonny/Slate-To-UCS` at `94bbeab` (merge of PR #3), plus the
  M1 scaffold branch. No tags, no releases.
- **Planning:** `docs/PLAN.md` revision 2 reconciles the handover brief
  (`ttucs/docs/plans/ttucs-studio.md`, 2026-11-10) with packaging, signing, CI/CD, docs and
  changelog mechanics. Nineteen sections, phases 0–8. `docs/LAYOUT.md` is the file-level view and
  `docs/MILESTONES.md` the dated plan.
- **Phase 0 is done.** `ttucs serve` merged into `ttucs` main as PR #14 (`0cee598`), with
  `src/ttucs/serve.py`, `docs/serve-api.md` and `tests/test_serve.py`. Verified 2026-11-14: 28
  serve tests pass; a live round-trip with `TTUCS_OFFLINE=1` and an empty `TTUCS_HOME` returned
  `schema_version: 1`, `warm: true`, `manifest_verified: true`, a 401 without the token, a 413 over
  the body cap, a list-shaped `/predict`, working `/parse` and `/taxonomy/*`, and a clean
  `/shutdown`.
- **M1 scaffold is in place.** A Cargo workspace with `crates/slate-ucs`, `crates/slate-engine`,
  `crates/slate-cli` and `tools/xtask`; `version.toml` as the single source of the product version
  with `cargo xtask verify-versions` enforcing agreement; `rust-toolchain.toml`, `rustfmt.toml`,
  `deny.toml`, `.gitattributes`, `.cargo/config.toml` (the `xtask` alias), `justfile`,
  `package.json` and `pnpm-workspace.yaml`; `packages/contract/openapi.yaml`; ADRs 0001–0006;
  `docs/index.md` and `docs/book.toml`; `CONTRIBUTING.md`, `CODEOWNERS`, `changelog.d/`; and the
  `ci.yml`, `nightly.yml` and `docs.yml` workflows. Verified locally on Windows 2026-11-14:
  `cargo test --workspace` passes 197 tests, `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` are clean, and `cargo xtask
  verify-versions`, `docs-check` and `changelog --check` all pass. The three-OS CI matrix has not
  run yet.
- **Golden corpus:** `fixtures/golden/predictions.jsonl`, 412 rows (332 dev + 80 realistic),
  generated by `tools/gen_golden.py` from the pinned `ttucs`. Each row carries both the raw
  description and the `parsed_description` the reference pipeline extracted, because the slate
  parser is not ported yet.
- **Open decisions:** Azure Artifact Signing eligibility (individual US/CA or organisation);
  minimum REAPER version (6.x or 7.x); Windows arm64 in or out; whether `slate` ships in the MSI.
  Rename is recommended for v1. Deadlines in `docs/MILESTONES.md` §10.
- **Not yet created:** `crates/slate-audio`, `crates/slate-tagger`, `apps/desktop`,
  `.clang-format`, `stylua.toml`, the Lua ReaScripts and the C++ REAPER shim. They belong to M2–M4.
- **Untracked and not ignored:** nothing. `.idea/` and `.abacusai/` are ignored as of PR #1.
