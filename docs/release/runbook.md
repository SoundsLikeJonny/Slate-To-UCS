# Release runbook

The steps a release actually takes, in order, with the command for each. A tag is a build
request, not a release: nothing is published until the smoke tests pass and a reviewer
approves.

## 1. Prepare

```bash
cargo xtask changelog --release 0.2.0   # move [Unreleased] into a dated section
cargo xtask bump 0.2.0                  # writes version.toml into every manifest
cargo xtask verify-versions             # fails if any manifest disagrees
just check && just test
```

Commit as `chore(release): 0.2.0`, open a PR, merge it. The version must be on `main`
before the tag exists, so the tag names a commit that was tested.

## 2. Tag

```bash
git tag -a v0.2.0 -m "Slate-To-UCS 0.2.0"
git push origin v0.2.0
```

`build.yml` runs: builds the CLI, the extension, the UI, the MSI and the NSIS installer,
signs them, verifies the signature, writes an SBOM, attests provenance, and uploads
everything to a **draft** release.

If `AZURE_SIGNING_ENABLED` is not `true`, the artefacts are renamed `-unsigned` and the
draft still appears. That is deliberate: it makes the missing identity visible without
blocking the build.

## 3. Verify

```bash
gh workflow run sign-verify.yml -f tag=v0.2.0
```

This downloads the draft and asserts a valid, timestamped signature on every installer. It
fails on an unsigned artefact, which is the case it is built to catch.

## 4. Smoke test and publish

```bash
gh workflow run release.yml -f tag=v0.2.0
```

`release.yml` runs on a clean `windows-2022` runner and, in order:

1. refuses an unsigned or untimestamped artefact;
2. installs silently with `msiexec /i /qn`;
3. asserts the nine installed files and runs `slate.exe --version`;
4. uninstalls silently with `msiexec /x /qn`;
5. asserts no residue: no files under the install directory, no Start Menu folder, no
   Add/Remove Programs entry.

Only then does the `publish` job run, and it waits for a reviewer on the `release`
environment before flipping the draft to published.

## 5. After

- Confirm the release page shows the MSI, the NSIS installer, the SBOM and the provenance
  attestation.
- Run the user-facing check from `docs/release/signing.md` against the downloaded MSI.
- If anything is wrong, delete the draft and the tag, fix, and re-tag. Never replace an
  artefact under an existing tag: the hash that was signed must be the hash that ships.

## Rollback

A published release is not edited. Publish a new patch version with the fix and mark the
old one as superseded in its release notes. If the defect is a security issue, follow the
advisory path in `SECURITY.md` instead.

## What is not automated yet

- macOS DMG and notarisation, and the Linux deb/rpm/AppImage, are phase 8 and have no
  workflow.
- The upgrade-over-previous-version smoke test needs a published previous version to
  upgrade from; until 0.1.0 is published it is skipped rather than faked.
