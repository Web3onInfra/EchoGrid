"""EchoGrid hum detection package."""

from echogrid_hum.detector import (
    DetectorParams,
    DetectorState,
    FrameResult,
    StreamingPowerDownDetector,
)
from echogrid_hum.dsp import (
    compute_band_energy,
    compute_band_energy_db,
    compute_band_snr_db,
    compute_frame_stft,
    compute_harmonic_comb_energy,
    compute_harmonic_comb_energy_db,
    compute_welch_psd,
    estimate_noise_floor_db,
)

__version__ = "0.1.0"

__all__ = [
    "DetectorParams",
    "DetectorState",
    "FrameResult",
    "StreamingPowerDownDetector",
    "compute_band_energy",
    "compute_band_energy_db",
    "compute_band_snr_db",
    "compute_frame_stft",
    "compute_harmonic_comb_energy",
    "compute_harmonic_comb_energy_db",
    "compute_welch_psd",
    "estimate_noise_floor_db",
]
