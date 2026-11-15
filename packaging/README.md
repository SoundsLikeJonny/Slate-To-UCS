# Packaging

The installers for all three platforms, and the scripts that build them.

| file | function |
|---|---|
| `windows/slate.wxs` | WiX 3.14 source for the per-machine MSI. `UpgradeCode` is fixed; `ProductCode` is `*`. |
| `windows/build-msi.ps1` | Compiles and links the MSI. Reads the version from `version.toml`. |
| `windows/slate.nsi` | NSIS source for the `.exe` installer. |
| `windows/build-nsis.ps1` | Compiles the NSIS installer. |
| `windows/smoke-install.ps1` | Silent install, upgrade and uninstall against a real MSI. Needs elevation. |
| `windows/licence.rtf` | The licence page, generated from `LICENSE`. |
| `macos/build-dmg.sh` | Signs, packages and optionally notarises the `.app` into a DMG. |
| `macos/entitlements.plist` | Hardened-runtime entitlements. |
| `linux/stage.sh` | The staging tree the deb, the rpm and the AppImage all package. |
| `linux/build-deb.sh` | `dpkg-deb` package. |
| `linux/build-rpm.sh` | `rpmbuild` package. |
| `linux/build-appimage.sh` | AppImage. |
| `linux/slate.desktop`, `linux/*.metainfo.xml` | Desktop entry and AppStream metadata. |
| `tests/test_layout.sh` | Asserts the staging tree and every script's syntax. Runs anywhere with a shell. |
| `*/out/` | Build output. Ignored by git. |

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

```sh
# Linux: needs dpkg-dev, rpm, zstd and squashfs-tools (or appimagetool)
sh packaging/linux/build-deb.sh
sh packaging/linux/build-rpm.sh
sh packaging/linux/build-appimage.sh

# macOS: needs hdiutil, codesign and, for --notarise, notarytool
sh packaging/macos/build-dmg.sh
SLATE_SIGN_IDENTITY="Developer ID Application: ..." sh packaging/macos/build-dmg.sh --notarise
```

The Windows scripts accept `-SidecarDir` to include the `ttucs` onedir tree; the Linux
scripts accept `--sidecar`. Without it the installers are built without the sidecar, which
is how they are smoke-tested before the sidecar exists.

`sh packaging/tests/test_layout.sh` asserts the Linux staging tree, the desktop entry, the
AppStream metadata and every script's syntax. It needs no packager, so it runs on any host
with a shell — including this one, where `dpkg-deb` and `rpmbuild` do not exist.

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
