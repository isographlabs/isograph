import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { transform } from '@babel/core';
import { afterAll, describe, expect, test } from 'vitest';
import plugin from './BabelPluginIsograph';

/** @type {string[]} */
const projectDirectories = [];

afterAll(() => {
  projectDirectories.forEach((directory) =>
    fs.rmSync(directory, { recursive: true, force: true }),
  );
});

/**
 * Writes `config` as `isograph.config.json` into a new temp directory.
 * @param {Record<string, unknown>} config
 * @returns {string} the directory
 */
function projectWithConfig(config) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'isograph-babel-'));
  projectDirectories.push(directory);
  fs.writeFileSync(
    path.join(directory, 'isograph.config.json'),
    JSON.stringify(config),
  );
  return directory;
}

/**
 * @param {string} code
 * @param {string} projectDirectory
 * @param {string} relativeFilename
 * @param {'module' | 'script'} sourceType
 */
function transformIn(
  code,
  projectDirectory,
  relativeFilename = 'src/components/Home/Header/File.ts',
  sourceType = 'module',
) {
  return (
    transform(code, {
      babelrc: false,
      configFile: false,
      sourceType,
      filename: path.join(projectDirectory, relativeFilename),
      plugins: [[plugin, { searchFrom: projectDirectory }]],
    })?.code ?? ''
  );
}

describe('Babel plugin Isograph', () => {
  const project = projectWithConfig({ source_files: ['src/**'] });

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

    expect(transformIn(code, project)).toBe('export const HomeRoute = x => x;');
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

    expect(transformIn(code, project)).toBe(
      [
        'export const HomeRoute = function HomeRouteComponent() {',
        "  return 'Render';",
        '};',
      ].join('\n'),
    );
  });

  test('should treat pointer as field', () => {
    const code = 'export const p = iso(`pointer Query.p to Pet { id }`)(f);';

    expect(transformIn(code, project)).toBe('export const p = f;');
  });

  test('should transform an entrypoint in a module to an import', () => {
    const code = `function test() { const a=iso(\`entrypoint Query.HomeRoute\`); }`;

    expect(transformIn(code, project)).toBe(
      [
        'import _HomeRoute from "../../../../__isograph/Query/HomeRoute/entrypoint.ts";',
        'function test() {',
        '  const a = _HomeRoute;',
        '}',
      ].join('\n'),
    );
  });

  test('should import an entrypoint once when it appears twice', () => {
    const code =
      'const a = iso(`entrypoint Query.HomeRoute`); const b = iso(`entrypoint Query.HomeRoute`);';

    expect(transformIn(code, project)).toBe(
      [
        'import _HomeRoute from "../../../../__isograph/Query/HomeRoute/entrypoint.ts";',
        'const a = _HomeRoute;',
        'const b = _HomeRoute;',
      ].join('\n'),
    );
  });

  test('should transform an entrypoint in a script to a require call', () => {
    const code = 'const a = iso(`entrypoint Query.HomeRoute`);';

    expect(
      transformIn(
        code,
        project,
        'src/components/Home/Header/File.ts',
        'script',
      ),
    ).toBe(
      'const a = require("../../../../__isograph/Query/HomeRoute/entrypoint.ts").default;',
    );
  });

  test('should resolve the artifact under output_directory', () => {
    const outputProject = projectWithConfig({
      source_files: ['src/**'],
      output_directory: 'src/generated',
    });
    const code = 'const a = iso(`entrypoint Query.HomeRoute`);';

    expect(transformIn(code, outputProject, 'src/pages/Home.ts')).toBe(
      [
        'import _HomeRoute from "../generated/__isograph/Query/HomeRoute/entrypoint.ts";',
        'const a = _HomeRoute;',
      ].join('\n'),
    );
  });

  test('should prefix ./ when the artifact is in the same folder', () => {
    const code = 'const a = iso(`entrypoint Query.HomeRoute`);';

    expect(transformIn(code, project, 'File.ts')).toBe(
      [
        'import _HomeRoute from "./__isograph/Query/HomeRoute/entrypoint.ts";',
        'const a = _HomeRoute;',
      ].join('\n'),
    );
  });

  test('should prefix ./ when the artifact is below the file', () => {
    const outputProject = projectWithConfig({
      source_files: ['src/**'],
      output_directory: 'src/generated',
    });
    const code = 'const a = iso(`entrypoint Query.HomeRoute`);';

    expect(transformIn(code, outputProject, 'File.ts')).toBe(
      [
        'import _HomeRoute from "./src/generated/__isograph/Query/HomeRoute/entrypoint.ts";',
        'const a = _HomeRoute;',
      ].join('\n'),
    );
  });

  test('should reject an unknown keyword', () => {
    const code = 'const a = iso(`unknown Query.HomeRoute`);';

    expect(() => transformIn(code, project)).toThrow(
      'Malformed iso literal. I hope the iso compiler failed to accept this literal!',
    );
  });

  test('should reject a field call with two arguments', () => {
    const code = 'const a = iso(`field Query.HomeRoute { id }`)(f, g);';

    expect(() => transformIn(code, project)).toThrow(
      'Invalid iso tag usage. The iso function should be passed at most one argument.',
    );
  });

  test('should reject a substitution', () => {
    const code = 'const a = iso(`field Query.${x} { id }`)(f);';

    expect(() => transformIn(code, project)).toThrow(
      'BabelPluginIsograph: Substitutions are not allowed in iso fragments.',
    );
  });

  test('should reject a project without a config', () => {
    const directory = fs.mkdtempSync(
      path.join(os.tmpdir(), 'isograph-babel-no-config-'),
    );
    projectDirectories.push(directory);

    expect(() =>
      transform('const a = 1;', {
        babelrc: false,
        configFile: false,
        filename: path.join(directory, 'File.ts'),
        plugins: [[plugin, { searchFrom: directory }]],
      }),
    ).toThrow(
      'No config found. Do you have a isograph.config.json file somewhere?',
    );
  });
});
