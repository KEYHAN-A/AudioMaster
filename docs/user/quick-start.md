# AudioMaster — Quick Start Guide

## Installation

### macOS
1. Download the latest `.dmg` from [Releases](https://github.com/KEYHAN-A/AudioMaster/releases)
2. Open the DMG and drag AudioMaster to Applications
3. Launch AudioMaster

### CLI Installation
```bash
# Download the latest binary
curl -L https://github.com/KEYHAN-A/AudioMaster/releases/latest/download/mastering-cli-macos-arm64 -o mastering
chmod +x mastering
sudo mv mastering /usr/local/bin/
```

## First Steps

### 1. Import Audio
- Drag and drop audio files into the app, or
- Press `Cmd+O` to open a file picker
- Supported formats: WAV, FLAC, MP3, OGG, M4A

### 2. Analyze
- Click **Analyze All** or press `Cmd+R`
- View real-time waveforms, LUFS meters, and spectrum analysis
- Check metrics: LUFS, RMS, Peak, Dynamic Range, Stereo Width

### 3. Master
- Click **Master All** or press `Cmd+M`
- Select a preset: Streaming (-14 LUFS), CD (-9), Vinyl (-12), Loud (-6)
- Choose a backend: Auto, Matchering, AI, or Local ML
- Review before/after comparison

### 4. Export
- Mastered files are saved alongside originals with `_mastered` suffix
- Choose output format: WAV, FLAC, or MP3
- Select bit depth: 16, 24, or 32-bit

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+O` | Open files |
| `Cmd+R` | Analyze all tracks |
| `Cmd+M` | Master all tracks |
| `Escape` | Close dialog |

## Configuration

Configuration is stored at `~/.config/mastering/config.toml`.

```toml
[general]
default_backend = "auto"
target_lufs = -14.0
default_format = "wav"
default_bit_depth = 24

[ai]
default_provider = "ollama"

[ai.ollama]
endpoint = "http://localhost:11434"
model = "llama3"
```

## Backends

### Auto (Default)
Automatically selects the best available backend. Uses Matchering when a reference track is provided, otherwise AI.

### Matchering
Reference-based mastering. Requires:
- Python 3.8+ with `matchering` package
- A reference track for matching

### AI
LLM-assisted mastering. Supports:
- **Ollama** (local, free) — Install from [ollama.com](https://ollama.com)
- **OpenAI** — Requires API key
- **Anthropic** — Requires API key
- **KeyhanStudio** — Central AI gateway

### Local ML
Local machine learning inference. Experimental.

## Troubleshooting

### Python not found
```
Error: Python environment unavailable
```
Install Python 3.8+ and ensure it's in your PATH:
```bash
python3 --version
pip install -r python/requirements.txt
```

### Backend unavailable
Run diagnostics from Settings > Diagnostics to check backend availability.

### FFmpeg not found
FFmpeg is required for format conversion (FLAC, MP3):
```bash
brew install ffmpeg
```
