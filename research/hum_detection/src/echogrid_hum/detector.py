"""Streaming power-down detector for EchoGrid Phase 0.

Strictly frame-based (frame-in, state-out, no lookahead) detector with adaptive
noise-floor estimation, hysteresis thresholding, and sustained drop verification.
Designed to be directly portable to JavaScript (Web Audio API) or Rust.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from enum import Enum

import numpy as np


class DetectorState(str, Enum):
    ON = "ON"
    PENDING_DROP = "PENDING_DROP"
    OFF_DETECTED = "OFF_DETECTED"


@dataclass(frozen=True)
class DetectorParams:
    """Immutable detector parameters, serializable to/from JSON."""

    sample_rate: int = 16000
    frame_size: int = 1024
    hop_size: int = 512
    # Usable bands derived from PR 7 band-SNR analysis (50Hz mains, 60Hz mains, 120Hz harmonic)
    bands: tuple[tuple[float, float], ...] = (
        (45.0, 55.0),
        (55.0, 65.0),
        (115.0, 125.0),
    )
    drop_threshold_db: float = 6.0
    hysteresis_db: float = 2.0
    min_sustained_drop_s: float = 3.0
    alpha_noise: float = 0.05
    alpha_signal: float = 0.25

    def to_json(self, indent: int = 2) -> str:
        d = asdict(self)
        d["bands"] = [list(b) for b in self.bands]
        return json.dumps(d, indent=indent)

    @classmethod
    def from_json(cls, json_str: str) -> DetectorParams:
        d = json.loads(json_str)
        d["bands"] = tuple(tuple(b) for b in d["bands"])
        return cls(**d)


@dataclass
class FrameResult:
    """Output for an individual streaming frame."""

    timestamp_s: float
    state: DetectorState
    event_triggered: bool
    current_energy_db: float
    running_baseline_db: float
    drop_duration_s: float


class StreamingPowerDownDetector:
    """Frame-by-frame detector tracking appliance hum power-down transitions."""

    def __init__(self, params: DetectorParams | None = None) -> None:
        self.params = params if params is not None else DetectorParams()
        self.window = np.hanning(self.params.frame_size)
        self.fft_freqs = np.fft.rfftfreq(self.params.frame_size, d=1.0 / self.params.sample_rate)

        # Precompute band bin masks for maximum streaming efficiency
        self.band_masks = [
            (self.fft_freqs >= f_low) & (self.fft_freqs <= f_high)
            for f_low, f_high in self.params.bands
        ]

        self.reset()

    def reset(self) -> None:
        """Reset internal streaming state."""
        self.state = DetectorState.ON
        self.frames_processed = 0
        self.running_baseline_db: float | None = None
        self.sustained_drop_s: float = 0.0
        self.has_triggered: bool = False

    def _compute_frame_band_energy_db(self, frame: np.ndarray) -> float:
        """Calculate total energy across designated usable bands for one windowed frame."""
        if len(frame) < self.params.frame_size:
            padded = np.zeros(self.params.frame_size, dtype=np.float32)
            padded[: len(frame)] = frame
            frame = padded

        windowed = frame * self.window
        fft_mag = np.abs(np.fft.rfft(windowed)) ** 2

        total_power = 0.0
        for mask in self.band_masks:
            total_power += float(np.sum(fft_mag[mask]))

        return float(10.0 * np.log10(max(total_power, 1e-12)))

    def process_energy_frame(self, energy_db: float) -> FrameResult:
        """Process one frame given its precomputed band energy in dB.

        Enables direct vector replay without FFT recomputation (contract for JS/Rust ports).
        """
        frame_time_s = self.frames_processed * (self.params.hop_size / self.params.sample_rate)
        frame_duration_s = self.params.hop_size / self.params.sample_rate
        self.frames_processed += 1

        if self.running_baseline_db is None:
            self.running_baseline_db = energy_db

        event_triggered = False
        drop_threshold = self.running_baseline_db - self.params.drop_threshold_db
        recovery_threshold = drop_threshold + self.params.hysteresis_db

        if energy_db < drop_threshold:
            self.sustained_drop_s += frame_duration_s
            if self.sustained_drop_s >= self.params.min_sustained_drop_s:
                if self.state != DetectorState.OFF_DETECTED:
                    self.state = DetectorState.OFF_DETECTED
                    if not self.has_triggered:
                        event_triggered = True
                        self.has_triggered = True
            else:
                if self.state == DetectorState.ON:
                    self.state = DetectorState.PENDING_DROP
        elif energy_db >= recovery_threshold:
            self.sustained_drop_s = 0.0
            self.state = DetectorState.ON
            self.running_baseline_db = (
                1.0 - self.params.alpha_signal
            ) * self.running_baseline_db + self.params.alpha_signal * energy_db

        return FrameResult(
            timestamp_s=round(frame_time_s, 3),
            state=self.state,
            event_triggered=event_triggered,
            current_energy_db=round(energy_db, 2),
            running_baseline_db=round(self.running_baseline_db, 2),
            drop_duration_s=round(self.sustained_drop_s, 3),
        )

    def process_frame(self, frame: np.ndarray) -> FrameResult:
        """Process one audio frame. Strictly causal; no lookahead."""
        energy_db = self._compute_frame_band_energy_db(frame)
        return self.process_energy_frame(energy_db)

    def process_audio(self, audio: np.ndarray) -> list[FrameResult]:
        """Stream through an entire audio array frame-by-frame."""
        results: list[FrameResult] = []
        hop = self.params.hop_size
        frame_size = self.params.frame_size

        for i in range(0, len(audio) - frame_size + 1, hop):
            frame = audio[i : i + frame_size]
            results.append(self.process_frame(frame))

        return results
