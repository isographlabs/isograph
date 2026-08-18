# One value type

Defaults reject `$` at parse time. parse-arguments.md and parse-variables.md land two trees for that: `NonConstantValue` and `ConstantValue`, with a second object-literal path.

This doc replaces the second tree. Lands after parse-variables.md. The parse-time rejection of `$` stays; the type of a default is the same value type as an argument, or a narrower type that cannot represent `Variable`, written out here when we do this.

## Landing checklist

1. Write the types, the parse functions, the call sites, and the tests. Do not start until this doc names them.
2. Move this doc to refactors/past.
