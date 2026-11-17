import type { Locale } from '$lib/stores/ui.svelte';

const units: Record<Locale, { hours: string; minutes: string; seconds: string }> = {
  id: { hours: 'j', minutes: 'm', seconds: 'd' },
  en: { hours: 'h', minutes: 'm', seconds: 's' },
  zh: { hours: '小时', minutes: '分钟', seconds: '秒' }
};

export function formatDuration(milliseconds: number, locale: Locale = 'id'): string {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  const label = units[locale];
  if (hours > 0) return `${hours}${label.hours} ${minutes}${label.minutes}`;
  if (minutes > 0) return `${minutes}${label.minutes} ${rest}${label.seconds}`;
  return `${rest}${label.seconds}`;
}
