'use strict';

const pathModule = require('path');

function compileTag(t, path, config) {
  const callee = path.node.callee;
  if (t.isIdentifier(callee) && callee.name === 'iso' && path.node.arguments) {
    const { keyword, type, field } = getTypeAndField(path);
    if (keyword === 'entrypoint') {
      // This throws if the tag is invalid
      compileImportStatement(t, path, type, field, 'entrypoint', config);
    } else if (keyword === 'field') {
      // No-op
      return false;
    } else {
      throw new Error(
        "Invalid iso tag usage. Expected 'entrypoint' or 'field'.",
      );
    }
  }
  return false;
}

const typeAndFieldRegex = new RegExp(
  '\\s*(entrypoint|field)\\s*([^\\.\\s]+)\\.([^\\s\\(]+)',
  'm',
);

function getTypeAndField(path) {
  if (path.node.arguments.length !== 1) {
    throw new Error(
      `BabelPluginIsograph: Iso invocation require one parameter, found ${path.node.arguments.length}`,
    );
  }
  const quasis = path.node.arguments[0].quasis;
  if (quasis.length !== 1) {
    throw new Error(
      'BabelPluginIsograph: Substitutions are not allowed in iso fragments.',
    );
  }

  const content = quasis[0].value.raw;
  const typeAndField = typeAndFieldRegex.exec(content);

  const keyword = typeAndField[1];
  const type = typeAndField[2];
  const field = typeAndField[3];

  if (keyword == null || type == null || field == null) {
    throw new Error(
      'Malformed iso literal. I hope the iso compiler failed to accept this literal!',
    );
  }
  return { keyword, type, field };
}

function compileImportStatement(t, path, type, field, artifactType, config) {
  const filename = path.state.filename;
  const folder = pathModule.dirname(filename);
  const cwd = path.state.cwd;
  const artifactDirectory = pathModule.join(
    cwd,
    config.artifact_directory ?? config.project_root,
  );

  const fileToArtifactDir = pathModule.relative(folder, artifactDirectory);
  const artifactDirToArtifact = `/__isograph/${type}/${field}/${artifactType}.ts`;
  let fileToArtifact = pathModule.join(
    fileToArtifactDir,
    artifactDirToArtifact,
  );

  // If we do not have to traverse upward, e.g. if the resolver is in
  // src/HomePage, and the artifact directory is src/, then fileToArtifact
  // will start with a /. require('/...') is not good, as that is treated
  // as an absolute path. Or something. It should instead be './...'.
  if (fileToArtifact.startsWith('/')) {
    fileToArtifact = '.' + fileToArtifact;
  }

  path.replaceWith(
    t.memberExpression(
      t.CallExpression(t.Identifier('require'), [
        t.StringLiteral(fileToArtifact),
      ]),
      t.Identifier('default'),
    ),
  );
}

module.exports = compileTag;
