"""Spectral survey report generation for EchoGrid Phase 0.

Produces per-source Power Spectral Density (PSD) and spectrogram plots for
low-frequency appliance hums (20-500 Hz) and generates reports/spectra/README.md.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import matplotlib
import numpy as np
import soundfile as sf

matplotlib.use("Agg")
import matplotlib.pyplot as plt

from echogrid_hum.dsp import (
    compute_frame_stft,
    compute_welch_psd,
    estimate_noise_floor_db,
)
from echogrid_hum.fetch_datasets import get_repo_root, setup_sample_dataset


def collect_source_audio_files(data_dir: Path) -> dict[str, list[Path]]:
    """Scan data/own and data/public for appliance recordings grouped by source category."""
    sources: dict[str, list[Path]] = {
        "air_conditioner": [],
        "refrigerator": [],
        "pump": [],
        "fan": [],
        "engine_idling": [],
    }

    # 1. User's own recordings take precedence
    own_dir = data_dir / "own"
    if own_dir.exists():
        for wav in own_dir.glob("*.wav"):
            for s in sources:
                if s in wav.stem.lower():
                    sources[s].append(wav)

    # 2. Public datasets and CI samples
    public_dir = data_dir / "public"
    if public_dir.exists():
        for wav in public_dir.rglob("*.wav"):
            for s in sources:
                if s in wav.stem.lower() and wav not in sources[s]:
                    sources[s].append(wav)

    return sources


def analyze_and_plot_source(
    source_name: str,
    wav_files: list[Path],
    output_dir: Path,
) -> dict[str, float | str | int]:
    """Compute PSD and spectrogram, generate plots, and return spectral summary statistics."""
    merged_audio: list[np.ndarray] = []
    target_sr = 16000

    for f in wav_files:
        try:
            data, sr = sf.read(f)
            if data.ndim > 1:
                data = np.mean(data, axis=1)
            # Resample or match simple target
            if sr != target_sr and len(data) > 0:
                indices = np.linspace(0, len(data) - 1, int(len(data) * target_sr / sr))
                data = np.interp(indices, np.arange(len(data)), data)
            merged_audio.append(data.astype(np.float32))
        except Exception as e:
            print(f"Warning: could not read {f}: {e}")

    if not merged_audio:
        return {
            "source": source_name,
            "peak_hz": 0.0,
            "prominence_db": 0.0,
            "harmonic_family": "None",
            "band_range": "N/A",
            "clips_count": 0,
        }

    audio = np.concatenate(merged_audio)
    freqs, psd = compute_welch_psd(audio, sr=target_sr, nperseg=2048)

    # Filter to 20 - 500 Hz region
    band_mask = (freqs >= 20.0) & (freqs <= 500.0)
    b_freqs = freqs[band_mask]
    b_psd = psd[band_mask]

    noise_floor_db = estimate_noise_floor_db(psd, percentile=15.0)
    psd_db = 10.0 * np.log10(np.maximum(b_psd, 1e-12))

    peak_idx = int(np.argmax(psd_db))
    peak_hz = float(b_freqs[peak_idx])
    peak_power_db = float(psd_db[peak_idx])
    prominence_db = float(max(0.0, peak_power_db - noise_floor_db))

    # Detect harmonic family
    if abs(peak_hz % 50.0) < 4.0 or abs((peak_hz % 50.0) - 50.0) < 4.0:
        family = "50 Hz mains"
    elif abs(peak_hz % 60.0) < 4.0 or abs((peak_hz % 60.0) - 60.0) < 4.0:
        family = "60 Hz mains"
    else:
        family = "Mechanical / Rotor"

    band_range = f"{max(20.0, peak_hz - 10.0):.0f}-{min(500.0, peak_hz + 10.0):.0f} Hz"

    output_dir.mkdir(parents=True, exist_ok=True)

    # 1. Generate PSD Curve Plot
    fig, ax = plt.subplots(figsize=(7, 3.5), tight_layout=True)
    ax.plot(b_freqs, psd_db, color="#0284c7", lw=1.5, label="Average PSD")
    ax.axhline(
        noise_floor_db,
        color="#dc2626",
        ls="--",
        lw=1.2,
        label=f"Noise Floor ({noise_floor_db:.1f} dB)",
    )
    ax.scatter(
        [peak_hz],
        [peak_power_db],
        color="#e11d48",
        zorder=5,
        label=f"Peak {peak_hz:.1f} Hz (+{prominence_db:.1f} dB)",
    )
    ax.set_title(f"{source_name.replace('_', ' ').title()} - Spectral Survey (20-500 Hz)")
    ax.set_xlabel("Frequency (Hz)")
    ax.set_ylabel("Power Density (dB/Hz)")
    ax.set_xlim(20, 500)
    ax.grid(True, alpha=0.3)
    ax.legend(loc="upper right", fontsize=8)
    psd_png = output_dir / f"{source_name}_psd.png"
    fig.savefig(psd_png, dpi=120)
    plt.close(fig)

    # 2. Generate Spectrogram Plot
    stft_freqs, times, zxx = compute_frame_stft(
        audio[: min(len(audio), target_sr * 10)], sr=target_sr, nperseg=1024
    )
    s_mask = (stft_freqs >= 20.0) & (stft_freqs <= 500.0)
    spec_mag_db = 20.0 * np.log10(np.maximum(np.abs(zxx[s_mask, :]), 1e-6))

    fig, ax = plt.subplots(figsize=(7, 3.5), tight_layout=True)
    im = ax.pcolormesh(times, stft_freqs[s_mask], spec_mag_db, shading="gouraud", cmap="viridis")
    fig.colorbar(im, ax=ax, label="Magnitude (dB)")
    ax.set_title(f"{source_name.replace('_', ' ').title()} - Spectrogram (20-500 Hz)")
    ax.set_xlabel("Time (s)")
    ax.set_ylabel("Frequency (Hz)")
    spec_png = output_dir / f"{source_name}_spectrogram.png"
    fig.savefig(spec_png, dpi=120)
    plt.close(fig)

    return {
        "source": source_name,
        "peak_hz": round(peak_hz, 1),
        "prominence_db": round(prominence_db, 1),
        "harmonic_family": family,
        "band_range": band_range,
        "clips_count": len(wav_files),
    }


def generate_spectral_survey(data_dir: Path, output_dir: Path) -> Path:
    """Run full spectral survey and write reports/spectra/README.md."""
    sources = collect_source_audio_files(data_dir)

    # If no audio files exist at all, populate samples
    total_files = sum(len(v) for v in sources.values())
    if total_files == 0:
        print("No audio clips found. Generating CI sample dataset in data/public/samples...")
        setup_sample_dataset(data_dir / "public")
        sources = collect_source_audio_files(data_dir)

    output_dir.mkdir(parents=True, exist_ok=True)
    summary_rows = []

    for src_name, files in sources.items():
        if not files:
            continue
        stats = analyze_and_plot_source(src_name, files, output_dir)
        summary_rows.append(stats)

    readme_path = output_dir / "README.md"
    with readme_path.open("w", encoding="utf-8") as f:
        f.write("# Spectral Survey Report (20–500 Hz)\n\n")
        f.write(
            "Survey of appliance acoustic hum frequencies, harmonic series, "
            "and observable SNR above ambient noise floor.\n\n"
        )
        f.write("## Summary Table of Detectable Bands\n\n")
        header = (
            "| Appliance Source | Peak Frequency (Hz) | Prominence above Noise (dB) "
            "| Harmonic Family | Usable Band Range | Audio Clips |\n"
        )
        f.write(header)
        f.write("|---|---|---|---|---|---|\n")
        for r in summary_rows:
            f.write(
                f"| {r['source']} | {r['peak_hz']} Hz | +{r['prominence_db']} dB | "
                f"{r['harmonic_family']} | {r['band_range']} | {r['clips_count']} |\n"
            )
        f.write("\n## Generated Visualizations\n\n")
        for r in summary_rows:
            src = r["source"]
            f.write(f"### {src.replace('_', ' ').title()}\n")
            f.write(f"- **PSD Plot**: ![{src} PSD]({src}_psd.png)\n")
            f.write(f"- **Spectrogram**: ![{src} Spectrogram]({src}_spectrogram.png)\n\n")

    return readme_path


def main() -> int:
    repo_root = get_repo_root()
    parser = argparse.ArgumentParser(description="Generate appliance acoustic spectral survey.")
    parser.add_argument("--data-dir", type=Path, default=repo_root / "data")
    parser.add_argument("--output-dir", type=Path, default=repo_root / "reports" / "spectra")
    args = parser.parse_args()

    readme = generate_spectral_survey(args.data_dir, args.output_dir)
    print(f"[OK] Spectral survey generated: {readme}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
