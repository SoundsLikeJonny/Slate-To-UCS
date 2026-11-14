# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- The M1 scaffold: a Cargo workspace with `slate-ucs`, `slate-engine`, `slate-cli` and `xtask`,
  `version.toml` as the single source of the product version, `rust-toolchain.toml`, `rustfmt.toml`,
  `deny.toml`, `.gitattributes`, a `justfile` with one recipe per task, and a pnpm workspace root
  for the desktop app that lands in M2.
- `slate-ucs`: a port of `ttucs.taxonomy` and `ttucs.text`. The UCS list is compiled in with a
  pinned SHA-256, CatID resolution handles the two known aliases and `CATEGORY-SUBCATEGORY`, and
  the filename builder implements `CatID(-UserCategory)_(VendorCategory-)FXName_CreatorID_SourceID
  (_UserData)` with sanitising, round-trip parsing and an extension splitter.
- `slate-engine`: the reference implementation of the three launcher steps, endpoint discovery
  through `$TTUCS_HOME/serve.json`, a `Token` type whose `Debug` and `Display` both print
  `<redacted>`, the `schema_version` gate, typed wire types, and a minimal in-tree HTTP/1.1 client
  (ADR-0006) so the core install carries no TLS stack.
- `slate-cli`: `slate predict` and `slate doctor`, with documented exit codes 0, 1, 2 and 3.
- `xtask`: `bump`, `verify-versions`, `changelog --check` and `--release`, `docs-check`,
  `gen-golden`, `sync-taxonomy`, `coverage` and `fuzz`.
- `packages/contract/openapi.yaml`: the build-time contract for all six routes, with the runtime
  gate remaining the integer `schema_version` (ADR-0004).
- `fixtures/golden/predictions.jsonl`: 412 rows generated from the pinned `ttucs` over the dev and
  realistic description sets, plus `tools/gen_golden.py` to regenerate or verify it.
- `crates/slate-ucs/tests/conformance.rs` asserts the Rust port matches the reference on `fx_name`,
  `keywords`, `filename` and CatID resolution, and that every golden filename round-trips.
- `crates/slate-ucs/tests/properties.rs`: six property tests over sanitising, filename round-trip,
  FX-name budgeting and tokenising.
- CI: `ci.yml` with a three-OS Rust matrix, a coverage floor job, version and changelog gates, a
  documentation link check, a golden-corpus drift check and `cargo deny`; `nightly.yml` with
  mutation testing and a fuzz smoke; `docs.yml` building mdBook to Pages.
- `CONTRIBUTING.md`, `CODEOWNERS`, `docs/index.md`, `docs/book.toml`, `changelog.d/README.md` and
  ADRs 0001 to 0006.
- `docs/PLAN.md`: the build plan for the product family — Slate-To-UCS Desktop, the REAPER
  extension and the REAPER scripts — over one engine and one contract. Covers the process model,
  the `ttucs serve` contract and launcher steps, the repository layout, technology choices with
  their rejected alternatives, documentation structure, versioning and changelog mechanics, CI/CD
  workflows and merge gates, the Windows MSI with its Authenticode PKI chain and signing order,
  phases 0–8 with exit criteria, risks and open decisions. Revision 2 reconciles the handover brief
  at `ttucs/docs/plans/ttucs-studio.md` (2026-11-10), which revision 1 was written without.
- `AGENTS.md`: the rulebook for contributors, human or AI. Twelve hard rules (branch and PR
  discipline, no third-party sound-library metadata, never commit weights or the REAPER SDK,
  offline-first, only the parsed description is classified, held-out evaluation hygiene, truthful
  reporting, changelog and SemVer, secrets, privacy, contract compatibility, SDK fetched not
  vendored), the high-level data flow, the repository map, PowerShell-first commands, the install
  and offline verification checklist, the telemetry allowlist, naming conventions, the versioning
  and changelog rules, the release sequence and the current state.
