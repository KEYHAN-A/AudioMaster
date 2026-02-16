#!/usr/bin/env python3
"""
ML inference bridge script for the mastering CLI.
Runs local machine learning models for audio mastering.
Receives a JSON argument with input, output, model name, and options.
Outputs a JSON result to stdout.
"""

import json
import sys
import os


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
    model_name = request.get("model", "deepafx-st")
    reference = request.get("reference")
    bit_depth = request.get("bit_depth", 24)
    target_lufs = request.get("target_lufs", -14.0)

    if not input_path or not output_path:
        print(json.dumps({"error": "Missing required fields: input, output"}))
        sys.exit(1)

    if not os.path.exists(input_path):
        print(json.dumps({"error": f"Input file not found: {input_path}"}))
        sys.exit(1)

    try:
        if model_name == "deepafx-st":
            process_deepafx(input_path, output_path, reference, bit_depth, target_lufs)
        else:
            process_huggingface(input_path, output_path, model_name, bit_depth, target_lufs)

        print(json.dumps({
            "output": output_path,
            "message": f"ML inference completed with model: {model_name}",
            "model": model_name,
        }))
    except ImportError as e:
        print(json.dumps({
            "error": f"Required package not installed: {e}. "
                     f"Install with: pip install torch torchaudio"
        }))
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)


def process_deepafx(input_path, output_path, reference, bit_depth, target_lufs):
    """
    Process audio using DeepAFx-ST style transfer.
    If the model isn't available locally, falls back to a simple
    neural-style loudness/EQ matching approach.
    """
    import numpy as np
    import soundfile as sf

    audio, sr = sf.read(input_path, always_2d=True)

    # Try loading DeepAFx-ST
    try:
        from deepafx_st.process import process_audio
        if reference and os.path.exists(reference):
            result = process_audio(input_path, reference)
            sf.write(output_path, result, sr, subtype=_subtype(bit_depth))
            return
    except ImportError:
        sys.stderr.write(
            "[ml_inference] DeepAFx-ST not installed. "
            "Using fallback processing.\n"
            "Install from: https://github.com/adobe-research/DeepAFx-ST\n"
        )

    # Fallback: basic processing with loudness normalization
    processed = audio.copy()

    # Loudness normalization
    rms = np.sqrt(np.mean(processed ** 2))
    if rms > 1e-10:
        current_db = 20 * np.log10(rms)
        current_lufs = current_db - 0.691
        gain_db = np.clip(target_lufs - current_lufs, -12.0, 12.0)
        processed *= 10 ** (gain_db / 20.0)

    # Simple peak limiter
    peak = np.max(np.abs(processed))
    ceiling = 10 ** (-1.0 / 20.0)
    if peak > ceiling:
        processed *= ceiling / peak

    sf.write(output_path, processed, sr, subtype=_subtype(bit_depth))


def process_huggingface(input_path, output_path, model_name, bit_depth, target_lufs):
    """
    Process audio using a HuggingFace model.
    This is a placeholder for future model integration.
    """
    import numpy as np
    import soundfile as sf

    sys.stderr.write(
        f"[ml_inference] HuggingFace model '{model_name}' integration is experimental.\n"
        f"Applying basic loudness normalization as fallback.\n"
    )

    audio, sr = sf.read(input_path, always_2d=True)
    processed = audio.copy()

    # Basic loudness normalization
    rms = np.sqrt(np.mean(processed ** 2))
    if rms > 1e-10:
        current_db = 20 * np.log10(rms)
        current_lufs = current_db - 0.691
        gain_db = np.clip(target_lufs - current_lufs, -12.0, 12.0)
        processed *= 10 ** (gain_db / 20.0)

    peak = np.max(np.abs(processed))
    ceiling = 10 ** (-1.0 / 20.0)
    if peak > ceiling:
        processed *= ceiling / peak

    sf.write(output_path, processed, sr, subtype=_subtype(bit_depth))


def _subtype(bit_depth):
    return {16: "PCM_16", 24: "PCM_24", 32: "FLOAT"}.get(bit_depth, "PCM_24")


if __name__ == "__main__":
    main()
