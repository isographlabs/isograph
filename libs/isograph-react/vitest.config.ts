import path from 'node:path';
import babel from 'vite-plugin-babel';
import commonjs from 'vite-plugin-commonjs';
import { defineProject, type UserWorkspaceConfig } from 'vitest/config';

const project: UserWorkspaceConfig = defineProject({
  plugins: [
    babel({
      filter: /\.[jt]sx?$/,
      babelConfig: {
        presets: ['@babel/preset-typescript'],
        plugins: [
          [
            path.join(
              __dirname,
              '../isograph-babel-plugin/BabelPluginIsograph',
            ),
            { searchFrom: __dirname },
          ],
        ],
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
      '@iso': './src/tests/__isograph/iso.ts',
    },
  },
});

export default project;
