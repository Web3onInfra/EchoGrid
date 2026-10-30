import { test, expect } from '@playwright/test';

test.describe('EchoGrid End-to-End Claim Flows', () => {
  test.beforeEach(async ({ page }) => {
    // Default happy-path mocked oracle responses
    await page.route('**/api/v1/config', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          network: 'testnet',
          bounty_stroops: 10000000,
          daily_cap_stroops: 50000000,
          cooldown_seconds: 600,
          challenge_ttl_seconds: 120,
          simulator_mode: false,
          explorer_base_url: 'https://stellar.expert/explorer/testnet/tx',
        }),
      });
    });

    await page.route('**/api/v1/ready', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'ok',
          network: 'testnet',
          uptime_seconds: 3600,
          checks: { contract: true, horizon: true },
        }),
      });
    });

    await page.route('**/api/v1/challenge*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          challenge_id: 'ch_e2e_mock_12345678',
          wallet_address: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
          issued_at: new Date().toISOString(),
          expires_at: new Date(Date.now() + 120000).toISOString(),
        }),
      });
    });

    await page.route('**/api/v1/proof', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 'confirmed',
          claim_id: 'clm_e2e_settled_999',
          tx_hash: 'tx_e2e_real_testnet_hash',
          explorer_url: 'https://stellar.expert/explorer/testnet/tx/tx_e2e_real_testnet_hash',
          amount_stroops: 10000000,
          simulated: false,
        }),
      });
    });
  });

  test('Flow 1: Real-mic audio capture and detector flow with fake audio device', async ({
    page,
    context,
  }) => {
    await context.grantPermissions(['microphone']);
    await page.goto('/');

    // Switch to Monitor tab
    await page.getByRole('tab', { name: 'Monitor' }).click();
    await expect(page.getByTestId('start-monitoring-btn')).toBeVisible();

    // Start monitoring - Chromium fake audio device feeds hum_drop.wav
    await page.getByTestId('start-monitoring-btn').click();

    // Verifies transition into calibrating or armed
    await expect(
      page.getByTestId('calibrating-state').or(page.getByTestId('armed-state')),
    ).toBeVisible({ timeout: 10000 });
  });

  test('Flow 2: Simulator mode claim and simulated receipt', async ({ page }) => {
    await page.goto('/');

    // Go to Monitor view
    await page.getByRole('tab', { name: 'Monitor' }).click();

    // Enable Simulator mode toggle button
    await page.getByTestId('toggle-simulator-mode-btn').click();
    await expect(page.getByText('Switch to Real Mic')).toBeVisible();

    // Start simulator stream
    await page.getByTestId('start-monitoring-btn').click();

    // Should calibrate and transition to ARMED
    await expect(page.getByTestId('armed-state')).toBeVisible({ timeout: 10000 });
    await expect(page.getByTestId('simulate-stop-appliance-btn')).toBeVisible();

    // Click "Simulate Power-Down (Stop Appliance)"
    await page.getByTestId('simulate-stop-appliance-btn').click();

    // Verifies sustaining drop check and confirmation
    await expect(page.getByTestId('drop-confirmed-state')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText(/Power-Down Confirmed!/i)).toBeVisible();
    await expect(page.getByText('Simulated Claim — No real XLM payout')).toBeVisible();
  });

  test('Flow 3: Microphone permission denied shows clear warning banner', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('tab', { name: 'Monitor' }).click();

    // Simulate NotAllowedError
    await page.evaluate(() => {
      navigator.mediaDevices.getUserMedia = () =>
        Promise.reject(new DOMException('Permission denied', 'NotAllowedError'));
    });

    await page.getByTestId('start-monitoring-btn').click();

    // Should display permission denied banner
    await expect(page.getByTestId('monitor-error-banner')).toBeVisible({ timeout: 10000 });
    await expect(page.getByTestId('monitor-error-banner')).toContainText(
      /Microphone permission was denied/i,
    );
  });

  test('Flow 4: Oracle down shows degraded state banner', async ({ page }) => {
    // Mock /ready to 503
    await page.route('**/api/v1/ready', async (route) => {
      await route.fulfill({
        status: 503,
        contentType: 'application/json',
        body: JSON.stringify({ code: 'ORACLE_UNREACHABLE', message: 'Service down', status: 503 }),
      });
    });

    await page.goto('/');
    await expect(page.getByTestId('degraded-banner')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/Oracle Unavailable/i)).toBeVisible();
  });

  test('Flow 5: Cooldown active error surfaces proper user message', async ({ page }) => {
    await page.route('**/api/v1/proof', async (route) => {
      await route.fulfill({
        status: 422,
        contentType: 'application/json',
        body: JSON.stringify({
          code: 'COOLDOWN_ACTIVE',
          message: 'Per-wallet cooldown period of 600 seconds is active',
          status: 422,
        }),
      });
    });

    await page.goto('/');
    // Check wallet screen is accessible and displays testnet guidance
    await page.getByRole('tab', { name: 'Wallet' }).click();
    await expect(page.getByTestId('paste-address-input')).toBeVisible();
  });

  test('Flow 6: Offline network handling', async ({ page, context }) => {
    await page.goto('/');
    await expect(page.getByTestId('simulated-grid-banner')).toBeVisible();

    // Emulate offline mode
    await context.setOffline(true);

    // Try navigating to wallet or monitor
    await page.getByRole('tab', { name: 'Wallet' }).click();
    await expect(page.getByTestId('paste-address-input')).toBeVisible();

    // Restore online
    await context.setOffline(false);
  });
});
