import '@testing-library/jest-dom/vitest';
import '@testing-library/svelte/vitest';

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false
  })
});

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

Object.defineProperty(globalThis, 'ResizeObserver', {
  writable: true,
  value: ResizeObserverStub
});

Object.defineProperty(document, 'fonts', {
  configurable: true,
  value: { ready: Promise.resolve(), check: () => true, load: () => Promise.resolve([]) }
});

Object.defineProperty(navigator, 'onLine', { configurable: true, value: true });
