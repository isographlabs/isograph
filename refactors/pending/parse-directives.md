# parse-directives: `@name` and `@name(args)`

Lands after parse-pointers.md. Every host that isograph attaches directives to already exists: entrypoints, fields, pointers, scalar selections, object selections.

The grammar stage stores the raw `@name` form. Typed sets (`EntrypointDirectiveSet`, `ScalarSelectionDirectiveSet`, `ClientScalarSelectableDirectiveSet`, …) are a later stage. Unknown names parse.

## The grammar this doc accepts

Zero or more, in the same chunk as the host, not separated by commas:

```
@ <Identifier> [<paren group>]
```

The paren group is `consume_argument_list`.

Sites, matching `crates/isograph_lang_parser/src/parse_iso_literal.rs`:

- entrypoint: after `Type.name`, last item of the chunk
- field: after variable definitions, before the description
- pointer: after the target type, before the description
- selection: after arguments, before the nested selection set

A line break before `@` ends the host chunk. `bar @loadable` is one selection. `bar\n@loadable` is a scalar `bar` plus a failed selection at `@`. That is language change 1 in parsing-plan.md.

## Change 1: `directives.rs`

Origin: `IsographFieldDirective` in `crates/isograph_lang_types/src/isograph_directives.rs` and `parse_directives` in `parse_iso_literal.rs`. Delta: the vec is `IsographFieldDirectiveList` so it can carry `ResolvePosition` and a span; `arguments` is `Option<WithSpan<ArgumentList>>`; no generated span for the empty case (`None` instead); the name is `IsographDirectiveNameWrapper`.

```rust
// from crates/isograph_parser/src/directives.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    ArgumentList, ChunkContentItem, Expectation, IsographResolutionNode, NonBracketToken,
    NonBracketTokenKind, ParseError, SemanticToken, consume_argument_list,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectiveListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographFieldDirectiveList(
    #[resolve_field] pub Vec<WithSpan<IsographFieldDirective>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectiveListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographFieldDirective {
    #[resolve_field]
    pub name: WithSpan<IsographDirectiveNameWrapper>,
    #[resolve_field]
    #[parent_variant(IsographFieldDirective)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectivePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographDirectiveNameWrapper(common_lang_types::IsographDirectiveName);

#[derive(Debug)]
pub enum IsographFieldDirectiveListParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    Selection(SelectionPath<'a>),
}

pub type IsographFieldDirectiveListPath<'a> = PositionResolutionPath<
    &'a IsographFieldDirectiveList,
    IsographFieldDirectiveListParent<'a>,
>;

pub type IsographFieldDirectivePath<'a> =
    PositionResolutionPath<&'a IsographFieldDirective, IsographFieldDirectiveListPath<'a>>;

pub type IsographDirectiveNameWrapperPath<'a> =
    PositionResolutionPath<&'a IsographDirectiveNameWrapper, IsographFieldDirectivePath<'a>>;
```

The list is not a chunk list and does not use `Slot`. Directives are sequential items of the host chunk.

`IsographFieldDirectiveList` is the type isograph does not name (`Vec<IsographFieldDirective>` stored in a `directive_set` field). The wrapper exists so the vec has a span and a parent.

one-kind-of-selection.md left `ArgumentList.parent_type` as `SelectionPath`. This doc introduces the enum.

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
);

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, SelectionPath<'a>>;
```

After:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<SelectionFieldArgument, UnparsedChunkItems>>>,
);

pub enum ArgumentListParent<'a> {
    Selection(SelectionPath<'a>),
    IsographFieldDirective(IsographFieldDirectivePath<'a>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;
```

`Selection.arguments` respells to `#[resolve_field]` + `#[parent_variant(Selection)]`.

