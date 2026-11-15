#!/bin/sh
# Build, sign, notarise and staple the macOS disk image.
#
# Usage:
#   build-dmg.sh [--app <Slate-To-UCS.app>] [--out <dir>] [--identity <id>] [--notarise]
#
# Without --app the script stages a minimal bundle from target/release/slate, which is how
# the layout and the DMG are exercised before the Tauri bundle exists. With --app it uses
# the bundle Tauri produced.
#
# Signing order matters and is not negotiable:
#   1. sign every nested Mach-O, innermost first
#   2. sign the bundle
#   3. build the DMG
#   4. sign the DMG
#   5. notarise, then staple the ticket to the DMG
# Rebuilding after signing invalidates the signature, so nothing is rebuilt here.
set -eu

die() {
    printf 'build-dmg: %s\n' "$1" >&2
    exit 1
}

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)

out="$root/packaging/macos/out"
app=""
identity=${SLATE_SIGN_IDENTITY:-}
notarise=0

while [ $# -gt 0 ]; do
    case "$1" in
        --app) app=$2; shift 2 ;;
        --out) out=$2; shift 2 ;;
        --identity) identity=$2; shift 2 ;;
        --notarise) notarise=1; shift ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) die "unknown option $1" ;;
    esac
done

[ "$(uname -s)" = "Darwin" ] || die "this script needs macOS: hdiutil, codesign and notarytool are Apple tools"

version=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$root/version.toml" | head -1)
[ -n "$version" ] || die "version.toml has no version line"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM

if [ -z "$app" ]; then
    slate="$root/target/release/slate"
    [ -f "$slate" ] || die "missing binary: $slate"
    app="$work/Slate-To-UCS.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    install -m 0755 "$slate" "$app/Contents/MacOS/slate"
    install -m 0644 "$root/apps/desktop/src-tauri/icons/icon.png" "$app/Contents/Resources/icon.png"
    cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Slate-To-UCS</string>
    <key>CFBundleDisplayName</key><string>Slate-To-UCS</string>
    <key>CFBundleIdentifier</key><string>io.github.soundslikejonny.SlateToUCS</string>
    <key>CFBundleExecutable</key><string>slate</string>
    <key>CFBundleIconFile</key><string>icon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
fi

[ -d "$app" ] || die "no such bundle: $app"

if [ -n "$identity" ]; then
    # Innermost first: a nested binary signed after its container invalidates the container.
    find "$app/Contents" -type f -perm -u+x | while read -r nested; do
        case "$nested" in
            *.dylib|*/MacOS/*)
                codesign --force --timestamp --options runtime \
                    --entitlements "$here/entitlements.plist" \
                    --sign "$identity" "$nested"
                ;;
        esac
    done
    codesign --force --timestamp --options runtime \
        --entitlements "$here/entitlements.plist" \
        --sign "$identity" "$app"
    codesign --verify --deep --strict --verbose=2 "$app"
else
    printf 'no signing identity: the DMG will be unsigned\n' >&2
fi

mkdir -p "$out"
dmg="$out/Slate-To-UCS-${version}-$(uname -m).dmg"
rm -f "$dmg"
hdiutil create -quiet -volname "Slate-To-UCS $version" -srcfolder "$app" \
    -fs HFS+ -format UDZO -ov "$dmg" >/dev/null
[ -f "$dmg" ] || die "hdiutil reported success but $dmg does not exist"

if [ -n "$identity" ]; then
    codesign --force --timestamp --sign "$identity" "$dmg"
    codesign --verify --verbose=2 "$dmg"
fi

if [ "$notarise" -eq 1 ]; then
    [ -n "${APPLE_ID:-}" ] || die "APPLE_ID is required for --notarise"
    [ -n "${APPLE_TEAM_ID:-}" ] || die "APPLE_TEAM_ID is required for --notarise"
    [ -n "${APPLE_APP_PASSWORD:-}" ] || die "APPLE_APP_PASSWORD is required for --notarise"
    xcrun notarytool submit "$dmg" \
        --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_PASSWORD" \
        --wait
    # Stapling attaches the ticket to the file, so a machine that is offline at install
    # time can still validate it. Without this the notarisation only helps online.
    xcrun stapler staple "$dmg"
    xcrun stapler validate "$dmg"
    spctl --assess --type open --context context:primary-signature -v "$dmg"
fi

printf 'built %s (%s bytes)\n' "$dmg" "$(wc -c < "$dmg" | tr -d ' ')"
printf '%s\n' "$dmg"
