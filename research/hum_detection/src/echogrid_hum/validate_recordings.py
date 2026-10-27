"""Recording loader and validation utilities for EchoGrid Phase 0."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import numpy as np
import soundfile as sf


def load_recording_with_sidecar(
    wav_path: Path,
    sidecar_path: Path | None = None,
) -> tuple[np.ndarray, int, dict[str, Any]]:
    """Load audio recording WAV and its accompanying JSON sidecar metadata."""
    if not wav_path.exists():
        raise FileNotFoundError(f"Audio file not found: {wav_path}")

    audio_data, sample_rate = sf.read(wav_path)

    if sidecar_path is None:
        sidecar_path = wav_path.with_suffix(".json")

    if not sidecar_path.exists():
        raise FileNotFoundError(f"Sidecar metadata not found: {sidecar_path}")

    with sidecar_path.open("r", encoding="utf-8") as f:
        metadata = json.load(f)

    return audio_data, sample_rate, metadata


def validate_single_recording(wav_path: Path, sidecar_path: Path) -> list[str]:
    """Validate audio and sidecar integrity. Returns list of error messages (empty if valid)."""
    errors: list[str] = []

    try:
        audio_data, sr, meta = load_recording_with_sidecar(wav_path, sidecar_path)
    except Exception as e:
        return [f"Failed to load pair ({wav_path.name}, {sidecar_path.name}): {e}"]

    # 1. Audio checks
    duration_s = len(audio_data) / sr
    if duration_s <= 0:
        errors.append("Audio file is empty.")

    # 2. Sidecar schema checks
    required_keys = ["sample_rate", "events", "metadata"]
    for k in required_keys:
        if k not in meta:
            errors.append(f"Sidecar missing required top-level key: '{k}'")

    if "sample_rate" in meta and meta["sample_rate"] != sr:
        errors.append(f"Sample rate mismatch: sidecar says {meta['sample_rate']}, WAV is {sr}")

    events = meta.get("events", [])
    if not isinstance(events, list):
        errors.append("Sidecar 'events' field must be a list.")
    else:
        last_t = -1.0
        for i, ev in enumerate(events):
            if "type" not in ev or "timestamp_s" not in ev:
                errors.append(f"Event #{i} missing 'type' or 'timestamp_s'")
                continue
            if ev["type"] not in ("ON", "OFF"):
                errors.append(f"Event #{i} has invalid type: '{ev['type']}'")
            t = float(ev["timestamp_s"])
            if t < last_t:
                errors.append(f"Event #{i} timestamp {t}s is before preceding event {last_t}s")
            if t > duration_s + 0.5:  # allow 0.5s slack
                errors.append(f"Event #{i} timestamp {t}s exceeds audio duration {duration_s:.2f}s")
            last_t = t

    return errors


def generate_synthetic_ground_truth_sample(
    out_dir: Path,
    base_name: str = "synthetic_fridge_sample",
) -> tuple[Path, Path]:
    """Generate a synthetic ground-truth audio sample and matching sidecar."""
    out_dir.mkdir(parents=True, exist_ok=True)
    wav_path = out_dir / f"{base_name}.wav"
    json_path = out_dir / f"{base_name}.json"

    sr = 48000
    duration_s = 10.0
    t = np.linspace(0, duration_s, int(sr * duration_s), endpoint=False)

    # 0s - 5s ON (50Hz + 100Hz hum), 5s - 10s OFF (mild noise)
    on_mask = t < 5.0
    sig = np.zeros_like(t)
    sig[on_mask] = 0.3 * np.sin(2 * np.pi * 50 * t[on_mask]) + 0.1 * np.sin(
        2 * np.pi * 100 * t[on_mask]
    )
    sig += 0.02 * np.random.default_rng(42).standard_normal(len(t))
    sig = (sig / np.max(np.abs(sig)) * 0.7).astype(np.float32)

    sf.write(wav_path, sig, sr, subtype="PCM_16")

    sidecar = {
        "audio_file": wav_path.name,
        "sample_rate": sr,
        "duration_s": duration_s,
        "settings": {
            "audioContextSampleRate": sr,
            "trackSettings": {
                "autoGainControl": False,
                "channelCount": 1,
                "echoCancellation": False,
                "noiseSuppression": False,
                "sampleRate": sr,
            },
        },
        "metadata": {"source": "refrigerator", "distance_m": 0.5},
        "events": [
            {"type": "ON", "timestamp_s": 0.0},
            {"type": "OFF", "timestamp_s": 5.0},
        ],
    }

    with json_path.open("w", encoding="utf-8") as f:
        json.dump(sidecar, f, indent=2)

    return wav_path, json_path


def validate_all_in_dir(target_dir: Path) -> bool:
    """Validate all .wav / .json pairs in the given directory."""
    if not target_dir.exists():
        print(f"Directory not found: {target_dir}")
        return True

    wav_files = list(target_dir.glob("*.wav"))
    if not wav_files:
        print(f"No .wav files found in {target_dir}")
        return True

    all_passed = True
    for wav_file in wav_files:
        json_file = wav_file.with_suffix(".json")
        errors = validate_single_recording(wav_file, json_file)
        if errors:
            all_passed = False
            print(f"[FAIL] {wav_file.name}:")
            for err in errors:
                print(f"  - {err}")
        else:
            print(f"[PASS] {wav_file.name} with sidecar {json_file.name}")

    return all_passed


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate acoustic recordings and sidecar schemas."
    )
    parser.add_argument(
        "--dir",
        type=Path,
        default=Path("data/own"),
        help="Directory containing .wav and .json recording pairs to validate.",
    )
    parser.add_argument(
        "--test-synthetic",
        action="store_true",
        help="Generate and validate a temporary synthetic sample pair.",
    )
    args = parser.parse_args()

    if args.test_synthetic:
        temp_dir = Path("data/public/samples/synthetic_test")
        wav, js = generate_synthetic_ground_truth_sample(temp_dir)
        errors = validate_single_recording(wav, js)
        if errors:
            print("[FAIL] Synthetic validation failed:")
            for e in errors:
                print(f"  - {e}")
            return 1
        print("[PASS] Synthetic validation passed successfully.")
        return 0

    success = validate_all_in_dir(args.dir)
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
