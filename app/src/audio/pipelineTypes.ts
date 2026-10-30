import { FrameResult, DetectorParams } from '../detector/types';
import { FrameFeatures } from './features';

export type WorkerInMessage =
  | { type: 'PROCESS_AUDIO'; samples: Float32Array }
  | { type: 'RESET' }
  | { type: 'SET_PARAMS'; params: Partial<DetectorParams> };

export type WorkerOutMessage =
  | {
      type: 'FRAME_PROCESSED';
      frameResult: FrameResult;
      features: {
        totalBandEnergyDb: number;
        bandEnergiesDb: number[];
        fftMagnitudes: Float32Array;
      };
      processingTimeMs: number;
    }
  | { type: 'RESET_ACK' };

export interface PipelineFrameEvent {
  frameResult: FrameResult;
  features: FrameFeatures;
  processingTimeMs: number;
}

export type PipelineListener = (event: PipelineFrameEvent) => void;
