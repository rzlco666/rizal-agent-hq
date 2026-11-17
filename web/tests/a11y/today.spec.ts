import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

const routes = ['/', '/?empty=1'];

test('today has no serious or critical axe violations', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  for (const route of routes) {
    await page.goto(route);
    const results = await new AxeBuilder({ page }).analyze();
    const blocking = results.violations.filter(
      (violation) => violation.impact === 'serious' || violation.impact === 'critical'
    );
    expect(
      blocking.map((violation) => `${violation.id}: ${violation.nodes.map((n) => n.target).join(', ')}`)
    ).toEqual([]);
  }
});

test('agent row is keyboard reachable with a visible focus ring', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  const row = page.getByTestId('agent-row').first();
  await row.focus();
  await expect(row).toBeFocused();

  const focus = await row.evaluate((element) => {
    const style = getComputedStyle(element);
    return { style: style.outlineStyle, width: parseFloat(style.outlineWidth) };
  });
  expect(focus.style).not.toBe('none');
  expect(focus.width).toBeGreaterThanOrEqual(2);

  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/agents\/agent%3Aomp$/);
});
