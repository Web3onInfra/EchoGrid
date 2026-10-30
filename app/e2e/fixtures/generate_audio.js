import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Generates an 8-second 16 kHz mono 16-bit PCM WAV:
 * - 0.0s - 4.0s: 60 Hz mains hum + 120 Hz harmonic (~0.2 amplitude)
 * - 4.0s - 8.0s: silence (amplitude 0.0)
 */
export function generateHumDropWav(outputPath) {
  const sampleRate = 16000;
  const numChannels = 1;
  const bitsPerSample = 16;
  const durationS = 8.0;
  const numSamples = Math.floor(sampleRate * durationS);

  const blockAlign = (numChannels * bitsPerSample) / 8;
  const byteRate = sampleRate * blockAlign;
  const dataSize = numSamples * blockAlign;
  const headerSize = 44;
  const totalFileSize = headerSize + dataSize;

  const buffer = Buffer.alloc(totalFileSize);

  // RIFF header
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(totalFileSize - 8, 4);
  buffer.write('WAVE', 8);

  // 'fmt ' chunk
  buffer.write('fmt ', 12);
  buffer.writeUInt32LE(16, 16); // subchunk1 size (16 for PCM)
  buffer.writeUInt16LE(1, 20); // audio format (1 = PCM)
  buffer.writeUInt16LE(numChannels, 22);
  buffer.writeUInt32LE(sampleRate, 24);
  buffer.writeUInt32LE(byteRate, 28);
  buffer.writeUInt16LE(blockAlign, 32);
  buffer.writeUInt16LE(bitsPerSample, 34);

  // 'data' chunk
  buffer.write('data', 36);
  buffer.writeUInt32LE(dataSize, 40);

  let offset = 44;
  for (let i = 0; i < numSamples; i++) {
    const t = i / sampleRate;
    let sample = 0;

    if (t < 4.0) {
      // 60 Hz primary + 120 Hz secondary hum
      const hum60 = Math.sin(2 * Math.PI * 60 * t) * 0.15;
      const hum120 = Math.sin(2 * Math.PI * 120 * t) * 0.08;
      sample = hum60 + hum120;
    } else {
      sample = 0.0;
    }

    // Clamp to 16-bit signed integer [-32768, 32767]
    const clamped = Math.max(-1.0, Math.min(1.0, sample));
    const int16 = Math.floor(clamped * 32767);
    buffer.writeInt16LE(int16, offset);
    offset += 2;
  }

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, buffer);
  return outputPath;
}

if (process.argv[1] === __filename) {
  const dest = path.join(__dirname, 'hum_drop.wav');
  generateHumDropWav(dest);
  console.log(`Generated fixture WAV at ${dest}`);
}
