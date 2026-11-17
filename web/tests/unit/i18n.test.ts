import { t } from '$lib/i18n';
import { ui } from '$lib/stores/ui.svelte';
import en from '$lib/i18n/en.json';
import id from '$lib/i18n/id.json';
import zh from '$lib/i18n/zh.json';
import { beforeEach, expect, test } from 'vitest';

const keys = (obj: Record<string, string>) => Object.keys(obj).sort();

beforeEach(() => {
  localStorage.clear();
  ui.locale = 'id';
});

test('missing key is visible in dev, not silently blank', () => {
  expect(t('tidak.ada' as never)).toContain('tidak.ada');
});

test('locale persists', () => {
  ui.locale = 'en';
  expect(localStorage.getItem('hq-locale')).toBe('en');
});

test('all locales have the same keys', () => {
  expect(keys(en)).toEqual(keys(id));
  expect(keys(zh)).toEqual(keys(id));
});

test('params replace without html injection', () => {
  const payload = '<img src=x onerror=alert(1)>';
  const rendered = t('attention.count', { count: payload });
  expect(rendered).not.toContain('<img');
  expect(rendered).toContain(payload);
});
