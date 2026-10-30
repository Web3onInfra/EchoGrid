import defaultParamsJson from '../../../research/hum_detection/detector_params.json';
import { DetectorParams, DetectorState, FrameResult } from './types';

export const defaultDetectorParams: DetectorParams = defaultParamsJson as DetectorParams;

export class StreamingPowerDownDetector {
  public readonly params: DetectorParams;
  public state: DetectorState = DetectorState.ON;
  public framesProcessed: number = 0;
  public runningBaselineDb: number | null = null;
  public sustainedDropS: number = 0.0;
  public hasTriggered: boolean = false;

  constructor(params?: Partial<DetectorParams>) {
    this.params = { ...defaultDetectorParams, ...params };
    this.reset();
  }

  public reset(): void {
    this.state = DetectorState.ON;
    this.framesProcessed = 0;
    this.runningBaselineDb = null;
    this.sustainedDropS = 0.0;
    this.hasTriggered = false;
  }

  public processEnergyFrame(energyDb: number): FrameResult {
    const frameTimeS = this.framesProcessed * (this.params.hop_size / this.params.sample_rate);
    const frameDurationS = this.params.hop_size / this.params.sample_rate;
    this.framesProcessed += 1;

    if (this.runningBaselineDb === null) {
      this.runningBaselineDb = energyDb;
    }

    let eventTriggered = false;
    const dropThreshold = this.runningBaselineDb - this.params.drop_threshold_db;
    const recoveryThreshold = dropThreshold + this.params.hysteresis_db;

    if (energyDb < dropThreshold) {
      this.sustainedDropS += frameDurationS;
      if (this.sustainedDropS >= this.params.min_sustained_drop_s) {
        if (this.state !== DetectorState.OFF_DETECTED) {
          this.state = DetectorState.OFF_DETECTED;
          if (!this.hasTriggered) {
            eventTriggered = true;
            this.hasTriggered = true;
          }
        }
      } else {
        if (this.state === DetectorState.ON) {
          this.state = DetectorState.PENDING_DROP;
        }
      }
    } else if (energyDb >= recoveryThreshold) {
      this.sustainedDropS = 0.0;
      this.state = DetectorState.ON;
      this.runningBaselineDb =
        (1.0 - this.params.alpha_signal) * this.runningBaselineDb +
        this.params.alpha_signal * energyDb;
    }

    return {
      timestamp_s: Math.round(frameTimeS * 1000) / 1000,
      state: this.state,
      event_triggered: eventTriggered,
      current_energy_db: Math.round(energyDb * 100) / 100,
      running_baseline_db: Math.round((this.runningBaselineDb ?? 0) * 100) / 100,
      drop_duration_s: Math.round(this.sustainedDropS * 1000) / 1000,
    };
  }
}
