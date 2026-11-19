# Every directive in an iso literal is unexpected

In v1, the iso parser parses every directive as it does today (tokens, spans, `DirectiveName` highlighting, resolve leaves) and checks each directive name against the set of supported directives. That set is empty in v1. The parser reports the parse error "unexpected directive" on the span of each directive whose name is not in the set. Adding a directive later is adding a variant to that set.

Upstream supports `@component`, `@loadable`, and `@updatable`. Upstream's parser parses a generic directive list (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:512`, `parse_directives`) and then deserializes it with serde into a typed directive set. A selection's set is deserialized in the parser (`parse_iso_literal.rs:458` for `ObjectSelectionDirectiveSet`, `parse_iso_literal.rs:469` for `ScalarSelectionDirectiveSet`). The sets are `../isograph/crates/isograph_lang_types/src/selection_directive_set.rs:9` (`Loadable`, `Updatable`, `None`), `selection_directive_set.rs:17` (`Updatable`, `None`), and `../isograph/crates/isograph_lang_types/src/client_selectable_directive_set.rs:6` (`Component`, `None`). An unknown name fails `from_isograph_field_directives` (`../isograph/crates/isograph_lang_types/src/isograph_directives.rs:19`), which reports one diagnostic on the span of the whole directive list.

This plan deviates from upstream in two ways. v1 supports no directive, by user decision: v1 scope is a minimal compiler. The error is one per directive on that directive's span, by user decision, so each unsupported directive is marked on its own. The check stays in the parser, as upstream's selection check is.

v1 also drops everything `@loadable` drove (change 3), by user decision.

## Why `@loadable` goes too

`crates/isograph_compiler/src/artifact_types.rs` reads `@loadable` on a selection:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
const LOADABLE_DIRECTIVE: &str = "loadable";

