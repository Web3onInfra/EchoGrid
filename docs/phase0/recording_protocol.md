# Phase 0 Acoustic Recording Protocol

This protocol defines the standardized procedure for gathering real acoustic appliance recordings using the browser capture page (`research/capture/index.html`) on mobile phones and laptops.

## 1. Objectives
- Evaluate whether browser Web Audio API pipelines preserve low-frequency appliance hums (20–500 Hz).
- Measure attenuation across distances (0.5 m, 2 m, 4 m).
- Collect ground-truth ON / OFF transition markers for detector validation.

## 2. Target Sources & Distances
| Appliance / Source | Distances | Minimum Sequence Duration |
|---|---|---|
| Refrigerator / Freezer | 0.5 m, 2.0 m, 4.0 m | 60s Compressor ON -> Cycle OFF -> 60s ON |
| Air Conditioner / Heat Pump | 0.5 m, 2.0 m, 4.0 m | 60s Fan/Compressor ON -> Turn OFF -> 60s ON |
| Water Pump / Sump Pump | 0.5 m, 2.0 m, 4.0 m | 60s Pump ON -> Turn OFF -> 60s ON |
| Room / Ventilation Fan | 0.5 m, 2.0 m, 4.0 m | 60s High ON -> Turn OFF -> 60s ON |
| Background Negative (Speech / TV) | 1.0 m - 3.0 m | 60s Conversational speech / TV audio (no appliance) |

## 3. Recording Procedure
1. Open `research/capture/index.html` in mobile Chrome, Safari, or desktop browser via local file or static server.
2. Ensure browser permissions are granted for microphone access.
3. Verify that browser processing flags (`echoCancellation`, `noiseSuppression`, `autoGainControl`) are reported as disabled in the diagnostics panel.
4. Position the device microphone facing the appliance at the designated distance (0.5 m, 2.0 m, or 4.0 m).
5. Press **Start Recording**. The initial state is marked as `ON`.
6. Run for at least 60 seconds of stable appliance operation.
7. Switch off or power down the appliance and immediately press **Mark OFF**.
8. Record at least 30–60 seconds in the OFF state.
9. Power the appliance back on and immediately press **Mark ON**.
10. Press **Stop & Download**. Move the downloaded `.wav` and `.json` files to `data/own/`.

## 4. Sidecar JSON Schema
Every recording produces a companion `.json` file alongside `<name>.wav`:

```json
{
  "audio_file": "refrigerator_0.5m_2026-10-28T00-00-00-000Z.wav",
  "sample_rate": 48000,
  "duration_s": 154.21,
  "settings": {
    "audioContextSampleRate": 48000,
    "trackSettings": {
      "autoGainControl": false,
      "channelCount": 1,
      "echoCancellation": false,
      "noiseSuppression": false,
      "sampleRate": 48000
    }
  },
  "metadata": {
    "source": "refrigerator",
    "distance_m": 0.5
  },
  "events": [
    { "type": "ON", "timestamp_s": 0.0 },
    { "type": "OFF", "timestamp_s": 65.2 },
    { "type": "ON", "timestamp_s": 122.4 }
  ]
}
```

## 5. Automated Validation
Run the validator after recording:
```bash
uv run python research/hum_detection/validate_recordings.py
```
All valid pairs will be validated against duration, audio format, and chronological event sequence.
