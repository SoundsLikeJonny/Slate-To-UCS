#!/bin/sh
# Assert the Linux staging tree and the packaging scripts, on any host with a shell.
#
# This is the part of the Linux packaging that can be checked without dpkg, rpmbuild or
# appimagetool: the layout every packager consumes, the desktop entry, the AppStream
# metadata and the scripts' own syntax. The packagers themselves are exercised in CI on
# ubuntu-22.04, where those tools exist.
#
# Usage: test_layout.sh
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)

checks=0
failures=0

ok() {
    checks=$((checks + 1))
    printf '  ok   %s\n' "$1"
}

fail() {
    checks=$((checks + 1))
    failures=$((failures + 1))
    printf '  FAIL %s\n' "$1" >&2
}

expect_file() {
    if [ -f "$1" ]; then ok "exists: $2"; else fail "missing: $2"; fi
}

expect_absent() {
    if [ -e "$1" ]; then fail "should not exist: $2"; else ok "absent: $2"; fi
}

expect_exec() {
    # Windows synthesises POSIX modes from ACLs, so the exact 0755 string is not portable.
    # The invariant that matters is that every class may execute and none may write.
    actual=$(ls -l "$1" | cut -c1-10)
    case "$actual" in
        -rwxr-xr-x|-rwxrwxr-x|-rwxr-xr-x*) ok "executable, not world-writable: $2" ;;
        *) fail "mode $actual is not an executable non-world-writable file: $2" ;;
    esac
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM

# The host binary is `slate` on Unix and `slate.exe` on Windows. The layout test asserts
# where the payload lands, not what the host calls it, so the path is resolved here and
# passed explicitly rather than left to stage.sh's Unix default.
slate_bin="$root/target/release/slate"
[ -f "$slate_bin" ] || slate_bin="$root/target/release/slate.exe"
[ -f "$slate_bin" ] || { printf 'missing target/release/slate; run cargo build --release -p slate-cli\n' >&2; exit 1; }

printf 'syntax\n'
for script in "$here"/../linux/*.sh "$here"/../macos/*.sh "$0"; do
    name=$(basename "$script")
    if sh -n "$script" 2>/dev/null; then ok "sh -n $name"; else fail "sh -n $name"; fi
done

# `sh -n` only parses, so it cannot catch a bashism that expands at run time. Brace
# expansion is the one that bit: `mkdir -p "$work"/{A,B}` under dash creates a single
# directory literally named `{A,B}`, and the failure surfaces much later as a missing
# path. These scripts run under `sh`, so brace expansion is never available to them.
printf 'posix sh only\n'
for script in "$here"/../linux/*.sh "$here"/../macos/*.sh; do
    name=$(basename "$script")
    if grep -nE '(mkdir|cp|rm|ls|install|tar)[^|;]*\{[^}]*,[^}]*\}' "$script" >/dev/null 2>&1; then
        fail "brace expansion in $name (dash does not expand it)"
    else
        ok "no brace expansion in $name"
    fi
    if grep -nE '(^|[;&|(]|if |elif |while |until )[[:space:]]*\[\[ |^[[:space:]]*local |^[[:space:]]*function ' "$script" >/dev/null 2>&1; then
        fail "bash-only syntax in $name"
    else
        ok "no bash-only syntax in $name"
    fi
done

printf 'staging\n'
sh "$here/../linux/stage.sh" "$work/tree" --slate "$slate_bin" >/dev/null
expect_file "$work/tree/usr/bin/slate" "usr/bin/slate"
expect_exec "$work/tree/usr/bin/slate" "usr/bin/slate"
expect_file "$work/tree/usr/share/applications/slate.desktop" "slate.desktop"
expect_file "$work/tree/usr/share/metainfo/io.github.soundslikejonny.SlateToUCS.metainfo.xml" "metainfo"
expect_file "$work/tree/usr/share/icons/hicolor/256x256/apps/slate.png" "icon"
expect_file "$work/tree/usr/share/doc/slate/LICENSE" "LICENSE"
expect_file "$work/tree/usr/share/doc/slate/README.md" "README.md"
expect_absent "$work/tree/usr/lib/slate/slate_reaper.so" "extension without --reaper-so"
expect_absent "$work/tree/usr/lib/slate/ttucs" "sidecar without --sidecar"

printf 'optional payloads\n'
sh "$here/../linux/stage.sh" "$work/full" \
    --slate "$slate_bin" \
    --reaper-so "$slate_bin" \
    --sidecar "$work/tree/usr/share/doc/slate" >/dev/null
expect_file "$work/full/usr/lib/slate/slate_reaper.so" "extension with --reaper-so"
expect_file "$work/full/usr/lib/slate/ttucs/LICENSE" "sidecar with --sidecar"

printf 'desktop entry\n'
desktop="$here/../linux/slate.desktop"
for key in Type Name Exec Icon Categories; do
    if grep -q "^$key=" "$desktop"; then ok "desktop has $key"; else fail "desktop lacks $key"; fi
done
if grep -q '^Exec=slate$' "$desktop"; then ok "Exec is the bare binary name"; else fail "Exec is not 'slate'"; fi

printf 'appstream metadata\n'
meta="$here/../linux/io.github.soundslikejonny.SlateToUCS.metainfo.xml"
if command -v python >/dev/null 2>&1; then
    if python -c "import xml.etree.ElementTree as E,sys;E.parse(sys.argv[1])" "$meta" 2>/dev/null; then
        ok "metainfo parses as XML"
    else
        fail "metainfo does not parse as XML"
    fi
else
    printf '  skip metainfo XML parse: no python\n'
fi
if grep -q '<id>io.github.soundslikejonny.SlateToUCS</id>' "$meta"; then
    ok "metainfo id matches the bundle identifier"
else
    fail "metainfo id does not match the bundle identifier"
fi
if grep -q '<launchable type="desktop-id">slate.desktop</launchable>' "$meta"; then
    ok "metainfo launchable names the desktop file"
else
    fail "metainfo launchable does not name the desktop file"
fi

printf 'spec template\n'
spec="$here/../linux/slate.spec.in"
for field in Name Version License Source0 %install %files %changelog; do
    if grep -q "^$field" "$spec"; then ok "spec has $field"; else fail "spec lacks $field"; fi
done
if grep -q '@VERSION@' "$spec" && grep -q '@ARCH@' "$spec"; then
    ok "spec keeps its substitution placeholders"
else
    fail "spec lost a substitution placeholder"
fi

printf '\n%d checks, %d failures\n' "$checks" "$failures"
[ "$failures" -eq 0 ]
