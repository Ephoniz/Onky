# Changelog

All notable changes to Audio Program are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Windows mixing: pick playback and recording devices as strips, including VB-Cable and other existing cables. Streams use WASAPI shared mode with the master output as the clock. Unplugged devices show on their strip and reconnect. The session is saved and restored.
- Windows loopback driver (`driver/`): a PortCls WaveRT driver, started from Microsoft's `simpleaudiosample`, that creates "Audio Program Cable N" cables. Each cable is a stereo render endpoint looped to a stereo capture endpoint (48 kHz, 32-bit PCM). There are 8 cables by default, and the `CableCount` registry value sets 1 to 16, applied on device restart.
- INF and WDK project with a pinned NuGet SDK/WDK, and a `driver` CI workflow that builds, test-signs and uploads `.sys`/`.inf`/`.cat`.
- PowerShell scripts for test certificates, signing, test mode, install/uninstall (pnputil or devcon), cable count and status.
- Sonicanvas smoke-test package and a step-by-step Windows testing guide (`driver/TESTING.md`).
- Host unit tests for the cable ring buffer and cable count setting, and Pester tests for the driver scripts.

## [0.1.0-alpha.1] - 2026-10-16

### Added

- Unsigned all-users Windows installer and a portable mixer zip, with checksums. Pull-request builds are artifacts. A release tag must match `Cargo.toml` and this changelog.
- The Windows mixer opens its window without a console. Audio setup no longer puts the UI thread in a multithreaded COM apartment, which closed the app immediately. The window is drawn with Direct3D, because Windows often only has OpenGL 1.1.
- On startup, Windows checks GitHub Releases for a newer version, verifies `SHA256SUMS.txt`, and runs that setup. The check can be turned off, and a failed check still opens the app.
- Real-time mixer graph with gain, mute, multi-route, and adaptive resampling of every device that is not the master clock.
- PipeWire virtual cables on Linux, removed when the process exits.
- WASAPI shared-mode hardware listing on Windows. Virtual cables are left to the kernel driver in `driver/`.
- Mixer window and a `--headless` session.
- Allocation check for the mix callback, and a release-mode CPU budget test.
- Agent rules in `AGENTS.md`. Test changes require the owner's review.
