import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';

describe('Deploy Configuration and Security Headers', () => {
  const publicDir = path.resolve(__dirname, '../../public');
  const appDir = path.resolve(__dirname, '../../');
  const vercelWorkflowPath = path.resolve(
    __dirname,
    '../../../.github/workflows/deploy-vercel.yml',
  );

  it('contains Cloudflare Pages _headers with strict CSP and Permissions-Policy', () => {
    const headersPath = path.join(publicDir, '_headers');
    expect(fs.existsSync(headersPath)).toBe(true);

    const content = fs.readFileSync(headersPath, 'utf8');
    expect(content).toContain('X-Frame-Options: DENY');
    expect(content).toContain('X-Content-Type-Options: nosniff');
    expect(content).toContain('Permissions-Policy:');
    expect(content).toContain('microphone=(self)');
    expect(content).toContain('screen-wake-lock=(self)');
    expect(content).toContain('Content-Security-Policy:');
    expect(content).toContain("default-src 'self'");
    expect(content).toContain("connect-src 'self'");
  });

  it('contains _routes.json configured for Single Page App routing', () => {
    const routesPath = path.join(publicDir, '_routes.json');
    expect(fs.existsSync(routesPath)).toBe(true);

    const routes = JSON.parse(fs.readFileSync(routesPath, 'utf8'));
    expect(routes.version).toBe(1);
    expect(routes.include).toContain('/*');
    expect(routes.exclude).toBeDefined();
  });

  it('contains _redirects for 200 rewrite SPA fallback', () => {
    const redirectsPath = path.join(publicDir, '_redirects');
    expect(fs.existsSync(redirectsPath)).toBe(true);

    const content = fs.readFileSync(redirectsPath, 'utf8');
    expect(content).toMatch(/\/\*\s+\/index\.html\s+200/);
  });

  it('contains vercel.json with SPA rewrites, strict CSP, and immutable asset cache headers', () => {
    const vercelConfigPath = path.join(appDir, 'vercel.json');
    expect(fs.existsSync(vercelConfigPath)).toBe(true);

    const vercelConfig = JSON.parse(fs.readFileSync(vercelConfigPath, 'utf8'));
    expect(vercelConfig.cleanUrls).toBe(true);
    expect(vercelConfig.rewrites).toBeDefined();
    expect(
      vercelConfig.rewrites.some((r: { destination: string }) => r.destination === '/index.html'),
    ).toBe(true);
    expect(vercelConfig.headers).toBeDefined();

    const rootHeaders = vercelConfig.headers.find((h: { source: string }) => h.source === '/(.*)');
    expect(rootHeaders).toBeDefined();
    const csp = rootHeaders.headers.find(
      (kv: { key: string }) => kv.key === 'Content-Security-Policy',
    );
    expect(csp).toBeDefined();
    expect(csp.value).toContain("default-src 'self'");

    const permissions = rootHeaders.headers.find(
      (kv: { key: string }) => kv.key === 'Permissions-Policy',
    );
    expect(permissions).toBeDefined();
    expect(permissions.value).toContain('microphone=(self)');
    expect(permissions.value).toContain('screen-wake-lock=(self)');
  });

  it('configures Vercel deployment workflow with build and smoke testing', () => {
    expect(fs.existsSync(vercelWorkflowPath)).toBe(true);
    const content = fs.readFileSync(vercelWorkflowPath, 'utf8');
    expect(content).toContain('vercel build');
    expect(content).toContain('vercel deploy');
    expect(content).toContain('smoke-test.js');
    expect(content).toContain('VERCEL_TOKEN');
  });
});
