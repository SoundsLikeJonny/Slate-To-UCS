# Packaging

The Windows installers, and the scripts that build them.

| file | function |
|---|---|
| `windows/slate.wxs` | WiX 3.14 source for the per-machine MSI. `UpgradeCode` is fixed; `ProductCode` is `*`. |
| `windows/build-msi.ps1` | Compiles and links the MSI. Reads the version from `version.toml`. |
| `windows/slate.nsi` | NSIS source for the `.exe` installer. |
| `windows/build-nsis.ps1` | Compiles the NSIS installer. |
| `windows/licence.rtf` | The licence page, generated from `LICENSE`. |
| `windows/out/` | Build output. Ignored by git. |

## Building

```powershell
cargo build --release -p slate-cli
cargo xtask fetch-reaper-sdk
cmake -S apps/reaper/extension -B apps/reaper/extension/build -DSLATE_BUILD_EXTENSION=ON
cmake --build apps/reaper/extension/build --config Release

$env:WIX_BIN = "C:\path\to\wix314"        # a directory holding candle.exe and light.exe
$env:MAKENSIS = "C:\path\to\makensis.exe"
pwsh packaging/windows/build-msi.ps1
pwsh packaging/windows/build-nsis.ps1
```

Both scripts accept `-SidecarDir` to include the `ttucs` onedir tree. Without it the
installers are built without the sidecar, which is how they are smoke-tested before the
sidecar exists.

## Rules

1. `UpgradeCode` never changes. It is how Windows Installer recognises a previous version;
   changing it turns an upgrade into a second side-by-side install.
2. Nothing is written into a user profile. The REAPER integration is installed by the
   application into `%APPDATA%\REAPER` on request, because a per-machine installer cannot
   enumerate users and cannot handle a portable REAPER. See `PLAN.md` 14.6.
3. The MSI and the NSIS installer must agree on the install layout and the Add/Remove
   Programs display name. Two installers that disagree produce two uninstallers that each
   remove half of the other's work.
4. Sign after building, never rebuild after signing. The hash that was signed is the hash
   that ships.
