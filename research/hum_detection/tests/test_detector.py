from __future__ import annotations

import dataclasses

import numpy as np
import pytest

from echogrid_hum.detector import (
    DetectorParams,
    DetectorState,
    StreamingPowerDownDetector,
)


def test_params_frozen_and_json_roundtrip():
    params = DetectorParams(min_sustained_drop_s=2.5, drop_threshold_db=7.0)

    # Immutability check
    with pytest.raises(dataclasses.FrozenInstanceError):
        params.min_sustained_drop_s = 5.0  # type: ignore

    # Serialization check
    json_repr = params.to_json()
    recovered = DetectorParams.from_json(json_repr)
    assert recovered == params
    assert recovered.bands == params.bands


def test_property_no_trigger_on_constant_noise():
    """Property test 1: Stationary white noise must never trigger an OFF event."""
    sr = 16000
    duration_s = 15.0
    rng = np.random.default_rng(42)
    noise = 0.05 * rng.standard_normal(int(sr * duration_s)).astype(np.float32)

    detector = StreamingPowerDownDetector(DetectorParams(sample_rate=sr))
    results = detector.process_audio(noise)

    triggers = [r for r in results if r.event_triggered]
    assert len(triggers) == 0
    assert all(r.state != DetectorState.OFF_DETECTED for r in results)


def test_property_trigger_on_clean_on_to_off_step():
    """Property test 2: Sustained transition from active hum to silence triggers once."""
    sr = 16000
    t_on = 6.0
    t_off = 6.0
    rng = np.random.default_rng(101)

    t1 = np.linspace(0, t_on, int(sr * t_on), endpoint=False)
    # Active 50 Hz appliance hum + noise
    sig_on = (0.4 * np.sin(2 * np.pi * 50 * t1) + 0.02 * rng.standard_normal(len(t1))).astype(
        np.float32
    )

    t2 = np.linspace(0, t_off, int(sr * t_off), endpoint=False)
    # Powered off: ambient noise only (-20 dB drop)
    sig_off = (0.02 * rng.standard_normal(len(t2))).astype(np.float32)

    full_audio = np.concatenate([sig_on, sig_off])

    params = DetectorParams(sample_rate=sr, min_sustained_drop_s=3.0, drop_threshold_db=6.0)
    detector = StreamingPowerDownDetector(params)
    results = detector.process_audio(full_audio)

    triggers = [r for r in results if r.event_triggered]
    assert len(triggers) == 1

    trigger_frame = triggers[0]
    # Drop begins at 6.0s, sustained 3.0s -> trigger should occur around 9.0s
    assert 8.8 <= trigger_frame.timestamp_s <= 9.3
    assert trigger_frame.state == DetectorState.OFF_DETECTED

    # Later frames remain OFF_DETECTED without re-triggering
    final_frames = results[-10:]
    assert all(f.state == DetectorState.OFF_DETECTED for f in final_frames)
    assert all(not f.event_triggered for f in final_frames)


def test_property_no_trigger_on_on_with_amplitude_jitter():
    """Property test 3: Hum running with random amplitude fluctuations (±2 dB) must not trigger."""
    sr = 16000
    duration_s = 12.0
    rng = np.random.default_rng(202)

    t = np.linspace(0, duration_s, int(sr * duration_s), endpoint=False)
    # Amplitude modulating slowly between 0.35 and 0.45 (+/- ~1.5 dB)
    jitter = 0.4 + 0.05 * np.sin(2 * np.pi * 0.5 * t)
    sig = (jitter * np.sin(2 * np.pi * 50 * t) + 0.03 * rng.standard_normal(len(t))).astype(
        np.float32
    )

    detector = StreamingPowerDownDetector(DetectorParams(sample_rate=sr, drop_threshold_db=6.0))
    results = detector.process_audio(sig)

    triggers = [r for r in results if r.event_triggered]
    assert len(triggers) == 0
    assert all(r.state == DetectorState.ON for r in results)


def test_temporary_dip_does_not_trigger():
    """A transient dip shorter than min_sustained_drop_s must recover without triggering."""
    sr = 16000
    rng = np.random.default_rng(303)

    # 4s ON -> 1.0s dip -> 4s ON (dip is 1s, min sustained is 3s)
    t_on = np.linspace(0, 4.0, int(sr * 4.0), endpoint=False)
    on_audio = (0.4 * np.sin(2 * np.pi * 50 * t_on) + 0.02 * rng.standard_normal(len(t_on))).astype(
        np.float32
    )

    t_dip = np.linspace(0, 1.0, int(sr * 1.0), endpoint=False)
    dip_audio = (0.02 * rng.standard_normal(len(t_dip))).astype(np.float32)

    audio = np.concatenate([on_audio, dip_audio, on_audio])

    detector = StreamingPowerDownDetector(
        DetectorParams(sample_rate=sr, min_sustained_drop_s=3.0, drop_threshold_db=6.0)
    )
    results = detector.process_audio(audio)

    # Must see PENDING_DROP during dip, but zero confirmed triggers, and final state ON
    pending = [r for r in results if r.state == DetectorState.PENDING_DROP]
    assert len(pending) > 0

    triggers = [r for r in results if r.event_triggered]
    assert len(triggers) == 0
    assert results[-1].state == DetectorState.ON
