# ON/OFF Band-SNR Analysis Report

Evaluation of frequency band energy ratios ($SNR = 10 \log_{10}(E_{ON} / E_{OFF})$) across appliances and distances.

## Usable Band Criterion
- **Criterion**: A candidate frequency band is designated **usable** if it exhibits an ON/OFF SNR of **at least 6.0 dB** across **at least 80% of recordings** for that appliance source.

## Usable Bands per Appliance

| Appliance Source | Usable Frequency Bands (SNR ≥ 6 dB on ≥ 80% clips) |
|---|---|
| air_conditioner | 50Hz_fundamental (≥6dB on 100%), 60Hz_fundamental (≥6dB on 100%), 120Hz_harmonic (≥6dB on 100%), sub_hum_rumble (≥6dB on 100%) |
| fan | 120Hz_harmonic (≥6dB on 100%) |
| pump | 50Hz_fundamental (≥6dB on 100%), 60Hz_fundamental (≥6dB on 100%), 120Hz_harmonic (≥6dB on 100%) |

## Per-Recording Band SNR Measurements (dB)

| File | Source | Distance | 50Hz_fundamental | 60Hz_fundamental | 100Hz_harmonic | 120Hz_harmonic | 150Hz_harmonic | 180Hz_harmonic | sub_hum_rumble |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| air_conditioner_0.5m_sample.wav | air_conditioner | 0.5m | 48.5 dB | 45.1 dB | 0.0 dB | 26.3 dB | 0.0 dB | 0.0 dB | 39.2 dB |
| air_conditioner_2.0m_sample.wav | air_conditioner | 2.0m | 35.9 dB | 33.1 dB | 0.0 dB | 14.4 dB | 0.0 dB | 0.0 dB | 27.1 dB |
| air_conditioner_4.0m_sample.wav | air_conditioner | 4.0m | 30.4 dB | 26.9 dB | 0.0 dB | 8.7 dB | 0.0 dB | 0.0 dB | 22.4 dB |
| pump_0.5m_sample.wav | pump | 0.5m | 44.9 dB | 49.4 dB | 0.0 dB | 41.5 dB | 0.0 dB | 0.0 dB | 19.0 dB |
| pump_2.0m_sample.wav | pump | 2.0m | 31.8 dB | 36.6 dB | 0.0 dB | 29.7 dB | 0.0 dB | 0.0 dB | 7.0 dB |
| pump_4.0m_sample.wav | pump | 4.0m | 26.9 dB | 30.6 dB | 0.0 dB | 23.8 dB | 0.0 dB | 0.0 dB | 4.0 dB |
| fan_0.5m_sample.wav | fan | 0.5m | 1.9 dB | 5.2 dB | 0.0 dB | 46.7 dB | 0.0 dB | 0.0 dB | 1.4 dB |
| fan_2.0m_sample.wav | fan | 2.0m | -0.4 dB | 0.6 dB | 0.0 dB | 33.9 dB | 0.0 dB | 0.0 dB | 0.3 dB |
| fan_4.0m_sample.wav | fan | 4.0m | -0.3 dB | 0.3 dB | 0.0 dB | 28.4 dB | 0.0 dB | 0.0 dB | -0.3 dB |
