import path from 'path';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import commonjs from 'vite-plugin-commonjs';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react({
      babel: {
        babelrc: true,
      },
    }),
    commonjs({
      advanced: {
        importRules: 'merge',
      },
    }),
  ],
  resolve: {
    alias: {
      '@iso': path.resolve(__dirname, './src/components/__isograph/iso.ts'),
    },
  },
  optimizeDeps: {
    include: ['@isograph/react'],
  },
});
