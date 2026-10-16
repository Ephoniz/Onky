#!/usr/bin/env bash
# Fail when a release tag, Cargo.toml, and CHANGELOG.md disagree.
# On success, write that changelog section and version metadata.
set -euo pipefail

usage() {
    echo "usage: check-release-version.sh --print-cargo-version [--cargo FILE]" >&2
    echo "       check-release-version.sh --tag TAG [--cargo FILE] [--changelog FILE] [--notes FILE] [--meta FILE]" >&2
    exit 2
}

cargo_file="Cargo.toml"
changelog_file="CHANGELOG.md"
notes_file=""
meta_file=""
tag=""
print_only=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --print-cargo-version) print_only=1; shift ;;
        --tag)
            tag=${2:-}
            shift 2
            ;;
        --cargo)
            cargo_file=${2:-}
            shift 2
            ;;
        --changelog)
            changelog_file=${2:-}
            shift 2
            ;;
        --notes)
            notes_file=${2:-}
            shift 2
            ;;
        --meta)
            meta_file=${2:-}
            shift 2
            ;;
        *) usage ;;
    esac
done

read_cargo_version() {
    local file=$1
    awk '
        BEGIN { found = 0 }
        /^\[workspace\.package\]/ { in_pkg = 1; next }
        /^\[/ { in_pkg = 0 }
        in_pkg && /^version[[:space:]]*=/ {
            if (match($0, /version[[:space:]]*=[[:space:]]*"[^"]+"/)) {
                line = substr($0, RSTART, RLENGTH)
                sub(/^version[[:space:]]*=[[:space:]]*"/, "", line)
                sub(/"$/, "", line)
                print line
                found = 1
                exit
            }
        }
        END { if (!found) exit 1 }
    ' "$file"
}

if [[ ! -f "$cargo_file" ]]; then
    echo "missing Cargo.toml: $cargo_file" >&2
    exit 1
fi

version="$(read_cargo_version "$cargo_file")" || {
    echo "could not read workspace.package version from $cargo_file" >&2
    exit 1
}

if [[ -z "$version" ]]; then
    echo "empty version in $cargo_file" >&2
    exit 1
fi

if [[ "$print_only" -eq 1 ]]; then
    printf '%s\n' "$version"
    exit 0
fi

if [[ -z "$tag" ]]; then
    usage
fi

case "$tag" in
    v*) release_version=${tag#v} ;;
    *)
        echo "tag must start with v, found $tag" >&2
        exit 1
        ;;
esac

if [[ "$release_version" != "$version" ]]; then
    echo "tag $tag does not match Cargo.toml version $version" >&2
    exit 1
fi

if [[ ! -f "$changelog_file" ]]; then
    echo "missing changelog: $changelog_file" >&2
    exit 1
fi

escaped=""
for ((i = 0; i < ${#release_version}; i++)); do
    char="${release_version:i:1}"
    case "$char" in
        '.' | '\' | '*' | '^' | '$' | '(' | ')' | '+' | '?' | '[' | ']' | '{' | '|' | '/')
            escaped+="\\$char"
            ;;
        *)
            escaped+="$char"
            ;;
    esac
done

status=0
body="$(awk -v ver="$escaped" '
    BEGIN { found = 0; n = 0 }
    {
        gsub(/\r/, "")
    }
    $0 ~ "^## \\[" ver "\\]([[:space:]]|$)" { found = 1; next }
    found && $0 ~ "^## " { exit }
    found { lines[++n] = $0 }
    END {
        if (!found) exit 2
        start = 1
        end = n
        while (start <= end && lines[start] ~ /^[[:space:]]*$/) start++
        while (end >= start && lines[end] ~ /^[[:space:]]*$/) end--
        if (start > end) exit 3
        for (i = start; i <= end; i++) print lines[i]
    }
' "$changelog_file")" || status=$?

if [[ "$status" -eq 2 ]]; then
    echo "CHANGELOG.md has no section for $release_version" >&2
    exit 1
fi
if [[ "$status" -eq 3 || -z "${body//[[:space:]]/}" ]]; then
    echo "CHANGELOG.md section for $release_version is empty" >&2
    exit 1
fi
if [[ "$status" -ne 0 ]]; then
    echo "could not read $changelog_file" >&2
    exit 1
fi

prerelease=false
if [[ "$release_version" =~ -(alpha|beta|rc)(\.|$) ]]; then
    prerelease=true
fi

if [[ -n "$notes_file" ]]; then
    printf '%s\n' "$body" > "$notes_file"
fi
if [[ -n "$meta_file" ]]; then
    printf 'version=%s\nprerelease=%s\n' "$release_version" "$prerelease" > "$meta_file"
fi
