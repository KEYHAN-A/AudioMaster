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
- Supported inputs: WAV, AIFF, FLAC, MP3, OGG, and M4A/AAC

### 2. Analyze
- Click **Analyze All** or press `Cmd+R`
- View real-time waveforms, LUFS meters, and spectrum analysis
- Check metrics: LUFS, RMS, Peak, Dynamic Range, Stereo Width

### 3. Master
- Click **Master All** or press `Cmd+M`
- Select a preset: Streaming (-14 LUFS), CD (-9), Vinyl (-12), Loud (-6)
- Choose a backend: Auto, Native, Matchering, AI, or Local ML
- With multiple songs, keep **Album continuity mode** enabled and choose a new output directory
- Render the 30-second level-matched A/B preview before committing a delivery

### 4. Export
- Mastered files are saved alongside originals with `_mastered` suffix
- Choose archive output WAV, AIFF, or FLAC; MP3 and AAC are distribution copies
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
Without a reference, Auto uses the built-in Native engine. With a reference, it tries Matchering and safely falls back to Native.

### Native
The production default. Analysis, EQ, compression, stereo control, loudness targeting, limiting, dither, and delivery verification run locally in Rust with no Python or network dependency.

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

Desktop API keys are write-only and stored in Keychain, Credential Manager, or the platform secret service. They are not saved in `config.toml` or returned to the webview after saving.

### Local ML
Local machine learning inference. Experimental.

## KeyhanStudio Cloud

Open **Settings > Account** to sign in using the browser. Refresh tokens are stored in the operating-system credential vault. Settings and presets can be synchronized with revision conflict protection; audio and local file paths are never uploaded. Early-access enrollment and feedback are available in the same panel.

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
FFmpeg and FFprobe are required for metadata-preserving delivery, previews, codecs, and bounded long-form processing:
```bash
brew install ffmpeg
```
