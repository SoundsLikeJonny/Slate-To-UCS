# 0004 — `schema_version` at runtime, OpenAPI at build time

Date: 2026-11-14
Status: accepted

## Context

Three clients in three languages need to agree on the wire shape. Two kinds of check are possible:
a runtime gate that a client evaluates against a live server, and a build-time gate that proves the
generated clients match the specification.

A runtime gate needs to be cheap, dependency-free and understandable in a log line. A build-time
gate needs to be expressive enough to generate code.

## Decision

Both, with different jobs.

- `schema_version` is an integer in every response envelope and in `/health`. A client refuses an
  unknown value and says so in the UI. It is the runtime gate.
- `packages/contract/openapi.yaml` is the build-time contract. TypeScript and Rust clients are
  generated from it and CI fails on drift. It is not a runtime dependency.

## Consequences

- An old client against a new server fails with a clear message instead of a parse error.
- Generated clients cannot silently diverge from the specification.
- Two artefacts describe one contract, so a change touches both. The drift check makes forgetting
  one a build failure rather than a runtime surprise.
- The integer carries no compatibility information beyond "same or not". That is deliberate: a
  range check invites guessing at a shape the client does not know.

## Revisit when

- a client needs to tolerate two contract versions at once, which would mean a compatibility
  matrix rather than a single integer.
