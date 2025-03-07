# Isograph data model (IDM)

## Core building blocks

There core building blocks that make up the **Isograph schema** are:

- **Entities**. For now, this means objects and scalars defined in the GraphQL schema, but it may eventually include client-defined scalars and objects.
  - Note that client fields implicitly define a client scalar, but that's not a concept that's present in the code. As such, maybe the concept of client scalars is unnecessary.
- **Selectables**. These were previously called scalar fields, linked fields, client fields and client pointers. But they essentially are arrows that point to an entity.
- **Selections**. Client scalar selectables have selections, which form a selection set.

There are also selection sets and associated functions, each discussed later.

## Scalar vs object

All of these objects are divided into two categories: scalar and object. e.g. a scalar selection contains a pointer to a scalar selectable, which contains a pointer to a scalar entity. This should allow us to avoid having code like:

```rust
fn validate_scalar_selection(scalar_selection: ScalarSelection) {
  match scalar_selection.target_selectable.kind {
    SelectableKind::Object => panic!("Invalid state")
    _ => {}
  }
}
```

## Client vs server

All core building blocks can be divided into two categories: client and server.

This is a bit of a misnomer. It should be more like "defined locally" vs "defined elsewhere and taken as a given." Or, for selection sets, the categorization might be "defined elsewhere ONLY" and "maybe defined locally."

The key thing to note is that client selectables have selection sets. The selections in this set can include other client selectables, but no cycle can be formed, i.e. they must eventually bottom out at a bunch of server selections. There are no server selections.

## Creation of a valid Isograph schema

> Status: this section describes how we would like to build the Isograph schema, but not how we currently build it. (3/6/25)

In order to create a valid Isograph schema, we proceed as follows:

### Server

We start by processing all server building blocks.

- First, we create all of the entities. We immediately know whether an entity is a scalar or object. We do not process any selectables that are on the entities.
- Then, we create all of the server selectables. Each server selectable points to an entity (which either has already been defined, or we emit an error), so we can categorize the selectable as a scalar selectable or an object selectable.
  - After we process a selectable, we mutate its entity to register the selectable.

At this point, we have a valid schema!

> This is as far as we can get when we parse a GraphQL schema.

We may choose to do this multiple times. Consider, for example, a schema (InternalSchema) which extends another (ExternalSchema). We should first validate ExternalSchema, and ensure it is in a valid state, and then add InternalSchema, and ensure that the combination is in a valid state. In other words, it shouldn't be the case that ExternalSchema is invalid when examined on its own (for example, because it contains fields that point to undefined types), but made valid by the inclusion of the InternalSchema (e.g. it defines those missing types.)

### Client

Finally, we get around to validating the client code.

- Currently, there is no such thing as a client entity, but if there were, we would process them first.
- Then, we process client selectables (client fields and pointers), but _ignore_ their selection sets.
- Then, we process client selections. Pointers from selections to selectables are one-way (selection -> selectable), so we don't need to modify the selectable.
  - But maybe, for the LSP, if we want to implement "find all references", this should be a two-way pointer! In which case, we would mutate the selectable.

### Client -> Server

Now, we have a valid schema that contains both client and server fields. But we can collapse it, and turn it into a schema that contains only server fields!

This is what we're going to have to do if we have Isograph running in the browser, talking to an Isograph process running on the server (e.g. doing server-side calculation of some client fields), which is talking to a backend.

> Will any instance of Isograph ever need to know about multiple levels of server selectables? Probably not. But it might need to know _which_ server schema something came from, i.e. Isograph can probably made to generate multiple queries, but have it seem like it's all coming from the same backend.

> An Isograph instance talking to another Isograph instance (on the server) probably requires some sort of namespacing, so that the server-calculation version of a given field does not clash with the client-calculated version.

> Perhaps namespaces fall out of schemas extending each other. I haven't thought this through.

## Selection sets

