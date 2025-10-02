'use strict';

const compileTag = require('./compileTag');
const cosmiconfig = require('cosmiconfig');

const configExplorer = cosmiconfig('isograph', {
  searchPlaces: ['isograph.config.json'],
  loaders: {
    '.json': cosmiconfig.loadJson,
  },
});
/** @type {NonNullable<import("cosmiconfig").CosmiconfigResult>} */
let IsographConfig;

/** @typedef {import("@babel/core")} babel*/

/**
 * @typedef {Object} Context
 * @property  {typeof babel.types} [types]
 * */

/**
 * @typedef {Object} Options
 * @property  {string} [searchFrom]
 * */

/**
 * @param {Context} context
 * @param {Options} options
 * @returns {babel.PluginObj} */
module.exports = function BabelPluginIsograph(context, options) {
  const result = configExplorer.searchSync(options.searchFrom);
  if (result) {
    IsographConfig = result;
  } else {
    throw new Error(
      'No config found. Do you have a isograph.config.json file somewhere?',
    );
  }

  const { types } = context;
  if (!types) {
    throw new Error(
      'BabelPluginIsograph: Expected plugin context to include "types", but got:' +
        String(context),
    );
  }

  /** @type {babel.Visitor<babel.PluginPass>} */
  const visitor = {
    CallExpression(path) {
      compileTag(types, path, IsographConfig);
    },
  };

  return {
    visitor: {
      Program(path, state) {
        path.traverse(visitor, state);
      },
    },
  };
};
