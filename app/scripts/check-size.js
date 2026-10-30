import fs from 'fs';
import path from 'path';
import zlib from 'zlib';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const assetsDir = path.resolve(__dirname, '../dist/assets');
if (!fs.existsSync(assetsDir)) {
  console.error('dist/assets not found. Run npm run build first.');
  process.exit(1);
}

const files = fs.readdirSync(assetsDir);
const indexFile = files.find((f) => f.startsWith('index-') && f.endsWith('.js'));
if (!indexFile) {
  console.error('No index-*.js found in dist/assets');
  process.exit(1);
}

const raw = fs.readFileSync(path.join(assetsDir, indexFile));
const gz = zlib.gzipSync(raw).length;
const gzKb = (gz / 1024).toFixed(2);
console.log(`Initial JS chunk (${indexFile}): ${gzKb} KB gzipped (Budget: <= 200 KB)`);

if (gz > 200 * 1024) {
  console.error(`FAILURE: Initial bundle ${gzKb} KB exceeds 200 KB gzipped budget!`);
  process.exit(1);
}
console.log('SUCCESS: Initial bundle size within 200 KB budget.');
