from __future__ import annotations

import numpy as np

from echogrid_hum.dsp import (
    compute_band_energy_db,
    compute_band_snr_db,
    compute_frame_stft,
    compute_harmonic_comb_energy_db,
    compute_welch_psd,
    estimate_noise_floor_db,
)


def generate_synthetic_tone(
    freq: float,
    duration_s: float = 2.0,
    sr: int = 4000,
    amplitude: float = 0.5,
    noise_level: float = 0.05,
    seed: int = 42,
) -> tuple[np.ndarray, np.ndarray]:
    """Generate deterministic tone plus white noise."""
    t = np.linspace(0, duration_s, int(sr * duration_s), endpoint=False)
    rng = np.random.default_rng(seed)
    noise = noise_level * rng.standard_normal(len(t))
    signal = amplitude * np.sin(2 * np.pi * freq * t) + noise
    return signal, t


def test_welch_psd_and_band_energy():
    sr = 4000
    tone_100hz, _ = generate_synthetic_tone(freq=100.0, sr=sr, amplitude=0.8, noise_level=0.01)
    freqs, psd = compute_welch_psd(tone_100hz, sr=sr, nperseg=1024)

    # 1. 100 Hz band energy must dominate non-harmonic bands
    energy_target_db = compute_band_energy_db(freqs, psd, f_low=95.0, f_high=105.0)
    energy_adjacent_db = compute_band_energy_db(freqs, psd, f_low=145.0, f_high=155.0)

    assert energy_target_db > energy_adjacent_db + 15.0  # At least 15 dB prominence


def test_mains_harmonic_comb_energy():
    sr = 4000
    t = np.linspace(0, 2.0, sr * 2, endpoint=False)
    # 50 Hz mains fundamental + 100 Hz second harmonic + 150 Hz third harmonic
    signal_50hz = (
        0.5 * np.sin(2 * np.pi * 50.0 * t)
        + 0.3 * np.sin(2 * np.pi * 100.0 * t)
        + 0.2 * np.sin(2 * np.pi * 150.0 * t)
    )

    freqs, psd = compute_welch_psd(signal_50hz, sr=sr, nperseg=1024)

    comb_50_db = compute_harmonic_comb_energy_db(freqs, psd, fundamental=50.0, num_harmonics=3)
    comb_60_db = compute_harmonic_comb_energy_db(freqs, psd, fundamental=60.0, num_harmonics=3)

    # 50 Hz comb should be significantly stronger than 60 Hz comb on a 50 Hz signal
    assert comb_50_db > comb_60_db + 12.0


def test_tone_on_off_band_snr():
    sr = 4000
    duration_s = 2.0
    rng = np.random.default_rng(123)

    t = np.linspace(0, duration_s, int(sr * duration_s), endpoint=False)
    noise_only = 0.05 * rng.standard_normal(len(t))
    tone_on = 0.5 * np.sin(2 * np.pi * 100.0 * t) + noise_only

    freqs, on_psd = compute_welch_psd(tone_on, sr=sr, nperseg=1024)
    _, off_psd = compute_welch_psd(noise_only, sr=sr, nperseg=1024)

    snr_db = compute_band_snr_db(on_psd, off_psd, freqs, f_low=95.0, f_high=105.0)
    assert snr_db > 15.0  # SNR of 100 Hz tone vs noise must be > 15 dB


def test_noise_floor_estimator():
    sr = 4000
    rng = np.random.default_rng(999)
    noise = 0.1 * rng.standard_normal(sr * 2)

    freqs, psd = compute_welch_psd(noise, sr=sr, nperseg=1024)
    floor_db = estimate_noise_floor_db(psd, percentile=25.0)

    # White noise floor should be relatively uniform and negative in dB
    assert -60.0 < floor_db < -20.0


def test_stft_computation():
    sr = 4000
    tone, _ = generate_synthetic_tone(freq=120.0, duration_s=1.0, sr=sr)
    freqs, times, zxx = compute_frame_stft(tone, sr=sr, nperseg=512, noverlap=256)

    assert len(freqs) == 512 // 2 + 1
    assert len(times) > 0
    assert zxx.shape == (len(freqs), len(times))
