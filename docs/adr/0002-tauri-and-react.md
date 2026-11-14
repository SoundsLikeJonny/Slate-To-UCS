# 0002 — Tauri 2 with React for the desktop shell

Date: 2026-11-14
Status: accepted

## Context

The app needs native file dialogs, drag and drop, a background sidecar process, an installer per
platform and a review UI dense enough for a table of a few hundred files with a diff pane.

Electron would deliver the UI quickly but ships a second runtime and roughly 150 MB before any
application code. Qt would mean a C++ UI and a licensing review. A native Rust UI toolkit would
mean rebuilding table, virtualisation and text editing from scratch.

The owner confirmed Tauri 2 with React on 2026-11-10. That is recorded in the handover brief and is
not open for substitution.

## Decision

Tauri 2 for the shell, React with TypeScript and Vite for the UI, Rust for the native side.

## Consequences

- The shell is roughly 10 to 20 MB, and the installer size is dominated by the Python sidecar and
  torch, not by the shell.
- The Rust native side shares `slate-audio` and `slate-ucs` with the CLI, so decoding and filename
  rules exist once.
- The UI runs in WebView2 on Windows, WKWebView on macOS and WebKitGTK on Linux. Those differ, so
  CSS stays conservative and both Windows and macOS are tested before a milestone signs off.
- Tauri's bundler produces the MSI, the NSIS executable, the DMG and the deb, and its updater
  verifies a signed manifest.

## Revisit when

- a Tauri limitation blocks a milestone. Raise it with the owner rather than switching shells: the
  cost of a shell change is the whole UI layer.
