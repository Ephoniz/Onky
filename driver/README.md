# Audio Program Cable (Windows driver)

A small Windows kernel audio driver that adds virtual loopback cables, as an
open-source replacement for VB-Cable. Each cable is a stereo playback endpoint
looped to a stereo recording endpoint:

- **Speakers (Audio Program Cable N)**, what apps play into
- **Microphone (Audio Program Cable N)**, where the same audio comes out

The driver only copies audio from one side to the other. It does no mixing,
gain or format conversion; the Audio Program app does that in user mode.

| | |
| --- | --- |
| Model | PortCls WaveRT, one root-enumerated device (`ROOT\AudioProgramCable`) |
| Cables | 8 by default, 1 to 16 via the `CableCount` registry value, applied on device restart |
| Format | 48 kHz, 2 channels, 32-bit PCM |
| Buffering | 250 ms ring per cable; capture starts 20 ms behind playback and is kept within 100 ms |
| OS | Windows 10 version 2004 (build 19041) or newer, x64 |
| Status | Test-signed development build; not Microsoft-signed |

## Layout

| Path | What it is |
| --- | --- |
| `src/` | Driver source. Started from Microsoft's `simpleaudiosample` (MS-PL). |
| `src/loopback_ring.h`, `src/loopback.h` | The cable ring buffer (MIT). Kernel-independent, unit-tested on the host. |
| `src/cable_config.h` | Cable count limits, template and friendly names (MIT), unit-tested on the host. |
| `tests/` | Host unit tests (C++) and Pester tests for the scripts. |
| `AudioProgramCable.inx` | INF template; StampInf turns it into `AudioProgramCable.inf`. |
| `AudioProgramCable.vcxproj` | WDK project (WDM driver, `WindowsKernelModeDriver10.0`). |
| `packages.config`, `Directory.Build.props` | Pinned Windows SDK/WDK from NuGet (10.0.26100.6584). |
| `build.ps1` | Builds, packages and optionally test-signs the driver. |
| `scripts/` | Test certificate, signing, test mode, install/uninstall, cable count, status. |
| `smoke-test/sonicanvas/` | Builds an independent driver to check the test PC setup. |
| `TESTING.md` | Step-by-step Windows testing guide with a pass/fail checklist. |

## How it works

At device start the adapter reads
`HKLM\SYSTEM\CurrentControlSet\Services\AudioProgramCable\Parameters\CableCount`
(REG_DWORD, clamped to 1..16, default 8) and, for each cable, registers four
PortCls subdevices: a render wave and topology filter and a capture wave and
topology filter. Their interface settings are copied from the four template
interfaces in the INF, and each cable's interfaces get the friendly name
"Audio Program Cable N".

Each cable owns one ring buffer. The render stream writes the audio the
client put in its WaveRT buffer into the ring as the playback position
advances; the capture stream reads from the ring into its own WaveRT buffer.
When playback stops or falls behind, capture gets silence instead of stale
audio. Several apps can record from the same cable, but each capture
endpoint's audio engine opens one stream, so Windows shares it between apps.

Changing the cable count takes effect when the device restarts
(`scripts\Set-CableCount.ps1` does this with `pnputil /restart-device`).

## Build

On Windows with Visual Studio 2022 ("Desktop development with C++"), from a
PowerShell prompt:

```powershell
cd driver
.\build.ps1 -RestorePackages -TestSign
```

`-RestorePackages` downloads the pinned SDK and WDK from NuGet, so no WDK
install is needed. The package lands in `driver\build\package`:
`AudioProgramCable.sys`, `.inf`, `.cat`, the test certificate
`AudioProgramCableTest.cer`, the scripts and `TESTING.md`.

The CI workflow `.github/workflows/driver.yml` does the same on
`windows-latest` and uploads the `audio-program-cable-x64` artifact. It also
runs the host unit tests, checks and tests the PowerShell scripts, and builds
the Sonicanvas smoke-test package.

Tests that need no Windows, from the repository root:

```sh
for test in driver/tests/*.cpp; do
  c++ -std=c++17 -Wall -Wextra -Werror -I driver/src "$test" -o test-bin && ./test-bin
done
pwsh -c 'Invoke-Pester driver/tests'   # Pester 5
```

## Install and test

Follow [TESTING.md](TESTING.md). In short, in an elevated PowerShell in the
package folder:

```powershell
.\scripts\Enable-TestSigning.ps1      # then reboot (Secure Boot must be off)
.\scripts\Install-Driver.ps1
.\scripts\Get-CableStatus.ps1
.\scripts\Set-CableCount.ps1 -Count 4
.\scripts\Uninstall-Driver.ps1 -RemoveCertificate
```

`Install-Driver.ps1` stages the package with `pnputil /add-driver` and creates
the root device through SetupAPI, as `devcon install` does (pnputil cannot
create root devices). Pass `-Devcon <path to devcon.exe>` to use devcon
instead.

## Licences

- Files derived from Microsoft's Windows-driver-samples keep their Microsoft
  copyright and are under the Microsoft Public License: see each file's header
  and [LICENSES/MS-PL.txt](LICENSES/MS-PL.txt).
- New files are MIT: [LICENSES/MIT.txt](LICENSES/MIT.txt).
- No GPL code is used. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
