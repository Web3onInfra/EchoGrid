"""ON/OFF Band-SNR analysis for EchoGrid Phase 0.

Evaluates per-band Signal-to-Noise Ratio (ON energy / OFF energy in dB)
across appliance sources and distances, identifying usable bands (>=6 dB on >=80% clips).
Outputs reports/band_snr.md.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

from echogrid_hum.dsp import compute_band_energy, compute_welch_psd
from echogrid_hum.fetch_datasets import get_repo_root, setup_sample_dataset
from echogrid_hum.validate_recordings import load_recording_with_sidecar

CANDIDATE_BANDS: dict[str, tuple[float, float]] = {
    "50Hz_fundamental": (45.0, 55.0),
    "60Hz_fundamental": (55.0, 65.0),
    "100Hz_harmonic": (95.0, 105.0),
    "120Hz_harmonic": (115.0, 125.0),
    "150Hz_harmonic": (145.0, 155.0),
    "180Hz_harmonic": (175.0, 185.0),
    "sub_hum_rumble": (20.0, 40.0),
}


def build_synthetic_transitions_if_needed(data_dir: Path) -> list[tuple[Path, Path]]:
    """Ensure at least one set of ON/OFF recordings with sidecar markers exists."""
    pairs: list[tuple[Path, Path]] = []
    own_dir = data_dir / "own"
    if own_dir.exists():
        for wav in own_dir.glob("*.wav"):
            js = wav.with_suffix(".json")
            if js.exists():
                pairs.append((wav, js))

    if pairs:
        return pairs

    # Generate synthetic ON/OFF test series across distances (0.5m, 2m, 4m)
    synth_dir = data_dir / "public" / "synthetic_transitions"
    synth_dir.mkdir(parents=True, exist_ok=True)
    setup_sample_dataset(data_dir / "public")

    sources = ["air_conditioner", "pump", "fan"]
    distances = [0.5, 2.0, 4.0]
    sr = 16000

    idx = 0
    for src in sources:
        for dist in distances:
            wav_path = synth_dir / f"{src}_{dist}m_sample.wav"
            js_path = synth_dir / f"{src}_{dist}m_sample.json"

            # Distance attenuation: 1/dist
            atten = 0.5 / dist
            t_total = 6.0
            t = np.linspace(0, t_total, int(sr * t_total), endpoint=False)
            rng = np.random.default_rng(100 + idx)
            noise = 0.02 * rng.standard_normal(len(t))

            # ON during [0s, 3s], OFF during [3s, 6s]
            on_mask = t < 3.0
            sig = np.copy(noise)
            if src == "air_conditioner":
                sig[on_mask] += atten * (
                    0.4 * np.sin(2 * np.pi * 50 * t[on_mask])
                    + 0.25 * np.sin(2 * np.pi * 100 * t[on_mask])
                )
            elif src == "pump":
                sig[on_mask] += atten * (
                    0.4 * np.sin(2 * np.pi * 60 * t[on_mask])
                    + 0.2 * np.sin(2 * np.pi * 120 * t[on_mask])
                )
            else:  # fan
                sig[on_mask] += atten * (0.35 * np.sin(2 * np.pi * 120 * t[on_mask]))

            sig = (sig / np.max(np.abs(sig)) * 0.7).astype(np.float32)
            sf.write(wav_path, sig, sr, subtype="PCM_16")

            sidecar = {
                "audio_file": wav_path.name,
                "sample_rate": sr,
                "metadata": {"source": src, "distance_m": dist},
                "events": [
                    {"type": "ON", "timestamp_s": 0.0},
                    {"type": "OFF", "timestamp_s": 3.0},
                ],
            }
            with js_path.open("w", encoding="utf-8") as f:
                json.dump(sidecar, f, indent=2)

            pairs.append((wav_path, js_path))
            idx += 1

    return pairs


def calculate_recording_band_snrs(wav_path: Path, js_path: Path) -> dict[str, float | str]:
    """Calculate SNR for each candidate band between ON and OFF ground-truth segments."""
    audio, sr, meta = load_recording_with_sidecar(wav_path, js_path)
    events = meta.get("events", [])
    duration_s = len(audio) / sr

    # Extract ON and OFF intervals
    on_intervals = []
    off_intervals = []
    current_state = "ON"
    last_t = 0.0

    for ev in events:
        t = min(float(ev["timestamp_s"]), duration_s)
        if t > last_t:
            if current_state == "ON":
                on_intervals.append((last_t, t))
            else:
                off_intervals.append((last_t, t))
        current_state = ev["type"]
        last_t = t

    if last_t < duration_s:
        if current_state == "ON":
            on_intervals.append((last_t, duration_s))
        else:
            off_intervals.append((last_t, duration_s))

    # Slice samples
    on_audio = []
    for s, e in on_intervals:
        on_audio.append(audio[int(s * sr) : int(e * sr)])
    off_audio = []
    for s, e in off_intervals:
        off_audio.append(audio[int(s * sr) : int(e * sr)])

    merged_on = np.concatenate(on_audio) if on_audio else np.array([])
    merged_off = np.concatenate(off_audio) if off_audio else np.array([])

    if len(merged_on) < 256 or len(merged_off) < 256:
        return {}

    on_freqs, on_psd = compute_welch_psd(merged_on, sr=sr, nperseg=1024)
    off_freqs, off_psd = compute_welch_psd(merged_off, sr=sr, nperseg=1024)

    source = meta.get("metadata", {}).get("source", wav_path.stem.split("_")[0])
    distance = meta.get("metadata", {}).get("distance_m", 1.0)

    row: dict[str, float | str] = {
        "file": wav_path.name,
        "source": source,
        "distance_m": distance,
    }

    for band_name, (f_low, f_high) in CANDIDATE_BANDS.items():
        p_on = compute_band_energy(on_freqs, on_psd, f_low, f_high)
        p_off = compute_band_energy(off_freqs, off_psd, f_low, f_high)
        snr_db = 10.0 * np.log10(max(p_on, 1e-12) / max(p_off, 1e-12))
        row[band_name] = round(snr_db, 2)

    return row


def generate_band_snr_report(data_dir: Path, output_file: Path) -> Path:
    """Run full band SNR analysis and output reports/band_snr.md."""
    pairs = build_synthetic_transitions_if_needed(data_dir)
    results: list[dict[str, float | str]] = []

    for wav, js in pairs:
        row = calculate_recording_band_snrs(wav, js)
        if row:
            results.append(row)

    # Determine Usable Bands: SNR >= 6 dB on >= 80% of recordings per source
    sources = sorted(list({str(r["source"]) for r in results}))
    usable_summary: dict[str, list[str]] = {}

    for src in sources:
        src_rows = [r for r in results if r["source"] == src]
        usable_summary[src] = []
        for band in CANDIDATE_BANDS:
            snr_values = [float(r[band]) for r in src_rows if band in r]
            if not snr_values:
                continue
            pass_rate = sum(1 for v in snr_values if v >= 6.0) / len(snr_values)
            if pass_rate >= 0.80:
                usable_summary[src].append(f"{band} (≥6dB on {pass_rate * 100:.0f}%)")

    output_file.parent.mkdir(parents=True, exist_ok=True)
    with output_file.open("w", encoding="utf-8") as f:
        f.write("# ON/OFF Band-SNR Analysis Report\n\n")
        f.write(
            "Evaluation of frequency band energy ratios "
            "($SNR = 10 \\log_{10}(E_{ON} / E_{OFF})$) across appliances and distances.\n\n"
        )
        f.write("## Usable Band Criterion\n")
        f.write(
            "- **Criterion**: A candidate frequency band is designated **usable** "
            "if it exhibits an ON/OFF SNR of **at least 6.0 dB** across "
            "**at least 80% of recordings** for that appliance source.\n\n"
        )

        f.write("## Usable Bands per Appliance\n\n")
        f.write("| Appliance Source | Usable Frequency Bands (SNR ≥ 6 dB on ≥ 80% clips) |\n")
        f.write("|---|---|\n")
        for src, bands in usable_summary.items():
            band_str = ", ".join(bands) if bands else "None meet ≥6 dB threshold"
            f.write(f"| {src} | {band_str} |\n")

        f.write("\n## Per-Recording Band SNR Measurements (dB)\n\n")
        headers = ["File", "Source", "Distance", *CANDIDATE_BANDS.keys()]
        f.write("| " + " | ".join(headers) + " |\n")
        f.write("| " + " | ".join(["---"] * len(headers)) + " |\n")
        for r in results:
            row_vals = [
                str(r["file"]),
                str(r["source"]),
                f"{r['distance_m']}m",
                *[f"{r.get(b, 0.0):.1f} dB" for b in CANDIDATE_BANDS.keys()],
            ]
            f.write("| " + " | ".join(row_vals) + " |\n")

    return output_file


def main() -> int:
    repo_root = get_repo_root()
    parser = argparse.ArgumentParser(
        description="Analyze ON/OFF band SNR for appliance recordings."
    )
    parser.add_argument("--data-dir", type=Path, default=repo_root / "data")
    parser.add_argument("--output", type=Path, default=repo_root / "reports" / "band_snr.md")
    args = parser.parse_args()

    report = generate_band_snr_report(args.data_dir, args.output)
    print(f"[OK] Band SNR analysis generated: {report}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
