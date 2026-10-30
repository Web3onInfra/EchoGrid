"""Export deterministic audio -> band-energy fixtures for TypeScript parity testing.

Generates synthetic audio signals (sine tones, dual tones, silence, out-of-band tones,
and a power-down sequence) and computes exact STFT windowed band energies matching
the Phase 0 detector implementation.

Outputs to research/fixtures/feature_parity.json.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np

from echogrid_hum.detector import DetectorParams, StreamingPowerDownDetector


def generate_feature_fixtures() -> dict:
    params = DetectorParams()
    detector = StreamingPowerDownDetector(params)
    sr = params.sample_rate
    n = params.frame_size

    t = np.arange(n) / sr

    test_cases = []

    # 1. Pure 50 Hz mains hum (amplitude 0.5)
    audio_50 = 0.5 * np.sin(2.0 * np.pi * 50.0 * t).astype(np.float32)
    e_total_50 = detector._compute_frame_band_energy_db(audio_50)
    test_cases.append(
        {
            "name": "sine_50hz",
            "description": "Pure 50 Hz fundamental tone at amplitude 0.5",
            "audio": [round(float(x), 5) for x in audio_50],
            "expected_total_energy_db": round(e_total_50, 4),
        }
    )

    # 2. Pure 60 Hz mains hum (amplitude 0.5)
    audio_60 = 0.5 * np.sin(2.0 * np.pi * 60.0 * t).astype(np.float32)
    e_total_60 = detector._compute_frame_band_energy_db(audio_60)
    test_cases.append(
        {
            "name": "sine_60hz",
            "description": "Pure 60 Hz fundamental tone at amplitude 0.5",
            "audio": [round(float(x), 5) for x in audio_60],
            "expected_total_energy_db": round(e_total_60, 4),
        }
    )

    # 3. Pure 120 Hz harmonic (amplitude 0.5)
    audio_120 = 0.5 * np.sin(2.0 * np.pi * 120.0 * t).astype(np.float32)
    e_total_120 = detector._compute_frame_band_energy_db(audio_120)
    test_cases.append(
        {
            "name": "sine_120hz",
            "description": "Pure 120 Hz second harmonic tone at amplitude 0.5",
            "audio": [round(float(x), 5) for x in audio_120],
            "expected_total_energy_db": round(e_total_120, 4),
        }
    )

    # 4. Dual tone 60 Hz + 120 Hz (typical appliance acoustic profile)
    tone_60 = 0.35 * np.sin(2.0 * np.pi * 60.0 * t)
    tone_120 = 0.15 * np.sin(2.0 * np.pi * 120.0 * t)
    audio_dual = (tone_60 + tone_120).astype(np.float32)
    e_total_dual = detector._compute_frame_band_energy_db(audio_dual)
    test_cases.append(
        {
            "name": "dual_60_120hz",
            "description": "Appliance profile with 60 Hz fundamental + 120 Hz harmonic",
            "audio": [round(float(x), 5) for x in audio_dual],
            "expected_total_energy_db": round(e_total_dual, 4),
        }
    )

    # 5. Silence (all zeros)
    audio_silence = np.zeros(n, dtype=np.float32)
    e_total_silence = detector._compute_frame_band_energy_db(audio_silence)
    test_cases.append(
        {
            "name": "silence",
            "description": "Silent frame (zero power, hits eps 1e-12 floor)",
            "audio": [round(float(x), 5) for x in audio_silence],
            "expected_total_energy_db": round(e_total_silence, 4),
        }
    )

    # 6. Out-of-band 1000 Hz whistle tone (should have negligible energy in monitored bands)
    audio_1000 = 0.5 * np.sin(2.0 * np.pi * 1000.0 * t).astype(np.float32)
    e_total_1000 = detector._compute_frame_band_energy_db(audio_1000)
    test_cases.append(
        {
            "name": "sine_1000hz_out_of_band",
            "description": "High frequency 1000 Hz tone outside monitored mains hum bands",
            "audio": [round(float(x), 5) for x in audio_1000],
            "expected_total_energy_db": round(e_total_1000, 4),
        }
    )

    # 7. Multi-frame streaming sequence: 5 frames on (60 Hz) followed by 5 frames off (silence)
    num_stream_frames = 10
    total_samples = params.frame_size + (num_stream_frames - 1) * params.hop_size
    t_stream = np.arange(total_samples) / sr
    stream_audio = np.zeros(total_samples, dtype=np.float32)
    on_samples = params.frame_size + 4 * params.hop_size
    stream_audio[:on_samples] = 0.4 * np.sin(2.0 * np.pi * 60.0 * t_stream[:on_samples])

    stream_frame_energies = []
    for f_idx in range(num_stream_frames):
        start = f_idx * params.hop_size
        frame = stream_audio[start : start + params.frame_size]
        stream_frame_energies.append(round(detector._compute_frame_band_energy_db(frame), 4))

    return {
        "metadata": {
            "sample_rate": sr,
            "frame_size": params.frame_size,
            "hop_size": params.hop_size,
            "bands": [list(b) for b in params.bands],
            "description": "STFT band energy parity fixtures for TypeScript and Rust ports",
        },
        "single_frame_cases": test_cases,
        "streaming_sequence": {
            "name": "power_down_stream",
            "audio": [round(float(x), 5) for x in stream_audio],
            "expected_frame_energies_db": stream_frame_energies,
        },
    }


def main():
    root = Path(__file__).resolve().parent.parent.parent
    fixtures_dir = root / "research" / "fixtures"
    fixtures_dir.mkdir(parents=True, exist_ok=True)
    out_file = fixtures_dir / "feature_parity.json"

    data = generate_feature_fixtures()
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)

    print(f"Exported feature parity fixtures to: {out_file}")
    print(f"Total single-frame test cases: {len(data['single_frame_cases'])}")
    print(f"Streaming sequence samples: {len(data['streaming_sequence']['audio'])}")


if __name__ == "__main__":
    main()
