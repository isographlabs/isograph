; Types
;------

(type_annotation
	(identifier) @type)

(directive) @type

; Properties
;-----------

(field
	(identifier) @property)

(field
  (alias
    (identifier) @property))

(object
  (object_field
    (identifier) @property))

; Variable Definitions and Arguments
;-----------------------------------

(entrypoint_declaration
  (parent_object_entity_name_and_selectable_name) @variable)

(client_field_declaration
  (parent_object_entity_name_and_selectable_name) @variable)

(client_pointer_declaration
  (parent_object_entity_name_and_selectable_name) @variable)

(argument
  (identifier) @parameter)

(variable_definition
  (variable) @parameter)

(argument
  (value
    (variable) @variable))

; Constants
;----------

(string) @string

(block_string) @string

(integer) @number

(boolean) @boolean

; Literals
;---------

(description) @comment

; Keywords
;----------

[
  "entrypoint"
  "field"
  "pointer"
  "to"
] @keyword

; Punctuation
;------------

[
 "("
 ")"
 "["
 "]"
 "{"
 "}"
] @punctuation.bracket

"=" @operator

":" @punctuation.delimiter

"!" @punctuation.special
