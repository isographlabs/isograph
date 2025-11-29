import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: {
    sourcemap: true,
  },
  exports: true,
  logLevel: 'error',
  unbundle: true,
  format: ['commonjs', 'esm'],
});
