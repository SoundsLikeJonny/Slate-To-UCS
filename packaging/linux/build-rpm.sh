#!/bin/sh
# Build the RPM from the staged tree.
#
# Usage: build-rpm.sh [--out <dir>] [--slate <path>] [--reaper-so <path>] [--sidecar <dir>]
#
# rpmbuild wants a source tarball and a spec, so the staged tree is archived first. The
# spec is generated from slate.spec.in with the version and the architecture substituted,
# which keeps the version in one place: version.toml.
set -eu

die() {
    printf 'build-rpm: %s\n' "$1" >&2
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
    *--slate*) probe=$(printf '%s' "$stage_args" | sed -n 's/.*--slate \([^ ]*\).*/\1/p') ;;
    *) probe=$slate_bin ;;
esac
[ -f "$probe" ] || die "missing binary: $probe"

machine=$(od -An -tx1 -j18 -N2 "$probe" | tr -d ' \n')
case "$machine" in
    3e00) arch=x86_64 ;;
    b700) arch=aarch64 ;;
    *) die "unrecognised ELF machine '$machine' in $probe" ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM

mkdir -p "$work"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
sh "$here/stage.sh" "$work/payload" $stage_args >/dev/null

# The tarball's top directory must match %setup -q, which expects name-version.
tar -C "$work/payload" -czf "$work/SOURCES/slate-to-ucs-$version.tar.gz" usr

sed -e "s/@VERSION@/$version/g" -e "s/@ARCH@/$arch/g" \
    "$here/slate.spec.in" > "$work/SPECS/slate-to-ucs.spec"

rpmbuild --define "_topdir $work" --define "_build_id_links none" \
    -bb "$work/SPECS/slate-to-ucs.spec" >/dev/null

built=$(find "$work/RPMS" -name '*.rpm' -type f | head -1)
[ -n "$built" ] || die "rpmbuild produced no .rpm"

mkdir -p "$out"
rpm="$out/$(basename "$built")"
cp "$built" "$rpm"

printf 'built %s (%s bytes)\n' "$rpm" "$(wc -c < "$rpm" | tr -d ' ')"
printf '%s\n' "$rpm"
