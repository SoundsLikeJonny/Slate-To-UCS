# Changelog fragments

One file per pull request, consumed at release by `cargo xtask changelog --release X.Y.Z`.

## Format

Name the file `<pr>-<slug>.md`, for example `42-rf64-ds64.md`. Start it with front matter:

```markdown
---
type: added
---

`slate-audio` now reads RF64 files with a `ds64` chunk, so field recordings above 4 GB open
instead of failing with `UnsupportedFormat`.
```

`type` is one of `added`, `changed`, `deprecated`, `removed`, `fixed`, `security`, `breaking`.
A `breaking` fragment is rendered under **Changed** with a **Breaking:** prefix.

## When a fragment is required

Any pull request touching `crates/`, `apps/`, `packages/contract/` or `tools/`. A pull request
carrying the `skip-changelog` label is exempt, and the reviewer decides whether that is honest.

## Until the assembler lands

`cargo xtask changelog --release` exists and is tested, but the fragment reader is not wired in
yet. Until it is, write entries directly under `## [Unreleased]` in `CHANGELOG.md` and leave this
directory empty apart from this file.
