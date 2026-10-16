#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
    echo "check-release-version: $1" >&2
    exit 1
}

expect_fail() {
    local label=$1
    shift
    local err status
    set +e
    err="$("$@" 2>&1 >/dev/null)"
    status=$?
    set -e
    if [[ "$status" -eq 0 ]]; then
        fail "expected failure: $label"
    fi
    [[ -n "$err" ]] || fail "$label failed without a message"
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

write_cargo() {
    local dir=$1
    local version=$2
    cat > "$dir/Cargo.toml" <<EOF
[workspace]
resolver = "2"

[workspace.package]
version = "${version}"
edition = "2021"

[other]
version = "9.9.9"
EOF
}

# The real manifest parses, including a decoy version outside workspace.package.
parsed="$(./scripts/check-release-version.sh --print-cargo-version)"
from_file="$(sed -n '/^\[workspace\.package\]$/,/^\[/p' Cargo.toml | sed -n 's/^version = "\(.*\)"/\1/p')"
[[ "$parsed" == "$from_file" && -n "$parsed" ]] || fail "real Cargo.toml parsed as [$parsed] not [$from_file]"

write_cargo "$tmp" "0.1.0"
cat > "$tmp/CHANGELOG.md" <<'EOF'
## [Unreleased]

### Added

- newer

## [0.1.0] - 2026-10-16

### Added

- Mixer installer.

## [0.1.0-alpha.1]

- not this one

## [0x1y0]

- dotted version must not match this
EOF

./scripts/check-release-version.sh \
    --tag v0.1.0 \
    --cargo "$tmp/Cargo.toml" \
    --changelog "$tmp/CHANGELOG.md" \
    --notes "$tmp/notes.md" \
    --meta "$tmp/meta.txt"

grep -q "Mixer installer." "$tmp/notes.md" || fail "notes missing the section body"
grep -q "newer" "$tmp/notes.md" && fail "notes included Unreleased"
grep -q "not this one" "$tmp/notes.md" && fail "notes included a later section"
grep -q "dotted version" "$tmp/notes.md" && fail "notes matched a dotted lookalike"
grep -q '^## ' "$tmp/notes.md" && fail "notes included the heading"
[[ "$(cat "$tmp/meta.txt")" == $'version=0.1.0\nprerelease=false' ]] || fail "stable meta was $(cat "$tmp/meta.txt")"

# 0.1.0 must not accept the 0.1.0-alpha.1 heading.
cat > "$tmp/CHANGELOG.md" <<'EOF'
## [0.1.0-alpha.1]

- alpha only
EOF
expect_fail "prefix heading" \
    ./scripts/check-release-version.sh --tag v0.1.0 --cargo "$tmp/Cargo.toml" --changelog "$tmp/CHANGELOG.md"

write_cargo "$tmp" "0.1.1"
expect_fail "cargo mismatch" \
    ./scripts/check-release-version.sh --tag v0.1.0 --cargo "$tmp/Cargo.toml" --changelog "$tmp/CHANGELOG.md"

write_cargo "$tmp" "0.1.0"
cat > "$tmp/CHANGELOG.md" <<'EOF'
## [Unreleased]

- nothing released
EOF
expect_fail "missing section" \
    ./scripts/check-release-version.sh --tag v0.1.0 --cargo "$tmp/Cargo.toml" --changelog "$tmp/CHANGELOG.md"

cat > "$tmp/CHANGELOG.md" <<'EOF'
## [0.1.0]

## [0.0.1]

- old
EOF
expect_fail "empty section" \
    ./scripts/check-release-version.sh --tag v0.1.0 --cargo "$tmp/Cargo.toml" --changelog "$tmp/CHANGELOG.md"

expect_fail "tag without v" \
    ./scripts/check-release-version.sh --tag 0.1.0 --cargo "$tmp/Cargo.toml" --changelog "$tmp/CHANGELOG.md"

# CRLF changelog still yields the section.
write_cargo "$tmp" "0.1.0-alpha.1"
printf '## [Unreleased]\r\n\r\n- later\r\n\r\n## [0.1.0-alpha.1] - 2026-10-16\r\n\r\n- First alpha.\r\n' > "$tmp/CHANGELOG.md"
./scripts/check-release-version.sh \
    --tag v0.1.0-alpha.1 \
    --cargo "$tmp/Cargo.toml" \
    --changelog "$tmp/CHANGELOG.md" \
    --notes "$tmp/alpha-notes.md" \
    --meta "$tmp/alpha-meta.txt"
grep -q "First alpha." "$tmp/alpha-notes.md" || fail "CRLF notes missing the body"
grep -q "later" "$tmp/alpha-notes.md" && fail "CRLF notes included Unreleased"
[[ "$(cat "$tmp/alpha-meta.txt")" == $'version=0.1.0-alpha.1\nprerelease=true' ]] || fail "alpha meta was $(cat "$tmp/alpha-meta.txt")"

check_channel() {
    local version=$1
    local want=$2
    write_cargo "$tmp" "$version"
    printf '## [%s]\n\n- note\n' "$version" > "$tmp/CHANGELOG.md"
    ./scripts/check-release-version.sh \
        --tag "v$version" \
        --cargo "$tmp/Cargo.toml" \
        --changelog "$tmp/CHANGELOG.md" \
        --meta "$tmp/channel.txt"
    grep -q "prerelease=$want" "$tmp/channel.txt" || fail "$version prerelease wanted $want"
}

check_channel "1.0.0-beta" true
check_channel "1.0.0-beta.2" true
check_channel "1.0.0-rc.1" true
check_channel "1.2.3" false
check_channel "0.1.0-pr.12" false

echo "ok check-release-version"
