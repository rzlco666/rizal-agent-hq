export type Theme = 'day' | 'night' | 'system';
export type Locale = 'id' | 'en' | 'zh';
export type Connection = 'live' | 'offline';

function stored<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  if (typeof localStorage === 'undefined') return fallback;
  const value = localStorage.getItem(key);
  return allowed.includes(value as T) ? (value as T) : fallback;
}

export function htmlLang(locale: Locale): string {
  return locale === 'zh' ? 'zh-Hans' : locale;
}

class UiStore {
  _theme = $state<Theme>(stored('hq-theme', ['day', 'night', 'system'], 'system'));
  _locale = $state<Locale>(stored('hq-locale', ['id', 'en', 'zh'], 'id'));
  railOpen = $state(true);
  sidebarOpen = $state(false);
  paletteOpen = $state(false);
  reducedMotion = $state(false);
  connection = $state<Connection>(
    typeof navigator !== 'undefined' && navigator.onLine === false ? 'offline' : 'live'
  );

  get theme(): Theme {
    return this._theme;
  }

  set theme(value: Theme) {
    this._theme = value;
    if (typeof localStorage !== 'undefined') localStorage.setItem('hq-theme', value);
    this.applyDocumentTheme();
  }

  get locale(): Locale {
    return this._locale;
  }

  set locale(value: Locale) {
    this._locale = value;
    if (typeof localStorage !== 'undefined') localStorage.setItem('hq-locale', value);
    if (typeof document !== 'undefined') document.documentElement.lang = htmlLang(value);
  }

  applyDocumentTheme() {
    if (typeof document === 'undefined') return;
    const night =
      this.theme === 'night' ||
      (this.theme === 'system' &&
        typeof window !== 'undefined' &&
        window.matchMedia('(prefers-color-scheme: dark)').matches);
    document.documentElement.dataset.theme = night ? 'night' : 'day';
  }

  init() {
    this.applyDocumentTheme();
    if (typeof document !== 'undefined') document.documentElement.lang = htmlLang(this.locale);
    if (typeof window === 'undefined') return () => {};

    const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
    const scheme = window.matchMedia('(prefers-color-scheme: dark)');
    const syncMotion = () => (this.reducedMotion = motion.matches);
    const syncConnection = () => (this.connection = navigator.onLine ? 'live' : 'offline');
    const syncScheme = () => {
      if (this.theme === 'system') this.applyDocumentTheme();
    };
    syncMotion();
    syncConnection();

    motion.addEventListener('change', syncMotion);
    scheme.addEventListener('change', syncScheme);
    window.addEventListener('online', syncConnection);
    window.addEventListener('offline', syncConnection);

    return () => {
      motion.removeEventListener('change', syncMotion);
      scheme.removeEventListener('change', syncScheme);
      window.removeEventListener('online', syncConnection);
      window.removeEventListener('offline', syncConnection);
    };
  }
}

export const ui = new UiStore();
