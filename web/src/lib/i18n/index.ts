import { ui, type Locale } from '$lib/stores/ui.svelte';
import en from './en.json';
import id from './id.json';
import zh from './zh.json';

export type TranslationKey = keyof typeof id;
export type TranslationParams = Record<string, string | number>;

const catalogs: Record<Locale, Record<string, string>> = { id, en, zh };

export function t(key: TranslationKey, params?: TranslationParams): string {
  const template = catalogs[ui.locale][key] ?? catalogs.id[key] ?? String(key);
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : match
  );
}

export function translationKeys(): string[] {
  return Object.keys(id);
}
