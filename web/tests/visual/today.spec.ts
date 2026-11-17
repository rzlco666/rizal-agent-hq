import type { Page } from '@playwright/test';
import { expect, test } from '@playwright/test';

async function settle(page: Page) {
  await page.evaluate(() => document.fonts.ready);
  await expect(page.getByTestId('kpi-working')).toBeVisible();
  await expect(page.getByTestId('agent-row').first()).toBeVisible();
  if ((page.viewportSize()?.width ?? 1440) < 768) {
    await expect(page.getByTestId('bottom-nav')).toBeVisible();
  } else {
    await expect(page.getByTestId('nav-rail')).toBeVisible();
  }
  await expect(page.locator('.uplot')).toBeVisible();
  await page.waitForFunction(() => {
    const canvas = document.querySelector('.uplot canvas') as HTMLCanvasElement | null;
    if (!canvas) return false;
    const ctx = canvas.getContext('2d');
    if (!ctx) return false;
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
    for (let i = 3; i < data.length; i += 4) if (data[i] > 0) return true;
    return false;
  });
}

test('today matches day snapshot at 1440x900', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => localStorage.setItem('hq-theme', 'day'));
  await page.goto('/');
  await settle(page);
  await expect(page).toHaveScreenshot('today-day-1440x900.png');
});

test('today matches night snapshot at 1440x900', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => localStorage.setItem('hq-theme', 'night'));
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'night');
  await settle(page);
  await expect(page).toHaveScreenshot('today-night-1440x900.png');
});

test('today matches day snapshot at 390x844', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.addInitScript(() => localStorage.setItem('hq-theme', 'day'));
  await page.goto('/');
  await settle(page);
  await expect(page).toHaveScreenshot('today-day-390x844.png');
});
