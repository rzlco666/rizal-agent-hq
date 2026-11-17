import type { Locale } from '$lib/stores/ui.svelte';

const tags: Record<Locale, string> = {
  id: 'id-ID',
  en: 'en-US',
  zh: 'zh-CN'
};

export function formatTime(value: string | Date, locale: Locale = 'id'): string {
  return new Intl.DateTimeFormat(tags[locale], {
    timeZone: 'Asia/Jakarta',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23'
  }).format(typeof value === 'string' ? new Date(value) : value);
}

export function formatDate(value: string | Date, locale: Locale = 'id'): string {
  return new Intl.DateTimeFormat(tags[locale], {
    timeZone: 'Asia/Jakarta',
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  }).format(typeof value === 'string' ? new Date(value) : value);
}

export function formatRelative(
  value: string | Date,
  now: number = Date.now(),
  locale: Locale = 'id'
): string {
  const date = typeof value === 'string' ? new Date(value) : value;
  const elapsed = now - date.getTime();
  const minutes = Math.round(elapsed / 60_000);
  if (minutes < 60) {
    if (locale === 'en') return `${Math.max(0, minutes)} min ago`;
    if (locale === 'zh') return `${Math.max(0, minutes)} 分钟前`;
    return `${Math.max(0, minutes)} menit lalu`;
  }
  const hours = Math.round(minutes / 60);
  if (hours < 24) {
    if (locale === 'en') return `${hours} h ago`;
    if (locale === 'zh') return `${hours} 小时前`;
    return `${hours} jam lalu`;
  }
  return formatDate(date, locale);
}
