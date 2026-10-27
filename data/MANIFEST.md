# Dataset Manifest

This manifest documents the acoustic datasets, folder layout, and reference entries used across Phase 0 research.

## Directory Layout
- `data/public/`: Automated, checksummed public dataset subsets (git-ignored).
  - `data/public/urbansound8k/`: `air_conditioner` (target) and negatives (`children_playing`, `street_music`, `dog_bark`, `engine_idling`).
  - `data/public/mimii/`: Normal operation sounds for `pump` and `fan`.
  - `data/public/samples/`: Lightweight synthetic or truncated samples for fast CI validation.
- `data/own/`: Private phone/browser captures recorded via `research/capture/index.html` (git-ignored).

## Public Dataset Sources & Licensing
| Dataset | Subset / Class | License | Checksum / Source |
|---|---|---|---|
| UrbanSound8K | `air_conditioner`, `children_playing`, `dog_bark`, `engine_idling`, `street_music` | CC BY-NC 4.0 | Zenodo / Official Release Archive |
| MIMII (DCASE2020) | `pump` (id 00 normal), `fan` (id 00 normal) | CC BY 4.0 | Zenodo DCASE 2020 Task 2 Dataset |

## Freesound Refrigerator Hum Clips (Manual CC0 Placeholders)
User-provided CC0 Freesound audio clips can be placed into `data/public/freesound_fridge/`:

| Clip ID | Title / Description | Duration | Freesound URL | License |
|---|---|---|---|---|
| `fs_fridge_01` | Domestic Refrigerator Compressor Steady Hum | ~60s | https://freesound.org/s/placeholder1/ | CC0 1.0 |
| `fs_fridge_02` | Kitchen Fridge Motor Cycle-Down Transition | ~45s | https://freesound.org/s/placeholder2/ | CC0 1.0 |
| `fs_fridge_03` | Freezer Chest Hum with Mains Tone | ~60s | https://freesound.org/s/placeholder3/ | CC0 1.0 |
