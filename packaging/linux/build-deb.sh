#!/bin/sh
# Build the Debian package from the staged tree.
#
# Usage: build-deb.sh [--out <dir>] [--slate <path>] [--reaper-so <path>] [--sidecar <dir>]
#
# The package is architecture-specific because it ships a compiled binary. `dpkg-deb`
# refuses a control file whose Architecture disagrees with the payload, so the value is
# derived from the binary rather than hard-coded.
set -eu

die() {
    printf 'build-deb: %s\n' "$1" >&2
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

slate_bin="$root/target/release/slate"
case "$stage_args" in
    *--slate*) ;;
    *) [ -f "$slate_bin" ] || die "missing binary: $slate_bin" ;;
esac

# Debian architecture names, from the ELF header rather than from the host, so a
# cross-built package is labelled correctly.
probe=${slate_bin}
case "$stage_args" in
    *--slate*) probe=$(printf '%s' "$stage_args" | sed -n 's/.*--slate \([^ ]*\).*/\1/p') ;;
esac
machine=$(od -An -tx1 -j18 -N2 "$probe" | tr -d ' \n')
case "$machine" in
    3e00) arch=amd64 ;;
    b700) arch=arm64 ;;
    *) die "unrecognised ELF machine '$machine' in $probe" ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM

sh "$here/stage.sh" "$work/tree" $stage_args >/dev/null

mkdir -p "$work/tree/DEBIAN"
installed_size=$(du -sk "$work/tree" | cut -f1)

cat > "$work/tree/DEBIAN/control" <<EOF
Package: slate-to-ucs
Version: $version
Section: sound
Priority: optional
Architecture: $arch
Installed-Size: $installed_size
Maintainer: Jon Evans <jonnythejet@gmail.com>
Homepage: https://github.com/SoundsLikeJonny/Slate-To-UCS
Description: label sound effects with UCS metadata
 Slate-To-UCS reads a sound effect, transcribes any spoken slate and suggests
 Universal Category System metadata for it: category, subcategory, CatID,
 keywords and a UCS file name.
 .
 Everything runs locally. The classifier is served on loopback and offline mode
 is the default, so no audio, transcript or file name leaves the machine.
EOF

# The binary is the only thing that must not be stripped by the packager; dpkg-deb does
# not strip, but it does compress, and a pre-compressed binary would defeat that.
cat > "$work/tree/DEBIAN/md5sums" <<EOF
EOF
(cd "$work/tree" && find usr -type f -exec md5sum {} + | sed 's|  |  |') > "$work/tree/DEBIAN/md5sums"

mkdir -p "$out"
deb="$out/slate-to-ucs_${version}_${arch}.deb"
dpkg-deb --root-owner-group --build "$work/tree" "$deb" >/dev/null
[ -f "$deb" ] || die "dpkg-deb reported success but $deb does not exist"

printf 'built %s (%s bytes)\n' "$deb" "$(wc -c < "$deb" | tr -d ' ')"
printf '%s\n' "$deb"
