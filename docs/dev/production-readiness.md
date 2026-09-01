# Production readiness

This is the release contract. A checked item must have reproducible evidence attached to the release candidate.

## Implemented foundation

- [x] Native deterministic DSP is the default non-reference engine.
- [x] AI output is schema-parsed, bounded, and rendered only by native DSP.
- [x] Delivered files are re-analyzed before publication; limiter ceiling violations fail the job.
- [x] WAV and real FFmpeg MP3 paths have end-to-end regression coverage.
- [x] Album output is staged and published as a complete set.
- [x] Analysis caching validates source size and modification time.
- [x] KeyhanStudio uses shared device authentication, rotating refresh tokens, and the operating-system credential vault.
- [x] Cloud schemas contain preferences/presets only and reject audio/path fields server-side.
- [x] Desktop CSP is enabled; formatting, tests, Clippy, frontend build, Python syntax, and dependency audits are CI gates.
- [x] AIFF and optional AAC delivery preserve source metadata through FFmpeg; archive defaults remain WAV/FLAC/AIFF.
- [x] AI advisor output uses a rejectable versioned `MasteringPlan` envelope.
- [x] Desktop jobs expose cooperative cancellation and stage/frame progress; unpublished candidates remain temporary.
- [x] Long-form sources use bounded-prefix secondary analysis, full-duration streaming EBU measurements, and a bounded-memory render graph.
- [x] Native reference targets, low-end mono protection, dynamic sibilance control, and four-times oversampled look-ahead limiting are built in.
- [x] Level-matched 30-second A/B previews, overwrite confirmation, album reports, and SHA-256 delivery checksums are available in desktop flows.
- [x] All remote-provider keys and cloud refresh tokens are write-only OS-vault credentials in the desktop app.
- [x] Meter qualification manifests, corpus/listening templates, performance budgets, privacy policy, cloud/release runbooks, and SBOM generation are source-controlled gates.

## Required before public production release

- [ ] Qualify integrated loudness, LRA, and true peak against approved EBU/ITU test vectors. Record vector hashes, expected/actual results, and tolerances.
- [ ] Run a curated corpus through null tests and listening review with at least two mastering engineers, including mono, anti-correlated, clipped, DC-offset, sparse, bass-heavy, bright, and high-crest material.
- [ ] Attach the two-hour, 192 kHz/32-bit streaming benchmark evidence from every release reference machine.
- [ ] Verify cloud conflict, token rotation/reuse, offline, and rate-limit behavior against staging.
- [ ] Complete macOS signing/notarization and Windows code-signing smoke tests, updater signatures, and rollback.
- [ ] Execute accessibility, keyboard, screen-reader, suspend/resume, low-disk, Unicode-path, and network-loss tests.
- [ ] Publish performance budgets and benchmark results for 44.1/48/96/192 kHz sources.
- [ ] Complete privacy policy, telemetry consent, retention/deletion policy, SBOM, third-party notices, and support ownership.

Linux remains beta until desktop integration, secret-service behavior, and packaging pass the same suite.
