import {
  formatNumber,
  formatRelative,
  formatTime,
  formatTokens,
  formatUsd
} from '$lib/format';
import { expect, test } from 'vitest';

test('numbers use locale grouping and tabular figures', () => {
  expect(formatNumber(1234567, 'id')).toBe('1.234.567');
  expect(formatNumber(1234567, 'en')).toBe('1,234,567');
});

test('usd uses three decimals below one', () => {
  expect(formatUsd(0.5, 'en')).toBe('$0.500');
  expect(formatUsd(12.3, 'en')).toBe('$12.30');
});

test('tokens use compact above one million with exact title', () => {
  const tokens = formatTokens(1500000, 'en');
  expect(tokens.label).toBe('1.5M');
  expect(tokens.title).toBe('1,500,000 tokens');
});

test('time renders in asia jakarta', () => {
  expect(formatTime('2026-01-01T00:00:00Z', 'en')).toMatch(/^07[.:]00$/);
});

test('relative time switches to absolute after a day', () => {
  const now = new Date('2026-01-10T12:00:00Z').getTime();
  expect(formatRelative(new Date(now - 60_000).toISOString(), now, 'en')).toContain('min');
  expect(formatRelative(new Date(now - 2 * 86_400_000).toISOString(), now, 'en')).toMatch(/Jan/);
});
