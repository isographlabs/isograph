import babel from 'vite-plugin-babel';
import commonjs from 'vite-plugin-commonjs';
import { defineProject } from 'vitest/config';

export default defineProject({
  plugins: [
    babel({
      filter: /\.[jt]sx?$/,
      babelConfig: {
        presets: ['@babel/preset-typescript'],
        plugins: [require('../isograph-babel-plugin/BabelPluginIsograph')],
      },
    }),
    commonjs(),
  ],
});
