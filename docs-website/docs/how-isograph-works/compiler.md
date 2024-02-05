# The Isograph compiler

:::warning
This page is intended to serve as a guide to learning about the Isograph compiler. However, it is likely to be out-of-date and inaccurate, so please consult the [source code](https://github.com/isographlabs/isograph/tree/main/crates) and use your best judgment.
:::

## Big picture

After installing the compiler with `yarn install --dev @isograph/compiler`, `yarn run iso` will run the compiler. It has two modes: batch mode and watch mode.

### Batch mode

Calling `yarn iso --config ./isograph.config.json` runs the Isograph compiler in batch mode. This means it will do a complete run-through (i.e. completely compile the project). Once it completes (or encounters errors), the process will end.

At a very high level, the Isograph compiler does the following:

- It will parse [the Isograph config file](../../isograph-config).
- It will parse and validate the GraphQL schema.
- It will parse and validate `iso` invocations.
- It will generate artifacts.

If during any of these steps, one or more validation errors are generated, the compiler will print those errors and not continue compiling.

You can find this in the [`handle_compile_command`](https://github.com/isographlabs/isograph/blob/df07f01b5978fc4be8bbeedf779012a2462e8b24/crates/isograph_cli/src/batch_compile.rs#L87-L196).

### `watch` mode

If you run `yarn iso --config ./isograph.config/json --watch`, the compiler will run in watch mode.

In this mode, the compiler creates a watcher for the various files/folders (e.g. schema, schema extensions, folder containing the components), and repeatedly runs the compiler in batch mode.

:::note
No state is preserved across runs, e.g. if you modify a component, we still re-parse and re-validate the schema. Re-using state from previous batch compilation runs remains to be implemented.
:::

Since watch mode is a simple wrapper around batch mode, the rest of this document will only discuss batch mode.

## Crates

The Isograph compiler contains the following crates. The most important ones are marked with a 🟢:

- `common_lang_types`
- `graphql_lang_types`: GraphQL types that are also used by Isograph. (This is a smell. These types should only be used by `graphql_schema_parser`.)
- 🟢 `graphql_schema_parser`: An LL(1) parser for GraphQL schema documents and GraphQL schema extension documents, **not** for fragments or operations.
- 🟢 `isograph_cli`: The package which exposes the CLI for the Isograph compiler. It also includes the artifact generation code.
- 🟢 `isograph_lang_parser`: An LL(1) parser for Isograph literals
- `isograph_lang_types`: Some common types.
- 🟢 `isograph_schema`: The in-memory representation of the Isograph schema. This includes server fields and fields generated from `iso` invocations. It should probably not include representations of `iso` entrypoints, but currently does.
- `string_key_newtype`: A library for generating typesafe newtype wrappers around `StringKey` types.
- `u32_newtypes`: A library for generating typesafe newtype wrappers around `u32` types.

## The Isograph schema

### Representation

The Isograph schema (`struct Schema`) is represented by an object that contains:

- a vector of available server fields
- a vector of available resolvers
- a vector of available objects
- a vector of available scalar types
- a map going from names to object or scalar types. This ensures that every type name is unique.

Each object contains:

- a vector of available server field ids
- a vector of available resolver field ids
- an optional id field
- a map of field names to a generic type (in the fully validated schema, this is a map from field names to an enum containing a scalar ID or an object ID.)

### Use of generics

The `Schema` struct is generic over a type implementing `SchemaValidationState`, which is a trait that contains some associated types. When a schema is first constructed, those types are unvalidated (e.g. some generic types are basically unvalidated strings.) As we progressively validate the schema, the unvalidated string types are changed to the appropriate type of ID.

For example, `SchemaServerField`s have a field whose type is the `FieldTypeAssociatedData` associated type. For an unvalidated schema, that type is `UnvalidatedTypeName` (a wrapper around an arbitrary string.) After we validate that that `UnvalidatedTypeName` refers to a type which exists, we construct a new schema whose `FieldTypeAssociatedData` is an enum containing a scalar ID or an object ID.

The reason we cannot start by constructing this union is that when we initially construct an object, it may have fields whose types have not yet been defined. Consider this example:

```graphql
type Foo {
  bar: Bar
}

type Bar {
  foo: Foo
}
```

In this case, we cannot validate that either object is valid, until we have created both `Foo` and `Bar`, meaning we have to create both objects in an unvalidated state.

Likewise, `iso` literals can reference each other, so we must also do a two-pass

### Validation pipeline

Ideally and in the long term, we want the following validation pipeline. During some steps, the generic type associated with the schema will change:

:::warning
The types in this list are simplified and a bit idealized. For example, there is no enum named `FieldValidation`, and the unvalidated resolver field generic type is actually `()`, because the field name is always stored on the field.
:::

#### GraphQL schema validation

- Parse the GraphQL schema
- Create an unvalidated schema from the GraphQL schema (field generic type: `enum FieldValidationState { Unvalidated(String), Validated(ObjectIdOrScalarId) }`)
- Validate that all fields point to existing types (field generic type is the same, but now all fields have `FieldValidation::Validated`)
- Extend the schema with all schema extensions, giving fields the validation state `FieldValidationState::Unvalidated(String)`
- Validate that all fields point to existing types (field generic type: `ObjectIdOrScalarId`)

#### Magical fields

- Process `@exposeOn` directives, which create additional fields, but not new types.

#### Iso literals

- Parse all iso literals (iso literal)
- Extend the schema with the iso literals (resolver scalar field generic type: `UnvalidatedFieldName`; resolver linked field generic type: `UnvalidatedFieldName`)
- Validate that all selections exist (resolver scalar field generic type: `ServerScalarFieldIdOrResolverId`, resolver linked field generic type: `ObjectId`)

:::note
Once we support resolvers that return IDs and which can be selected through (e.g. `best_friend { name }`, where `best_friend` is a resolver), the validated resolver linked field generic type will be an enum containing an id of either a `ResolverWhichReturnsId` or an object.
:::

## Parsing [the Isograph config file](../../isograph-config)

There are two representations of the config:

- the [deserializable representation](https://github.com/isographlabs/isograph/blob/main/crates/isograph_cli/src/config.rs), in the `isograph_cli` crate. This implements `Deserialize`, i.e. we know how to take a JSON object and create this struct.
- the [isograph schema config](https://github.com/isographlabs/isograph/blob/main/crates/isograph_schema/src/compilation_options.rs), which is actually used by the Isograph and GraphQL schema parsers.

These are basically identical. The split may be overengineering for now.

## Artifact generation

- An Isograph artifact refers to a [generated file](../generated-artifacts) that the Isograph runtime uses. For example, reader artifacts are used to read the fields that a resolver needs from the Isograph store.
- Artifact generation is a bit haphazard and can use some improvement.
- Given a validated schema, we generate a bunch of data structures representing _what_ we want to write to each file.
- Then, we clear the target directory, and for each artifact, write the contents to the file.
- Writing generation is last, after all validation, so that we don't leave the target directory in an invalid state.
