import type { Locale } from '$lib/stores/ui.svelte';

const tags: Record<Locale, string> = {
  id: 'id-ID',
  en: 'en-US',
  zh: 'zh-CN'
};

const tokenUnits: Record<Locale, string> = {
  id: 'token',
  en: 'tokens',
  zh: '个令牌'
};

export function formatNumber(value: number, locale: Locale = 'id'): string {
  return new Intl.NumberFormat(tags[locale]).format(value);
}

export function formatUsd(value: number, locale: Locale = 'id'): string {
  const digits = value > 0 && value < 1 ? 3 : 2;
  return new Intl.NumberFormat(tags[locale], {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: digits,
    maximumFractionDigits: digits
  }).format(value);
}

export function formatTokens(
  value: number,
  locale: Locale = 'id'
): { label: string; title: string } {
  const label =
    Math.abs(value) >= 1_000_000
      ? new Intl.NumberFormat(tags[locale], {
          notation: 'compact',
          maximumFractionDigits: 1
        }).format(value)
      : formatNumber(value, locale);
  return { label, title: `${formatNumber(value, locale)} ${tokenUnits[locale]}` };
}
