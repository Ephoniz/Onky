#!/usr/bin/env bash
# Write SHA256SUMS.txt for the files in a directory.
# Lowercase hash, two spaces, file name, LF, no BOM. The checksum file is not hashed.
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: write-sha256sums.sh DIRECTORY" >&2
    exit 2
fi

dir=$1
if [[ ! -d "$dir" ]]; then
    echo "not a directory: $dir" >&2
    exit 1
fi

file_sha256() {
    local file=$1
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum -- "$file" | awk '{print tolower($1)}'
        return
    fi
    if command -v powershell.exe >/dev/null 2>&1; then
        powershell.exe -NoProfile -Command "(Get-FileHash -Algorithm SHA256 -LiteralPath '$file').Hash.ToLower()" | tr -d '\r'
        return
    fi
    echo "sha256sum is not available" >&2
    exit 1
}

shopt -s nullglob
files=()
for path in "$dir"/*; do
    [[ -f "$path" ]] || continue
    name="$(basename "$path")"
    [[ "$name" == "SHA256SUMS.txt" || "$name" == "SHA256SUMS.txt.tmp" ]] && continue
    files+=("$name")
done

if [[ ${#files[@]} -eq 0 ]]; then
    echo "no files to hash in $dir" >&2
    exit 1
fi

mapfile -t sorted < <(printf '%s\n' "${files[@]}" | LC_ALL=C sort)
tmp="$dir/SHA256SUMS.txt.tmp"
: > "$tmp"
for name in "${sorted[@]}"; do
    hash="$(file_sha256 "$dir/$name")"
    printf '%s  %s\n' "$hash" "$name" >> "$tmp"
done
mv "$tmp" "$dir/SHA256SUMS.txt"
