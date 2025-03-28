import fs from 'node:fs/promises';
import path from 'node:path';
import url from 'node:url';
import { transform } from '@swc/core';
import { describe, expect, test } from 'vitest';

const pluginName = 'swc_isograph_plugin.wasm';

const transformCode = (code: string, options = {}, filename = '') => {
  return transform(code, {
    jsc: {
      parser: {
        syntax: 'ecmascript',
      },
      target: 'es2018',
      experimental: {
        plugins: [
          [
            path.join(
              // @ts-ignore
              path.dirname(url.fileURLToPath(import.meta.url)),
              '..',
              pluginName,
            ),
            options,
          ],
        ],
      },
    },
    filename,
  });
};

async function walkDir(
  dir: URL,
  callback: (
    dir: string,
    input: string,
    config?: Record<string, unknown>,
    filename?: string,
    baseDir?: string,
  ) => Promise<void>,
) {
  const dirs = await fs.readdir(dir);
  const baseDir = url.fileURLToPath(dir);

  for (const dir of dirs) {
    const inputFilePath = path.join(baseDir, dir, 'input.js');
    const configPath = path.join(baseDir, dir, 'isograph.config.json');

    const isographConfig = await fs.readFile(configPath, 'utf-8').then(
      (json) => {
        return JSON.parse(json);
      },
      (_) => undefined,
    );

    const config = {
      // must be an absolute path
      root_dir: path.join(baseDir, dir),
      ...(isographConfig || {}),
    };

    const filename = path.join(
      baseDir,
      dir,
      `/src/components/Home/Header/File.ts`,
    );

    try {
      const input = await fs.readFile(inputFilePath, 'utf-8');
      await callback(dir, input, config, filename, baseDir);
    } catch (e) {
      console.log(e);
    }
  }
}

describe('Should load swc-plugin-isograph wasm plugin correctly', async () => {
  await walkDir(
    new URL(
      '../../../crates/swc_isograph_plugin/transform/tests/fixtures/base',
      // @ts-ignore
      import.meta.url,
    ),
    async (dir, input, config, filename) => {
      await test(`Should transform ${dir} correctly`, async () => {
        const { code } = await transformCode(input, config, filename);
        expect(code).toMatchSnapshot();
      });
    },
  );
});
