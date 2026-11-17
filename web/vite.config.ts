import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8900',
      '/ws': { target: 'ws://127.0.0.1:8900', ws: true },
      '/events': { target: 'http://127.0.0.1:8900', ws: true }
    }
  },
  preview: {
    proxy: {
      '/api': 'http://127.0.0.1:8900',
      '/ws': { target: 'ws://127.0.0.1:8900', ws: true },
      '/events': { target: 'http://127.0.0.1:8900', ws: true }
    }
  },
  build: { sourcemap: false }
});
