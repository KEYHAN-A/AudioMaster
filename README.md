# AudioMaster

AI-powered music mastering tool built with Rust, Tauri, and Python.

## Overview

AudioMaster is a comprehensive music mastering application that combines Rust's performance for audio processing with Python's machine learning ecosystem. It features a Tauri-based desktop application, a CLI tool, and a web frontend for AI-driven audio mastering workflows.

## Features

- **AI-Powered Mastering** — Neural network-based audio mastering and enhancement
- **Rust Core** — High-performance audio processing with `mastering-core` library
- **CLI Tool** — Command-line interface for batch and scriptable mastering
- **Desktop App** — Tauri-based cross-platform desktop application
- **Python ML Pipeline** — Machine learning inference and FX processing in Python
- **Web Frontend** — Vite-powered web interface for mastering control
- **Documentation** — User guides, developer docs, and runbooks
- **Website** — Project landing page with screenshots and information

## Tech Stack

| Category | Technology |
|----------|-----------|
| Core (Rust) | Rust 2021, serde, tokio, symphonia, hound |
| CLI | clap (derive), tracing, indicatif |
| Desktop App | Tauri (^2.x) |
| Web Frontend | Vue 3, Vite |
| Python Backend | PyTorch, torchaudio, numpy |
| Build | Cargo workspace, Vite |
| License | GPL-3.0-or-later |
| Version | 1.2.0 |

## Project Structure

```
AudioMaster/
├── Cargo.toml                     # Rust workspace definition
├── Cargo.lock                     # Dependency lock file
├── package.json                   # Node dependencies (Vite/frontend)
├── vite.config.js                 # Vite configuration
├── index.html                     # Web frontend entry
├── LICENSE                        # GPL-3.0 license
├── README.md                      # This file
├── CONTRIBUTING.md                # Contribution guidelines
├── crates/                        # Rust workspace crates
│   ├── mastering-core/            # Core audio processing library
│   │   └── Cargo.toml
│   │   └── src/                   # Library source
│   └── mastering-cli/             # CLI binary
│       └── Cargo.toml
│       └── src/main.rs
├── src-tauri/                     # Tauri desktop app
│   ├── Cargo.toml                 # Tauri Rust deps
│   ├── tauri.conf.json            # Tauri configuration
│   ├── build.rs                   # Build script
│   ├── capabilities/              # Tauri capabilities
│   ├── gen/                       # Generated Tauri files
│   └── icons/                     # App icons
├── python/                        # Python ML pipeline
│   ├── requirements.txt           # Python dependencies
│   ├── matchering_bridge.py       # Matchering integration
│   ├── ml_inference.py            # ML inference engine
│   └── apply_fx.py                # Audio FX application
├── docs/                          # Documentation
│   ├── runbooks/                  # Operational runbooks
│   ├── user/                      # User documentation
│   └── dev/                       # Developer documentation
├── website/                       # Project website
│   ├── index.html                 # Landing page
│   ├── style.css                  # Website styles
│   ├── icon.png / icon.svg        # Branding assets
│   ├── sitemap.xml                # SEO sitemap
│   └── robots.txt                 # SEO robots
├── screenshot.png                 # Application screenshot
├── dist/                          # Build output
└── .github/                       # GitHub workflows
    └── workflows/                 # CI/CD pipelines
```

## Installation

### Prerequisites

- **Rust** — Latest stable toolchain (`rustup`)
- **Python 3.10+** — For ML inference pipeline
- **Node.js 18+** — For web frontend
- **Cargo** — Rust package manager

### Rust Core

```bash
# Clone and build the Rust workspace
cd AudioMaster
cargo build --release

# Run the CLI tool
cargo run -p mastering-cli -- --help
```

### Python ML Pipeline

```bash
cd python
pip install -r requirements.txt
```

### Desktop App (Tauri)

```bash
# Install Tauri CLI
cargo install tauri-cli --version "^2"

# Build the desktop app
cd src-tauri
cargo tauri dev      # Development
cargo tauri build    # Production build
```

### Web Frontend

```bash
npm install
npm run dev    # Start on port 1421
npm run build  # Production build
```

## Usage

### CLI

```bash
# Master an audio file
cargo run -p mastering-cli -- input.wav -o output.wav

# Dry run (preview settings)
cargo run -p mastering-cli -- input.wav --dry-run

# Apply custom preset
cargo run -p mastering-cli -- input.wav --preset pop
```

### Desktop App

Launch the Tauri desktop application for a GUI-based mastering experience with real-time preview.

### Python API

```python
from python.ml_inference import MLModel
from python.apply_fx import apply_effects

# Load model and process audio
model = MLModel.load("path/to/model")
result = model.infer(audio_data)
processed = apply_fx(result, fx_params)
```

## Development

### Building

```bash
# Build all workspace crates
cargo build

# Run tests
cargo test

# Run with coverage
cargo llvm-cov

# Build frontend
npm run build
```

### Contributing

See `CONTRIBUTING.md` for contribution guidelines.

### Documentation

- `docs/user/` — User documentation
- `docs/dev/` — Developer documentation
- `docs/runbooks/` — Operational runbooks

## License

GPL-3.0-or-later
