import { AudioPipelineProcessor } from './pipelineProcessor';
import { WorkerInMessage, WorkerOutMessage } from './pipelineTypes';

let processor: AudioPipelineProcessor | null = null;

function getOrCreateProcessor(): AudioPipelineProcessor {
  if (!processor) {
    processor = new AudioPipelineProcessor();
  }
  return processor;
}

self.onmessage = (event: MessageEvent<WorkerInMessage>) => {
  const msg = event.data;
  const proc = getOrCreateProcessor();

  switch (msg.type) {
    case 'PROCESS_AUDIO': {
      const results = proc.pushAndProcess(msg.samples);
      for (const res of results) {
        const out: WorkerOutMessage = {
          type: 'FRAME_PROCESSED',
          frameResult: res.frameResult,
          features: {
            totalBandEnergyDb: res.features.totalBandEnergyDb,
            bandEnergiesDb: res.features.bandEnergiesDb,
            fftMagnitudes: res.features.fftMagnitudes,
          },
          processingTimeMs: res.processingTimeMs,
        };
        // Post processed frame result
        (self as unknown as Worker).postMessage(out, [res.features.fftMagnitudes.buffer]);
      }
      break;
    }
    case 'RESET': {
      proc.reset();
      const ack: WorkerOutMessage = { type: 'RESET_ACK' };
      self.postMessage(ack);
      break;
    }
    case 'SET_PARAMS': {
      processor = new AudioPipelineProcessor(msg.params);
      const ack: WorkerOutMessage = { type: 'RESET_ACK' };
      self.postMessage(ack);
      break;
    }
  }
};
