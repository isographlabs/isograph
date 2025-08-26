/**
 * @file Isograph grammar for tree-sitter
 * @author Isograph Labs
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: 'isograph',

  supertypes: ($) => [$.declaration, $.value],
  externals: ($) => [$._newline],
  extras: () => [/[ \t\r\n\f\ufeff]/],

  rules: {
    source_file: ($) => $.declaration,
    declaration: ($) =>
      choice(
        $.entrypoint_declaration,
        $.client_field_declaration,
        $.client_pointer_declaration,
      ),
    entrypoint_declaration: ($) =>
      seq(
        'entrypoint',
        $.parent_object_entity_name_and_selectable_name,
        optional($.directives),
      ),
    client_field_declaration: ($) =>
      seq(
        'field',
        $.parent_object_entity_name_and_selectable_name,
        optional($.variable_definitions),
        optional($.directives),
        optional($.description),
        $.selection_set,
      ),
    client_pointer_declaration: ($) =>
      seq(
        'pointer',
        $.parent_object_entity_name_and_selectable_name,
        'to',
        $.type_annotation,
        optional($.variable_definitions),
        optional($.directives),
        optional($.description),
        $.selection_set,
      ),
    parent_object_entity_name_and_selectable_name: ($) =>
      seq(
        field('parent_object_entity_name', $.identifier),
        '.',
        field('selectable_name', $.identifier),
      ),
    default_value: ($) => seq('=', $.value),
    variable_definitions: ($) => seq('(', sep($.variable_definition, ','), ')'),
    variable_definition: ($) =>
      seq($.variable, ':', $.type_annotation, optional($.default_value)),
    selection_set: ($) =>
      seq(
        '{',
        optional($._newline),
        sep($.field, $._newline),
        optional($._newline),
        '}',
      ),
    field: ($) =>
      seq(
        optional($.alias),
        $.identifier,
        optional($.arguments),
        optional($.directives),
        optional($.selection_set),
      ),
    alias: ($) => seq($.identifier, ':'),
    arguments: ($) =>
      seq(
        '(',
        optional($._newline),
        sep($.argument, $._newline),
        optional($._newline),
        ')',
      ),
    argument: ($) => seq($.identifier, ':', $.value),
    value: ($) =>
      choice($.variable, $.string, $.integer, $.object, $.null, $.boolean),
    variable: ($) => seq('$', $.identifier),
    string: ($) =>
      seq(
        '"',
        repeat(
          choice(
            $._escaped_character,
            $._escaped_unicode,
            $._string_characters,
          ),
        ),
        '"',
      ),
    block_string: () =>
      seq(
        '"""',
        repeat(choice('\\"""', /[\u0009\u000A\u000D\u0020-\uFFFF]/)),
        '"""',
      ),
    integer: () => /-?(0|[1-9][0-9]*)/,
    boolean: () => choice('true', 'false'),
    null: () => 'null',
    object: ($) =>
      seq(
        '{',
        optional($._newline),
        sep($.object_field, $._newline),
        optional($._newline),
        '}',
      ),
    object_field: ($) => seq($.identifier, ':', $.value),
    _escaped_character: () => /\\["\\/bfnrt]/,
    _escaped_unicode: () => /\\u[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]"/,
    _line_terminator: () => /\n|\r|\r\n/,
    _string_characters: () => /[\u0009\u0020\u0021\u0023-\u005B\u005D-\uFFFF]+/,
    directives: ($) => repeat1($.directive),
    directive: ($) => seq('@', $.identifier, optional($.arguments)),
    type_annotation: ($) =>
      choice(
        seq($.identifier, optional('!')),
        seq('[', $.type_annotation, ']', optional('!')),
      ),
    identifier: () => /[a-zA-Z_][a-zA-Z0-9_]*/,
    description: ($) => choice($.string, $.block_string),
  },
});

/**
 * Creates a rule to match one or more of the rules separated by a separator
 *
 * @param {Rule} rule
 * @param {Rule | string} separator
 *
 * @returns {SeqRule}
 */
function sep1(rule, separator) {
  return seq(rule, repeat(seq(separator, rule)));
}

/**
 * Creates a rule to optionally match one or more of the rules separated by a separator
 *
 * @param {Rule} rule
 * @param {Rule | string} separator
 *
 * @returns {ChoiceRule}
 */
function sep(rule, separator) {
  return optional(sep1(rule, separator));
}
