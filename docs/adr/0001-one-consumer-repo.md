# 0001 — One consumer repository, `ttucs` separate

Date: 2026-11-14
Status: accepted

## Context

The handover brief at `ttucs/docs/plans/ttucs-studio.md` proposes three repositories: `ttucs`,
`ttucs-studio` and `ttucs-reaper`. Its stated rationale is that the core install stays small and
that the REAPER SDK must never be vendored in `ttucs`.

Both of those are constraints on `ttucs`, not on the number of consumer repositories. A single
consumer repository satisfies them equally: the SDK is fetched at build time into a gitignored
directory and never enters either repository.

The desktop app and the REAPER extension share three things that change together:

- the wire contract (`schema_version`, the endpoint table, the launcher steps);
- the UCS filename algorithm, which both must produce identically;
- the release version, because a mismatched extension and app is not a supported configuration.

## Decision

One consumer repository, `Slate-To-UCS`, holding the desktop app, the REAPER extension and the
scripts. `ttucs` stays a separate repository, consumed as a pinned dependency.

## Consequences

- A cross-surface change is one pull request, one CI run and one changelog entry.
- Version skew between an installed extension and a newer app is impossible by construction.
- CI is larger and slower than three small repositories would be; path filters and caching absorb
  most of it.
- A contributor who only wants the REAPER extension still clones the whole tree.

## Revisit when

- a surface needs a release cadence an order of magnitude faster than the other;
- a third party needs `slate-ucs` or `slate-audio` independently, which is solved by publishing to
  crates.io from this repository rather than by splitting it;
- the REAPER SDK licence review forbids co-location, which it should not, since the SDK is never
  committed.
