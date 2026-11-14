# The bundled `ttucs` sidecar

This directory is the mount point for the PyInstaller `onedir` tree that ships with the app.
It is empty in the repository on purpose: the tree is 250–400 MB, it is built by the packaging
step, and committing it would put a binary blob in git that no reviewer can read.

`tauri.conf.json` lists this path under `bundle.resources`, and Tauri refuses to build when the
path is missing, so this README keeps the directory present.

## What the packaging step puts here

| platform | executable | notes |
|---|---|---|
| Windows | `ttucs.exe` | plus `_internal/` and the model weights |
| macOS | `ttucs` | same tree, copied into `Contents/Resources/resources/ttucs` |
| Linux | `ttucs` | same tree |

`slate-shell` resolves the executable as `<resource dir>/resources/ttucs/ttucs[.exe]` and reports
`is_installed() == false` when it is absent, so a development build shows a clear "engine not
installed" state instead of failing at spawn time.

## Building the tree

Not written yet. It needs a pinned `ttucs` wheel, a pinned `ttucs-model-bow`, and a PyInstaller
spec; the SHA-256 values in `fetch-model.ps1` must come from a real release rather than being
invented. Tracked in `docs/MILESTONES.md` §6.
