"""Digital Signal Processing (DSP) feature extraction for appliance hum detection.

Pure functions for spectral estimation, band energies, harmonic comb analysis,
and noise-floor estimation. Designed for streaming-compatible frame-by-frame porting.
"""

from __future__ import annotations

import numpy as np
from scipy import signal


def compute_welch_psd(
    audio: np.ndarray,
    sr: int,
    nperseg: int = 1024,
    noverlap: int | None = None,
) -> tuple[np.ndarray, np.ndarray]:
    """Compute Power Spectral Density (PSD) using Welch's average periodogram method."""
    if len(audio) == 0:
        return np.array([], dtype=np.float64), np.array([], dtype=np.float64)
    if noverlap is None:
        noverlap = nperseg // 2
    actual_nperseg = min(nperseg, len(audio))
    actual_noverlap = min(noverlap, actual_nperseg - 1) if actual_nperseg > 1 else 0
    freqs, psd = signal.welch(
        audio,
        fs=sr,
        nperseg=actual_nperseg,
        noverlap=actual_noverlap,
        scaling="density",
    )
    return freqs, psd


def compute_band_energy(
    freqs: np.ndarray,
    psd: np.ndarray,
    f_low: float,
    f_high: float,
) -> float:
    """Compute linear power integral over frequency band [f_low, f_high]."""
    if len(freqs) == 0 or len(psd) == 0 or f_high <= f_low:
        return 0.0
    mask = (freqs >= f_low) & (freqs <= f_high)
    if not np.any(mask):
        return 0.0
    # Numerical integration: discrete sum scaled by bin width
    df = freqs[1] - freqs[0] if len(freqs) > 1 else 1.0
    return float(np.sum(psd[mask]) * df)


def compute_band_energy_db(
    freqs: np.ndarray,
    psd: np.ndarray,
    f_low: float,
    f_high: float,
    eps: float = 1e-12,
) -> float:
    """Compute band energy expressed in decibels (dB)."""
    linear_energy = compute_band_energy(freqs, psd, f_low, f_high)
    return float(10.0 * np.log10(max(linear_energy, eps)))


def estimate_noise_floor_db(
    psd: np.ndarray,
    percentile: float = 20.0,
    eps: float = 1e-12,
) -> float:
    """Estimate background noise floor in dB from lower quantile of spectral power."""
    if len(psd) == 0:
        return float(10.0 * np.log10(eps))
    floor_linear = float(np.percentile(psd, percentile))
    return float(10.0 * np.log10(max(floor_linear, eps)))


def compute_harmonic_comb_energy(
    freqs: np.ndarray,
    psd: np.ndarray,
    fundamental: float,
    num_harmonics: int = 4,
    half_width: float = 2.0,
) -> float:
    """Compute sum of energy across harmonic series: k * fundamental for k=1..num_harmonics."""
    total_energy = 0.0
    for k in range(1, num_harmonics + 1):
        center = k * fundamental
        f_low = max(0.0, center - half_width)
        f_high = center + half_width
        total_energy += compute_band_energy(freqs, psd, f_low, f_high)
    return total_energy


def compute_harmonic_comb_energy_db(
    freqs: np.ndarray,
    psd: np.ndarray,
    fundamental: float,
    num_harmonics: int = 4,
    half_width: float = 2.0,
    eps: float = 1e-12,
) -> float:
    """Compute harmonic series energy in decibels (dB)."""
    comb_linear = compute_harmonic_comb_energy(freqs, psd, fundamental, num_harmonics, half_width)
    return float(10.0 * np.log10(max(comb_linear, eps)))


def compute_band_snr_db(
    signal_psd: np.ndarray,
    noise_psd: np.ndarray,
    freqs: np.ndarray,
    f_low: float,
    f_high: float,
    eps: float = 1e-12,
) -> float:
    """Calculate Signal-to-Noise Ratio (SNR) in dB for a designated band."""
    sig_power = compute_band_energy(freqs, signal_psd, f_low, f_high)
    noise_power = compute_band_energy(freqs, noise_psd, f_low, f_high)
    ratio = max(sig_power, eps) / max(noise_power, eps)
    return float(10.0 * np.log10(ratio))


def compute_frame_stft(
    signal_data: np.ndarray,
    sr: int,
    nperseg: int = 1024,
    noverlap: int = 512,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Compute short-time Fourier transform (STFT) matrix."""
    return signal.stft(signal_data, fs=sr, nperseg=nperseg, noverlap=noverlap)
