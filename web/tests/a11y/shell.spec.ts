import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

const viewports = [
  { width: 1440, height: 900 },
  { width: 1024, height: 768 },
  { width: 390, height: 844 }
];

test('shell has no serious or critical axe violations', async ({ page }) => {
  for (const viewport of viewports) {
    await page.setViewportSize(viewport);
    await page.goto('/');
    const results = await new AxeBuilder({ page }).analyze();
    const blocking = results.violations.filter(
      (violation) => violation.impact === 'serious' || violation.impact === 'critical'
    );
    expect(
      blocking.map((violation) => `${violation.id}: ${violation.nodes.map((n) => n.target).join(', ')}`)
    ).toEqual([]);
  }
});

test('focus ring is visible on every interactive control', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: /buka palet perintah/i })).toBeVisible();
  const total = await page.evaluate(() =>
    Array.from(
      document.querySelectorAll<HTMLElement>(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
    ).filter((element) => element.getClientRects().length > 0).length
  );
  expect(total).toBeGreaterThanOrEqual(8);

  async function currentFocus() {
    return page.evaluate(() => {
      const element = document.activeElement as HTMLElement | null;
      if (!element || element === document.body) return null;
      const style = getComputedStyle(element);
      return { style: style.outlineStyle, width: parseFloat(style.outlineWidth) };
    });
  }

  await page.getByRole('link', { name: /lihat/i }).focus();
  await page.keyboard.press('Shift+Tab');
  await page.keyboard.press('Tab');
  const first = await currentFocus();
  expect(first).not.toBeNull();
  expect(first?.style).not.toBe('none');
  expect(first?.width ?? 0).toBeGreaterThanOrEqual(2);

  for (let i = 0; i < total - 1; i++) {
    await page.keyboard.press('Tab');
    const focus = await currentFocus();
    expect(focus, `focusable control ${i + 2}`).not.toBeNull();
    expect(focus?.style, `focusable control ${i + 2}`).not.toBe('none');
    expect(focus?.width ?? 0, `focusable control ${i + 2}`).toBeGreaterThanOrEqual(2);
  }
});

test('attention rail count is announced politely', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await expect(page.getByTestId('attention-count')).toHaveAttribute('aria-live', 'polite');
});
