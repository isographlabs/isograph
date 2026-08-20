# Mental model

The schema is a graph. An entity is a node. A selectable is a named pointer from an entity to a wrapper of an entity.

There is one kind of entity and one kind of selectable. `User`, `String`, `Query`, and the result of `field User.Avatar` are all entities.

Isograph takes upstream entities and selectables, for example from a GraphQL schema, and adds ones defined in the project. A `SelectableDeclaration` (iso keyword `field`) adds a selectable. An `EntityDeclaration` (iso keyword `type`, future) adds a named entity.

The building blocks are `Wrapper`, `Entity`, `EntityDeclaration`, `Selectable`, `SelectableDeclaration`, `Selection`, `SelectionSet`, and `Entrypoint`.

## Wrapper

A wrapper is a type former over an entity. Wrappers nest.

```rust
enum Wrapper {
    Entity(Entity),
    List(Box<Wrapper>),
    Null(Box<Wrapper>),
}
```

`List` is `[W]`. `Null` is `W | null`.

```text
Foo
Foo | null
[Foo]
[Foo | null]
[Foo] | null
```

A GraphQL type annotation is a wrapper:

```text
Foo!      ->  Foo
Foo       ->  Foo | null
[Foo!]!   ->  [Foo]
[Foo]     ->  [Foo | null] | null
```

A selectable's target is a `Wrapper`. Nested selections select on the inner entity, not on the wrapper.

## Entity

An entity is named or anonymous.

```rust
enum Entity {
    Named(NamedEntity),
    Anonymous(AnonymousEntity),
}

struct NamedEntity {
    name: EntityName,
    selectables: BTreeMap<SelectableName, Selectable>,
}

struct AnonymousEntity {
    defined_by: Selectable,
    selectables: BTreeMap<SelectableName, Selectable>,
}
```

A named entity has a name such as `User`, `Query`, or `String`. Upstream supplies some. The project adds others.

An anonymous entity is created by a selectable declaration with no `to` clause, for example `field Foo.Bar`. It has exactly one incoming selectable: the selectable that declaration defines. It has no name, so no other selectable can point at it.

`defined_by` is that unique incoming selectable. The selectable itself lives on the parent entity.

A selectable declaration's parent is a named entity. That entity may be upstream (`field User.Avatar` where `User` comes from GraphQL) or from an entity declaration.

## EntityDeclaration

Future. An `EntityDeclaration` adds a named entity. The iso keyword is `type`. Upstream named entities are not entity declarations. GraphQL `type User` and `scalar ID` are upstream; they are not this form.

```rust
struct EntityDeclaration {
    name: EntityName,
}
```

```text
type Friend
```

This defines the named entity `Friend`. A selectable declaration can point at it with `to Friend`. Anonymous entities are not produced by an entity declaration; they are produced by a selectable declaration with no `to`.

## Selectable

A selectable is a named pointer from an entity to a wrapper of an entity. Upstream supplies some. The project adds others.

```rust
struct Selectable {
    parent: Entity,
    name: SelectableName,
    target: Wrapper,
    arguments: Vec<ArgumentDefinition>,
}
```

It is identified by `(parent, name)`: `User.name`, `User.friends`, `User.Avatar`, `Query.HomePage`.

Nested selections under a selection of this selectable are selections on the inner entity of `target`.

## SelectableDeclaration

A `SelectableDeclaration` adds a selectable. The iso keyword is `field`. Upstream selectables are not selectable declarations. GraphQL fields are upstream; they are not this form.

Every selectable declaration produces exactly one selectable. Not every selectable comes from a declaration.

```rust
struct SelectableDeclaration {
    parent: NamedEntity,
    name: SelectableName,
    to: Option<Wrapper>,
    arguments: Vec<ArgumentDefinition>,
    selection_set: Option<SelectionSet>,
}
```

If `to` is `None`, the declaration creates an anonymous entity and a selectable whose `target` is that entity (identity wrapper). If `to` is `Some(w)`, the declaration creates a selectable whose `target` is `w`. No new entity.

The selection set, when present, selects selectables of `parent`. It is what the declaration reads. It does not declare selectables of the target.

An iso selectable declaration with no `to`:

```text
field User.Avatar {
  name
  avatarUrl
}
```

This creates an anonymous entity and the selectable `User.Avatar` pointing at it. The selection set `{ name, avatarUrl }` selects `User.name` and `User.avatarUrl`. Those are inputs the declaration reads from `User`. The anonymous entity is the result of `Avatar`.

An iso selectable declaration with `to`:

```text
field User.bestFriend to User {
  friends {
    id
  }
}
```

