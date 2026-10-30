#!/usr/bin/env node

/**
 * EchoGrid Live Smoke Test Tooling
 * Tests connectivity, CORS headers, contract configuration, and PWA assets against live deployed PWA & Oracle.
 * Usage: node app/scripts/smoke-test.js [target-origin] [oracle-url] [--skip-oracle] [--skip-pwa]
 */

import path from 'path';
import { fileURLToPath } from 'url';

export function normalizeUrl(rawUrl, defaultProtocol = 'http:') {
  if (!rawUrl) return '';
  let urlStr = rawUrl.trim();
  if (!/^https?:\/\//i.test(urlStr)) {
    urlStr = `${defaultProtocol}//${urlStr}`;
  }
  return urlStr.replace(/\/$/, '');
}

export async function runCheck(name, fn) {
  try {
    await fn();
    console.log(`[CHECK] ${name}... PASS`);
    return true;
  } catch (err) {
    console.log(`[CHECK] ${name}... FAIL\n  --> ${err.message}`);
    return false;
  }
}

export async function fetchJson(url, options = {}) {
  const headers = { Connection: 'close', ...(options.headers || {}) };
  const res = await fetch(url, { ...options, headers });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  }
  const contentType = res.headers.get('content-type') || '';
  if (!contentType.includes('application/json') && !contentType.includes('+json')) {
    throw new Error(`Expected JSON response, got ${contentType || 'unknown content-type'}`);
  }
  return res.json();
}

export async function checkOracleHealth(oracleUrl) {
  const base = normalizeUrl(oracleUrl);
  const data = await fetchJson(`${base}/api/v1/health`);
  if (data.status !== 'ok') throw new Error(`Unexpected status: ${data.status}`);
}

