import { defineConfig } from 'tsdown';

export default defineConfig({
  workspace: [
    './libs/isograph-disposable-types',
    './libs/isograph-react',
    './libs/isograph-react-disposable-state',
    './libs/isograph-reference-counted-pointer',
  ],
  entry: ['src/index.ts'],
  dts: {
    sourcemap: true,
  },
  exports: {
    devExports: true,
    unbundle: true,
  },
  logLevel: 'error',
  shims: true, // shims for __dirname
  unbundle: true,
  format: ['commonjs', 'esm'],
});
