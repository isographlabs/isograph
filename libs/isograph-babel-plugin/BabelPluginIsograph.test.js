import { transform } from '@babel/core';
import { describe, expect, test, vi } from 'vitest';
import plugin from './BabelPluginIsograph';

// @ts-ignore
async function mock(mockedUri, stub) {
  const { Module } = await import('module');
  // @ts-ignore
  Module._load_original = Module._load;
  // @ts-ignore
  Module._load = (uri, parent) => {
    if (uri === mockedUri) return stub;
    // @ts-ignore
    return Module._load_original(uri, parent);
  };
}

// In order to test `require`
vi.hoisted(
  () =>
    void mock('cosmiconfig', () => ({
      searchSync: () => ({
        config: {
          project_root: './src/components',
          schema: './backend/schema.graphql',
          schema_extensions: ['./backend/schema-extension.graphql'],
          options: {
            module: 'esmodule',
          },
        },
        filepath:
          '/home/pablocrov/code/isograph/libs/isograph-babel-plugin/isograph.config.json',
      }),
    })),
);

describe('Babel plugin Isograph', () => {
  const transformerOpts = {
    babelrc: false,
    filename: './src/components/Home/Header/File.ts',
    plugins: [[plugin, {}]],
  };

  test('should return an identity for non called iso function', () => {
    const code = `
      export const HomeRoute = iso(\`
          field Query.HomeRoute @component {
            pets {
              id
            }
          }
        \`);
      `;

    const result = transform(code, transformerOpts) ?? { code: '' };

    expect(result.code).toMatchInlineSnapshot(
      `"export const HomeRoute = x => x;"`,
    );
  });

  test('should preserve function call when iso applied', () => {
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

    const result = transform(code, transformerOpts) ?? { code: '' };

    expect(result.code).toMatchInlineSnapshot(`
        "export const HomeRoute = function HomeRouteComponent() {
          return 'Render';
        };"
      `);
  });

  test('should transform the iso function to a require call', () => {
    const code = `iso(\`entrypoint Query.HomeRoute\`);`;

    const result = transform(code, transformerOpts) ?? { code: '' };

    expect(result.code).toMatchInlineSnapshot(
      `
      "import _HomeRoute from "../../__isograph/Query/HomeRoute/entrypoint.ts";
      _HomeRoute;"
    `,
    );
  });
});