- `docs/LAYOUT.md`: the file-level view of the planned repository. The annotated top-level tree,
  then a table per area — root files, the five `crates/`, the Tauri app's React tree and Rust shell,
  the C++ REAPER extension and its Lua scripts, the contract package, `xtask`, fixtures and model
  pins, `docs/`, `.github/` and `changelog.d/` — with each file's kind, the phase it appears in and
  what it does. Closes with the layering rules and a "where new code goes" table, so a file that
  does not fit an existing row is a design question rather than a new directory.
- `docs/MILESTONES.md`: dated goals for the next eight weeks, M0 through M4, plus M5 outside the
  window so packaging is not a surprise. Each milestone carries its weeks, the phases it covers, a
  per-week goal table, an exit criterion that is either met or not, and its own risks. Also records
  the capacity assumptions, five decision deadlines with defaults if unanswered, the weekly cadence,
  six slip triggers and a status table updated at each Sunday check.
- `SECURITY.md`: the security policy. Private reporting through GitHub advisories with response
  targets, a supported-version table, an in-scope and out-of-scope list, the threat model with its
  five boundaries and what is explicitly not defended against, download verification by checksum,
  Sigstore bundle and build provenance, the Authenticode PKI chain with the fingerprints published
  at the first signed release, the `signtool` and `codesign` verification commands, the SmartScreen
  reputation caveat, the telemetry allowlist restated as a security boundary, dependency and supply
  chain controls, and a 90-day coordinated disclosure policy with safe harbour.
- `docs/CODE_STYLE.md`: the per-language standard, made binding by `AGENTS.md` rule 13. Covers what
  a comment must contain (module header, public API doc, why-comment, `// SAFETY:`, issue-linked
  TODO) and must never be (commented-out code, restatement, banner art, attribution), then naming
  and idiom tables for Rust, TypeScript and React, C++, Lua and Python, plus data and configuration
  files. Fixes the domain vocabulary (`cat_id`, `CatID`, `bext`, `iXML`, the three `decision`
  values) so code, JSON, docs and UI copy agree. Closes with the enforcement table: every check a
  tool can make is a blocking CI gate, and only naming and comment quality are left to review.
- `docs/TESTING.md`: the testing standard, made binding by `AGENTS.md` rules 14 and 15. States what
  must be tested (every `pub` function, every error variant, every branch, every boundary, every
  parser and writer, plus the seams where the product actually breaks), test naming and
  arrange-act-assert structure, per-language tooling for Rust, TypeScript, C++, Lua and Python,
  property-based and fuzz targets, the golden conformance oracle, determinism rules, per-crate
  coverage floors, a flaky-test policy that forbids retries and `sleep`, and the CI gates. Also
  covers assertions in production code per language, including why `assert` is wrong in Python
  production code and why an assertion must never have a side effect.
- `CHANGELOG.md`: this file, following Keep a Changelog 1.1.0 with `## [Unreleased]` at the top and
  compare links at the bottom. Entries are written directly under `[Unreleased]` until the
  `changelog.d/` fragment tooling lands, after which every PR adds a fragment and
  `just changelog X.Y.Z` assembles the release section.

### Changed
- `docs/PLAN.md` revision 2 supersedes revision 1 on four points, following the owner's decisions
  recorded in the handover brief: the engine is `ttucs serve` (Python standard library
  `ThreadingHTTPServer`) rather than a Rust daemon; the desktop UI is React rather than Svelte; the
  classifier stays the Python `bow` model with no Rust port in v1, so the planned `slate-infer`
  phase is removed; and the runtime compatibility gate is the integer `schema_version` rather than
  OpenAPI, which becomes the build-time and codegen contract. The `slate-core` crate is split into
  `slate-audio`, `slate-ucs`, `slate-tagger`, `slate-engine` and `slate-cli`.

[Unreleased]: https://github.com/SoundsLikeJonny/Slate-To-UCS/commits/main
