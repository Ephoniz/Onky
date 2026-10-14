# Sonicanvas smoke test

This folder builds and installs a second, independent virtual audio driver:
[Sonicanvas](https://github.com/open-string/sonicanvas-windows-driver)
(MIT licence), pinned to commit `d38f16a0fa848a7b85563b3e5fb65b82dd7721db`.

Use it to check the test PC before blaming the Audio Program driver. If
Sonicanvas installs, shows up in Sound settings and loops audio, then test
signing, certificate trust and the install scripts all work on that PC. If
Sonicanvas fails the same way Audio Program Cable fails, the problem is the PC
setup, not our driver.

Nothing from Sonicanvas is copied into this repository. The build script
clones the pinned commit and runs Sonicanvas's own build and packaging
scripts, which sign the package with a throwaway test certificate.

## What it creates

One root device, `Root\SonicanvasAudio` ("Sonicanvas Virtual Audio"), with
two endpoints:

- **Sonicanvas Send** (playback)
- **Sonicanvas Microphone** (recording)

Audio played into Send can be recorded from Microphone (48 kHz, stereo by
default, 16-bit PCM).

## Get the package

Either download the `sonicanvas-smoke-test` artifact from the `driver` CI
workflow, or build it on a Windows PC with Visual Studio 2022 (C++ workload),
git and nuget.exe:

```powershell
.\Build-SonicanvasPackage.ps1
# Output: driver\build\smoke-test\sonicanvas
```

## Install, test, remove

Test signing must be on first (see `driver/TESTING.md`, steps 1 and 2). Then,
in an elevated PowerShell inside the package folder:

```powershell
.\Install-Sonicanvas.ps1
```

Open **Settings > System > Sound**. You should see "Sonicanvas Send" under
Output and "Sonicanvas Microphone" under Input. Run the same playback and
recording test as in `driver/TESTING.md` step 6, with these two devices.

The two drivers can coexist, but remove Sonicanvas before testing Audio
Program Cable so the results are not mixed up:

```powershell
.\Uninstall-Sonicanvas.ps1
```

This removes the device, deletes the driver package from the Driver Store and
removes the Sonicanvas test certificate.
