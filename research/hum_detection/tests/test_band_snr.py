from __future__ import annotations

from pathlib import Path

from echogrid_hum.band_snr import generate_band_snr_report


def test_generate_band_snr_report(tmp_path: Path):
    data_dir = tmp_path / "data"
    output_file = tmp_path / "reports" / "band_snr.md"

    report_path = generate_band_snr_report(data_dir=data_dir, output_file=output_file)

    assert report_path.exists()
    content = report_path.read_text(encoding="utf-8")
    assert "ON/OFF Band-SNR Analysis Report" in content
    assert "Usable Band Criterion" in content
    assert "Usable Bands per Appliance" in content
    assert "air_conditioner" in content
    assert "pump" in content
    assert "fan" in content
