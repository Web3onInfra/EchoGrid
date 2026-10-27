from __future__ import annotations

from pathlib import Path

import soundfile as sf

from echogrid_hum.fetch_datasets import (
    compute_sha256,
    generate_synthetic_sample,
    setup_sample_dataset,
    verify_dataset_checksums,
)


def test_generate_synthetic_sample(tmp_path: Path):
    target = tmp_path / "ac_test.wav"
    hash_1 = generate_synthetic_sample(target, category="air_conditioner", duration_s=1.0, seed=42)

    assert target.exists()
    data, sr = sf.read(target)
    assert sr == 16000
    assert len(data) == 16000

    # Determinism check
    target_2 = tmp_path / "ac_test_2.wav"
    hash_2 = generate_synthetic_sample(
        target_2, category="air_conditioner", duration_s=1.0, seed=42
    )
    assert hash_1 == hash_2
    assert compute_sha256(target) == hash_1


def test_setup_sample_dataset_and_verify(tmp_path: Path):
    manifest = setup_sample_dataset(tmp_path)
    assert len(manifest) == 7  # 5 urbansound8k classes + 2 mimii classes

    # All generated files must pass verification
    assert verify_dataset_checksums(manifest, tmp_path) is True

    # Mutated file must fail verification
    first_key = next(iter(manifest.keys()))
    mutated_file = tmp_path / first_key
    with mutated_file.open("ab") as f:
        f.write(b"corruption")

    assert verify_dataset_checksums(manifest, tmp_path) is False


def test_main_sample_flag(tmp_path: Path, monkeypatch):
    from echogrid_hum.fetch_datasets import main

    test_args = ["fetch_datasets", "--sample", "--out-dir", str(tmp_path)]
    monkeypatch.setattr("sys.argv", test_args)
    exit_code = main()
    assert exit_code == 0
    assert (tmp_path / "samples" / "urbansound8k" / "air_conditioner_sample.wav").exists()
