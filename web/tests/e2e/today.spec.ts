import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('hq-theme', 'day');
    localStorage.setItem('hq-locale', 'id');
  });
  await page.goto('/');
});

test('kpi strip shows four honest figures', async ({ page }) => {
  await expect(page.getByTestId('kpi-working')).toBeVisible();
  await expect(page.getByTestId('kpi-waiting')).toBeVisible();
  await expect(page.getByTestId('kpi-tokens')).toContainText('jt');
  await expect(page.getByTestId('kpi-cash')).toContainText('$');
  await expect(page.getByTestId('kpi-shadow')).toContainText('$');
});

test('agent table shows status, sentence, usage, alerts', async ({ page }) => {
  const first = page.getByTestId('agent-row').first();
  await expect(first).toBeVisible();
  await expect(first).toContainText('OMP');
  await expect(first.getByTestId('status-dot')).toBeVisible();
  await expect(first.getByTestId('activity')).not.toBeEmpty();
  await first.click();
  await expect(page).toHaveURL(/\/agents\/agent%3Aomp$/);
});

test('ledger has one main chart and a range switch', async ({ page }) => {
  await expect(page.getByRole('tab', { name: '24 jam' })).toBeVisible();
  await expect(page.getByTestId('ledger-chart')).toHaveCount(1);
  await expect(page.locator('.uplot')).toHaveCount(1);
  await page.getByRole('tab', { name: '7 hari' }).click();
  await expect(page.getByTestId('ledger-chart')).toHaveCount(1);
  await expect(page.locator('.uplot')).toHaveCount(1);
});


test('ack from rail updates count without reload', async ({ page }) => {
  test.skip((page.viewportSize()?.width ?? 0) < 1280, 'desktop rail contract');
  const count = page.getByTestId('alert-count');
  await expect(count).toBeVisible();
  const before = (await count.textContent())?.trim() ?? '';
  expect(before).not.toBe('');
  await page.getByTestId('attention-row').first().getByRole('button', { name: /ack/i }).click();
  await expect(count).not.toHaveText(before);
});

test('offline banner appears and disappears', async ({ page }) => {
  await expect(page.getByTestId('kpi-working')).toBeVisible();
  await page.evaluate(() => window.dispatchEvent(new Event('hq-offline')));
  await expect(page.getByTestId('connection-banner')).toContainText(/Menyambung/);
  await page.evaluate(() => window.dispatchEvent(new Event('hq-online')));
  await expect(page.getByTestId('connection-banner')).toHaveCount(0);
});

test('empty today is honest', async ({ page }) => {
  await page.goto('/?empty=1');
  const empty = page.getByTestId('empty-today');
  await expect(empty).toContainText('Belum ada agent');
  const lines = (await empty.innerText())
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  expect(lines.length).toBeGreaterThanOrEqual(2);
});

test('panels report errors with retry', async ({ page }) => {
  await page.route('**/api/v1/overview*', (route) =>
    route.request().method() === 'GET'
      ? route.fulfill({ status: 500, contentType: 'application/json', body: '{"error":"boom"}' })
      : route.continue()
  );
  await page.reload();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page.getByRole('alert').getByRole('button', { name: 'Coba lagi' })).toBeVisible();
});
test('costs failure shows panel error with retry', async ({ page }) => {
  test.skip((page.viewportSize()?.width ?? 0) < 1280, 'desktop error panel contract');
  await page.route('**/api/v1/costs*', (route) =>
    route.fulfill({ status: 500, contentType: 'application/json', body: '{"error":"boom"}' })
  );
  await page.reload();
  const panel = page.locator('.ledger-slot').getByRole('alert');
  await expect(panel).toBeVisible();
  await expect(panel.getByRole('button', { name: 'Coba lagi' })).toBeVisible();
});

test('ws patch updates rendered agent without reload', async ({ page }) => {
  await expect(page.locator('[data-live-state="live"]')).toBeVisible();
  const activity = page
    .locator('[data-testid="agent-row"][data-agent-id="agent:omp"]')
    .getByTestId('activity');
  await expect(activity).toHaveText('Menunggu keputusan Rizal.');
  const event = (status: string) => ({
    events: [
      {
        event_ts_ms: Date.now(),
        kind: 'agent_status_changed',
        subject: 'agent:omp',
        source: 'e2e-ws-patch',
        payload: { status }
      }
    ]
  });
  try {
    const response = await page.request.post('/api/v1/dev/events', { data: event('bekerja') });
    expect(response.ok()).toBeTruthy();
    await expect(activity).toHaveText('Sedang bekerja.', { timeout: 10_000 });
  } finally {
    await page.request.post('/api/v1/dev/events', {
      data: event('menunggu-rizal')
    });
    await expect(activity).toHaveText('Menunggu keputusan Rizal.', { timeout: 10_000 });
  }
});
