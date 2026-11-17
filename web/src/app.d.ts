declare global {
  namespace App {}

  interface Window {
    __HQ_THEME__?: 'day' | 'night' | 'system';
  }
}

export {};
