from __future__ import annotations

import json
from pathlib import Path

from echogrid_hum.detector import DetectorParams, StreamingPowerDownDetector


def test_replay_golden_vectors():
    """CI Replay Test: strictly asserts every frame in golden_vectors.json
    against expected detector state and trigger flags.
    """
    pkg_dir = Path(__file__).resolve().parent.parent
    vectors_file = pkg_dir / "golden_vectors.json"
    params_file = pkg_dir / "detector_params.json"

    assert vectors_file.exists(), f"Golden vectors file missing: {vectors_file}"
    assert params_file.exists(), f"Detector params file missing: {params_file}"

    with vectors_file.open("r", encoding="utf-8") as f:
        data = json.load(f)

    with params_file.open("r", encoding="utf-8") as f:
        params_json = f.read()

    params = DetectorParams.from_json(params_json)
    scenarios = data["scenarios"]

    assert len(scenarios) >= 3

    for scen in scenarios:
        name = scen["scenario_name"]
        frames = scen["frames"]
        detector = StreamingPowerDownDetector(params)

        for i, frame_spec in enumerate(frames):
            energy_db = float(frame_spec["energy_db"])
            res = detector.process_energy_frame(energy_db)

            assert res.state.value == frame_spec["expected_state"], (
                f"Scenario '{name}' frame {i}: state mismatch: "
                f"expected {frame_spec['expected_state']}, got {res.state.value}"
            )
            assert res.event_triggered == frame_spec["expected_triggered"], (
                f"Scenario '{name}' frame {i}: event_triggered mismatch: "
                f"expected {frame_spec['expected_triggered']}, got {res.event_triggered}"
            )
