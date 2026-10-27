from __future__ import annotations

from pathlib import Path

from echogrid_hum.eval import run_evaluation_suite


def test_run_evaluation_suite(tmp_path: Path):
    data_dir = tmp_path / "data"
    output_file = tmp_path / "reports" / "eval.md"

    report_path = run_evaluation_suite(data_dir=data_dir, output_file=output_file)

    assert report_path.exists()
    content = report_path.read_text(encoding="utf-8")
    assert "Appliance Hum Power-Down Detector Evaluation" in content
    assert "Threshold Sweep Results" in content
    assert "Drop Threshold (dB)" in content
    assert "6.0 dB" in content
    assert "False Triggers / Hour" in content
