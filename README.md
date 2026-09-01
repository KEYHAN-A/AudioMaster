# AudioMaster

[![Release](https://img.shields.io/badge/release-v1.2.1-38bdf8)](https://github.com/KEYHAN-A/AudioMaster/releases/tag/v1.2.1)
[![CI](https://github.com/KEYHAN-A/AudioMaster/actions/workflows/ci.yml/badge.svg)](https://github.com/KEYHAN-A/AudioMaster/actions/workflows/ci.yml)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-a78bfa)](https://github.com/KEYHAN-A/AudioMaster/releases/latest)
[![License](https://img.shields.io/badge/license-GPL--3.0-blue)](LICENSE)
[![Website](https://img.shields.io/badge/website-audiomaster.keyhan.info-22c55e)](https://audiomaster.keyhan.info)

AudioMaster is a local-first music mastering engine and desktop application by KEYHAN STUDIO. The production path is implemented in Rust. AI providers may advise a mastering plan, but they never process audio and every proposed parameter is validated before native DSP rendering.

**Current production release:** [AudioMaster v1.2.1](https://github.com/KEYHAN-A/AudioMaster/releases/tag/v1.2.1)

**Unified account:** [core.keyhan.info/account](https://core.keyhan.info/account)

## AudioMaster v1.2.1

This release completes the production migration to the standalone KEYHAN STUDIO Core platform:

- Unified KEYHAN STUDIO authentication and account management through `core.keyhan.info`.
- Revisioned cloud settings and preset synchronization, early-access registration, and feedback submission through Core.
- Production mastering advice through the bounded KEYHAN STUDIO advisor while native DSP remains authoritative.
- Website sign-in and newsletter subscription through the same account platform.
- Automatic migration of saved legacy AudioMaster API settings to the Core endpoint.
- Verified native installers for macOS, Windows, and Linux.

Existing desktop users should install v1.2.1 to move their authentication and cloud data to KEYHAN STUDIO Core.

## Download

| Platform | Package |
| --- | --- |
| macOS — Universal | [Download DMG](https://github.com/KEYHAN-A/AudioMaster/releases/latest/download/AudioMaster_1.2.1_universal.dmg) |
| Windows — x64 | [Download installer](https://github.com/KEYHAN-A/AudioMaster/releases/latest/download/AudioMaster_1.2.1_x64-setup.exe) |
| Linux — x86_64 | [Download AppImage](https://github.com/KEYHAN-A/AudioMaster/releases/latest/download/AudioMaster_1.2.1_amd64.AppImage) |

Additional Debian, RPM, MSI, app archive, and CLI artifacts are available on the [release page](https://github.com/KEYHAN-A/AudioMaster/releases/tag/v1.2.1).

## Current capabilities

- Deterministic stereo mastering with native EQ, linked compression, M/S width control, loudness gain, lookahead limiting, true-peak enforcement, and deterministic TPDF dither.
- BS.1770-style K-weighted integrated, short-term, and momentary loudness; LRA; sample and true peak; PLR; crest factor; DC; clipping; stereo correlation; and spectral-band analysis.
- Verified WAV, AIFF, FLAC, 320 kbps MP3, and optional AAC delivery with metadata mapping. Files remain hidden until the encoded deliverable passes post-render analysis.
- Transactional album mastering that retains a bounded amount of intentional track-to-track loudness contrast.
- Optional reference matching and experimental Python ML compatibility backends.
- Constrained Ollama, LM Studio, KeyhanStudio, OpenAI, and Anthropic advisors; native DSP remains authoritative.
- KEYHAN STUDIO Core device-code sign-in, OS-vault refresh-token storage, revisioned settings/preset sync, early access, and feedback. Audio is never included in cloud request types.
- CLI and Vue/Tauri desktop interfaces.

## Requirements

- Rust stable
- Node.js 20+
- FFmpeg/FFprobe for metadata-preserving delivery, previews, codecs, and bounded long-form processing
- Python 3.10+ only when using Matchering or the experimental local-ML adapter
- Linux desktop builds additionally require the Tauri/WebKitGTK system packages

## Build and quality gate

```bash
npm ci
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run build
python3 -m py_compile python/*.py
```

Run the desktop app with `npm run tauri dev`, or build installers with `npm run tauri build`.

## CLI

```bash
# Analyze a file
cargo run -p mastering-cli -- analyze mix.wav

# Native production master
cargo run -p mastering-cli -- master mix.wav --backend native --preset streaming -o mix_mastered.wav

# Reference-assisted master (uses Matchering when available)
cargo run -p mastering-cli -- master mix.wav --reference reference.wav

# Cohesive album master; input order is retained
cargo run -p mastering-cli -- album song01.wav song02.wav song03.wav \
  --output-directory ./masters --backend native --preset streaming
```

Use `cargo run -p mastering-cli -- --help` for all options.

## Architecture

- `crates/mastering-core`: analysis, native DSP, safety validation, delivery pipeline, albums, and cloud protocol.
- `crates/mastering-cli`: automation-oriented command-line interface.
- `src-tauri`: secure desktop boundary, credential-vault integration, diagnostics, and IPC commands.
- `src`: Vue desktop UI.
- `python`: optional compatibility adapters; not part of the default production render path.
- `docs`: user, engineering, qualification, and operations guidance.

The standalone KEYHAN STUDIO Core service provides shared authentication, account sessions, entitlements, product data, and cloud APIs. Its AudioMaster sync endpoint rejects audio-like fields and local file paths; mastering audio remains on the user's machine.

## Release policy

macOS and Windows are production release gates. Linux artifacts are currently beta and are allowed to fail independently in the release matrix. A release is not publishable until the checklist in [docs/dev/production-readiness.md](docs/dev/production-readiness.md) is complete, including qualification against approved loudness/true-peak vectors and platform signing/notarization.

## License

GPL-3.0-or-later.
