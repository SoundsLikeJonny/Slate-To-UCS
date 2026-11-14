# 0003 — `ttucs serve` is the only inference owner

Date: 2026-11-14
Status: accepted

## Context

Three surfaces need the same classifier: the desktop app, the REAPER extension and the scripts.
Loading the model costs one to two seconds and a few hundred megabytes. The model is PyTorch and is
not safe to call concurrently.

Alternatives considered: link the classifier into each surface, shell out to `ttucs predict` per
call, or run one shared server.

`ttucs predict` costs two to four seconds of process start per call, which makes per-item tagging
in REAPER unusable. Linking it into each surface means three copies of the model in memory and
three places for a crash.

## Decision

One `ttucs serve` process per machine, loopback only, token-gated, owning the only loaded model.
Every surface is a client. `ttucs predict --json` stays supported for scripting but is not the
integration path.

## Consequences

- One model load serves every surface, so per-item tagging is interactive.
- A model or decoder fault cannot take down REAPER mid-session.
- The classifier can move from Python to ONNX behind the same contract without a client change.
- There is a process to supervise, a port to secure and a startup handshake to implement twice:
  once in `slate-engine` and once in the C++ shim.
- The engine is Python, so the installer carries CPython and torch and is 250 to 400 MB.

## Revisit when

- the ONNX export project makes a Rust engine viable, which removes the Python sidecar but not the
  server: the process boundary is what buys crash isolation and a single model load.
