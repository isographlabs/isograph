import babel from 'vite-plugin-babel';
import commonjs from 'vite-plugin-commonjs';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [
    babel({
      filter: /\.[jt]sx?$/,
      babelConfig: {
        presets: ['@babel/preset-typescript'],
        plugins: ['../isograph-babel-plugin/BabelPluginIsograph'],
      },
    }),
    commonjs(),
  ],
});
