import path from 'node:path';
import url from 'node:url';
import { transform } from '@swc/core';
import { describe, expect, test } from 'vitest';

const pluginPath = path.join(
  path.dirname(url.fileURLToPath(import.meta.url)),
  '..',
  'swc_isograph_plugin.wasm',
);

const rootDir = '/project';

function transformWithPlugin(
  code: string,
  pluginOptions: Record<string, unknown>,
  filename = '/project/src/components/HomeRoute.js',
) {
  return transform(code, {
    filename,
    isModule: true,
    jsc: {
      parser: { syntax: 'ecmascript' },
      target: 'es2018',
      experimental: { plugins: [[pluginPath, pluginOptions]] },
    },
  });
}

describe('the swc_isograph_plugin wasm under @swc/core', () => {
  test('an entrypoint becomes a hoisted import', async () => {
    const { code } = await transformWithPlugin(
      'export const a = iso(`entrypoint Query.HomeRoute`);',
      { root_dir: rootDir, config: { source_files: ['src/**'] } },
    );

    expect(code).toBe(
      [
        'import _Query__HomeRoute from "../../__isograph/Query/HomeRoute/entrypoint.ts";',
        'export const a = _Query__HomeRoute;',
        '',
      ].join('\n'),
    );
  });

  test('a called field literal becomes its argument', async () => {
    const { code } = await transformWithPlugin(
      'export const HomeRoute = iso(`field Query.HomeRoute { id }`)(f);',
      { root_dir: rootDir, config: { output_directory: 'src' } },
    );

    expect(code).toBe('export const HomeRoute = f;\n');
  });

  test('an entity literal becomes undefined', async () => {
    const { code } = await transformWithPlugin(
      'iso(`entity Owner`);\nexport const a = iso(`entity Owner`);',
      { root_dir: rootDir, config: {} },
    );

    expect(code).toBe('void 0;\nexport const a = void 0;\n');
  });

  test('invalid plugin options are an error', async () => {
    await expect(
      transformWithPlugin('const a = 1;', { config: {} }),
    ).rejects.toThrow('The Isograph SWC plugin options are invalid');
  });

  test('an invalid iso literal is an error', async () => {
    await expect(
      transformWithPlugin('const a = iso(`unknown Query.HomeRoute`);', {
        root_dir: rootDir,
        config: {},
      }),
    ).rejects.toThrow(
      "Invalid iso tag usage. Expected 'entity', 'entrypoint', 'field' or 'pointer'.",
    );
  });
});
