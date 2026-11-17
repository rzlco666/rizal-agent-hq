declare global {
  namespace App {
    interface Error {
      requestId?: string;
    }
  }

  interface Window {
    __HQ_THEME__?: 'day' | 'night' | 'system';
  }
}

export {};
