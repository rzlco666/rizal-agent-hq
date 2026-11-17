import { translate } from '$lib/i18n';
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
    return translate(locale, 'format.minutesAgo', { count: Math.max(0, minutes) });
  }
  const hours = Math.round(minutes / 60);
  if (hours < 24) {
    return translate(locale, 'format.hoursAgo', { count: hours });
  }
  return formatDate(date, locale);
}