```rust
// from crates/isograph_parser/src/directives.rs
fn next_is_at(cursor: &mut ItemCursor<'_>) -> bool {
    matches!(
        cursor.peek().map(|peek| peek.view().item.reference()),
        Some(ChunkContentItem::NonBracket(NonBracketToken(
            NonBracketTokenKind::At
        )))
    )
}

pub(crate) fn consume_directives(
    cursor: &mut ItemCursor<'_>,
) -> Result<Option<WithSpan<IsographFieldDirectiveList>>, WithSpan<ParseError>> {
    if !next_is_at(cursor) {
        return None.wrap_ok();
    }
    cursor
        .spanning(|cursor| {
            let mut directives = Vec::new();
            while let Some(at) =
                cursor.consume_token_if(NonBracketTokenKind::At, SemanticToken::DirectiveName)
            {
                directives.push(parse_directive_after_at(cursor, at.location)?);
            }
            IsographFieldDirectiveList(directives).wrap_ok()
        })
        .map(|list| list.wrap_some())
}

fn parse_directive_after_at(
    cursor: &mut ItemCursor<'_>,
    at: Span,
) -> Result<WithSpan<IsographFieldDirective>, WithSpan<ParseError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::DirectiveName)
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

`next_is_at` drops the peek before `spanning`. `@` without a name fails the host item (`parse_*` is all-or-nothing).

`lib.rs` adds `mod directives;` and `pub use directives::*;`.

## Change 2: hosts

Entrypoint. Origin field name: `directive_set`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct EntrypointDeclaration {
    #[resolve_field]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let directive_set = consume_directives(cursor)?;
    EntrypointDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name
            .interned()
            .map(ClientScalarSelectableNameWrapper),
        directive_set,
    }
    .wrap_ok()
```

`parse_entrypoint` becomes `Result` through `consume_directives`. It already returns `Result`.

Field. Origin field name: `directive_set`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationOrUsageList>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub description: Option<WithSpan<Description>>,
```

```rust
    let variable_definitions = consume_variable_declaration_list(cursor);
    let directive_set = consume_directives(cursor)?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
```

Pointer. Upstream field name is `directives`. The field here is `directive_set`: same name as the other two declaration kinds.

```rust
    #[resolve_field]
    #[parent_variant(PointerTarget)]
    pub target_type: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(ClientPointerDeclaration)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
    #[resolve_field]
    #[parent_variant(ClientPointerDeclaration)]
    pub description: Option<WithSpan<Description>>,
```

```rust
    let target_type = parse_type_annotation(cursor)?;
    let directive_set = consume_directives(cursor)?;
    let description = consume_description(cursor);
```

Selections. Upstream deserializes immediately into typed scalar/object directive sets. This stage stores the raw list on `Selection`.

```rust
// from crates/isograph_parser/src/selections.rs
pub struct Selection {
    #[resolve_field]
    pub reader_alias: Option<WithSpan<SelectionNameWrapper>>,
    #[resolve_field]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub selection_set: Option<WithSpan<SelectionSet>>,
}
```

```rust
// from crates/isograph_parser/src/selections.rs
    let arguments = consume_argument_list(cursor);
    let directive_set = consume_directives(cursor)?;
    let selection_set = consume_selection_set(cursor);
    Selection {
        reader_alias,
        name,
        arguments,
        directive_set,
        selection_set,
    }
    .wrap_ok()
```

`parse_selection` stays `Result`. The leftover test `a_directive_on_a_selection_is_trailing_leftover` is deleted; `@` is now consumed.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    IsographFieldDirectiveList(IsographFieldDirectiveListPath<'a>),
    IsographFieldDirective(IsographFieldDirectivePath<'a>),
    IsographDirectiveNameWrapper(IsographDirectiveNameWrapperPath<'a>),
```

A position on `@` answers `IsographFieldDirective` (the `@` span is part of the directive span; the name is the identifier). Hover on the identifier is `IsographDirectiveNameWrapper`.

