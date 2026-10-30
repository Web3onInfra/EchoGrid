import { defineConfig, devices } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const audioFixturePath = path.resolve(__dirname, 'e2e/fixtures/hum_drop.wav');

export default defineConfig({
  testDir: './e2e',
  timeout: 30000,
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
    viewport: { width: 390, height: 844 }, // Mobile-first viewport (iPhone 14)
  },
  projects: [
    {
      name: 'chromium-fake-audio',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 390, height: 844 },
        launchOptions: {
          args: [
            '--use-fake-ui-for-media-stream',
            '--use-fake-device-for-media-stream',
            `--use-file-for-fake-audio-capture=${audioFixturePath}`,
            '--autoplay-policy=no-user-gesture-required',
          ],
        },
      },
    },
  ],
  webServer: {
    command:
      'node --max-old-space-size=4096 --max-semi-space-size=64 ./node_modules/vite/bin/vite.js preview --port 5173',
    cwd: __dirname,
    port: 5173,
    reuseExistingServer: true,
  },
});
