#!/bin/sh
# Stage the Linux install tree that the deb, the rpm and the AppImage all package.
#
# One staging function, three packagers. If the three disagreed about where a file lives,
# three uninstallers would each remove part of another's work, which is the same failure
# the MSI and the NSIS installer are kept in step to avoid.
#
# Usage:
#   stage.sh <destdir> [--slate <path>] [--icon <path>] [--reaper-so <path>] [--sidecar <dir>]
#
# The tree it writes:
#   usr/bin/slate
#   usr/lib/slate/slate_reaper.so            (optional)
#   usr/lib/slate/ttucs/...                  (optional sidecar)
#   usr/share/applications/slate.desktop
#   usr/share/metainfo/io.github.soundslikejonny.SlateToUCS.metainfo.xml
#   usr/share/icons/hicolor/256x256/apps/slate.png
#   usr/share/doc/slate/LICENSE
#   usr/share/doc/slate/README.md
set -eu

die() {
    printf 'stage: %s\n' "$1" >&2
    exit 1
}

dest=""
slate=""
icon=""
reaper_so=""
sidecar=""

while [ $# -gt 0 ]; do
    case "$1" in
        --slate) slate=$2; shift 2 ;;
        --icon) icon=$2; shift 2 ;;
        --reaper-so) reaper_so=$2; shift 2 ;;
        --sidecar) sidecar=$2; shift 2 ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        -*) die "unknown option $1" ;;
        *) dest=$1; shift ;;
    esac
done

[ -n "$dest" ] || die "a destination directory is required"

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)

[ -n "$slate" ] || slate="$root/target/release/slate"
[ -n "$icon" ] || icon="$root/apps/desktop/src-tauri/icons/icon.png"

[ -f "$slate" ] || die "missing binary: $slate"
[ -f "$icon" ] || die "missing icon: $icon"

mkdir -p "$dest/usr/bin" \
         "$dest/usr/lib/slate" \
         "$dest/usr/share/applications" \
         "$dest/usr/share/metainfo" \
         "$dest/usr/share/icons/hicolor/256x256/apps" \
         "$dest/usr/share/doc/slate"

install -m 0755 "$slate" "$dest/usr/bin/slate"
install -m 0644 "$icon" "$dest/usr/share/icons/hicolor/256x256/apps/slate.png"
install -m 0644 "$here/slate.desktop" "$dest/usr/share/applications/slate.desktop"
install -m 0644 "$here/io.github.soundslikejonny.SlateToUCS.metainfo.xml" \
    "$dest/usr/share/metainfo/io.github.soundslikejonny.SlateToUCS.metainfo.xml"
install -m 0644 "$root/LICENSE" "$dest/usr/share/doc/slate/LICENSE"
install -m 0644 "$root/README.md" "$dest/usr/share/doc/slate/README.md"

if [ -n "$reaper_so" ]; then
    [ -f "$reaper_so" ] || die "missing REAPER extension: $reaper_so"
    install -m 0755 "$reaper_so" "$dest/usr/lib/slate/slate_reaper.so"
fi

if [ -n "$sidecar" ]; then
    [ -d "$sidecar" ] || die "missing sidecar directory: $sidecar"
    cp -R "$sidecar" "$dest/usr/lib/slate/ttucs"
fi

printf 'staged %s\n' "$dest"
