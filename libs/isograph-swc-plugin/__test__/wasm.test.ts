import path from 'node:path';
import url from 'node:url';
import { transformSync } from '@swc/core';
import { describe, expect, test } from 'vitest';

const pluginName = 'swc_isograph_plugin.wasm';

describe('SWC plugin Isograph', () => {
  const transform =  (
    code: string,
    options = {},
  ) => {
    return transformSync(code, {
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
      filename: 'test.js',
    });
  };

  test('should transform the iso function to an exported default', () => {
    const code = `
      export const HomeRoute = iso(\`
          field Query.HomeRoute @component {
            pets {
              id
            }
          }
        \`)(function HomeRouteComponent() {
        return 'Render';
      });
      `;

    const result = transform(code) ?? { code: '' };

    expect(result.code).toMatchInlineSnapshot(`
        "export const HomeRoute = function HomeRouteComponent() {
          return 'Render';
        };"
      `);
  });
});
