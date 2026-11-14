# Contributing to Slate-To-UCS

Read [AGENTS.md](AGENTS.md) first. It carries the hard rules, and rules 13 to 15 make
[docs/CODE_STYLE.md](docs/CODE_STYLE.md) and [docs/TESTING.md](docs/TESTING.md) binding.

## Setup

You need Rust (the pinned toolchain installs itself from `rust-toolchain.toml`), Python 3.10 or
newer, and `just`. Node and pnpm are only needed once the desktop app lands.

```bash
git clone https://github.com/SoundsLikeJonny/Slate-To-UCS.git
cd Slate-To-UCS
just setup
```

On Windows PowerShell 5.1, one command per line:

```powershell
git clone https://github.com/SoundsLikeJonny/Slate-To-UCS.git
cd Slate-To-UCS
just setup
```

The engine lives in a separate repository. To run anything end to end you also need `ttucs` with
the `serve` subcommand:

```powershell
pip install "ttucs>=0.3,<0.4"
ttucs serve --port 0 --token-file "$env:TEMP\serve.json"
```

## Before you open a pull request

```bash
just check
just test
```

`just check` runs formatting, clippy with warnings denied, the version agreement check, the
changelog check and the documentation link check. `just test` runs the workspace test suite,
including the conformance corpus.

Both must pass. A pull request that lowers coverage below a floor in
[docs/TESTING.md](docs/TESTING.md) fails.

## Pull requests

- One concern per pull request. A refactor and a behaviour change are two pull requests.
- Branch from `main`, name it `feat/...`, `fix/...`, `docs/...` or `chore/...`.
- Conventional Commits for the title: `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `chore:`.
- Add an entry under `## [Unreleased]` in [CHANGELOG.md](CHANGELOG.md) for any user-visible change.
  Once `changelog.d/` lands, add a fragment instead.
- Describe how you verified the change. "Tests pass" is not a verification of a user-facing path.
- Do not merge your own pull request. The owner reviews and merges.

## What gets rejected

- A `pub` function without a test.
- A bug fix without the test that would have caught it.
- A new dependency without a reason in the pull request and a licence that `deny.toml` allows.
- A conformance failure "fixed" by loosening the assertion.
- Any third-party sound-library metadata, in code, fixtures, docs or a screenshot.
- A retry or a `sleep` added to make a flaky test pass.

## Reporting a security issue

Do not open a public issue. See [SECURITY.md](SECURITY.md).
