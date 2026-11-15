#!/bin/sh
# Build the AppImage from the staged tree.
#
# Usage: build-appimage.sh [--out <dir>] [--slate <path>] [--reaper-so <path>] [--sidecar <dir>]
#
# An AppImage is a squashfs with an AppRun entry point and a .desktop file at the root.
# It needs no root to run, which is why it is the artefact for a recordist on a machine
# they do not administer.
set -eu

die() {
    printf 'build-appimage: %s\n' "$1" >&2
    exit 1
}

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)

out="$root/packaging/linux/out"
stage_args=""

while [ $# -gt 0 ]; do
    case "$1" in
        --out) out=$2; shift 2 ;;
        --slate) stage_args="$stage_args --slate $2"; shift 2 ;;
        --reaper-so) stage_args="$stage_args --reaper-so $2"; shift 2 ;;
        --sidecar) stage_args="$stage_args --sidecar $2"; shift 2 ;;
        -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
        *) die "unknown option $1" ;;
    esac
done

version=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$root/version.toml" | head -1)
[ -n "$version" ] || die "version.toml has no version line"

machine=$(od -An -tx1 -j18 -N2 "$root/target/release/slate" 2>/dev/null | tr -d ' \n' || true)
case "$machine" in
    3e00) arch=x86_64 ;;
    b700) arch=aarch64 ;;
    *) die "unrecognised ELF machine '$machine' in target/release/slate" ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM

sh "$here/stage.sh" "$work/AppDir" $stage_args >/dev/null

# AppImage's own layout: the desktop file and the icon live at the root as well as in
# their system locations, because the runtime reads them from there.
cp "$here/slate.desktop" "$work/AppDir/slate.desktop"
cp "$work/AppDir/usr/share/icons/hicolor/256x256/apps/slate.png" "$work/AppDir/slate.png"

cat > "$work/AppDir/AppRun" <<'EOF'
#!/bin/sh
# Resolve the payload relative to this file, so the AppImage runs from anywhere.
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec "$here/usr/bin/slate" "$@"
EOF
chmod 0755 "$work/AppDir/AppRun"

mkdir -p "$out"
appimage="$out/Slate-To-UCS-${version}-${arch}.AppImage"

if command -v appimagetool >/dev/null 2>&1; then
    ARCH=$arch appimagetool --no-appstream "$work/AppDir" "$appimage" >/dev/null
elif command -v mksquashfs >/dev/null 2>&1; then
    # A squashfs with the runtime prepended is what appimagetool produces; without the
    # tool the payload is still a valid squashfs, which is enough to assert the layout.
    mksquashfs "$work/AppDir" "$appimage" -noappend -comp zstd -quiet >/dev/null
else
    die "neither appimagetool nor mksquashfs is installed"
fi

[ -f "$appimage" ] || die "the packager reported success but $appimage does not exist"
chmod 0755 "$appimage"

printf 'built %s (%s bytes)\n' "$appimage" "$(wc -c < "$appimage" | tr -d ' ')"
printf '%s\n' "$appimage"
