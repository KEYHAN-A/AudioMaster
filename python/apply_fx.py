#!/usr/bin/env python3
"""
DSP effects bridge script for the mastering CLI.
Applies EQ, compression, limiting, and stereo adjustments using pedalboard.
Receives a JSON argument with input, output, and mastering parameters.
Outputs a JSON result to stdout.
"""

import json
import sys
import os
import numpy as np


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "No arguments provided"}))
        sys.exit(1)

    try:
        request = json.loads(sys.argv[1])
    except json.JSONDecodeError as e:
        print(json.dumps({"error": f"Invalid JSON: {e}"}))
        sys.exit(1)

    input_path = request.get("input")
    output_path = request.get("output")
    params = request.get("params", {})
    bit_depth = request.get("bit_depth", 24)

    if not input_path or not output_path:
        print(json.dumps({"error": "Missing required fields: input, output"}))
        sys.exit(1)

    if not os.path.exists(input_path):
        print(json.dumps({"error": f"Input file not found: {input_path}"}))
        sys.exit(1)

    try:
        apply_effects(input_path, output_path, params, bit_depth)
        print(json.dumps({
            "output": output_path,
            "message": "DSP effects applied successfully",
        }))
    except ImportError as e:
        # Fallback to soundfile + numpy if pedalboard isn't available
        sys.stderr.write(f"[apply_fx] pedalboard not available ({e}), using numpy fallback\n")
        try:
            apply_effects_fallback(input_path, output_path, params, bit_depth)
            print(json.dumps({
                "output": output_path,
                "message": "DSP effects applied (numpy fallback)",
            }))
        except Exception as e2:
            print(json.dumps({"error": str(e2)}))
            sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)


def apply_effects(input_path, output_path, params, bit_depth):
    """Apply effects using the pedalboard library."""
    from pedalboard import (
        Pedalboard,
        Compressor,
        Gain,
        HighShelfFilter,
        LowShelfFilter,
        PeakFilter,
        Limiter,
    )
    from pedalboard.io import AudioFile

    board = Pedalboard()

    # EQ bands
    for band in params.get("eq", []):
        freq = band.get("frequency", 1000)
        gain = band.get("gain_db", 0)
        q = band.get("q", 0.707)
        band_type = band.get("band_type", "peak")

        if abs(gain) < 0.1:
            continue

        if band_type == "low_shelf":
            board.append(LowShelfFilter(cutoff_frequency_hz=freq, gain_db=gain, q=q))
        elif band_type == "high_shelf":
            board.append(HighShelfFilter(cutoff_frequency_hz=freq, gain_db=gain, q=q))
        elif band_type == "peak":
            board.append(PeakFilter(cutoff_frequency_hz=freq, gain_db=gain, q=q))

    # Compression
    comp = params.get("compression", {})
    if comp:
        board.append(Compressor(
            threshold_db=comp.get("threshold_db", -20),
            ratio=comp.get("ratio", 4),
            attack_ms=comp.get("attack_ms", 10),
            release_ms=comp.get("release_ms", 100),
        ))

    # Makeup gain from compression
    makeup = comp.get("makeup_gain_db", 0)
    if abs(makeup) > 0.1:
        board.append(Gain(gain_db=makeup))

    # Limiter
    limiter_params = params.get("limiter", {})
    if limiter_params.get("enabled", True):
        ceiling = limiter_params.get("ceiling_db", -1.0)
        release = limiter_params.get("release_ms", 50)
        board.append(Limiter(
            threshold_db=ceiling,
            release_ms=release,
        ))

    # Process
    with AudioFile(input_path) as f:
        sample_rate = f.samplerate
        audio = f.read(f.frames)

    # Apply stereo width adjustment
    stereo = params.get("stereo", {})
    width = stereo.get("width", 1.0)
    if audio.shape[0] == 2 and abs(width - 1.0) > 0.01:
        mid = (audio[0] + audio[1]) / 2.0
        side = (audio[0] - audio[1]) / 2.0
        side *= width
        audio[0] = mid + side
        audio[1] = mid - side

    # Apply the pedalboard chain
    processed = board(audio, sample_rate)

    # Loudness normalization toward target LUFS
    target_lufs = params.get("target_lufs", -14.0)
    current_rms = np.sqrt(np.mean(processed ** 2))
    if current_rms > 1e-10:
        current_db = 20 * np.log10(current_rms)
        # Approximate LUFS as RMS - 0.691
        current_lufs_approx = current_db - 0.691
        gain_needed = target_lufs - current_lufs_approx
        # Limit the gain adjustment to avoid extreme changes
        gain_needed = np.clip(gain_needed, -12.0, 12.0)
        gain_linear = 10 ** (gain_needed / 20.0)
        processed *= gain_linear

    # Write output
    subtype_map = {16: "PCM_16", 24: "PCM_24", 32: "FLOAT"}
    subtype = subtype_map.get(bit_depth, "PCM_24")

    import soundfile as sf
    sf.write(output_path, processed.T, sample_rate, subtype=subtype)


def apply_effects_fallback(input_path, output_path, params, bit_depth):
    """Minimal fallback using only numpy and soundfile."""
    import soundfile as sf

    audio, sample_rate = sf.read(input_path, always_2d=True)
    audio = audio.T  # channels x samples

    # Simple stereo width
    stereo = params.get("stereo", {})
    width = stereo.get("width", 1.0)
    if audio.shape[0] == 2 and abs(width - 1.0) > 0.01:
        mid = (audio[0] + audio[1]) / 2.0
        side = (audio[0] - audio[1]) / 2.0
        side *= width
        audio[0] = mid + side
        audio[1] = mid - side

    # Simple gain for loudness target
    target_lufs = params.get("target_lufs", -14.0)
    current_rms = np.sqrt(np.mean(audio ** 2))
    if current_rms > 1e-10:
        current_db = 20 * np.log10(current_rms)
        current_lufs_approx = current_db - 0.691
        gain_needed = target_lufs - current_lufs_approx
        gain_needed = np.clip(gain_needed, -12.0, 12.0)
        gain_linear = 10 ** (gain_needed / 20.0)
        audio *= gain_linear

    # Simple limiter
    limiter = params.get("limiter", {})
    if limiter.get("enabled", True):
        ceiling_db = limiter.get("ceiling_db", -1.0)
        ceiling_linear = 10 ** (ceiling_db / 20.0)
        peak = np.max(np.abs(audio))
        if peak > ceiling_linear:
            audio *= ceiling_linear / peak

    subtype_map = {16: "PCM_16", 24: "PCM_24", 32: "FLOAT"}
    subtype = subtype_map.get(bit_depth, "PCM_24")
    sf.write(output_path, audio.T, sample_rate, subtype=subtype)


if __name__ == "__main__":
    main()
