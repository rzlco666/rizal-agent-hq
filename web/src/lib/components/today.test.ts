import { expect, test } from 'vitest';
import { buildLedgerPoints, sortAgents, statusRank } from './today';

test('status rank puts waiting first and offline last', () => {
  expect(statusRank('menunggu-rizal')).toBeLessThan(statusRank('error'));
  expect(statusRank('error')).toBeLessThan(statusRank('bekerja'));
  expect(statusRank('bekerja')).toBeLessThan(statusRank('berpikir'));
  expect(statusRank('berpikir')).toBeLessThan(statusRank('santai'));
  expect(statusRank('santai')).toBeLessThan(statusRank('offline'));
});

test('agent sort groups by status then alert count descending', () => {
  const rows = [
    { id: 'a', status: 'offline', alerts: 9 },
    { id: 'b', status: 'bekerja', alerts: 1 },
    { id: 'c', status: 'menunggu-rizal', alerts: 0 },
    { id: 'd', status: 'menunggu-rizal', alerts: 2 },
    { id: 'e', status: 'error', alerts: 4 }
  ];
  expect(sortAgents(rows).map((row) => row.id)).toEqual(['d', 'c', 'e', 'b', 'a']);
});

test('ledger points bucket real events without inventing values', () => {
  const now = 1_700_000_600_000;
  const points = buildLedgerPoints(
    [
      { ts_ms: now - 120_000, cash_usd: 0.5, shadow_usd: 1.5 },
      { ts_ms: now - 60_000, cash_usd: 0.25, shadow_usd: 0.75 }
    ],
    '24h',
    now
  );
  expect(points).toHaveLength(2);
  expect(points[0].cash_usd).toBe(0.5);
  expect(points[1].shadow_usd).toBe(0.75);
});

test('ledger points ignore events outside the selected window', () => {
  const now = 1_700_000_600_000;
  const points = buildLedgerPoints(
    [
      { ts_ms: now - 120_000, cash_usd: 0.5, shadow_usd: 1.5 },
      { ts_ms: now - 20 * 24 * 3_600_000, cash_usd: 9, shadow_usd: 9 }
    ],
    '24h',
    now
  );
  expect(points).toHaveLength(1);
});
