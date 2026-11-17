import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  resolve: { conditions: ['browser'] },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts', 'tests/unit/**/*.test.ts'],
    setupFiles: ['tests/unit/setup.ts'],
    restoreMocks: true
  }
});
