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
    variable_definitions: ($) => seq('(', commaSep($.variable_definition), ')'),
    variable_definition: ($) =>
      seq($.variable, ':', $.type_annotation, optional($.default_value)),
    selection_set: ($) => seq('{', repeat($.field), '}'),
    field: ($) =>
      seq(
        optional($.alias),
        $.identifier,
        optional($.arguments),
        optional($.directives),
        optional($.selection_set),
        optional(','),
      ),
    alias: ($) => seq($.identifier, ':'),
    arguments: ($) => seq('(', commaSep($.argument), ')'),
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
    object: ($) => seq('{', commaSep($.object_field), '}'),
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
 * Creates a rule to match one or more of the rules separated by a comma
 *
 * @param {Rule} rule
 *
 * @returns {SeqRule}
 */
function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

/**
 * Creates a rule to optionally match one or more of the rules separated by a comma
 *
 * @param {Rule} rule
 *
 * @returns {ChoiceRule}
 */
function commaSep(rule) {
  return optional(commaSep1(rule));
}
