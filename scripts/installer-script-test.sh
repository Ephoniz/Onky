#!/usr/bin/env bash
# Locks the installer decisions that do not need a Windows machine.
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
    echo "installer-script: $1" >&2
    exit 1
}

iss="packaging/windows/audio-program.iss"
package="packaging/windows/package.ps1"
test_ps1="packaging/windows/install-test.ps1"

require() {
    local file=$1
    local text=$2
    grep -F -q -e "$text" -- "$file" || fail "$file is missing [$text]"
}

require "$iss" "PrivilegesRequired=admin"
require "$iss" "ArchitecturesAllowed=x64compatible"
require "$iss" "OutputBaseFilename=AudioProgram-Setup-{#MyAppVersion}"
require "$iss" "Also delete your settings?"
require "$iss" "UninstallSilent"
require "$iss" "MB_DEFBUTTON2"
require "$iss" "Flags: postinstall nowait skipifsilent"
require "$iss" "Flags: unchecked"
require "$iss" "AppId={{F605ED42-60D8-49A9-A2E2-98BC6BB1579F}"
if grep -F -q "AppId={{F605ED42-60D8-49A9-A2E2-98BC6BB1579F}}" "$iss"; then
    fail "AppId closing brace is doubled"
fi
require "$package" 'AudioProgram-Setup-$Version.exe'
require "$package" 'AudioProgram-$Version-portable.zip'
require "$test_ps1" "silent uninstall deleted settings"
require "$test_ps1" "--headless"

if grep -E -q '^AppPublisher=' "$iss"; then
    fail "AppPublisher is set; unsigned builds omit it"
fi
if grep -E -i -q 'Install-Driver|Enable-TestSigning|Test cables|test mode' "$iss"; then
    fail "installer script mentions test cables"
fi

echo "ok installer-script"
