# Contributing to AudioMaster

Thank you for considering contributing to AudioMaster! This document provides guidelines for contributing to this project.

## Getting Started

1. Fork the repository on GitHub.
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/AudioMaster.git
   cd AudioMaster
   ```
3. Install dependencies:
   ```bash
   npm install
   pip install -r python/requirements.txt
   ```
4. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development

Run the app in development mode:
```bash
npx tauri dev
```

Run the Rust tests:
```bash
cargo test --workspace
```

## Making Changes

- Keep commits focused and atomic.
- Write clear commit messages describing the "why" not just the "what".
- Follow the existing code style (Rust: `cargo fmt`, JS/Vue: consistent with existing files).
- Add tests for new functionality when possible.

## Submitting a Pull Request

1. Push your branch to your fork.
2. Open a Pull Request against the `main` branch.
3. Describe your changes and link any related issues.
4. Ensure CI checks pass.

## Reporting Issues

- Use GitHub Issues to report bugs or request features.
- Include steps to reproduce, expected vs. actual behavior, and your environment (OS, Rust version, Python version).

## License

By contributing, you agree that your contributions will be licensed under the GPL-3.0 license.
