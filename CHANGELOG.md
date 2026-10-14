# Changelog

All notable changes to Audio Program are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Real-time mixer graph with gain, mute, multi-route, and adaptive resampling of every device that is not the master clock.
- PipeWire virtual cables on Linux, removed when the process exits.
- WASAPI shared-mode hardware listing on Windows. Virtual cables are left to the kernel driver in `driver/`.
- Mixer window and a `--headless` session.
- Allocation check for the mix callback, and a release-mode CPU budget test.
