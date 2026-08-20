# Mental model

The schema is a graph. An entity is a node. A selectable is a named pointer from an entity to a wrapper of an entity.

There is one kind of entity and one kind of selectable. `User`, `String`, `Query`, and the result of `field User.Avatar` are all entities. A GraphQL field and an iso `field` declaration are both selectable declarations. Each produces a selectable.

The building blocks are `Wrapper`, `Entity`, `Selectable`, `SelectableDeclaration`, `Selection`, `SelectionSet`, and `Entrypoint`.

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

A named entity has a name such as `User`, `Query`, or `String`.

An anonymous entity is created by a selectable declaration with no `to` clause, for example `field Foo.Bar`. It has exactly one incoming selectable: the selectable that declaration declares. It has no name, so no other selectable declaration can point at it.

`defined_by` is that unique incoming selectable. The selectable itself lives on the parent entity.

A selectable declaration's parent is a named entity (`field User.Avatar`, `type User { name: String }`).

## Selectable

A selectable is a named pointer from an entity to a wrapper of an entity.

```rust
struct Selectable {
    parent: Entity,
    name: SelectableName,
    target: Wrapper,
}
```

It is identified by `(parent, name)`: `User.name`, `User.friends`, `User.Avatar`, `Query.HomePage`.

Nested selections under a selection of this selectable are selections on the inner entity of `target`.

## SelectableDeclaration

A `SelectableDeclaration` produces a `Selectable`. Every selectable comes from exactly one declaration. Every declaration produces exactly one selectable. The iso keyword is `field`.

```rust
struct SelectableDeclaration {
    parent: NamedEntity,
    name: SelectableName,
    to: Option<Wrapper>,
    selection_set: Option<SelectionSet>,
}
```

If `to` is `None`, the declaration creates an anonymous entity and a selectable whose `target` is that entity (identity wrapper). If `to` is `Some(w)`, the declaration creates a selectable whose `target` is `w`. No new entity.

The selection set, when present, selects selectables of `parent`. It is what the declaration reads. It does not declare selectables of the target.

A GraphQL selectable declaration:

```graphql
type User {
  name: String!
  friends: [User!]!
}
```

`User.name` has `to: String`. `User.friends` has `to: [User]`. There is no selection set on these declarations.

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

## Selection

A selection is a use of a selectable in a selection set.

```rust
struct Selection {
    selectable: Selectable,
    selection_set: Option<SelectionSet>,
}
```

`name` in `{ name }` is a selection of `User.name`. `friends { name }` is a selection of `User.friends` with a nested selection set on `User`.

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

An entrypoint does not create a selectable. It marks one that a selectable declaration already declared.

## Example

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

```text
field User.Avatar {
  name
}

field Query.HomePage {
  user {
    Avatar
    friends {
      name
    }
  }
}

entrypoint Query.HomePage
```

Named entities: `Query`, `User`, `ID`, `String`.

Anonymous entities: the result of `User.Avatar`, the result of `Query.HomePage`. Each has one incoming selectable.

Selectables:

```text
Query.user       ->  User | null
User.id          ->  ID
User.name        ->  String
User.friends     ->  [User]
User.Avatar      ->  Avatar anonymous entity
Query.HomePage   ->  HomePage anonymous entity
```

Selectable declarations: the four GraphQL fields, plus the two iso fields.

Selections inside `User.Avatar`: `name`.

Selections inside `Query.HomePage`: `user { Avatar, friends { name } }`. `Avatar` is a selection of `User.Avatar` with no nested set. `friends { name }` is a selection of `User.friends` with a nested set on `User`.

Entrypoint: entity `Query`, selectable `HomePage`.
