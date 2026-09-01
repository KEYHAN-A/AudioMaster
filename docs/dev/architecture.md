# AudioMaster — Developer Guide

## Architecture Overview

AudioMaster is a local-first Rust mastering engine with a Vue 3 + Tauri v2 frontend. Python is retained only for optional compatibility backends.

```
┌─────────────────────────────┐
│  Vue 3 Frontend (Vite)      │
│  src/components/*.vue        │
│  src/composables/*.js        │
└──────────────┬──────────────┘
               │ Tauri IPC (invoke)
┌──────────────▼──────────────┐
│  Tauri v2 Backend (Rust)    │
│  src-tauri/src/commands.rs   │
│  src-tauri/src/telemetry.rs  │
└──────────────┬──────────────┘
               │ uses
┌──────────────▼──────────────┐
│  mastering-core (Rust lib)  │
│  crates/mastering-core/     │
│  ├── analysis/              │  Audio analysis (LUFS, RMS, Peak, etc.)
│  ├── dsp.rs                 │  Authoritative deterministic DSP
│  ├── backends/              │  Native, advisory AI, Matchering, ML
│  ├── pipeline/              │  Orchestration layer
│  ├── album.rs               │  Transactional album continuity
│  ├── cloud.rs               │  Audio-free KeyhanStudio protocol
│  ├── cache.rs               │  Result caching
│  ├── config.rs              │  Configuration management
│  ├── error.rs               │  Centralized error types
│  └── types.rs               │  Shared data types
└──────────────┬──────────────┘
               │ optional subprocess
┌──────────────▼──────────────┐
│  Python Compatibility       │
│  python/matchering_bridge.py │  Matchering reference matching
│  python/ml_inference.py      │  Local ML inference
└─────────────────────────────┘
```

## Development Setup

### Prerequisites
- Rust (stable) — `rustup`
- Node.js 18+ — `brew install node`
- Python 3.8+ — for Matchering and effects backends
- ffmpeg — `brew install ffmpeg`

### Build
```bash
npm install
pip install -r python/requirements.txt
npx tauri dev
```

### Test
```bash
# All tests
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# Core only
cargo test -p mastering-core

# With coverage
cargo llvm-cov --all-features --workspace
```

### CLI Only
```bash
cargo build --release -p mastering-cli
./target/release/mastering --help
```

## Code Style

### Rust
- Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- Use `anyhow::Result` for application code, custom errors for library boundaries
- No `unwrap()` in production code — use `?` or `context()`
- Prefer `tracing` over `println!`

### Vue/JavaScript
- Vue 3 Composition API with `<script setup>`
- Use composables for shared state (`src/composables/`)
- No TypeScript (current decision — may revisit)

### Python
- Python 3.8+ compatibility
- Use type hints
- Handle errors gracefully (the Rust side will catch subprocess failures)

## Adding a New Backend

The native backend is the production fallback and must remain available without Python or network access.

1. Create a new module in `crates/mastering-core/src/backends/`
2. Implement the backend struct with `new()`, `process()`, and `check_available()` methods
3. Add the variant to `MasteringEngine` enum in `backends/mod.rs`
4. Add dispatch in `MasteringEngine::process()`, `name()`, `check_available()`
5. Add to `Backend` enum in `types.rs`
6. Update `commands.rs` backend listing
7. Render to the requested temporary WAV path; format conversion and publication belong to the pipeline
8. Add delivered-file and failure-cleanup tests

## Error Handling

Use the centralized `MasteringError` enum for all library errors:
```rust
use crate::error::MasteringError;

// Create specific errors
MasteringError::audio_decode_failed("song.wav", "Unsupported codec")
MasteringError::network_timeout("Connection failed", true)  // can_retry
MasteringError::python_unavailable("Python not found in PATH")
```

The frontend receives structured `ErrorResponse` with:
- `message`: User-friendly error message
- `code`: Error category for programmatic handling
- `can_retry`: Whether retry makes sense
- `can_fallback`: Whether to offer alternative backend
- `suggested_action`: Human-readable suggestion

## Release Process

1. Update version in `Cargo.toml` workspace and `package.json`
2. Complete `docs/dev/production-readiness.md` with release evidence
3. Create a git tag: `git tag v1.x.0`
4. Push tag: `git push origin v1.x.0`
5. GitHub Actions builds release artifacts
6. Review, sign, notarize, and smoke-test the draft artifacts
7. Publish the release

## Monitoring

- **Error Tracking**: Sentry (configure via `SENTRY_DSN` env var)
- **Logging**: Structured JSON logs in `~/Library/Logs/AudioMaster/` (macOS)
- **Privacy**: Audio and local paths are excluded from cloud sync and remote telemetry
