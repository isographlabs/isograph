'use strict';

const { addDefault } = require('@babel/helper-module-imports');
const pathModule = require('path');
const os = require('os');

/**
 * @typedef {import("@babel/core")} babel
 * @param {typeof babel.types} t
 * @param {babel.NodePath<babel.types.CallExpression>} path
 * @param {NonNullable<import("cosmiconfig").CosmiconfigResult>} config
 */
function compileTag(t, path, config) {
  const callee = path.node.callee;
  if (t.isIdentifier(callee) && callee.name === 'iso' && path.node.arguments) {
    const { keyword, parentObjectEntityName, selectableName } =
      getParentObjectEntityNameAndSelectableName(path);
    if (keyword === 'entrypoint') {
      // This throws if the tag is invalid
      compileImportStatement(
        t,
        path,
        parentObjectEntityName,
        selectableName,
        'entrypoint',
        config,
      );
    } else if (keyword === 'field' || keyword === 'pointer') {
      if (t.isCallExpression(path.parentPath.node)) {
        const firstArg = path.parentPath.node.arguments[0];
        if (path.parentPath.node.arguments.length === 1 && firstArg != null) {
          path.parentPath.replaceWith(firstArg);
        } else {
          throw new Error(
            'Invalid iso tag usage. The iso function should be passed at most one argument.',
          );
        }
      } else {
        path.replaceWith(
          t.arrowFunctionExpression([t.identifier('x')], t.identifier('x')),
        );
      }
    } else {
      throw new Error(
        "Invalid iso tag usage. Expected 'entrypoint', 'field' or 'pointer'.",
      );
    }
  }
  return false;
}

const parentObjectEntityNameAndSelectableNameRegex = new RegExp(
  '\\s*(entrypoint|field|pointer)\\s*([^\\.\\s]+)\\.([^\\s\\(]+)',
  'm',
);

/**
 * @param {babel.NodePath<babel.types.CallExpression>} path
 **/
function getParentObjectEntityNameAndSelectableName(path) {
  const firstArg = path.node.arguments[0];
  if (path.node.arguments.length !== 1 || firstArg == null) {
    throw new Error(
      `BabelPluginIsograph: Iso invocation require one parameter, found ${path.node.arguments.length}`,
    );
  }

  if (firstArg.type !== 'TemplateLiteral') {
    throw new Error(
      'BabelPluginIsograph: Only template literals are allowed in iso fragments.',
    );
  }

  const quasis = firstArg.quasis;
  const firstQuasi = quasis[0];
  if (quasis.length !== 1 || firstQuasi == null) {
    throw new Error(
      'BabelPluginIsograph: Substitutions are not allowed in iso fragments.',
    );
  }

  const content = firstQuasi.value.raw;
  const typeAndField =
    parentObjectEntityNameAndSelectableNameRegex.exec(content);

  const keyword = typeAndField?.[1];
  const parentObjectEntityName = typeAndField?.[2];
  const selectableName = typeAndField?.[3];

  if (
    keyword == null ||
    parentObjectEntityName == null ||
    selectableName == null
  ) {
    throw new Error(
      'Malformed iso literal. I hope the iso compiler failed to accept this literal!',
    );
  }
  return { keyword, parentObjectEntityName, selectableName };
}

/**
 * @param {typeof babel.types} t
 * @param {babel.NodePath<babel.types.CallExpression>} path
 * @param {string} type
 * @param {string} field
 * @param {string} artifactType
 * @param {NonNullable<import("cosmiconfig").CosmiconfigResult>} config
 */
function compileImportStatement(t, path, type, field, artifactType, config) {
  const filename = path.state.filename;
  const folder = pathModule.dirname(filename);
  const cwd = pathModule.dirname(config.filepath);
  const artifactDirectory = pathModule.join(
    cwd,
    config.config['artifact_directory'] ?? config.config['project_root'],
  );
  const module = config.config['options']?.['module'];

  const fileToArtifactDir = pathModule.relative(folder, artifactDirectory);
  const artifactDirToArtifact = `/__isograph/${type}/${field}/${artifactType}.ts`;
  let fileToArtifact = pathModule.join(
    fileToArtifactDir,
    artifactDirToArtifact,
  );

  if (os.platform() === 'win32') {
    fileToArtifact = fileToArtifact.replace(/\\/g, '/');
  }

  // If we do not have to traverse upward, e.g. if the resolver is in
  // src/HomePage, and the artifact directory is src/, then fileToArtifact
  // will start with a /. require('/...') is not good, as that is treated
  // as an absolute path. Or something. It should instead be './...'.
  if (fileToArtifact.startsWith('/')) {
    fileToArtifact = '.' + fileToArtifact;
  }

  if (module === 'esmodule') {
    const program = path.scope.getProgramParent();
    const imports = /** @type {Map<string, string>} */ (
      program.data['imports'] ??= new Map()
    );

    let id = imports.get(fileToArtifact);
    if (id == null) {
      id = addDefault(path, fileToArtifact, { nameHint: field }).name;
      imports.set(fileToArtifact, id);
    }

    path.replaceWith(t.identifier(id));
  } else {
    path.replaceWith(
      t.memberExpression(
        t.callExpression(t.identifier('require'), [
          t.stringLiteral(fileToArtifact),
        ]),
        t.identifier('default'),
      ),
    );
  }
}

module.exports = compileTag;
