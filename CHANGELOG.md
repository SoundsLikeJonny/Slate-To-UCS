# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
