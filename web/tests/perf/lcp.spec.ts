import { expect, test } from '@playwright/test';

test('today lcp is under 1500ms locally', async ({ page }) => {
  await page.goto('/', { waitUntil: 'load' });
  const lcp = await page.evaluate(
    () =>
      new Promise<number>((resolve) => {
        let settled = false;
        const done = (value: number) => {
          if (settled) return;
          settled = true;
          resolve(value);
        };
        new PerformanceObserver((list) => {
          const entry = list.getEntries().at(-1);
          if (entry) done(entry.startTime);
        }).observe({ type: 'largest-contentful-paint', buffered: true });
        setTimeout(() => done(Number.POSITIVE_INFINITY), 2_000);
      })
  );
  console.log(`measured lcp: ${Math.round(lcp)}ms`);
  expect(lcp).toBeLessThan(1500);
});
