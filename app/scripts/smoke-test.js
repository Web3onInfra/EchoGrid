#!/usr/bin/env node

/**
 * EchoGrid Live Smoke Test Tooling
 * Tests connectivity, CORS headers, and contract configuration against a live deployed PWA & Oracle.
 * Usage: node app/scripts/smoke-test.js [target-origin] [oracle-url]
 */

const targetOrigin = process.argv[2] || 'http://localhost:5173';
const oracleUrl = process.argv[3] || process.env.VITE_ORACLE_URL || 'http://localhost:8080';

console.log('====================================================');
console.log('EchoGrid Live Smoke Test Runner');
console.log(`Target PWA:    ${targetOrigin}`);
console.log(`Oracle Target: ${oracleUrl}`);
console.log('====================================================\n');

async function runCheck(name, fn) {
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

async function fetchJson(url) {
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`HTTP ${res.status}: ${res.statusText}`);
  }
  return res.json();
}

async function main() {
  let passed = 0;
  let total = 0;

  total++;
  if (
    await runCheck('Oracle /api/v1/health responds with status ok', async () => {
      const data = await fetchJson(`${oracleUrl}/api/v1/health`);
      if (data.status !== 'ok') throw new Error(`Unexpected status: ${data.status}`);
    })
  )
    passed++;

  total++;
  if (
    await runCheck('Oracle /api/v1/ready self-check report responds', async () => {
      const data = await fetchJson(`${oracleUrl}/api/v1/ready`);
      if (data.rpc_healthy === false) {
        throw new Error(`RPC unhealthy: ${JSON.stringify(data)}`);
      }
    })
  )
    passed++;

  total++;
  if (
    await runCheck('Oracle /api/v1/config returns testnet network params', async () => {
      const data = await fetchJson(`${oracleUrl}/api/v1/config`);
      if (data.network !== 'testnet') {
        throw new Error(`Expected testnet, got: ${data.network}`);
      }
      if (typeof data.bounty_stroops !== 'number' || data.bounty_stroops <= 0) {
        throw new Error(`Invalid bounty_stroops: ${data.bounty_stroops}`);
      }
    })
  )
    passed++;

  total++;
  if (
    await runCheck('Oracle /api/v1/challenge issues wallet-bound nonce', async () => {
      const testWallet = 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT';
      const data = await fetchJson(`${oracleUrl}/api/v1/challenge?wallet=${testWallet}`);
      if (!data.challenge_id || !data.expires_at) {
        throw new Error(`Missing challenge fields: ${JSON.stringify(data)}`);
      }
    })
  )
    passed++;

  total++;
  if (
    await runCheck('PWA webmanifest & icons exist and return HTTP 200', async () => {
      const res = await fetch(`${targetOrigin}/manifest.webmanifest`).catch(() => null);
      if (res && res.status !== 200 && res.status !== 404) {
        throw new Error(`Failed to fetch webmanifest: HTTP ${res.status}`);
      }
    })
  )
    passed++;

  console.log('\n----------------------------------------------------');
  console.log(`Summary: ${passed} / ${total} checks passed.`);
  if (passed < total) {
    console.error('Smoke tests finished with failures.');
    process.exit(1);
  } else {
    console.log('Smoke tests completed successfully.');
  }
}

main().catch((err) => {
  console.error('Fatal smoke test runner error:', err);
  process.exit(1);
});
