export type TodayStatus =
  | 'bekerja'
  | 'berpikir'
  | 'menunggu-rizal'
  | 'santai'
  | 'offline'
  | 'error';

export interface LedgerInput {
  ts_ms: number;
  cash_usd: number;
  shadow_usd: number;
}

export interface LedgerPoint extends LedgerInput {
  requests?: number;
}

const statusOrder: Record<string, number> = {
  'menunggu-rizal': 0,
  error: 1,
  bekerja: 2,
  berpikir: 3,
  santai: 4,
  offline: 5
};

export function statusRank(status: string): number {
  return statusOrder[status] ?? statusOrder.offline;
}

export function sortAgents<T extends { id: string; status: string; alerts?: number }>(
  items: T[]
): T[] {
  return [...items].sort(
    (a, b) => statusRank(a.status) - statusRank(b.status) || (b.alerts ?? 0) - (a.alerts ?? 0)
  );
}

export function buildLedgerPoints(
  events: LedgerInput[],
  window: '24h' | '7d',
  now: number = Date.now()
): LedgerPoint[] {
  const span = window === '24h' ? 24 * 3_600_000 : 7 * 24 * 3_600_000;
  return events
    .filter((event) => event.ts_ms >= now - span && event.ts_ms <= now)
    .sort((a, b) => a.ts_ms - b.ts_ms)
    .map((event) => ({
      ts_ms: event.ts_ms,
      cash_usd: event.cash_usd,
      shadow_usd: event.shadow_usd
    }));
}
