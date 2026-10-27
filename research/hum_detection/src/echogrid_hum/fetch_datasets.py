"""Dataset fetching and checksum verification for Phase 0 hum detection."""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

TARGET_CLASSES = {
    "urbansound8k": [
        "air_conditioner",
        "children_playing",
        "street_music",
        "dog_bark",
        "engine_idling",
    ],
    "mimii": [
        "pump",
        "fan",
    ],
}

KNOWN_CHECKSUMS: dict[str, str] = {
    # Populated with SHA-256 checksums for verified assets
}


def compute_sha256(file_path: Path) -> str:
    """Compute the hex-encoded SHA-256 hash of a file."""
    hasher = hashlib.sha256()
    with file_path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def generate_synthetic_sample(
    output_path: Path,
    category: str,
    duration_s: float = 3.0,
    sr: int = 16000,
    seed: int = 42,
) -> str:
    """Generate a deterministic synthetic audio sample for CI-friendly test runs."""
    rng = np.random.default_rng(seed)
    t = np.linspace(0, duration_s, int(sr * duration_s), endpoint=False)

    if category == "air_conditioner":
        # 50 Hz fundamental + 100/150 Hz harmonics + low-frequency rumble + mild noise
        sig = (
            0.3 * np.sin(2 * np.pi * 50 * t)
            + 0.2 * np.sin(2 * np.pi * 100 * t)
            + 0.1 * np.sin(2 * np.pi * 150 * t)
            + 0.05 * rng.standard_normal(len(t))
        )
    elif category == "pump":
        # 60 Hz fundamental + 120/180 Hz harmonics + periodic mechanical pulse
        pulse = 0.1 * np.sin(2 * np.pi * 4 * t)
        sig = (
            (0.35 + pulse) * np.sin(2 * np.pi * 60 * t)
            + 0.15 * np.sin(2 * np.pi * 120 * t)
            + 0.04 * rng.standard_normal(len(t))
        )
    elif category == "fan":
        # Broadband noise filtered / peaked around 120 Hz blade-pass frequency
        sig = 0.25 * np.sin(2 * np.pi * 120 * t) + 0.15 * rng.standard_normal(len(t))
    elif category == "engine_idling":
        # 25 Hz sub-hum with irregular low-frequency noise
        sig = 0.3 * np.sin(2 * np.pi * 25 * t) + 0.1 * rng.standard_normal(len(t))
    else:
        # Negative / general background noise (speech/dog/music proxy)
        sig = (
            0.15 * np.sin(2 * np.pi * 440 * t)
            + 0.1 * np.sin(2 * np.pi * 880 * t)
            + 0.1 * rng.standard_normal(len(t))
        )

    # Normalize to prevent clipping
    max_val = np.max(np.abs(sig))
    if max_val > 0:
        sig = (sig / max_val * 0.7).astype(np.float32)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(output_path, sig, sr, subtype="PCM_16")
    return compute_sha256(output_path)


def setup_sample_dataset(base_dir: Path) -> dict[str, str]:
    """Generate all target and negative sample clips under data/public/samples/."""
    results = {}
    sample_dir = base_dir / "samples"

    idx = 0
    for dataset, classes in TARGET_CLASSES.items():
        for cls_name in classes:
            out_file = sample_dir / dataset / f"{cls_name}_sample.wav"
            sha = generate_synthetic_sample(out_file, category=cls_name, seed=42 + idx)
            results[str(out_file.relative_to(base_dir))] = sha
            idx += 1

    return results


def verify_dataset_checksums(manifest: dict[str, str], base_dir: Path) -> bool:
    """Verify that all files in the manifest match their recorded checksums."""
    all_valid = True
    for rel_path, expected_hash in manifest.items():
        file_path = base_dir / rel_path
        if not file_path.exists():
            print(f"[MISSING] {rel_path}")
            all_valid = False
            continue
        actual_hash = compute_sha256(file_path)
        if actual_hash != expected_hash:
            print(f"[HASH MISMATCH] {rel_path}: expected {expected_hash}, got {actual_hash}")
            all_valid = False
        else:
            print(f"[OK] {rel_path}")
    return all_valid


def get_repo_root() -> Path:
    """Find repository root by looking for .git or Cargo.toml."""
    current = Path.cwd().resolve()
    for p in [current, *current.parents]:
        if (p / ".git").exists() or (p / "Cargo.toml").exists():
            return p
    return current


def main() -> int:
    default_out = get_repo_root() / "data" / "public"
    parser = argparse.ArgumentParser(description="Fetch and verify research audio datasets.")
    parser.add_argument(
        "--sample",
        action="store_true",
        help="Generate a tiny CI-friendly sample dataset of all target and negative classes.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=default_out,
        help="Target base directory for public audio datasets.",
    )
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Verify checksums of existing files without downloading.",
    )
    args = parser.parse_args()

    out_dir = args.out_dir.resolve()
    print(f"Dataset root: {out_dir}")

    if args.sample:
        print("Generating CI-friendly sample dataset...")
        manifest = setup_sample_dataset(out_dir)
        print(f"Generated {len(manifest)} sample files.")
        valid = verify_dataset_checksums(manifest, out_dir)
        return 0 if valid else 1

    if args.verify_only:
        print("Verifying known dataset checksums...")
        valid = verify_dataset_checksums(KNOWN_CHECKSUMS, out_dir)
        return 0 if valid else 1

    print("Full remote download requires external network fetch.")
    print("Run with --sample for CI validation or see data/MANIFEST.md for manual placement.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
