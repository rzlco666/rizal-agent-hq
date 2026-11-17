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
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press('Tab');
    const focus = await page.evaluate(() => {
      const element = document.activeElement as HTMLElement | null;
      if (!element || element === document.body) return null;
      const style = getComputedStyle(element);
      return {
        tag: element.tagName,
        style: style.outlineStyle,
        width: parseFloat(style.outlineWidth)
      };
    });
    expect(focus, `focusable control ${i + 1}`).not.toBeNull();
    expect(focus?.style, `focusable control ${i + 1}`).not.toBe('none');
    expect(focus?.width ?? 0, `focusable control ${i + 1}`).toBeGreaterThanOrEqual(2);
  }
});

test('attention rail count is announced politely', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await expect(page.getByTestId('attention-count')).toHaveAttribute('aria-live', 'polite');
});
