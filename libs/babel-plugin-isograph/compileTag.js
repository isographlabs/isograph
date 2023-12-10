"use strict";

const pathModule = require("path");

function compileTag(t, path, config) {
  const tag = path.get("tag");

  if (tag.isIdentifier({ name: "iso" })) {
    // Don't do anything for iso tags
  }
  if (tag.isIdentifier({ name: "isoFetch" })) {
    return compileIsoFetchTag(t, path, config);
  }

  return false;
}

const typeAndFieldRegex = new RegExp("\\s*([^\\.\\s]+)\\.([^\\s\\(]+)", "m");

function compileIsoFetchTag(t, path, config) {
  // This throws if the tag is invalid
  const { type, field } = getTypeAndField(path);
  compileImportStatement(t, path, type, field, "entrypoint", config);
}

function getTypeAndField(path) {
  const quasis = path.node.quasi.quasis;

  if (quasis.length !== 1) {
    throw new Error(
      "BabelPluginIsograph: Substitutions are not allowed in iso fragments."
    );
  }

  const content = path.node.quasi.quasis[0].value.raw;
  const typeAndField = typeAndFieldRegex.exec(content);
  const type = typeAndField[1];
  const field = typeAndField[2];

  if (type == null || field == null) {
    throw new Error(
      "Malformed iso literal. I hope the iso compiler failed to accept this literal!"
    );
  }
  return { type, field };
}

function compileImportStatement(t, path, type, field, artifactType, config) {
  const filename = path.state.filename;
  const folder = pathModule.dirname(filename);
  const cwd = path.state.cwd;
  const artifactDirectory = pathModule.join(cwd, config.artifact_directory);

  const fileToArtifactDir = pathModule.relative(folder, artifactDirectory);
  const artifactDirToArtifact = `/__isograph/${type}/${field}/${artifactType}.isograph.ts`;
  const fileToArtifact = pathModule.join(
    fileToArtifactDir,
    artifactDirToArtifact
  );

  path.replaceWith(
    t.memberExpression(
      t.CallExpression(t.Identifier("require"), [
        t.StringLiteral(fileToArtifact),
      ]),
      t.Identifier("default")
    )
  );
}

module.exports = compileTag;
