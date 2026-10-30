import { describe, it, expect } from 'vitest';
import {
  workboxRuntimeCaching,
  ORACLE_ENDPOINTS_REGEX,
  STATIC_ASSETS_REGEX,
} from './workboxConfig';

describe('PWA Caching Strategy & Oracle Network-Only Assurance', () => {
  const oracleEndpoints = [
    'https://oracle.echogrid.org/api/v1/challenge?wallet=GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
    'http://localhost:8080/api/v1/proof',
    'https://oracle.echogrid.org/api/v1/claims/clm_0123456789abcdef',
    'http://localhost:8080/api/v1/ready',
    'http://localhost:8080/api/v1/config',
    'http://localhost:8080/api/v1/health',
  ];

  it('CRITICAL: asserts NetworkOnly handler matches all /challenge, /proof, and /claims routes', () => {
    const oracleRule = workboxRuntimeCaching.find(
      (rule) => rule.options?.cacheName === 'oracle-network-only',
    );
    expect(oracleRule).toBeDefined();
    expect(oracleRule?.handler).toBe('NetworkOnly');

    for (const url of oracleEndpoints) {
      expect(ORACLE_ENDPOINTS_REGEX.test(url)).toBe(true);
    }
  });

  it('asserts Oracle endpoints never match static asset caching rules', () => {
    for (const url of oracleEndpoints) {
      expect(STATIC_ASSETS_REGEX.test(url)).toBe(false);
    }
  });

  it('asserts static assets (js, css, html, svg, png) match StaleWhileRevalidate', () => {
    const staticRule = workboxRuntimeCaching.find(
      (rule) => rule.options?.cacheName === 'echogrid-static-assets',
    );
    expect(staticRule).toBeDefined();
    expect(staticRule?.handler).toBe('StaleWhileRevalidate');

    const sampleAssets = [
      'https://app.echogrid.org/assets/index-ABC123.js',
      'https://app.echogrid.org/assets/index-DEF456.css',
      'https://app.echogrid.org/icon.svg',
      'https://app.echogrid.org/pwa-192x192.png',
    ];

    for (const asset of sampleAssets) {
      expect(STATIC_ASSETS_REGEX.test(asset)).toBe(true);
    }
  });
});
