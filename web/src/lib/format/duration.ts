import { translate } from '$lib/i18n';
import type { Locale } from '$lib/stores/ui.svelte';

export function formatDuration(milliseconds: number, locale: Locale = 'id'): string {
  const seconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  const hourUnit = translate(locale, 'format.hoursShort');
  const minuteUnit = translate(locale, 'format.minutesShort');
  const secondUnit = translate(locale, 'format.secondsShort');
  if (hours > 0) return `${hours}${hourUnit} ${minutes}${minuteUnit}`;
  if (minutes > 0) return `${minutes}${minuteUnit} ${rest}${secondUnit}`;
  return `${rest}${secondUnit}`;
}
