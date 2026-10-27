from __future__ import annotations

import json
from pathlib import Path

from echogrid_hum.validate_recordings import (
    generate_synthetic_ground_truth_sample,
    load_recording_with_sidecar,
    validate_single_recording,
)


def test_synthetic_sample_validation_passes(tmp_path: Path):
    wav_path, json_path = generate_synthetic_ground_truth_sample(tmp_path)
    errors = validate_single_recording(wav_path, json_path)
    assert errors == []

    audio, sr, meta = load_recording_with_sidecar(wav_path, json_path)
    assert sr == 48000
    assert len(audio) == 48000 * 10
    assert meta["metadata"]["source"] == "refrigerator"


def test_validation_catches_invalid_events(tmp_path: Path):
    wav_path, json_path = generate_synthetic_ground_truth_sample(tmp_path)

    # Mutate JSON with out-of-order events
    with json_path.open("r", encoding="utf-8") as f:
        meta = json.load(f)

    meta["events"] = [
        {"type": "ON", "timestamp_s": 5.0},
        {"type": "OFF", "timestamp_s": 2.0},  # backwards!
    ]

    with json_path.open("w", encoding="utf-8") as f:
        json.dump(meta, f)

    errors = validate_single_recording(wav_path, json_path)
    assert len(errors) > 0
    assert any("before preceding event" in e for e in errors)


def test_validation_catches_missing_key(tmp_path: Path):
    wav_path, json_path = generate_synthetic_ground_truth_sample(tmp_path)

    with json_path.open("r", encoding="utf-8") as f:
        meta = json.load(f)

    del meta["events"]

    with json_path.open("w", encoding="utf-8") as f:
        json.dump(meta, f)

    errors = validate_single_recording(wav_path, json_path)
    assert len(errors) > 0
    assert any("missing required top-level key: 'events'" in e for e in errors)
