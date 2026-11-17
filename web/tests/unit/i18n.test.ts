import { t } from '$lib/i18n';
import { ui } from '$lib/stores/ui.svelte';
import en from '$lib/i18n/en.json';
import id from '$lib/i18n/id.json';
import zh from '$lib/i18n/zh.json';
import { findHardcodedCopy, localeKeyIssues } from '../../scripts/check-i18n.mjs';
import { beforeEach, expect, test } from 'vitest';

function keys(catalog: Record<string, string>): string[] {
  return Object.keys(catalog).sort();
}

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
  const host = document.createElement('div');
  host.textContent = rendered;
  expect(host.querySelector('img')).toBeNull();
  expect(host.textContent).toContain(payload);
});

test('html lang follows the locale map', () => {
  ui.locale = 'zh';
  expect(document.documentElement.lang).toBe('zh-Hans');
  ui.locale = 'en';
  expect(document.documentElement.lang).toBe('en');
  ui.locale = 'id';
  expect(document.documentElement.lang).toBe('id');
});

test('ui source has no hardcoded copy', () => {
  expect(localeKeyIssues()).toEqual([]);
  expect(findHardcodedCopy()).toEqual([]);
});
