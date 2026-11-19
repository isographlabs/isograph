import { transformAsync } from '@babel/core';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const fixtureDirectory = fileURLToPath(
  new URL('./fixtures/store/', import.meta.url),
);
const fixtureSources = path.join(fixtureDirectory, 'src') + path.sep;
const generatedArtifacts = path.join(fixtureSources, '__isograph') + path.sep;

export default defineConfig({
  test: { typecheck: { enabled: true, tsconfig: './tsconfig.test.json' } },
  plugins: [
    {
      name: 'isograph-babel-plugin',
      enforce: 'pre',
      async transform(code, id) {
        if (
          !id.endsWith('.ts') ||
          !id.startsWith(fixtureSources) ||
          id.startsWith(generatedArtifacts)
        ) {
          return null;
        }
        const result = await transformAsync(code, {
          filename: id,
          babelrc: false,
          configFile: false,
          sourceType: 'module',
          parserOpts: { plugins: ['typescript'] },
          plugins: [
            ['@isograph/babel-plugin', { searchFrom: fixtureDirectory }],
          ],
          sourceMaps: true,
        });
        if (result?.code == null) {
          throw new Error(`@isograph/babel-plugin returned no code for ${id}`);
        }
        return { code: result.code, map: result.map };
      },
    },
  ],
});
