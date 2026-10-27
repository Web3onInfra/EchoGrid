"""Golden vector generation and verification for EchoGrid streaming detector.

Establishes the cross-platform specification contract for Web Audio (JS) and Rust ports.
"""

from __future__ import annotations

import json
from pathlib import Path

from echogrid_hum.detector import DetectorParams, StreamingPowerDownDetector


def generate_golden_vectors(params: DetectorParams) -> dict:
    """Generate deterministic frame-by-frame test scenarios and expected outputs."""
    scenarios = []

    # Scenario 1: Clean power-down step
    det1 = StreamingPowerDownDetector(params)
    frames1 = []
    # 20 ON frames (50 dB)
    for _ in range(20):
        res = det1.process_energy_frame(50.0)
        frames1.append(
            {
                "energy_db": res.current_energy_db,
                "expected_state": res.state.value,
                "expected_triggered": res.event_triggered,
            }
        )
    # 105 OFF frames (30 dB, -20 dB drop for ~3.36s > 3.0s min_sustained_drop)
    for _ in range(105):
        res = det1.process_energy_frame(30.0)
        frames1.append(
            {
                "energy_db": res.current_energy_db,
                "expected_state": res.state.value,
                "expected_triggered": res.event_triggered,
            }
        )
    scenarios.append({"scenario_name": "clean_power_down", "frames": frames1})

    # Scenario 2: Jitter rejection (amplitude jitter +-1.5 dB)
    det2 = StreamingPowerDownDetector(params)
    frames2 = []
    for i in range(50):
        val = 50.0 + (1.5 if i % 2 == 0 else -1.5)
        res = det2.process_energy_frame(val)
        frames2.append(
            {
                "energy_db": res.current_energy_db,
                "expected_state": res.state.value,
                "expected_triggered": res.event_triggered,
            }
        )
    scenarios.append({"scenario_name": "jitter_rejection", "frames": frames2})

    # Scenario 3: Transient dip rejection (15 frames dip ~0.48s < 3.0s, then recover)
    det3 = StreamingPowerDownDetector(params)
    frames3 = []
    for _ in range(20):
        res = det3.process_energy_frame(50.0)
        frames3.append(
            {
                "energy_db": res.current_energy_db,
                "expected_state": res.state.value,
                "expected_triggered": res.event_triggered,
            }
        )
    for _ in range(15):
        res = det3.process_energy_frame(35.0)
        frames3.append(
            {
                "energy_db": res.current_energy_db,
                "expected_state": res.state.value,
                "expected_triggered": res.event_triggered,
            }
        )
    for _ in range(30):
        res = det3.process_energy_frame(50.0)
        frames3.append(
            {
                "energy_db": res.current_energy_db,
                "expected_state": res.state.value,
                "expected_triggered": res.event_triggered,
            }
        )
    scenarios.append({"scenario_name": "transient_dip_rejection", "frames": frames3})

    return {
        "params": json.loads(params.to_json()),
        "scenarios": scenarios,
    }


def export_golden_vectors_file(output_path: Path, params: DetectorParams | None = None) -> Path:
    """Generate and export golden test vectors to JSON."""
    if params is None:
        params = DetectorParams()
    vectors = generate_golden_vectors(params)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as f:
        json.dump(vectors, f, indent=2)
    return output_path
