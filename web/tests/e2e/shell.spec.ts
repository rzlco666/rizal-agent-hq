import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('hq-theme', 'day');
    localStorage.setItem('hq-locale', 'id');
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: /buka palet perintah/i })).toBeVisible();
});

test('shell has one primary nav and a fixed attention rail', async ({ page }) => {
  test.skip((page.viewportSize()?.width ?? 0) < 1280, 'desktop rail contract');
  await expect(page.getByTestId('nav-rail')).toBeVisible();
  await expect(page.getByTestId('attention-rail')).toBeVisible();
  await expect(page.getByTestId('nav-rail').getByRole('link')).toHaveCount(10);
  await expect(page.getByTestId('nav-rail').locator('[aria-current="page"]')).toHaveCount(1);
  await expect(page.getByRole('navigation', { name: /navigasi utama/i })).toBeVisible();
  await expect(page.locator('header').first()).toHaveCSS('height', '52px');
});

test('mobile shell shows a 56px bottom nav and hides the attention rail', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.getByTestId('bottom-nav')).toBeVisible();
  await expect(page.getByTestId('bottom-nav')).toHaveCSS('height', '56px');
  await expect(page.getByTestId('attention-rail')).toBeHidden();
  await expect(page.getByTestId('shell-main')).toHaveCSS('padding-bottom', '72px');
});


test('palette labels follow a locale change', async ({ page }) => {
  test.skip((page.viewportSize()?.width ?? 0) < 1280, 'locale switcher is desktop-only');
  await page.getByRole('button', { name: 'EN' }).click();
  await page.keyboard.press('Control+k');
  await expect(page.getByRole('option', { name: 'Office' })).toBeVisible();
  await page.keyboard.press('Escape');
});
test('cmd+k opens palette, filters, escapes', async ({ page }) => {
  await page.keyboard.press('Control+k');
  await expect(page.getByRole('dialog', { name: /perintah/i })).toBeVisible();
  await page.getByRole('combobox').fill('office');
  await expect(page.getByRole('option', { name: /kantor/i })).toBeVisible();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/office/);
  await page.keyboard.press('Control+k');
  await expect(page.getByRole('dialog', { name: /perintah/i })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: /perintah/i })).toBeHidden();
});

test('palette traps focus, locks scroll and restores focus', async ({ page }) => {
  const trigger = page.getByRole('button', { name: /buka palet perintah/i });
  await trigger.click();
  const dialog = page.getByRole('dialog', { name: /perintah/i });
  await expect(dialog).toBeVisible();
  await expect(page.locator('body')).toHaveCSS('overflow', 'hidden');
  await page.keyboard.press('Tab');
  await expect(dialog.locator(':focus')).toHaveCount(1);
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await expect(page.locator('body')).not.toHaveCSS('overflow', 'hidden');
  await expect(trigger).toBeFocused();
});
