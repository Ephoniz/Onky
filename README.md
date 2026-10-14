# Audio Program

Audio Program is a native desktop mixer with virtual audio cables. The engine and the app are Rust. The window is [egui](https://github.com/emilk/egui). Version 0.1.0.

The default session is two virtual stereo inputs and two virtual stereo outputs, with each input routed to the matching output. The mixer also supports gain, mute, and a routing matrix. Hardware devices that the backend can see are listed beside the strips.

## Linux

Linux uses PipeWire. PulseAudio applications go through `pipewire-pulse`; there is no PulseAudio backend. Start PipeWire and WirePlumber, then:

```sh
cargo run -p audio-program --release
```

`--headless` prints the cable names and keeps them up until the process exits. Nodes are created with `object.linger=false`, so PipeWire removes them when the client socket closes.

Virtual inputs are named `audio-program-virtual-input-N`. Virtual outputs are named `audio-program-virtual-output-N`. Play into an input and record from an output.

Build dependencies on Debian or Ubuntu:

```sh
sudo apt install pkg-config clang libclang-dev libpipewire-0.3-dev \
  libxkbcommon-dev libwayland-dev libx11-dev libegl1-mesa-dev libxcb1-dev \
  libxcursor-dev libxrandr-dev libxi-dev libxfixes-dev libgl1-mesa-dev libssl-dev
```

## Windows

Windows opens hardware endpoints with WASAPI shared mode. That path compiles here and has not been run on a Windows machine.

WASAPI does not create virtual cables. A shared-mode loopback of a speaker is a recording of that endpoint, and other applications cannot select it as a device. Selectable cables come from the kernel driver developed separately under `driver/`. Until that driver is loaded, creating a cable returns an error and the mixer shows it.

## Signing

A lab machine can load an unsigned driver with Windows test-signing. Public attestation signing needs an Extended Validation certificate, typically a few hundred dollars a year. That cost is budgeted before a signed driver release.

## Engine

The master clock is the monitoring hardware output when one is open, and an internal 10 ms clock otherwise. Every other device is resampled on each block, including devices that are already nominally 48 kHz. The resampler is allocated when the device is opened. The mix callback does not allocate, lock, or busy-wait.

On a reference session (4 stereo inputs, 4 stereo outputs, every route on, 48 kHz, 10 ms blocks, one master and seven adaptive resamplers) the release build on this Linux machine stays under 2% of one core for the mix callback and under 5% of one core for the audio thread. That measurement is Linux-only. It is not a Windows result.

## Develop

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.99.0). Dependencies stay on MIT-compatible licenses. The egui font crate also carries the SIL Open Font License and the Ubuntu Font Licence.

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cargo test -p audio-program-engine --release --lib reference_session_cpu_budget -- --nocapture
cargo deny check
```

Releases are git tags `vX.Y.Z`. The version in `Cargo.toml` is the version. See [CHANGELOG.md](CHANGELOG.md) and [CONTRIBUTING.md](CONTRIBUTING.md).
