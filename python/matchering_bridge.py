#!/usr/bin/env python3
"""
Matchering bridge script for the mastering CLI.
Receives a JSON argument with target, reference, output, and options.
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

    target = request.get("target")
    reference = request.get("reference")
    output = request.get("output")
    bit_depth = request.get("bit_depth", 24)
    no_limiter = request.get("no_limiter", False)

    if not target or not reference or not output:
        print(json.dumps({"error": "Missing required fields: target, reference, output"}))
        sys.exit(1)

    if not os.path.exists(target):
        print(json.dumps({"error": f"Target file not found: {target}"}))
        sys.exit(1)

    if not os.path.exists(reference):
        print(json.dumps({"error": f"Reference file not found: {reference}"}))
        sys.exit(1)

    try:
        import matchering as mg

        mg.log(lambda msg: sys.stderr.write(f"[matchering] {msg}\n"))

        results = []
        if bit_depth == 16:
            results.append(mg.pcm16(output))
        elif bit_depth == 32:
            results.append(mg.pcm32(output))
        else:
            results.append(mg.pcm24(output))

        config = mg.Config() if not no_limiter else mg.Config(
            limiter=False,
            limiter_attack=0,
            limiter_release=0,
        )

        # matchering.Config doesn't take limiter params that way, use defaults
        if no_limiter:
            mg.process(
                target=target,
                reference=reference,
                results=results,
                config=mg.Config(
                    threshold=0.9999,
                ),
            )
        else:
            mg.process(
                target=target,
                reference=reference,
                results=results,
            )

        print(json.dumps({
            "output": output,
            "message": f"Matchering completed: matched to reference ({os.path.basename(reference)})",
            "bit_depth": bit_depth,
        }))

    except ImportError:
        print(json.dumps({
            "error": "matchering package not installed. Run: pip install matchering"
        }))
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)


if __name__ == "__main__":
    main()
