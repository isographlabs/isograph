import path from 'path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
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
      '@iso/*': path.resolve(__dirname, './src/components/__isograph/*'),
    },
  },
});
