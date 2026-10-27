from __future__ import annotations

from pathlib import Path

from echogrid_hum.report import generate_spectral_survey


def test_generate_spectral_survey_pipeline(tmp_path: Path):
    data_dir = tmp_path / "data"
    output_dir = tmp_path / "reports" / "spectra"

    readme_path = generate_spectral_survey(data_dir=data_dir, output_dir=output_dir)

    assert readme_path.exists()
    content = readme_path.read_text(encoding="utf-8")
    assert "Spectral Survey Report" in content
    assert "air_conditioner" in content
    assert "pump" in content
    assert "fan" in content

    # Verify PNG artifacts were created
    assert (output_dir / "air_conditioner_psd.png").exists()
    assert (output_dir / "air_conditioner_spectrogram.png").exists()
