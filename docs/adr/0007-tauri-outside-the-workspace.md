# 0007 — The Tauri binary sits outside the Cargo workspace

Date: 2026-11-14
Status: accepted

## Context

The desktop app is a Tauri 2 window over a React front end. Its Rust half needs `tauri`,
`tauri-build` and, transitively, a WebView binding: WebView2 on Windows, WKWebView on macOS and
`webkit2gtk` on Linux.

The workspace is the unit `cargo test --workspace`, `cargo clippy --workspace` and
`cargo llvm-cov --workspace` operate on, and it is what the three-OS CI matrix builds. Adding the
Tauri crate to `members` would make every one of those jobs compile a WebView, and the Linux job
would additionally need `libwebkit2gtk-4.1-dev` and `libgtk-3-dev` installed before it could link.

## Decision

`apps/desktop/src-tauri` is listed in `[workspace] exclude`, and the logic that can be tested
without a display lives in `crates/slate-shell`, which is a normal member.

`slate-shell` owns the single-instance guard and the sidecar supervisor. The Tauri crate owns the
window, the capabilities file and the four IPC commands, and is built by `cargo tauri build` in the
packaging phase.

## Consequences

- `cargo test --workspace` stays fast and needs no WebView, no GTK headers and no display.
- The guard and the sidecar path resolution are covered by ordinary unit tests on all three
  operating systems.
- The Tauri crate is not compiled by CI yet, so a mistake in `lib.rs` or `tauri.conf.json` is not
  caught by the matrix. That is a real gap, recorded in `docs/MILESTONES.md` §6 rather than hidden.
- `cargo deny check` does not see the Tauri dependency tree. The packaging phase adds a separate
  `cargo deny --manifest-path apps/desktop/src-tauri/Cargo.toml` run when the crate starts being
  built in CI.

## Revisit when

- the packaging phase adds a `tauri` job, at which point the exclusion stays but the job must run
  `cargo check` and `cargo deny` against the manifest explicitly;
- or Tauri's Linux dependencies become available as a prebuilt action, which would make a workspace
  member affordable.