Selectables contain refetch strategies, though perhaps these will be optional in some cases. Refetch strategies contain refetch selection sets (though perhaps this isn't ideal modeling.) Most refetch selection sets will simply be the `id` field, but theoretically could be something else.

Client selectables also contain client reader selection sets.

### Regular vs. merged selection sets

> Status: there are some differences between what is described here (which is aspirational) and what is present in the codebase (3/6/25).

A (reader or refetch) selection set can be one of two types:

- those that may contain client selections (aka selection sets) and
- those that contain server selections only (aka merged selection sets).

Taking a selection and recursively replacing any client selection with its selection set is called merging a selection set.

When we form a merged selection set and encounter a client selectable, we continue merging:

- its refetch selection set, if the field is selected loadably, and
- its reader selection set otherwise.

## Entrypoints

> Entrypoints are currently represented by client fields. But they really should be selections, or selection sets. Client fields contain reader selection sets, so it's convenient enough for now, but ideally, we should represent entrypoints as selections.

For each entrypoint declarations we encounter, we create a merged selection set, and from that generate query text, normalization AST, etc.

For each loadable field we encounter when generating a merged selection set, we _also_ generate another entrypoint, i.e. create a merged selection set from that client selectable's refetch strategy and its reader selection set.

So:

- if a field is selected regularly, its reader selection set is merged into the parent query, and
- if a field is selected loadably, its refetch selection set is merged inot the parent query, **and** the refetch strategy and the reader selection set are used to generate another query.

---

Everything henceforce is speculative

---

## Speculative: Associated functions

> Currently, only resolver functions exist, and the rest is a bit speculative

Entities have associated functions. These are:

- constructors (called when an entity is added to the store)
- destructors (called when an entity is GCed and removed from the store)
- (possibly) functions that are called when an entity is modified

Client selectables have associated functions:

- Resolver functions (akin to render functions), i.e. called when we need to read the value

Client scalar selectables implicitly define a unique entity, so they may also have constructors and destructors.

### Constructors and destructors

Constructors and destructors can be run when a given entity is normalized into the store. Constructors might:

- immediately start network requests for loadable fields, instead of waiting until the component is rendered (the current solution), or
- initialize some values on an object.

It is probably a bad idea to have "externally visible" side effects, since its important to be able to do performance optimizations (like fetching optimistically).

## Speculative: Stateful entities

Currently, there are two types of state:

- React component state
- The Isograph store (aka network response cache)

Currently, for example, when a loadable field is fetched, that data is stored in React component state.

If, on the other hand, we kick off network requests for loadably selected fields when something is normalized, we need a place to keep that data. It will live in the store.

So, how does one select `UserComponent @loadable(autoFetch: true)` _and_ have the read-out value be a React component, but also have a few slots to store data?

We might imagine this is transformed into `UserComponent: loadable(selectionSet: UserComponent, autoFetch: true)`, i.e. the `loadable` selectable (which exists on all entities), and which (because it is a client scalar selectable) implicitly defines an entity, where we store the network request(s).

So, because `loadable` is a client scalar selectable, it has an associated resolver function, and can return whatever we want. So, problem solved?

## Speculative: selection sets as parameters

In order to make the `loadable` selectable work, we seem to have to support passing selection sets (or selections) as parameters to client selectables.

Perhaps this relates to the Children prop [proposed here](https://github.com/isographlabs/isograph/issues/287).

## Speculative: transforms

It would be nice to not hard-code transforms like `UserComponent @loadable` -> `loadable(selectionSet: UserComponent)`, but instead to expose this functionality to users.

Do "selection sets as parameters" require transforms?

## Speculative: Client entities

One type of client entity that we want to support is a subtype, e.g.:

- a client object entity `Friend` is a subtype of the server object entity `User`.
- we might want to enforce, statically, that if we have an updatable client selectable (named `best_friend`) with type `Friend`, you cannot assign any old `User`. Instead, for something to be assignable to `best_friend`, it must have been originally selected as a `Friend`.
- we might additionally want to enforce that all subtypes:
  - were fetched with certain fields, e.g. `secret_handshake`, or
  - we might define a selectable `asFriend`, which determines whether a given user is actually a friend.

Subtypes in this sense also make sense for server types! GraphQL doesn't have such a notion, though.
