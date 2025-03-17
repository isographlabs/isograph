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
          options: {},
        },
        filepath: 'file.ts',
      }),
    })),
);

describe('Babel plugin Isograph', () => {
  const transformerOpts = {
    babelrc: false,
    filename: 'file.ts',
    plugins: [[plugin, {}]],
  };

  test('should transform iso function to identity function', () => {
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

    expect(result.code).toMatchInlineSnapshot(`"export const HomeRoute = x => x;"`);
  });

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
      `"require("src/components/__isograph/Query/HomeRoute/entrypoint.ts").default;"`,
    );
  });
});
