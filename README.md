# AudioMaster

**AudioMaster** is a powerful, open-source AI-powered music mastering application. It is built for musicians, producers, and audio engineers who want professional-quality masters using intelligent audio analysis, multiple mastering backends, and a beautiful desktop interface.

![AudioMaster Screenshot](https://imgur.com/placeholder.png)

## Features

- **AI-Powered Mastering**: Leverage AI backends (Ollama, OpenAI, Anthropic, KeyhanStudio) to generate optimal mastering parameters from audio analysis.
- **Reference-Based Mastering**: Use [Matchering](https://github.com/sergree/matchering) to match your track's loudness, EQ, and dynamics to a reference track.
- **Real-Time Visualizations**: Waveform display, LUFS loudness meters, and 7-band spectrum analyzer with before/after comparison.
- **Batch Processing**: Import multiple tracks and master them all in one go with Analyze All / Master All.
- **Configurable Presets**: Streaming, CD, Vinyl, and Loud presets with customizable target LUFS, ceiling, and stereo width.
- **Native Audio Analysis**: Integrated LUFS, RMS, Peak, True Peak, Dynamic Range, Stereo Width, and Frequency Band analysis — all in Rust.
- **CLI Tool**: Full-featured command-line interface (`mastering-cli`) for scriptable mastering workflows.
- **Cross-Platform Desktop App**: Built with [Tauri v2](https://tauri.app/) for a lightweight, native experience.
- **Fast Performance**: Rust core library with zero-copy audio decoding via [Symphonia](https://github.com/pdeljanov/Symphonia).

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) 1.70+
- [Node.js](https://nodejs.org/) 18+ and npm
- [Python](https://www.python.org/) 3.8+ (for Matchering and effects backends)
- [FFmpeg](https://ffmpeg.org/) (must be installed and in your system PATH)

### Dependencies

Install the Python requirements for the mastering backends:
```bash
pip install -r python/requirements.txt
```

### Build from Source

```bash
# Clone the repository
git clone https://github.com/KEYHAN-A/AudioMaster.git
cd AudioMaster

# Install frontend dependencies
npm install

# Run in development mode
npx tauri dev

# Build a release
npx tauri build
```

### CLI Only

If you only need the command-line tool:
```bash
cargo build --release -p mastering-cli
./target/release/mastering --help
```

## Usage

1. **Launch** the application (or run `npx tauri dev` for development).
2. **Import Audio**: Click "Import Audio Files" or drag & drop your tracks into the window.
3. **Analyze**: Click "Analyze All" to run loudness and spectral analysis on every track.
4. **Review**: Inspect the waveform, LUFS meters, and spectrum analyzer for each track.
5. **Configure**: Open Settings to choose your mastering backend (Matchering, AI, Local ML) and preset.
6. **Master**: Click "Master All" to process all tracks. Results appear as before/after comparisons.
7. **Export**: Mastered files are saved alongside the originals with a `_mastered` suffix.

### CLI Usage

```bash
# Analyze a file
mastering analyze input.wav

# Master with default settings
mastering master input.wav -o output.wav

# Master with a reference track
mastering master input.wav -o output.wav --reference ref.wav

# Use a specific backend
mastering master input.wav -o output.wav --backend matchering
```

### Configuration

Copy the example config and edit to taste:
```bash
cp config.toml.example ~/.config/mastering/config.toml
```

## License

This project is licensed under the **GNU General Public License v3.0** — a strong copyleft license that ensures this software and all derivatives remain free and open-source.

See the [LICENSE](LICENSE) file for details.

## Author

Created by **KEYHAN-A** — [audiomaster.keyhan.info](https://audiomaster.keyhan.info)