This creates the selectable `User.bestFriend` whose target is the named entity `User`. The selection set selects selectables of the parent `User`. It is how the declaration computes which `User` to point at.

## Arguments

A selectable has argument definitions. A selection passes arguments to a selectable. A directive also takes arguments.

```rust
struct ArgumentDefinition {
    name: ArgumentName,
    type_: Wrapper,
    default_value: Option<ArgumentValue>,
}

struct Argument {
    name: ArgumentName,
    value: ArgumentValue,
}

enum ArgumentValue {
    Variable(VariableName),
    String(String),
    Integer(i64),
    Boolean(BooleanValue),
    Null,
    Object(Vec<ObjectEntry>),
    List(Vec<ArgumentValue>),
}

struct ObjectEntry {
    name: ValueKeyName,
    value: ArgumentValue,
}

enum BooleanValue {
    True,
    False,
}

struct Directive {
    name: DirectiveName,
    arguments: Vec<Argument>,
}
```

An argument definition is a name, a type, and an optional default. An upstream GraphQL field writes it as `name: Type`: `user(id: ID!)` defines `id` of type `ID`. A selectable declaration writes it as `$name: Type`: `field Query.HomePage($id: ID!)` defines `id` of type `ID`. The `$` is syntax. `$id: ID! = "x"` has a default.

An argument is a `name: value` pair. The names are argument definitions of the selectable or directive being applied. A value is a variable or a literal. A variable names an argument of the enclosing selectable declaration.

`user(id: $id)` passes `id` set to the variable `$id`. `user(id: 4)` passes a different value to the same definition.

A directive takes the same `Argument` type. `@loadable(lazyLoadArtifact: true)` passes `lazyLoadArtifact` set to `true`. `@component` has no arguments.

## Selection

A selection is a use of a selectable in a selection set. It may pass arguments to that selectable.

```rust
struct Selection {
    selectable: Selectable,
    arguments: Vec<Argument>,
    selection_set: Option<SelectionSet>,
}
```

`name` in `{ name }` is a selection of `User.name` with no arguments. `friends { name }` is a selection of `User.friends` with a nested selection set on `User`. `user(id: $id)` is a selection of `Query.user` with one argument, `id` set to the variable `$id`. `user(id: 4)` is a selection of the same selectable with a different argument value.

## SelectionSet

A selection set is a list of selections.

```rust
struct SelectionSet {
    selections: Vec<Selection>,
}
```

A selectable declaration may have a selection set on its parent entity. A selection may have a nested selection set on the inner entity of its selectable's target.

## Entrypoint

An entrypoint is an entity plus a selectable on that entity, marked fetchable.

```rust
struct Entrypoint {
    entity: Entity,
    selectable: Selectable,
}
```

`selectable` is a selectable of `entity`.

```text
entrypoint Query.HomePage
entrypoint User.Avatar
```

The entity does not have to be `Query`. `Query` is an ordinary named entity. Any entity that has the selectable can host an entrypoint.

An entrypoint does not create a selectable. It marks one that already exists.

## Example

Upstream GraphQL schema:

```graphql
type Query {
  user(id: ID!): User
}

type User {
  id: ID!
  name: String!
  friends: [User!]!
}

scalar ID
scalar String
```

In-project iso:

```text
field User.Avatar {
  name
}

field Query.HomePage($id: ID!) {
  user(id: $id) {
    Avatar
    friends {
      name
    }
  }
}

entrypoint Query.HomePage
```

Named entities from upstream: `Query`, `User`, `ID`, `String`.

Anonymous entities from in-project selectable declarations: the result of `User.Avatar`, the result of `Query.HomePage`. Each has one incoming selectable.

Selectables from upstream:

```text
Query.user       ->  User | null
User.id          ->  ID
User.name        ->  String
User.friends     ->  [User]
```

Selectables from in-project declarations:

```text
User.Avatar      ->  Avatar anonymous entity
Query.HomePage   ->  HomePage anonymous entity
```

`Query.user` has argument definition `id: ID`. `Query.HomePage` has argument definition `$id: ID!`.

Selections inside `User.Avatar`: `name`.

Selections inside `Query.HomePage`: `user(id: $id) { Avatar, friends { name } }`. `user(id: $id)` is a selection of `Query.user` with argument `id` set to `$id`, which names HomePage's argument `id`. `Avatar` is a selection of `User.Avatar` with no arguments and no nested set. `friends { name }` is a selection of `User.friends` with a nested set on `User`.

Entrypoint: entity `Query`, selectable `HomePage`.
