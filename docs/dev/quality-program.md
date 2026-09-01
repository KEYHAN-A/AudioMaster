# Audio quality and performance program

## Release performance budgets

Measure on the oldest supported Intel Mac, Apple Silicon Mac, and supported Windows reference machine with warm and cold caches. Report CPU model, RAM, OS, source hash, sample rate, channels, bit depth, duration, wall time, peak RSS, and output hash.

| Source | Analysis budget | Native render budget | Peak RSS budget |
|---|---:|---:|---:|
| 44.1/48 kHz stereo | 0.5× real time | 1.0× real time | 512 MiB |
| 96 kHz stereo | 0.8× real time | 1.5× real time | 640 MiB |
| 192 kHz stereo | 1.5× real time | 3.0× real time | 768 MiB |

Two-hour 192 kHz/32-bit stereo is the mandatory bounded-streaming soak case. Cancellation must remove unpublished temporary output, and free disk must never fall below the preflight reserve.

## Curated corpus

The release corpus must include mono, anti-correlated stereo, clipped, DC-offset, digital silence, sparse/acoustic, bass-heavy, bright/sibilant, high-crest, low-crest, Unicode filenames, malformed/truncated containers, and 44.1/48/96/192 kHz material. Store licensed audio outside Git and commit only hashes and provenance.

For every source, archive pre/post analysis, mastering plan schema, engine version, warnings, SHA-256, elapsed time, peak RSS, and a level-matched A/B render. A null test is required for bypass/unity plans and deterministic reruns.

## Listening approval

At least two mastering engineers independently review level-matched A/B files on calibrated monitors and headphones. Score tonal balance, transient integrity, distortion/pumping, sibilance, low-end translation, image/mono compatibility, and preference from 1–5. Any score below 3 or disagreement of two points requires adjudication. Neither engineer may see which render is the candidate before scoring.

## Desktop failure matrix

Before promotion, exercise keyboard-only and screen-reader navigation, 200% scaling, reduced motion, suspend/resume during analysis and render, cancellation at every stage, low disk, read-only output, overwrite confirmation, Unicode and very long paths, unplugged network, HTTP 429, expired credentials, cloud conflict, and interrupted update/rollback.
