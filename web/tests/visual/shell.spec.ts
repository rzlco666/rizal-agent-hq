import type { Page } from '@playwright/test';
import { expect, test } from '@playwright/test';

async function settle(page: Page) {
  await page.evaluate(() => document.fonts.ready);
  await expect(page.locator('[data-connection]')).toHaveAttribute('data-connection', 'live');
  if ((page.viewportSize()?.width ?? 1440) < 768) {
    await expect(page.getByTestId('bottom-nav')).toBeVisible();
  } else {
    await expect(page.getByTestId('nav-rail')).toBeVisible();
  }
}

test('shell matches day snapshot at 1440x900', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => localStorage.setItem('hq-theme', 'day'));
  await page.goto('/');
  await settle(page);
  await expect(page).toHaveScreenshot('shell-day-1440x900.png', { mask: [page.getByTestId('attention-rail')] });
});

test('shell matches night snapshot at 1440x900', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => localStorage.setItem('hq-theme', 'night'));
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'night');
  await settle(page);
  await expect(page).toHaveScreenshot('shell-night-1440x900.png', { mask: [page.getByTestId('attention-rail')] });
});

test('shell matches phone snapshot at 390x844', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.addInitScript(() => localStorage.setItem('hq-theme', 'day'));
  await page.goto('/');
  await settle(page);
  await expect(page).toHaveScreenshot('shell-day-390x844.png', { mask: [page.getByTestId('attention-rail')] });
});
