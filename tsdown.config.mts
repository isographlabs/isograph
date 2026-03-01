import { defineConfig, type UserConfig } from 'tsdown';

const config: UserConfig = defineConfig({
  entry: ['src/index.ts'],
  dts: {
    sourcemap: true,
  },
  exports: true,
  logLevel: 'error',
  unbundle: true,
  format: ['commonjs', 'esm'],
});
export default config;
