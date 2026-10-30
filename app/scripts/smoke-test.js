#!/usr/bin/env node

/**
 * EchoGrid Live Smoke Test Tooling
 * Tests connectivity, CORS headers, contract configuration, and PWA assets against live deployed PWA & Oracle.
 * Usage: node app/scripts/smoke-test.js [target-origin] [oracle-url] [--skip-oracle] [--skip-pwa]
 */

export async function runCheck(name, fn) {
  process.stdout.write(`[CHECK] ${name}... `);
  try {
    await fn();
    console.log('PASS');
    return true;
  } catch (err) {
    console.log(`FAIL\n  --> ${err.message}`);
    return false;
  }
}

export async function fetchJson(url) {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  }
  return res.json();
}

export async function checkOracleHealth(oracleUrl) {
  const base = oracleUrl.replace(/\/$/, '');
  const res = await fetch(`${base}/api/v1/health`);
  if (!res.ok) throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  const data = await res.json();
  if (data.status !== 'ok') throw new Error(`Unexpected status: ${data.status}`);
}

export async function checkOracleCors(oracleUrl, targetOrigin) {
  const base = oracleUrl.replace(/\/$/, '');
  const origin = (targetOrigin || 'http://localhost:5173').replace(/\/$/, '');
  const res = await fetch(`${base}/api/v1/health`, {
    headers: { Origin: origin },
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
}

export async function checkOracleReady(oracleUrl) {
  const base = oracleUrl.replace(/\/$/, '');
  const data = await fetchJson(`${base}/api/v1/ready`);
  if (data.rpc_healthy === false) {
    throw new Error(`RPC unhealthy: ${JSON.stringify(data)}`);
  }
}

export async function checkOracleConfig(oracleUrl) {
  const base = oracleUrl.replace(/\/$/, '');
  const data = await fetchJson(`${base}/api/v1/config`);
  if (data.network !== 'testnet') {
    throw new Error(`Expected testnet, got: ${data.network}`);
  }
  if (typeof data.bounty_stroops !== 'number' || data.bounty_stroops <= 0) {
    throw new Error(`Invalid bounty_stroops: ${data.bounty_stroops}`);
  }
}

export async function checkOracleChallenge(oracleUrl) {
  const base = oracleUrl.replace(/\/$/, '');
  const testWallet = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';
  const data = await fetchJson(`${base}/api/v1/challenge?wallet=${testWallet}`);
  if (!data.challenge_id || !data.expires_at) {
    throw new Error(`Missing challenge fields: ${JSON.stringify(data)}`);
  }
}

export async function checkPwaIndex(targetOrigin) {
  const base = targetOrigin.replace(/\/$/, '');
  const res = await fetch(`${base}/`);
  if (!res.ok) {
    throw new Error(`Failed to load index page: HTTP ${res.status}`);
  }
  const html = await res.text();
  if (!html.includes('<div id="root">') && !html.includes('EchoGrid')) {
    throw new Error('Index HTML missing expected root element or title');
  }
}

export async function checkPwaManifest(targetOrigin) {
  const base = targetOrigin.replace(/\/$/, '');
  const res = await fetch(`${base}/manifest.webmanifest`);
  if (!res.ok) {
    throw new Error(`Failed to fetch manifest: HTTP ${res.status}`);
  }
  const manifest = await res.json();
  if (!manifest.name || !manifest.short_name) {
    throw new Error('Manifest missing name or short_name');
  }
  if (!Array.isArray(manifest.icons) || manifest.icons.length === 0) {
    throw new Error('Manifest missing icons array');
  }
}

export async function checkPwaIcons(targetOrigin) {
  const base = targetOrigin.replace(/\/$/, '');
  const iconPaths = ['/icon.svg', '/pwa-192x192.png', '/pwa-512x512.png'];
  for (const iconPath of iconPaths) {
    const res = await fetch(`${base}${iconPath}`);
    if (!res.ok) {
      throw new Error(`Icon ${iconPath} returned HTTP ${res.status}`);
    }
  }
}

export async function checkPwaServiceWorker(targetOrigin) {
  const base = targetOrigin.replace(/\/$/, '');
  const res = await fetch(`${base}/sw.js`);
  if (!res.ok) {
    throw new Error(`Service worker (sw.js) returned HTTP ${res.status}`);
  }
}

export async function checkPwaHeaders(targetOrigin) {
  const base = targetOrigin.replace(/\/$/, '');
  const res = await fetch(`${base}/`);
  const permPolicy = res.headers.get('permissions-policy');
  const csp = res.headers.get('content-security-policy');
  if (permPolicy && !permPolicy.includes('microphone')) {
    throw new Error(`Permissions-Policy missing microphone permission: ${permPolicy}`);
  }
  if (csp && !csp.includes('connect-src')) {
    throw new Error(`CSP missing connect-src policy: ${csp}`);
  }
}

export async function runSmokeTests(options = {}) {
  const targetOrigin = (options.targetOrigin || 'http://localhost:5173').replace(/\/$/, '');
  const oracleUrl = (
    options.oracleUrl ||
    process.env.VITE_ORACLE_URL ||
    'http://localhost:8080'
  ).replace(/\/$/, '');
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
      await runCheck('PWA icons (SVG & PNG 192/512) exist and return HTTP 200', () =>
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

// CLI execution check
const isCli =
  import.meta.url === `file://${process.argv[1]?.replace(/\\/g, '/')}` ||
  process.argv[1]?.endsWith('smoke-test.js');

if (isCli) {
  const args = process.argv.slice(2);
  const skipOracle = args.includes('--skip-oracle');
  const skipPwa = args.includes('--skip-pwa');
  const positionalArgs = args.filter((a) => !a.startsWith('--'));

  const targetOrigin = positionalArgs[0] || 'http://localhost:5173';
  const oracleUrl = positionalArgs[1] || process.env.VITE_ORACLE_URL || 'http://localhost:8080';

  runSmokeTests({ targetOrigin, oracleUrl, skipOracle, skipPwa })
    .then(({ success }) => {
      if (!success) {
        console.error('Smoke tests finished with failures.');
        process.exitCode = 1;
      } else {
        console.log('Smoke tests completed successfully.');
      }
    })
    .catch((err) => {
      console.error('Fatal smoke test runner error:', err);
      process.exitCode = 1;
    });
}
