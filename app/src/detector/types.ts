export enum DetectorState {
  ON = 'ON',
  PENDING_DROP = 'PENDING_DROP',
  OFF_DETECTED = 'OFF_DETECTED',
}

export interface DetectorParams {
  sample_rate: number;
  frame_size: number;
  hop_size: number;
  bands: [number, number][];
  drop_threshold_db: number;
  hysteresis_db: number;
  min_sustained_drop_s: number;
  alpha_noise: number;
  alpha_signal: number;
}

export interface FrameResult {
  timestamp_s: number;
  state: DetectorState;
  event_triggered: boolean;
  current_energy_db: number;
  running_baseline_db: number;
  drop_duration_s: number;
}