export async function checkOracleCors(oracleUrl, targetOrigin) {
  const base = normalizeUrl(oracleUrl);
  let origin = 'http://localhost:5173';
  if (targetOrigin) {
    try {
      origin = new URL(targetOrigin).origin;
    } catch {
      origin = normalizeUrl(targetOrigin);
    }
  }

  // 1. Simple request Origin check on GET /api/v1/health
  const res = await fetch(`${base}/api/v1/health`, {
    headers: { Origin: origin, Connection: 'close' },
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  const corsOrigin = res.headers.get('access-control-allow-origin');
  if (!corsOrigin) {
    throw new Error(
      `Oracle rejected Origin "${origin}" (no Access-Control-Allow-Origin header returned)`,
    );
  }
  if (corsOrigin !== '*' && corsOrigin !== origin) {
    throw new Error(`CORS header mismatch: expected "${origin}" or "*", got "${corsOrigin}"`);
  }

  // 2. Preflight OPTIONS check on POST /api/v1/proof
  const preflightRes = await fetch(`${base}/api/v1/proof`, {
    method: 'OPTIONS',
    headers: {
      Origin: origin,
      'Access-Control-Request-Method': 'POST',
      'Access-Control-Request-Headers': 'content-type,x-request-id',
      Connection: 'close',
    },
  });
  if (!preflightRes.ok) {
    throw new Error(`CORS preflight OPTIONS /api/v1/proof returned HTTP ${preflightRes.status}`);
  }
  const preflightCorsOrigin = preflightRes.headers.get('access-control-allow-origin');
  if (!preflightCorsOrigin || (preflightCorsOrigin !== '*' && preflightCorsOrigin !== origin)) {
    throw new Error(
      `Preflight CORS header mismatch: expected "${origin}" or "*", got "${preflightCorsOrigin}"`,
    );
  }
}

export async function checkOracleReady(oracleUrl) {
  const base = normalizeUrl(oracleUrl);
  const data = await fetchJson(`${base}/api/v1/ready`);
  if (data.rpc_healthy === false) {
    throw new Error(`RPC unhealthy: ${JSON.stringify(data)}`);
  }
}

export async function checkOracleConfig(oracleUrl) {
  const base = normalizeUrl(oracleUrl);
  const data = await fetchJson(`${base}/api/v1/config`);
  if (data.network !== 'testnet') {
    throw new Error(`Expected testnet, got: ${data.network}`);
  }
  if (typeof data.bounty_stroops !== 'number' || data.bounty_stroops <= 0) {
    throw new Error(`Invalid bounty_stroops: ${data.bounty_stroops}`);
  }
}

export async function checkOracleChallenge(oracleUrl) {
  const base = normalizeUrl(oracleUrl);
  const testWallet = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';
  const data = await fetchJson(`${base}/api/v1/challenge?wallet=${testWallet}`);
  if (!data.challenge_id || typeof data.challenge_id !== 'string') {
    throw new Error(`Invalid or missing challenge_id: ${JSON.stringify(data)}`);
  }
  if (!data.expires_at || typeof data.expires_at !== 'number') {
    throw new Error(`Invalid or missing expires_at: ${JSON.stringify(data)}`);
  }
}

export async function checkPwaIndex(targetOrigin) {
  const base = normalizeUrl(targetOrigin);
  const res = await fetch(`${base}/`, {
    headers: { Connection: 'close' },
  });
  if (!res.ok) {
    throw new Error(`Failed to load index page: HTTP ${res.status}`);
  }
  const contentType = res.headers.get('content-type') || '';
  if (contentType && !contentType.includes('text/html')) {
    throw new Error(`Expected text/html content-type for index root, got: ${contentType}`);
  }
  const html = await res.text();
  if (!html.includes('<div id="root">') && !html.includes('EchoGrid')) {
    throw new Error('Index HTML missing expected root element or title');
  }
}

export async function checkPwaManifest(targetOrigin) {
  const base = normalizeUrl(targetOrigin);
  const res = await fetch(`${base}/manifest.webmanifest`, {
    headers: { Connection: 'close' },
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch manifest: HTTP ${res.status}`);
  }
  const contentType = res.headers.get('content-type') || '';
  if (contentType.includes('text/html')) {
    throw new Error('Manifest returned HTML (SPA fallback) instead of JSON');
  }
  const manifest = await res.json();
  if (!manifest.name || !manifest.short_name) {
    throw new Error('Manifest missing name or short_name');
  }
  if (!manifest.start_url) {
    throw new Error('Manifest missing start_url');
  }
  if (!manifest.display) {
    throw new Error('Manifest missing display');
  }
  if (!Array.isArray(manifest.icons) || manifest.icons.length === 0) {
    throw new Error('Manifest missing icons array');
  }
}

export async function checkPwaIcons(targetOrigin) {
  const base = normalizeUrl(targetOrigin);
  const iconPaths = [
    '/icon.svg',
    '/pwa-192x192.png',
    '/pwa-512x512.png',
    '/pwa-maskable-512x512.png',
  ];
  for (const iconPath of iconPaths) {
    const res = await fetch(`${base}${iconPath}`, {
      headers: { Connection: 'close' },
    });
    if (!res.ok) {
      throw new Error(`Icon ${iconPath} returned HTTP ${res.status}`);
    }
    const contentType = res.headers.get('content-type') || '';
    if (contentType.includes('text/html')) {
      throw new Error(
        `Icon ${iconPath} returned HTML (SPA rewrite fallback); file missing on host`,
      );
    }
    const contentType = res.headers.get('content-type') || '';
    if (contentType.includes('text/html')) {
      throw new Error(`Icon ${iconPath} returned HTML (SPA rewrite fallback); file missing on host`);
    }
  }
}

export async function checkPwaServiceWorker(targetOrigin) {
  const base = normalizeUrl(targetOrigin);
  const res = await fetch(`${base}/sw.js`, {
    headers: { Connection: 'close' },
  });
  if (!res.ok) {
    throw new Error(`Service worker (sw.js) returned HTTP ${res.status}`);
  }
  const contentType = res.headers.get('content-type') || '';
  if (contentType.includes('text/html')) {
    throw new Error(
      'Service worker (sw.js) returned HTML (SPA rewrite fallback); script missing on host',
    );
  }
  const text = await res.text();
  if (
    !text.includes('workbox') &&
    !text.includes('addEventListener') &&
    !text.includes('precacheAndRoute')
  ) {
    throw new Error('Service worker (sw.js) does not contain valid ServiceWorker script content');
  }
}

export async function checkPwaHeaders(targetOrigin) {
  const base = normalizeUrl(targetOrigin);
  const res = await fetch(`${base}/`, {
    headers: { Connection: 'close' },
  });
  if (!res.ok) {
    throw new Error(`Failed to fetch index headers: HTTP ${res.status}`);
  }
  const permPolicy = res.headers.get('permissions-policy');
  const csp = res.headers.get('content-security-policy');
  const xfo = res.headers.get('x-frame-options');
  const xcto = res.headers.get('x-content-type-options');

  const isDeployedHttps = base.startsWith('https://');

  if (isDeployedHttps) {
    if (!permPolicy) {
      throw new Error('Deployed PWA missing Permissions-Policy header');
    }
    if (!csp) {
      throw new Error('Deployed PWA missing Content-Security-Policy header');
    }
  }

  if (permPolicy && !permPolicy.includes('microphone')) {
    throw new Error(`Permissions-Policy missing microphone permission: ${permPolicy}`);
  }
  if (csp && !csp.includes('connect-src')) {
    throw new Error(`CSP missing connect-src policy: ${csp}`);
  }
  if (xfo && xfo.toUpperCase() !== 'DENY' && xfo.toUpperCase() !== 'SAMEORIGIN') {
    throw new Error(`Unexpected X-Frame-Options: ${xfo}`);
  }
  if (xcto && xcto.toLowerCase() !== 'nosniff') {
    throw new Error(`Unexpected X-Content-Type-Options: ${xcto}`);
  }
}

export async function runSmokeTests(options = {}) {
  const targetOrigin = normalizeUrl(options.targetOrigin || 'http://localhost:5173');
  const oracleUrl = normalizeUrl(
    options.oracleUrl || process.env.VITE_ORACLE_URL || 'http://localhost:8080',
  );
  const skipOracle = options.skipOracle ?? false;
  const skipPwa = options.skipPwa ?? false;

  console.log('====================================================');
  console.log('EchoGrid Live Smoke Test Runner');
  console.log(`Target PWA:    ${skipPwa ? '(SKIPPED)' : targetOrigin}`);
  console.log(`Oracle Target: ${skipOracle ? '(SKIPPED)' : oracleUrl}`);
  console.log('====================================================\n');

  let passed = 0;
  let total = 0;

  if (!skipOracle) {
    total++;
    if (
      await runCheck('Oracle /api/v1/health responds with status ok', () =>
        checkOracleHealth(oracleUrl),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('Oracle responds with valid CORS headers for PWA target', () =>
        checkOracleCors(oracleUrl, targetOrigin),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('Oracle /api/v1/ready self-check report responds', () =>
        checkOracleReady(oracleUrl),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('Oracle /api/v1/config returns testnet network params', () =>
        checkOracleConfig(oracleUrl),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('Oracle /api/v1/challenge issues wallet-bound nonce', () =>
        checkOracleChallenge(oracleUrl),
      )
    ) {
      passed++;
    }
  }

  if (!skipPwa) {
    total++;
    if (
      await runCheck('PWA index root returns HTTP 200 and valid HTML', () =>
        checkPwaIndex(targetOrigin),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('PWA webmanifest exists and contains valid metadata', () =>
        checkPwaManifest(targetOrigin),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('PWA icons (SVG & PNG 192/512/maskable) exist and return HTTP 200', () =>
        checkPwaIcons(targetOrigin),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('PWA service worker (sw.js) exists and returns HTTP 200', () =>
        checkPwaServiceWorker(targetOrigin),
      )
    ) {
      passed++;
    }

    total++;
    if (
      await runCheck('PWA security headers conform to CSP & Permissions-Policy', () =>
        checkPwaHeaders(targetOrigin),
      )
    ) {
      passed++;
    }
  }

  console.log('\n----------------------------------------------------');
  console.log(`Summary: ${passed} / ${total} checks passed.`);
  return { passed, total, success: total > 0 && passed === total };
}

export function isDirectExecution() {
  if (!process.argv[1]) return false;
  try {
    const thisFile = fileURLToPath(import.meta.url);
    const invokedFile = path.resolve(process.argv[1]);
    return thisFile === invokedFile;
  } catch {
    return false;
  }
}

export function parseCliArgs(argv = process.argv.slice(2)) {
  const skipOracle = argv.includes('--skip-oracle');
  const skipPwa = argv.includes('--skip-pwa');
  const showHelp = argv.includes('--help') || argv.includes('-h');

  const positional = argv.filter((a) => !a.startsWith('-'));

  let targetOrigin = '';
  let oracleUrl = '';

  if (skipPwa && !skipOracle) {
    oracleUrl = positional[0] || process.env.VITE_ORACLE_URL || 'http://localhost:8080';
    targetOrigin = 'http://localhost:5173';
  } else if (skipOracle && !skipPwa) {
    targetOrigin = positional[0] || 'http://localhost:5173';
    oracleUrl = process.env.VITE_ORACLE_URL || 'http://localhost:8080';
  } else {
    targetOrigin = positional[0] || 'http://localhost:5173';
    oracleUrl = positional[1] || process.env.VITE_ORACLE_URL || 'http://localhost:8080';
  }

  return { targetOrigin, oracleUrl, skipOracle, skipPwa, showHelp };
}

// CLI execution check
if (isDirectExecution()) {
  const { targetOrigin, oracleUrl, skipOracle, skipPwa, showHelp } = parseCliArgs();

  if (showHelp) {
    console.log(`
EchoGrid Live Smoke Test Tooling
Usage:
  node app/scripts/smoke-test.js [target-origin] [oracle-url] [options]

Options:
  --skip-oracle   Skip Oracle API connectivity & CORS checks
  --skip-pwa      Skip PWA assets and security header checks
  --help, -h      Show this help message

Examples:
  node app/scripts/smoke-test.js https://echogrid-app.pages.dev https://oracle.echogrid.org
  node app/scripts/smoke-test.js http://localhost:4173 --skip-oracle
  node app/scripts/smoke-test.js --skip-pwa http://localhost:8080
`);
    process.exitCode = 0;
  } else {
    runSmokeTests({ targetOrigin, oracleUrl, skipOracle, skipPwa })
      .then(({ success }) => {
        if (!success) {
          console.error('Smoke tests finished with failures.');
          process.exitCode = 1;
        } else {
          console.log('Smoke tests completed successfully.');
          process.exitCode = 0;
        }
      })
      .catch((err) => {
        console.error('Fatal smoke test runner error:', err);
        process.exitCode = 1;
      });
  }
}