/// `Loadable` when `selection` has `@loadable`.
fn loadability(selection: &Selection) -> Loadability {
    let is_loadable = selection
        .directive_set
        .iter()
        .flat_map(|directives| directives.item.0.iter())
        .any(|directive| directive.item.name.item.0.lookup() == LOADABLE_DIRECTIVE);
    if is_loadable {
        Loadability::Loadable
    } else {
        Loadability::Eager
    }
}
```

`Loadability` feeds `SelectedClientField::loadability`, `project_artifacts` computes the `loadable` set from it, and `FieldArtifacts::loadable_artifacts` decides whether `typescript_artifacts` writes the field's refetch reader, entrypoint, and normalization AST (`crates/typescript_artifacts/src/lib.rs:77`, `:92`). The pet demo uses it: `demos/pet-demo/src/PetDetailRoute.ts` selects `PetCheckinsCard(limit: 5) @loadable(lazyLoadArtifact: true)`, and `demos/pet-demo/src/__isograph/Pet/PetCheckinsCard/` holds `entrypoint.ts`, `normalization_ast.ts`, and `refetch_reader.ts` because of it. Tests that pin it: `crates/isograph_cli/src/compile.rs` `a_loadable_selection_generates_loadable_artifacts_only_when_reachable`, `crates/graphql_network_protocol/src/tests.rs` `a_selected_client_field_is_its_output_type_and_is_selected`.

A literal with any parse error contributes no artifact source (`crates/isograph_compiler/src/artifacts.rs`, `artifact_source` returns `None` when `parsed.errors` is non-empty). With an empty supported set, `field Query.PetDetailRoute` stops generating artifacts, and `loadability` can never return `Loadable` for a literal that generates artifacts. The `@loadable` path becomes code nothing reaches.

The user chose to remove the `@loadable` path from v1 (change 3) rather than support `@loadable` on selections, which is upstream's behavior and would need one supported set per directive position.

## Change 1: remove `@component` from the pet demo

Each pet demo field literal drops `@component`. The literals in `demos/pet-demo/src/`:

- `HomeRoute.ts`: `field Query.HomeRoute @component {`
- `Owner.ts`: `field Owner.OwnerBadge @component {`
- `PetStatsCard.ts`: `field Pet.PetStatsCard @component {`
- `PetBestFriendCard.ts`: `field Pet.PetBestFriendCard @component {`
- `PetDetailRoute.ts`: `field Query.PetDetailRoute($id: ID!) @component {`
- `PetCheckinsCard.ts`: `field Pet.PetCheckinsCard($limit: Int = 3) @component {`
- `PetSummaryCard.ts`: `field Pet.PetSummaryCard @component`
- `SpeciesRoute.ts`: `field Query.SpeciesRoute($species: PetSpecies!) @component {`

Before and after, for one:

```ts
// from demos/pet-demo/src/HomeRoute.ts
export const HomeRoute = iso(`
  field Query.HomeRoute @component {
```

```ts
// from demos/pet-demo/src/HomeRoute.ts
export const HomeRoute = iso(`
  field Query.HomeRoute {
```

The daemon regenerates `demos/pet-demo/src/__isograph/`. In `iso.ts`, each of those eight overloads returns `IdentityWithParam<P__f__param>` instead of `IdentityWithParamComponent<P__f__param>`. Each resolver is `function F({ data }) { ... }`, which type-checks against `(param: TParam) => TClientFieldReturn` as it did against `(data: TParam, componentProps: TComponentProps) => TClientFieldReturn`. `tsc --noEmit --strict` over `src` and `pnpm --filter pet-demo log-entrypoints` pass.

`demos/pet-demo/README.md`, "Sources", drops "(or `IdentityWithParamComponent` for `@component`)".

## Change 2: remove `ClientFieldKind` and `IdentityWithParamComponent`

Without `@component`, `ClientFieldKind` has one variant, `Plain`. The whole enum goes. Upstream keeps the distinction (`ClientScalarSelectableDirectiveSet::Component`, `client_selectable_directive_set.rs:6`); i2 drops it because `@component` is unsupported.

```rust
// from crates/isograph_compiler/src/schema.rs
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField(ClientFieldKind),
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
}

/// Whether a client field's declaration has `@component`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientFieldKind {
    Component,
    Plain,
}

pub enum SourceSelectableKind {
    ClientField(ClientFieldKind),
    ClientPointer(ClientPointerTarget),
}
```

```rust
// from crates/isograph_compiler/src/schema.rs
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
}

pub enum SourceSelectableKind {
    ClientField,
    ClientPointer(ClientPointerTarget),
}
```

`COMPONENT_DIRECTIVE` and `client_field_kind` in `schema.rs` are deleted. `source_selectable_definition` builds the kind without reading the directive set:

```rust
// from crates/isograph_compiler/src/schema.rs
        kind: match declaration.target_type.as_ref() {
            None => SourceSelectableKind::ClientField(client_field_kind(declaration)),
```

```rust
// from crates/isograph_compiler/src/schema.rs
        kind: match declaration.target_type.as_ref() {
            None => SourceSelectableKind::ClientField,
```

`SourceSelectableDefinition::selectable` maps `SourceSelectableKind::ClientField => SelectableKind::ClientField`. Every `SelectableKind::ClientField(_)` and `SourceSelectableKind::ClientField(_)` pattern becomes `ClientField` (`schema.rs` `target_entity` twice, `artifact_types.rs` `selection_type` and the scalar arm).

The test `a_component_directive_makes_a_component_client_field` in `schema.rs` is deleted, with its `kind_of` helper. The `@component` and `@loadable` cases it pinned have no remaining distinction to assert.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub enum FieldArtifactKind {
    ClientField(ClientFieldKind),
    Pointer(Wrapper<EntityName>),
}
```

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub enum FieldArtifactKind {
    ClientField,
    Pointer(Wrapper<EntityName>),
}
```

`project_artifacts` maps `SelectableKind::ClientField => FieldArtifactKind::ClientField`; the `link_types` walk matches `FieldArtifactKind::ClientField => Vec::new()`. `crates/isograph_compiler/src/lib.rs` stops re-exporting `ClientFieldKind`.

`crates/typescript_artifacts/src/iso_overloads.rs`:

```rust
// from crates/typescript_artifacts/src/iso_overloads.rs
    match kind {
        FieldArtifactKind::ClientField(ClientFieldKind::Plain) => {
            TypeScriptType::generic(IDENTITY_WITH_PARAM, param.wrap_vec())
        }
        FieldArtifactKind::ClientField(ClientFieldKind::Component) => {
            TypeScriptType::generic(IDENTITY_WITH_PARAM_COMPONENT, param.wrap_vec())
        }
```

```rust
// from crates/typescript_artifacts/src/iso_overloads.rs
    match kind {
        FieldArtifactKind::ClientField => {
            TypeScriptType::generic(IDENTITY_WITH_PARAM, param.wrap_vec())
        }
```

`IDENTITY_WITH_PARAM_COMPONENT`, the `identity_component` alias in `preamble`, its `t_props` and `component_function` closures, and the `IdentityWithParamComponent<any>` arm of `implementation`'s return union are deleted. The doc comments on `field_overload_return` and `preamble` drop `IdentityWithParamComponent`. `record_of_never` stays; `generate_types.rs` still calls it.

```rust
// from crates/typescript_artifacts/src/iso_overloads.rs
            return_type: TypeScriptType::Union(vec![
                TypeScriptType::generic(IDENTITY_WITH_PARAM, any().wrap_vec()),
                TypeScriptType::Keyword(TypeScriptKeyword::Object),
                TypeScriptType::Keyword(TypeScriptKeyword::Void),
            ]),
```

`crates/typescript_artifacts/src/lib.rs` tests: the `field` helper builds `FieldArtifactKind::ClientField`. `iso_overloads_are_typed_by_field_kind` no longer sets `home.kind` to a component; it asserts that `field Query.HomeRoute` ends with `): IdentityWithParam<Query__HomeRoute__param>;` and that `iso.ts` does not contain `IdentityWithParamComponent`.

`crates/isograph_lsp/src/hover.rs` test helper `pet_owner` builds `isograph_compiler::SelectableKind::ClientField`.

The daemon regenerates `demos/pet-demo/src/__isograph/iso.ts`, which loses the `IdentityWithParamComponent` alias and union arm.

## Change 3: remove the `@loadable` path

`Loadability`, `loadability`, and `LOADABLE_DIRECTIVE` in `crates/isograph_compiler/src/artifact_types.rs` are deleted. `SelectedClientField` would be left with one field, so it is deleted and `FieldTypes::selected` holds the selected fields directly:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct FieldTypes {
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    /// Every client field or pointer the selection set selects at any depth, in selection order.
    pub selected: Vec<SelectedClientField>,
}

pub struct SelectedClientField {
    pub field: EntityNameAndSelectableName,
    pub loadability: Loadability,
}
```

```rust
// from crates/isograph_compiler/src/artifact_types.rs
pub struct FieldTypes {
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    /// Every client field or pointer the selection set selects at any depth, in selection order.
    pub selected: Vec<EntityNameAndSelectableName>,
}
```

`object_type` and `selection_field` return `Vec<EntityNameAndSelectableName>`; `selected_self` builds the `EntityNameAndSelectableName`. `FieldArtifacts::loadable_artifacts` and the `loadable` set in `project_artifacts` are deleted; the reachability walk and the `output_types` set read `selected` without `.field`. `crates/isograph_compiler/src/lib.rs` stops re-exporting `Loadability` and `SelectedClientField`.

`crates/typescript_artifacts/src/lib.rs` `generate_artifacts` no longer writes `refetch_reader.ts`, and writes an entrypoint and normalization AST per `project.entrypoints` only:

```rust
// from crates/typescript_artifacts/src/lib.rs
    let per_entrypoint = project.entrypoints.iter().copied().flat_map(|entrypoint| {
```

`REFETCH_READER` in `entrypoint.rs` is deleted. The test `readers_refetch_readers_and_normalization_asts_are_empty_objects` becomes `readers_and_normalization_asts_are_empty_objects`, which generates `Pet.Checkins` as an entrypoint with a resolver reader and asserts that no `refetch_reader.ts` is written. `no_path_is_generated_twice` drops `loadable_artifacts`.

`crates/isograph_cli/src/compile.rs`: `generation_of` returns `(output_type, resolver_reader)`; `a_loadable_selection_generates_loadable_artifacts_only_when_reachable` is deleted. `crates/graphql_network_protocol/src/tests.rs` `a_selected_client_field_is_its_output_type_and_is_selected` selects `friends { f }` and asserts `selected == vec![named("Pet", "f"), named("Pet", "f")]`.

`demos/pet-demo/src/PetDetailRoute.ts` selects `PetCheckinsCard(limit: 5)`. The daemon deletes `src/__isograph/Pet/PetCheckinsCard/{entrypoint,normalization_ast,refetch_reader}.ts`. `demos/pet-demo/README.md` drops `@loadable`.

## Change 4: report every directive as unexpected

### The supported set

```rust
// from crates/isograph_parser/src/directives.rs
/// A directive name the iso parser accepts. v1 accepts none.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
enum SupportedDirectiveName {}
```

An empty enum with `strum::EnumString`. `"x".parse::<SupportedDirectiveName>()` is always `Err(strum::ParseError::VariantNotFound)` (strum 0.27's `ParseError` has that one variant). Adding `@loadable` later is a variant `#[strum(serialize = "loadable")] Loadable`, and the `match supported {}` arm in `parse_directive_after_at` stops compiling until the new variant is handled there. A `const SUPPORTED: &[&str] = &[]` would give the later reader a string to compare instead of a variant to match. A `bool` predicate `is_supported` is the boolean AGENTS.md rules out, and it also hands the reader no variant. `strum` is already a dependency of `isograph_parser`. The type is private to `directives.rs`; nothing outside reads it.

One set covers the three positions a directive list can sit in (`EntrypointDeclaration`, `SelectableDeclaration`, `Selection`). Upstream's sets differ by position; with an empty set the positions agree. The first directive that is valid in only some positions splits the set into one enum per position.

### The error

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum AstError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum AstError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
    #[error("unexpected directive")]
    UnexpectedDirective,
}
```

The message is exactly `unexpected directive`, by user decision; the other messages are capitalized sentences.

### The check

`parse_directive_after_at` parses as it does today, then checks the name. The error is reported with `ItemCursor::report_error`, which records it without failing the parse. The directive (with its name and arguments) stays in the AST, so highlighting, resolve, and the rest of the literal are unchanged.

```rust
// from crates/isograph_parser/src/directives.rs
fn parse_directive_after_at(
    cursor: &mut ItemCursor<'_>,
    at: Span,
) -> Result<WithSpan<IsographFieldDirective>, WithSpan<AstError>> {
    let name = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::DirectiveName,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let arguments = consume_argument_list(cursor);
    let end = arguments
        .as_ref()
        .map(|list| list.location.end)
        .unwrap_or(name.location.end);
    IsographFieldDirective {
        name: name.interned().map(IsographDirectiveNameWrapper),
        arguments,
    }
    .with_span(Span::new(at.start, end))
    .wrap_ok()
}
```

```rust
// from crates/isograph_parser/src/directives.rs
fn parse_directive_after_at(
    cursor: &mut ItemCursor<'_>,
    at: Span,
) -> Result<WithSpan<IsographFieldDirective>, WithSpan<AstError>> {
    let name = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::DirectiveName,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let arguments = consume_argument_list(cursor);
    let end = arguments
        .as_ref()
        .map(|list| list.location.end)
        .unwrap_or(name.location.end);
    let location = Span::new(at.start, end);
    match name.text().parse::<SupportedDirectiveName>() {
        Ok(supported) => match supported {},
        Err(strum::ParseError::VariantNotFound) => {
            cursor.report_error(AstError::UnexpectedDirective.with_span(location));
        }
    }
    IsographFieldDirective {
        name: name.interned().map(IsographDirectiveNameWrapper),
        arguments,
    }
    .with_span(location)
    .wrap_ok()
}
```

The error span is the directive's span: `@` through the name, or through the closing `)` of its argument list. Upstream's directive span stops at the name (`parse_iso_literal.rs:526`, `Span::join(token.location.span, name.location.span)`); i2's directive span already includes the arguments, and the user decision is the directive's span. The error is reported after the argument list is consumed, so an error inside the argument list precedes the directive's error in `ParsedIsoLiteral::errors`. Nothing sorts or reads that order.

Recovery is the existing parse. `@name` and its argument list are consumed, the directive is pushed, and `consume_directives` continues with the next `@`. Then the host continues: a field declaration's description and selection set, a selection's selection set, an entrypoint's end. `@` with no name stays the existing host failure (`at_without_a_name_fails_the_host`). A directive after the description stays the existing leftover error (`a_directive_after_the_description_is_leftover`); the grammar never parses it as a directive, so it reports no `unexpected directive`.

Unchanged: `IsographSemanticToken::DirectiveName` and `At`, the `DECORATOR` mapping in `crates/isograph_lsp/src/semantic_tokens.rs`, `IsographFieldDirectiveList`, `IsographFieldDirective`, `IsographDirectiveNameWrapper` and their resolve paths, `IsographResolutionNode`'s three directive variants, and the `completion.rs` / `entity_or_selectable.rs` arms for them. After change 3, the readers of the directive AST are resolve and the check.

Users see a diagnostic `unexpected directive` under `@component`, `@loadable(lazyLoadArtifact: true)`, or any other directive, in the editor and in the daemon's compile errors. The rest of the literal highlights, hovers, and validates. The literal produces no artifacts, as any literal with a parse error does, and the LSP does not format it.

### Tests

In `crates/isograph_parser/src/parse_iso_literal.rs`, the new tests below replace `an_entrypoint_directive_parses`, `a_field_directive_sits_between_variables_and_the_description`, and `a_selection_directive_with_arguments_parses`. These tests keep their names, fixtures, AST assertions, and token assertions, and replace `assert_eq!(errors, vec![])` with `assert_eq!(errors, unexpected_directives(text, &[..]))` naming each directive in the fixture: `a_field_directive_sits_between_the_target_and_the_description` (`@updatable`), `two_directives_on_one_selection_stay_in_one_list` (`@loadable`, `@updatable`), `an_unknown_directive_name_parses` (`@notARealDirective`), `a_description_and_selection_set_on_following_lines_attach` (`@component`), `a_directive_without_vars_or_to_parses` (`@component`), `a_full_header_includes_directives` (`@updatable`), `two_directives_on_a_field_stay_in_one_list` (`@a`, `@b`), `two_directives_on_an_entrypoint_stay_in_one_list` (`@a`, `@b`), `a_directive_with_empty_arguments_parses` (`@loadable()`), `a_selection_with_alias_arguments_directives_and_a_nested_set_parses` (`@loadable`). `a_to_after_a_directive_is_leftover` asserts `errors` is the `@component` error then the existing leftover error at `to`, in that order, since the directive is checked before the declaration ends. `field_directive_names_resolve_through_the_declaration` and `directive_names_resolve_through_the_host` keep their resolve assertions; they already ignore errors. "Parses" in a kept name still holds: the AST carries the directive.

A helper in the test module:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn unexpected_directives(text: &str, directives: &[&str]) -> Vec<WithSpan<AstError>> {
        directives
            .iter()
            .map(|directive| AstError::UnexpectedDirective.with_span(span_of(text, directive)))
            .collect()
    }
```

The tests that pin the new behavior, each with full semantic tokens asserted through `parsed`:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_field_directive_before_the_description_is_unexpected_and_the_rest_parses() {
        let text = "field Query.Foo($id: ID) @component \"the route\" { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::Description, "\"the route\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, unexpected_directives(text, &["@component"]));
        let field = as_selectable(parse.reference());
        assert!(field.variable_definitions.is_some());
        assert!(field.description.is_some());
        assert_eq!(selections(selection_set_of(field)).len(), 1);
    }

    #[test]
    fn a_selection_directive_with_arguments_is_unexpected_on_the_whole_directive() {
        let text = "field Query.Foo { bar @loadable(lazyLoadArtifact: true), baz }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "lazyLoadArtifact"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "true"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::FieldName, "baz"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(
            errors,
            unexpected_directives(text, &["@loadable(lazyLoadArtifact: true)"])
        );
        let items = selections(selection_set_of(as_selectable(parse.reference())));
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[1].item.reference()).name.location,
            span_of(text, "baz")
        );
    }

    #[test]
    fn each_of_several_directives_is_unexpected() {
        let text = "field Query.Foo @a @b(x: 1) { bar @c @d }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "a"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "b"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "x"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "c"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "d"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(
            errors,
            unexpected_directives(text, &["@a", "@b(x: 1)", "@c", "@d"])
        );
        let field = as_selectable(parse.reference());
        assert_eq!(
            field
                .directive_set
                .as_ref()
                .expect("the fixture carries directives")
                .item
                .0
                .len(),
            2
        );
    }

    #[test]
    fn an_entrypoint_directive_is_unexpected() {
        let text = "entrypoint Query.foo @lazyLoad";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "lazyLoad"),
            ],
        );
        assert_eq!(errors, unexpected_directives(text, &["@lazyLoad"]));
        assert_eq!(
            as_entrypoint(parse.reference()).name.location,
            span_of(text, "foo")
        );
    }

    #[test]
    fn a_directive_with_an_argument_error_reports_both() {
        let text = "field Query.Foo { bar @loadable(x: 99999999999999999999) }";
        let (_, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::At, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "x"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "99999999999999999999"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(
            errors,
            vec![
                AstError::IntegerDoesNotFitI64.with_span(span_of(text, "99999999999999999999")),
                AstError::UnexpectedDirective
                    .with_span(span_of(text, "@loadable(x: 99999999999999999999)")),
            ]
        );
    }
```

`a_directive_after_the_description_is_leftover` keeps its fixture and tokens and adds one assertion:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        assert!(
            errors
                .iter()
                .all(|error| error.item != AstError::UnexpectedDirective)
        );
```

`AstError::UnexpectedDirective.to_string()` is asserted to be `"unexpected directive"` in `parse_error.rs` `ast_error_unit_variants_use_their_messages`.

`crates/isograph_parser/src/format.rs`: `a_blank_line_after_a_comma_does_not_change_the_print` and `at_has_a_space_before` use fixtures with directives, and `formatted` asserts no parse errors. Those two call a test helper `formatted_with_unexpected_directives(text)`, which asserts that every parse error is `AstError::UnexpectedDirective` and then calls `format_recorded`. The formatter's `At` spacing stays for when a directive is supported.

`crates/isograph_cli/src/compile.rs` `HOME_ROUTE` and `crates/isograph_lsp/src/file_diagnostics.rs` `pet_demo_like_source_only_literals_have_no_diagnostics` drop `@component` from their fixtures. A new test in `file_diagnostics.rs` asserts that `iso(\`field Pet.f @component { id }\`)` has exactly one diagnostic, message `unexpected directive`, on the range of `@component`.

`crates/swc_isograph_plugin/tests/transform.rs` and `libs/isograph-babel-plugin/BabelPluginIsograph.test.js` keep `@component` in their fixtures. Neither plugin parses directives; the fixtures are literal text the transform copies.

## Change order

Each is one commit that passes the pre-commit hook.

1. `demos/pet-demo/src/*.ts` drop `@component`; regenerate `demos/pet-demo/src/__isograph/`; `demos/pet-demo/README.md`.
2. Remove `ClientFieldKind` and `IdentityWithParamComponent` (`crates/isograph_compiler/src/schema.rs`, `artifact_types.rs`, `lib.rs`; `crates/typescript_artifacts/src/iso_overloads.rs`, `lib.rs`; `crates/isograph_lsp/src/hover.rs`); regenerate `demos/pet-demo/src/__isograph/iso.ts`.
3. Remove the `@loadable` path (`crates/isograph_compiler/src/artifact_types.rs`, `lib.rs`; `crates/typescript_artifacts/src/lib.rs`, `entrypoint.rs`; `crates/isograph_cli/src/compile.rs`; `crates/graphql_network_protocol/src/tests.rs`; `demos/pet-demo/src/PetDetailRoute.ts`, the regenerated `src/__isograph/`, `README.md`).
4. `SupportedDirectiveName`, `AstError::UnexpectedDirective`, the check in `parse_directive_after_at`, and the test changes in `crates/isograph_parser`, `crates/isograph_cli/src/compile.rs`, and `crates/isograph_lsp/src/file_diagnostics.rs`. `docs-website/docs/design-docs/mental-model.md` line 225 states that v1 supports no directive and reports each as `unexpected directive`.
