# AudioMaster — Developer Guide

## Architecture Overview

AudioMaster is a hybrid Rust/Python application with a Vue 3 + Tauri v2 frontend.

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
│  ├── backends/              │  Processing backends (AI, Matchering, ML)
│  ├── pipeline/              │  Orchestration layer
│  ├── cache.rs               │  Result caching
│  ├── config.rs              │  Configuration management
│  ├── error.rs               │  Centralized error types
│  └── types.rs               │  Shared data types
└──────────────┬──────────────┘
               │ subprocess
┌──────────────▼──────────────┐
│  Python DSP Layer           │
│  python/apply_fx.py          │  Pedalboard-based effects
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

1. Create a new module in `crates/mastering-core/src/backends/`
2. Implement the backend struct with `new()`, `process()`, and `check_available()` methods
3. Add the variant to `MasteringEngine` enum in `backends/mod.rs`
4. Add dispatch in `MasteringEngine::process()`, `name()`, `check_available()`
5. Add to `Backend` enum in `types.rs`
6. Update `commands.rs` backend listing
7. Add tests in the new module

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
2. Create a git tag: `git tag v1.x.0`
3. Push tag: `git push origin v1.x.0`
4. GitHub Actions builds release artifacts
5. Review draft release on GitHub
6. Publish release

## Monitoring

- **Error Tracking**: Sentry (configure via `SENTRY_DSN` env var)
- **Logging**: Structured JSON logs in `~/Library/Logs/AudioMaster/` (macOS)
- **Analytics**: Local-first, opt-in via settings
