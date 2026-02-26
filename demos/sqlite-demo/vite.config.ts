/// <reference types="vitest/config" />
import path from 'path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        babelrc: true,
      },
    }),
  ],
  resolve: {
    alias: {
      '@iso': path.resolve(__dirname, './src/components/__isograph/iso.ts'),
    },
  },
  test: {
    // Exclude Playwright e2e tests from vitest (they run via `pnpm test:e2e`)
    exclude: ['e2e/**', 'node_modules/**'],
  },
});
