# 0005 — C++ REAPER shim over HTTP, no C ABI in v1

Date: 2026-11-14
Status: accepted

## Context

A REAPER extension is a shared library loaded into the host process. It can be written in C++
against the REAPER SDK, or in Rust through `rea-rs`, which reached 1.0 in October 2026.

Linking Rust into REAPER means a Rust panic or an allocator mismatch can take down a recording
session. The extension surface is small: register actions, read item notes, write notes and take
names, wrap undo blocks, and talk to the engine.

## Decision

C++17 against the REAPER SDK, talking HTTP to `ttucs serve`. No Rust crate is linked into the
extension in v1. The SDK is fetched at build time into a gitignored directory and never committed.

## Consequences

- A Rust fault cannot enter REAPER's address space.
- The shim is roughly 600 lines and has no dependency risk beyond libcurl.
- The launcher steps are implemented twice, in Rust and in C++. A change to one is a change to the
  other, and the contract document is the shared reference.
- HTTP on the main thread would stall the host, so requests run on a worker and results cross
  through a queue drained from a timer callback at about 30 Hz.

## Revisit when

- the shim passes roughly 1500 lines, at which point `rea-rs` and a shared `slate-ucs` become
  cheaper than maintaining two implementations;
- or a C ABI over `slate-ucs` is wanted for the filename fast path only, which does not require
  linking the whole engine.