## Tests

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn an_entrypoint_directive_parses() {
        let text = "entrypoint Query.foo @lazyLoad";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let directives = as_entrypoint(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries a directive");
        assert_eq!(directives.location, span_of(text, "@lazyLoad"));
        assert_eq!(directives.item.0.len(), 1);
        assert_eq!(
            directives.item.0[0].item.name.item,
            IsographDirectiveNameWrapper("lazyLoad".intern().to())
        );
        assert!(directives.item.0[0].item.arguments.is_none());
    }

    #[test]
    fn a_field_directive_sits_between_variables_and_the_description() {
        let text = "field Query.Foo($id: ID) @component \"the route\" { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let field = as_field(parse.reference());
        assert!(field.variable_definitions.is_some());
        assert_eq!(
            field
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .location,
            span_of(text, "@component")
        );
        assert!(field.description.is_some());
    }

    #[test]
    fn a_pointer_directive_sits_between_the_target_and_the_description() {
        let text = "pointer Pet.BestFriend to Pet @updatable \"x\" { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_pointer(parse.reference())
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .location,
            span_of(text, "@updatable")
        );
    }

    #[test]
    fn a_selection_directive_with_arguments_parses() {
        let text = "field Query.Foo { bar @loadable(lazyLoadArtifact: true) }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let selection = as_selection(selections(as_field(parse.reference()).selection_set.reference())[0].item.reference());
        let directives = selection
            .directive_set
            .as_ref()
            .expect("the fixture selects with a directive");
        assert_eq!(
            directives.location,
            span_of(text, "@loadable(lazyLoadArtifact: true)")
        );
        let arguments = directives.item.0[0]
            .item
            .arguments
            .as_ref()
            .expect("the fixture passes arguments");
        assert_eq!(arguments.item.0.len(), 1);
    }

    #[test]
    fn two_directives_on_one_selection_stay_in_one_list() {
        let text = "field Query.Foo { bar @loadable @updatable }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let directives = as_selection(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        )
        .directive_set
        .as_ref()
        .expect("the fixture selects with directives");
        assert_eq!(directives.item.0.len(), 2);
        assert_eq!(directives.location, span_of(text, "@loadable @updatable"));
    }

    #[test]
    fn a_directive_on_the_next_line_is_its_own_failed_selection() {
        let text = "field Query.Foo { bar\n@loadable }";
        let (parse, errors) = parsed(text);
        let items = selections(as_field(parse.reference()).selection_set.reference());
        assert_eq!(items.len(), 2);
        as_selection(items[0].item.reference());
        assert!(items[1].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == expected(Expectation::Selection, Found::Token(NonBracketTokenKind::At))
                && error.location == span_of(text, "@")
        }));
    }

    #[test]
    fn an_unknown_directive_name_parses() {
        let text = "entrypoint Query.foo @notARealDirective";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_entrypoint(parse.reference())
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .item
                .0[0]
                .item
                .name
                .item,
            IsographDirectiveNameWrapper("notARealDirective".intern().to())
        );
    }

    #[test]
    fn at_without_a_name_fails_the_host() {
        let text = "entrypoint Query.foo @";
        let end = span_of(text, "@").end;
        assert_no_declaration(
            text,
            expected(
                Expectation::Token(Identifier),
                Found::EndOfChunk,
            ),
            Span::new(end, end),
        );
    }

    #[test]
    fn directive_names_resolve_through_the_host() {
        let text = "field Query.Foo { bar @loadable }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "loadable")) {
            IsographResolutionNode::IsographDirectiveNameWrapper(name) => {
                match name.parent.parent.parent {
                    IsographFieldDirectiveListParent::Selection(_) => {}
                    parent => panic!("expected a scalar directive list, got {parent:?}"),
                }
            }
            node => panic!("expected the directive name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "@")) {
            IsographResolutionNode::IsographFieldDirective(_) => {}
            node => panic!("expected the directive, got {node:?}"),
        }
    }
```

The leftover-directive test in selections.rs is deleted.

## Landing checklist

1. `directives.rs`, the five host fields, `ArgumentListParent::IsographFieldDirective`, the resolution-node variants, the tests, the deleted leftover-directive test. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
