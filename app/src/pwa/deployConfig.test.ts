import { describe, it, expect } from 'vitest';
import fs from 'fs';
import path from 'path';

describe('Deploy Configuration and Security Headers', () => {
  const publicDir = path.resolve(__dirname, '../../public');
  const workflowPath = path.resolve(__dirname, '../../../.github/workflows/deploy-pages.yml');

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

  it('configures Cloudflare Pages deployment workflow with smoke testing', () => {
    expect(fs.existsSync(workflowPath)).toBe(true);
    const content = fs.readFileSync(workflowPath, 'utf8');
    expect(content).toContain('cloudflare/pages-action@v1');
    expect(content).toContain('smoke-test.js');
    expect(content).toContain('steps.deploy.outputs.url');
    expect(content).toContain('CLOUDFLARE_API_TOKEN');
  });
});
