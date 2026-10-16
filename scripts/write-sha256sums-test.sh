#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
    echo "write-sha256sums: $1" >&2
    exit 1
}

dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT

printf 'abc' > "$dir/a.txt"
printf 'xyz' > "$dir/b.txt"
printf 'stale\n' > "$dir/SHA256SUMS.txt"

./scripts/write-sha256sums.sh "$dir"

expected_a="ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
[[ "$(sed -n '1p' "$dir/SHA256SUMS.txt")" == "$expected_a  a.txt" ]] || fail "first line was [$(sed -n '1p' "$dir/SHA256SUMS.txt")]"
grep -q 'SHA256SUMS.txt' "$dir/SHA256SUMS.txt" && fail "checksum file hashed itself"
grep -q 'stale' "$dir/SHA256SUMS.txt" && fail "old checksum text survived"
marker="$(tail -c 1 "$dir/SHA256SUMS.txt"; printf x)"
[[ "$marker" == $'\nx' ]] || fail "missing trailing newline"
[[ "$(head -c 3 "$dir/SHA256SUMS.txt")" != $'\xef\xbb\xbf' ]] || fail "BOM"
(cd "$dir" && sha256sum -c SHA256SUMS.txt) || fail "sha256sum -c"

copy="$(cat "$dir/SHA256SUMS.txt")"
./scripts/write-sha256sums.sh "$dir"
[[ "$copy" == "$(cat "$dir/SHA256SUMS.txt")" ]] || fail "checksums were not stable"

empty="$(mktemp -d)"
if ./scripts/write-sha256sums.sh "$empty"; then
    fail "empty directory should fail"
fi
if ./scripts/write-sha256sums.sh "$dir/missing"; then
    fail "missing directory should fail"
fi

echo "ok write-sha256sums"
