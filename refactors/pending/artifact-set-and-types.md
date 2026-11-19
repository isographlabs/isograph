# The full artifact set: stub runtime ASTs, generated types, and scalar type mapping

This doc extends `refactors/pending/artifact-generation.md` (v0: `output_directory`, `iso.ts`, `{ kind: "Entrypoint", name }` entrypoint artifacts, `ArtifactValue` printed through SWC, all-or-nothing write with a diff) to every file upstream isograph generates. Files the runtime reads (reader ASTs, normalization ASTs, refetch readers) are generated at upstream's paths as modules whose default export is an empty object. Files TypeScript reads (param types, parameters types, output types, link types, the typed `iso.ts` overloads) are generated in full, by upstream's algorithm. The knowledge that GraphQL `String` is TypeScript `string` is split: the network protocol reports how a scalar's values are represented as data, and the host language maps that data to its own types.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`): `crates/artifact_content/src/` and the generated tree `demos/pet-demo/src/components/__isograph/`.

## End-user experience

```json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "output_directory": "src",
  "custom_scalars": { "Url": "String" }
}
```

The user writes resolvers without annotating their parameter. The parameter's type comes from the generated param type through the `iso` overload:

```ts
// from demos/pet-demo/src/Pet.ts
import { iso } from './__isograph/iso';

export const fullName = iso(`
  field Pet.fullName "The pet's first and last name, for example Makayla Balicki." {
    firstName
    lastName
  }
`)(function fullName({ data }) {
  return `${data.firstName} ${data.lastName}`;
});
```

`data.firstName` is `string`. Selecting a field that does not exist, or reading `data.nickname` without selecting it, is a TypeScript error at the resolver. A resolver that reads `data.nickname` where the schema says `nickname: String` gets `string | null`. A field selected under a list is a `ReadonlyArray`. A client field selected by another field has the type its resolver returns. A field with variables gets `parameters` typed from its variable definitions. `picture: Url!` is `string` because the config maps the custom scalar `Url` to `String`; without that entry it is `unknown`.

After the daemon compiles the pet demo, `src/__isograph/` holds:

```text
src/__isograph/iso.ts
src/__isograph/tsconfig.json
src/__isograph/Mutation/SetTagline/entrypoint.ts
src/__isograph/Mutation/SetTagline/normalization_ast.ts
src/__isograph/Mutation/SetTagline/output_type.ts
src/__isograph/Mutation/SetTagline/param_type.ts
src/__isograph/Mutation/SetTagline/parameters_type.ts
src/__isograph/Mutation/SetTagline/resolver_reader.ts
src/__isograph/Owner/__link/output_type.ts
src/__isograph/Owner/OwnerBadge/output_type.ts
src/__isograph/Owner/OwnerBadge/param_type.ts
src/__isograph/Owner/OwnerBadge/resolver_reader.ts
src/__isograph/Owner/displayName/output_type.ts
src/__isograph/Owner/displayName/param_type.ts
src/__isograph/Owner/displayName/resolver_reader.ts
src/__isograph/Pet/__link/output_type.ts
src/__isograph/Pet/PetBestFriendCard/{output_type,param_type,resolver_reader}.ts
src/__isograph/Pet/PetCheckinsCard/entrypoint.ts
src/__isograph/Pet/PetCheckinsCard/normalization_ast.ts
src/__isograph/Pet/PetCheckinsCard/output_type.ts
src/__isograph/Pet/PetCheckinsCard/param_type.ts
src/__isograph/Pet/PetCheckinsCard/parameters_type.ts
src/__isograph/Pet/PetCheckinsCard/refetch_reader.ts
src/__isograph/Pet/PetCheckinsCard/resolver_reader.ts
src/__isograph/Pet/PetStatsCard/{output_type,param_type,resolver_reader}.ts
src/__isograph/Pet/PetSummaryCard/{output_type,param_type,resolver_reader}.ts
src/__isograph/Pet/bestFriend/{output_type,param_type,resolver_reader}.ts
src/__isograph/Pet/fullName/{output_type,param_type,resolver_reader}.ts
src/__isograph/Pet/owner/{output_type,param_type,resolver_reader}.ts
src/__isograph/Query/HomeRoute/{entrypoint,normalization_ast,output_type,param_type,resolver_reader}.ts
src/__isograph/Query/PetDetailRoute/{entrypoint,normalization_ast,output_type,param_type,parameters_type,resolver_reader}.ts
src/__isograph/Query/SpeciesRoute/{entrypoint,normalization_ast,output_type,param_type,parameters_type,resolver_reader}.ts
```

`Pet/PetCheckinsCard` has an entrypoint and a refetch reader because `Query.PetDetailRoute` selects it with `@loadable`. `Pet/__link` and `Owner/__link` exist because `Pet.bestFriend` points to `Pet` and `Pet.owner` points to `Owner`.

When query generation lands (the GraphQL protocol plan), each entrypoint folder also gets `query_text.ts` and `raw_response_type.ts`, and `__refetch__<n>.ts` and `__refetch__query_text__<n>.ts` per refetch path. Those files are not in this doc.

## The artifact set

### Upstream inventory

Every file upstream's `get_artifact_path_and_content_impl` (`crates/artifact_content/src/generate_artifacts.rs`) writes, the rule for when it exists, and what i2 does with it:

- `<P>/<f>/param_type.ts`: every user-written client field and client pointer. i2 generates it in full.
- `<P>/<f>/parameters_type.ts`: every user-written client field or pointer with at least one variable definition. i2 generates it in full.
- `<P>/<f>/output_type.ts`: every entrypoint's field, and every client field or pointer that some field's selection set selects (`encountered_output_types`). i2 generates it in full.
- `<P>/__link/output_type.ts`: every entity whose `__link` is selected, and every entity a client pointer points to (imported by `iso.ts`). i2 generates it for every pointer target; i2 has no `__link` selectable yet (Later).
- `<P>/<f>/resolver_reader.ts`: every client field or pointer reachable from an entrypoint (`encountered_client_type_map`). i2 generates a stub.
- `<P>/<f>/refetch_reader.ts`: every client field reachable from an entrypoint and ever selected with `@loadable` (`was_ever_selected_loadably`). i2 generates a stub.
- `<P>/<f>/entrypoint.ts`, `normalization_ast.ts`: every `entrypoint` literal's field, and every loadably selected field. i2 generates the entrypoint as a value and the normalization AST as a stub.
- `<P>/<f>/query_text.ts`, `raw_response_type.ts`, `__refetch__<n>.ts`, `__refetch__query_text__<n>.ts`: the same fields. They need the merged selection set and refetch paths of query generation; the GraphQL protocol plan adds them.
- `iso.ts`: always. i2 generates it with typed overloads.
- `tsconfig.json`: always. i2 generates upstream's text.
- `persisted_documents.json`: when the config sets `persisted_documents`. i2 has no such option.
- `<P>/__refetch/{output_type,refetch_reader}.ts` and exposed-mutation fields (`Pet/set_pet_tagline/`): upstream's imperatively loaded fields, defined from `@exposeField` and the `node` refetch strategy. i2 has neither; the GraphQL protocol plan owns them.
- `<Union>/as<Member>/resolver_reader.ts`: upstream's refinement pointers (`asBlogItem`), reader condition artifacts. i2's grammar has no refinement and the protocol defines no `as<Member>` selectables yet. When the protocol defines them, each is a selectable whose target is `Union([Entity(Member), Null])`, and the type walk below gives it `({ ... } | null)` with no case of its own; its stub reader is Later with the selectable.

### Per field

For every `field` literal `field P.f`:

- `P/f/param_type.ts` (generated): `export type P__f__param = { readonly data: <data>; readonly parameters: <parameters>; }`, where `<data>` is the object type of the field's selection set and `<parameters>` is `P__f__parameters` imported from `./parameters_type` or `Record<PropertyKey, never>`. Imports: one `import type { X__y__output_type } from ".../X/y/output_type"` per client field the data type names, then the parameters import.
- `P/f/parameters_type.ts` (generated, when the field has variables): `export type P__f__parameters = { readonly v: T; readonly w?: T | null | void; }`, plus one local `type <InputObject> = { ... }` per input object its parameters name at any depth.
- `P/f/output_type.ts` (generated, when `f` is in the output-type set): `import type { <export> as resolver } from "<source>"; export type P__f__output_type = ReturnType<typeof resolver>;`.
- `P/f/resolver_reader.ts` (stub, when `f` is reachable from an entrypoint): `export default {};`.
- `P/f/refetch_reader.ts` (stub, when `f` is reachable and loadably selected): `export default {};`.
- `P/f/entrypoint.ts` and `P/f/normalization_ast.ts`, when `f` is loadably selected: as for an entrypoint below.

### Per entrypoint

For every `entrypoint P.f` literal:

- `P/f/entrypoint.ts` (generated value): v0's object with the stubs imported:

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
import Query__HomeRoute__normalization_ast from "./normalization_ast";
import Query__HomeRoute__resolver_reader from "./resolver_reader";
export default {
    kind: "Entrypoint",
    name: "Query.HomeRoute",
    normalizationAst: Query__HomeRoute__normalization_ast,
    readerArtifact: Query__HomeRoute__resolver_reader
};
```

- `P/f/normalization_ast.ts` (stub): `export default {};`.

The keys `normalizationAst` and `readerArtifact` are the names of `Entrypoint` in `refactors/pending/runtime/pico-runtime.md`. Upstream's `readerArtifact` is the default export of `resolver_reader.ts`, which is a thunk once readers are real, so importing that module's default export is already the shape the runtime reads.

### Per pointer target

For every entity `E` that is the inner entity of some pointer's `to` type:

- `E/__link/output_type.ts` (generated): `export type E____link__output_type = { readonly __link: string; readonly __typename: "E"; };` for a concrete entity. For an abstract entity (interface or union), the union of one such object per concrete type, and `never` when it has none. Upstream's text is `Link<"E">` from `@isograph/react`; see open decision 2.

### Top level

- `iso.ts` (generated): v0's file with typed overloads, below.
- `tsconfig.json` (generated, raw): upstream's `generate_ts_config` text, byte for byte:

```text
{
    "compilerOptions": {
        "noUnusedLocals": false,
        "noUnusedParameters": false,
        "jsx": "preserve",
        "esModuleInterop": true
    }
}
```

No trailing newline, as upstream writes it.

### Example: `Query.PetDetailRoute`

Shown as TypeScript; the exact spacing is `swc_ecma_codegen`'s.

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
// Query/PetDetailRoute/param_type.ts
import type { Owner__OwnerBadge__output_type } from "./../../Owner/OwnerBadge/output_type";
import type { Pet__PetBestFriendCard__output_type } from "./../../Pet/PetBestFriendCard/output_type";
import type { Pet__PetCheckinsCard__output_type } from "./../../Pet/PetCheckinsCard/output_type";
import type { Pet__PetStatsCard__output_type } from "./../../Pet/PetStatsCard/output_type";
import type { Pet__fullName__output_type } from "./../../Pet/fullName/output_type";
import type { Query__PetDetailRoute__parameters } from "./parameters_type";
export type Query__PetDetailRoute__param = {
    readonly data: {
        /**
Fetch a pet by id
*/ readonly pet: ({
            /**
The pet's ID
*/ readonly id: string;
            /**
The pet's first and last name, for example Makayla Balicki.
*/ readonly fullName: Pet__fullName__output_type;
            readonly PetStatsCard: Pet__PetStatsCard__output_type;
            readonly PetCheckinsCard: Pet__PetCheckinsCard__output_type;
            readonly PetBestFriendCard: Pet__PetBestFriendCard__output_type;
            readonly bestFriend: ({
                readonly fullName: Pet__fullName__output_type;
            } | null);
            readonly owner: {
                readonly OwnerBadge: Owner__OwnerBadge__output_type;
            };
        } | null);
    };
    readonly parameters: Query__PetDetailRoute__parameters;
};
```

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
// Query/PetDetailRoute/parameters_type.ts
export type Query__PetDetailRoute__parameters = {
    readonly id: string;
};
```

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
// Query/PetDetailRoute/output_type.ts
import type { PetDetailRoute as resolver } from "./../../../PetDetailRoute";
export type Query__PetDetailRoute__output_type = ReturnType<typeof resolver>;
```

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
// Pet/PetCheckinsCard/parameters_type.ts
export type Pet__PetCheckinsCard__parameters = {
    readonly limit?: number | null | void;
};
```

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
// Pet/__link/output_type.ts
export type Pet____link__output_type = {
    readonly __link: string;
    readonly __typename: "Pet";
};
```

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
// Pet/fullName/resolver_reader.ts
export default {};
```

Deltas from upstream's text for these files, beyond open decisions 2, 3, 4, and 6:

- Upstream prints `import { fullName as resolver } from '../../../Pet/fullName'`, a value import. i2 prints `import type`, which `typeof` accepts, so a generated type file never imports a source module at runtime.
- Upstream's output-type files start with `import type React from 'react';` whether or not the file uses `React`. i2 prints no unused import.
- Specifiers follow v0's `artifact_import_specifier` rule (`./`-prefixed, `.ts` dropped): `./../../Pet/fullName/output_type` where upstream writes `../../Pet/fullName/output_type`. Both name the same file.
- Imports print in specifier order. Upstream prints output-type imports in `EntityNameAndSelectableName` order.

## Scalars

### The problem

A server field's type in the param type is a TypeScript type for a GraphQL scalar. Something has to know that GraphQL `String` is TypeScript `string`. The GraphQL protocol must not name TypeScript, and the TypeScript host language must not name GraphQL.

### What upstream does

Upstream (`crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs`, `define_default_graphql_data_model_entities` and `insert_parsed_items_into_schema`) stores a TypeScript type name on every scalar entity, as `JavascriptName` in the entity's `target_platform` associated data:

```rust
// external isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs
lazy_static! {
    pub static ref UNKNOWN_JAVASCRIPT_TYPE: JavascriptName = "unknown".intern().into();
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
    pub static ref BOOLEAN_JAVASCRIPT_TYPE: JavascriptName = "boolean".intern().into();
    pub static ref NUMBER_JAVASCRIPT_TYPE: JavascriptName = "number".intern().into();
    pub static ref NEVER_JAVASCRIPT_TYPE: JavascriptName = "never".intern().into();
}
```

- `String` and `ID` are `string`, `Int` and `Float` are `number`, `Boolean` is `boolean`. These five entities are predefined; a schema that also declares one is a duplicate definition.
- Every `scalar` the schema declares is `unknown` (`demos/pet-demo/.../Query/HomeRoute/raw_response_type.ts` prints `picture: unknown` for `Url`).
- Every enum is `string`.
- `__typename` is a literal type: an entity `<Type>__discriminator` whose JavaScript name is `"Pet"`, and for a union the union of its members' names.
- There is no config option and no directive. `TargetPlatform::get_inner_text_for_selectable` reads the name back when printing.

The GraphQL protocol crate writes TypeScript names, and the TypeScript printer reads them as opaque text. This is the coupling i2 removes.

### The options

1. Every scalar is `unknown`. Nothing is coupled. Every resolver casts every string and number. Rejected: it discards what the GraphQL spec defines for its built-in scalars.
2. The five built-in scalars map to `string`, `number`, `boolean`, custom scalars are `unknown`. This is upstream's behavior. `Url`, `DateTime`, `JSON` are `unknown`.
3. Option 2, plus a config map from a custom scalar's entity name to how its values are represented. `"custom_scalars": { "Url": "String" }` makes `Url` `string`. The config is keyed by entity name, which is compiler vocabulary, and valued by a value kind, which is neither GraphQL nor TypeScript. A user whose server sends a JSON object for a scalar has no value kind for it and keeps `unknown`.
4. Option 2, plus a schema directive on the scalar: `scalar Url @javascriptType(name: "string")`. The TypeScript type is written in the GraphQL schema, so the protocol has to carry host-language text through a directive whose meaning it does not know. A schema downloaded from the server needs an `extend scalar Url @javascriptType(...)` extension file the user owns. A second host language needs a second directive.
5. Option 3 with arbitrary host-language type text as the config value (Relay's `customScalarTypes`: `"Url": "string"`, or an imported branded type). The TypeScript generator has to parse type text into SWC AST, which needs `swc_ecma_parser`, and the config value is TypeScript text, so it is host-language config.

### The option held: 3

The protocol reports a scalar entity's representation as data, in a vocabulary that names how values arrive, not what they are called in GraphQL or in TypeScript:

```rust
// from crates/common_lang_types/src/scalar_value_kind.rs
/// How the values of a scalar arrive in the data a network protocol returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize)]
pub enum ScalarValueKind {
    String,
    Number,
    Boolean,
}
```

```rust
// from crates/isograph_compiler/src/schema.rs
/// What the compiler knows about the values of a scalar entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScalarKind {
    /// The protocol defines how this scalar's values arrive.
    Known(ScalarValueKind),
    /// A scalar the protocol knows only by name. The config's `custom_scalars` may say how its
    /// values arrive.
    Custom(EntityName),
}
```

`ScalarValueKind` lives in `common_lang_types` because both `isograph_config` (the config's map values) and `isograph_compiler` name it, and neither depends on the other. `common_lang_types` enables serde's `derive` feature for it.

The GraphQL protocol maps its entity data to an entity kind (Schema model, Entity). `GraphQLEntityData::Leaf` splits into `Scalar` and `Enum` with this change, the first reader of the difference:

```rust
// from crates/graphql_network_protocol/src/lib.rs, in impl NetworkProtocol for GraphQLNetworkProtocol
    fn entity_kind(data: &GraphQLEntityData) -> EntityKind {
        match data {
            GraphQLEntityData::Scalar(name) => EntityKind::Scalar(scalar_kind(name.dereference())),
            GraphQLEntityData::Enum => {
                EntityKind::Scalar(ScalarKind::Known(ScalarValueKind::String))
            }
            GraphQLEntityData::InputObject
            | GraphQLEntityData::Object(_)
            | GraphQLEntityData::Interface(_)
            | GraphQLEntityData::Union => EntityKind::Object,
        }
    }

/// GraphQL's built-in scalars (spec 3.5) by name; every other scalar is custom.
fn scalar_kind(name: EntityName) -> ScalarKind {
    match name.lookup() {
        "String" | "ID" => ScalarKind::Known(ScalarValueKind::String),
        "Int" | "Float" => ScalarKind::Known(ScalarValueKind::Number),
        "Boolean" => ScalarKind::Known(ScalarValueKind::Boolean),
        _ => ScalarKind::Custom(name),
    }
}
```

The TypeScript host language maps the kind to a TypeScript keyword type and reads the config for `Custom`:

```rust
// from crates/isograph_extract_typescript/src/generate_types.rs
/// The TypeScript type of a scalar's values.
fn scalar_type(kind: ScalarKind, options: &GenerationOptions) -> TsType {
    match kind {
        ScalarKind::Known(value_kind) => value_kind_type(value_kind),
        ScalarKind::Custom(name) => options
            .custom_scalars
            .get(name.reference())
            .map(|value_kind| value_kind_type(value_kind.dereference()))
            .unwrap_or_else(|| keyword_type(TsKeywordTypeKind::TsUnknownKeyword)),
    }
}

fn value_kind_type(value_kind: ScalarValueKind) -> TsType {
    keyword_type(match value_kind {
        ScalarValueKind::String => TsKeywordTypeKind::TsStringKeyword,
        ScalarValueKind::Number => TsKeywordTypeKind::TsNumberKeyword,
        ScalarValueKind::Boolean => TsKeywordTypeKind::TsBooleanKeyword,
    })
}
```

Reasons: the protocol knows how its built-in scalars are serialized, and nothing else in the pipeline can know it; it reports that as `ScalarValueKind`, which TypeScript, Python, or Rust output can each map. The user knows how a custom scalar is serialized, and the project config is where per-project facts go; the config names no TypeScript, so one config serves any host language. A custom scalar not in the map keeps upstream's `unknown`, so a project without the key gets upstream's types. Option 5 is Later: it needs a type-text parser and is host-language config.

i2's schemas declare the built-in scalars (`demos/pet-demo/schema/scalars.graphql`). `scalar_kind` keys on the name, so a declared `scalar String` is the built-in `String`. Whether the GraphQL protocol also defines the five when the schema does not declare them (upstream does; the spec says a schema omits them) is the schema integration's decision; `entity_kind` gives the same kind either way.

### Config

```rust
// from crates/isograph_config/src/compilation_options.rs
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub schema: Vec<String>,
    /// The directory, relative to the config file's directory, whose `__isograph` folder holds
    /// the generated artifacts. Absent: the compiler writes no artifacts, and the transforms
    /// look in the config file's directory.
    #[serde(default)]
    pub output_directory: Option<PathBuf>,
    /// How the values of each custom scalar arrive, by the scalar's name. A custom scalar not
    /// listed has the host language's unknown type.
    #[serde(default)]
    pub custom_scalars: BTreeMap<String, ScalarValueKind>,
}
```

`"custom_scalars": { "Url": "String" }` deserializes through serde's unit-variant names. A value other than `"String"`, `"Number"`, `"Boolean"` is a config error naming the variants, as serde reports it.

## Schema model

### Wrapper

`Selectable.target_entity: Option<EntityName>` drops the list and nullability wrappers (`named_entity_of_type_annotation` in `crates/isograph_compiler/src/schema.rs`). Type generation needs them. The mental model's `Wrapper` becomes a type, generic over what it wraps, so the selectable's target is `Wrapper<EntityName>` and the generated type is `Wrapper<ArtifactInnerType>` with the same structure:

```rust
// from crates/isograph_compiler/src/wrapper.rs
/// A type former over `T` (mental-model.md, Wrapper). `List` is `[W]`; `Union` is `W | ...`,
/// and a nullable `W` is `Union([W, Null])`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Wrapper<T> {
    Inner(T),
    List(Box<Wrapper<T>>),
    Union(UnionWrapper<T>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnionWrapper<T>(pub Vec<WrapperVariant<T>>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WrapperVariant<T> {
    Inner(T),
    List(Box<Wrapper<T>>),
    Null,
}

impl<T> Wrapper<T> {
    /// The same structure with every `Inner` replaced by `f` of it, or the first error `f`
    /// returns.
    pub fn try_map<U, E>(&self, f: &mut impl FnMut(&T) -> Result<U, E>) -> Result<Wrapper<U>, E>;

    /// Every `Inner`, in order.
    pub fn inner_values(&self) -> impl Iterator<Item = &T>;

    /// `Union` with a `Null` variant at the top level.
    pub fn nullability(&self) -> Nullability;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nullability {
    Nullable,
    NonNull,
}
```

`Wrapper`, `UnionWrapper`, `WrapperVariant`, `inner_values`, and `wrapper_of_type_annotation` landed with integration.md change 2; `try_map`, `nullability`, and `Nullability` land with change 2 here, the type walk, their first reader. `try_map` takes `&mut impl FnMut` so it can recurse without boxing the closure. `inner_values` is a boxed iterator over the recursive structure (`Box<dyn Iterator<Item = &T> + '_>` behind the `impl`).

The iso type annotation converts structurally. A list whose inner type did not parse (`[!]`) has no wrapper:

```rust
// from crates/isograph_compiler/src/wrapper.rs
/// The wrapper `annotation` writes. `None` when a list's inner type did not parse.
pub fn wrapper_of_type_annotation(annotation: &TypeAnnotation) -> Option<Wrapper<EntityName>> {
    match annotation {
        TypeAnnotation::Named(named) => Wrapper::Inner(named.name.item.0).wrap_some(),
        TypeAnnotation::List(list) => Wrapper::List(wrapper_of_list(list)?.boxed()).wrap_some(),
        TypeAnnotation::Union(union) => Wrapper::Union(UnionWrapper(
            union
                .0
                .iter()
                .map(|variant| match variant.item.reference() {
                    UnionVariant::Named(named) => WrapperVariant::Inner(named.name.item.0).wrap_some(),
                    UnionVariant::List(list) => {
                        WrapperVariant::List(wrapper_of_list(list)?.boxed()).wrap_some()
                    }
                    UnionVariant::Null(_) => WrapperVariant::Null.wrap_some(),
                })
                .collect::<Option<Vec<_>>>()?,
        ))
        .wrap_some(),
    }
}

fn wrapper_of_list(list: &ListTypeAnnotation) -> Option<Wrapper<EntityName>> {
    wrapper_of_type_annotation(list.inner.as_ref()?.item.reference())
}
```

`to Pet` parses as `Union([Named(Pet), Null])` (`parse_type_annotation` in `crates/isograph_parser/src/variables.rs`), so it is `Union([Inner(Pet), Null])`. The schema integration converts GraphQL annotations by the mental model's table (`Foo!` is `Inner(Foo)`, `Foo` is `Union([Inner(Foo), Null])`).

### Selectable

Before:

```rust
// from crates/isograph_compiler/src/schema.rs
pub struct Selectable {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    /// `None` for a field declared without `to`.
    pub target_entity: Option<EntityName>,
    pub definition: DefinitionLocation,
}
```

After (landed with integration.md change 2):

```rust
// from crates/isograph_compiler/src/schema.rs
/// A named pointer from an entity to a wrapper of an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectable<TSelectableData> {
    pub name: SelectableName,
    pub parent_entity: EntityName,
    pub description: Option<DescriptionValue>,
    pub definition: DefinitionLocation,
    pub kind: SelectableKind<TSelectableData>,
}

/// Who defines a selectable, and what it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectableKind<TSelectableData> {
    /// `field P.f` with no `to`: its value is what its resolver returns.
    ClientField,
    /// `field P.f to T`: its resolver returns a link to a `T`.
    ClientPointer(ClientPointerTarget),
    /// Defined by the network protocol.
    Server(ServerSelectable<TSelectableData>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientPointerTarget {
    Parsed(Wrapper<EntityName>),
    /// A `to` type with a list whose inner type did not parse (`to [!]`).
    Unparsed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerSelectable<TSelectableData> {
    pub target: Wrapper<EntityName>,
    pub data: TSelectableData,
}
```

`selectable_of_declaration` sets the kind: `target_type` `None` is `ClientField`; `Some(annotation)` is `ClientPointer`, `Parsed` when `wrapper_of_type_annotation` returns a wrapper. `named_entity_of_type_annotation` and `named_entity_of_list` are deleted. `Selectable::target_entity()` is the first `inner_values()` of a `Parsed` or `Server` target and `None` otherwise; `selectable_target`, the hover text that prints `: **Pet**`, and validation read it. A union of two entities in a `to` clause selects on its first; rejecting it is validation.

`ClientField` becomes `ClientField(ClientFieldKind)` with change 2, whose type walk is the first reader of plain versus `@component`:

```rust
// from crates/isograph_compiler/src/schema.rs
/// Whether a client field's declaration has `@component`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientFieldKind {
    Component,
    Plain,
}
```

`ServerSelectable.data` is the protocol's `SelectableData` (query-generation.md); type generation reads only `target`. Server selectables are fields of objects and interfaces and refinements; input object fields become server selectables with change 2's input-object parameter types (open decision 6), their first reader.

### Entity

```rust
// from crates/isograph_compiler/src/schema.rs
/// A node of the schema graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entity<TEntityData> {
    pub name: EntityName,
    pub definition: DefinitionLocation,
    pub origin: EntityOrigin<TEntityData>,
}

/// Who defines an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityOrigin<TEntityData> {
    /// `entity Owner` in a source file.
    Source,
    Protocol(TEntityData),
}
```

This is the shape integration.md change 2 landed. The protocol's `EntityData` (`GraphQLEntityData`: `Leaf`, `InputObject`, `Object`, `Interface`, `Union`) is the one record of a protocol entity's kind. Change 2 adds the compiler's vocabulary as a function of that data, not a second stored field:

```rust
// from crates/isograph_compiler/src/schema.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityKind {
    /// Selections under a selection of this entity select its selectables.
    Object,
    /// Read whole. A selection of a selectable whose target is a scalar has no selection set.
    Scalar(ScalarKind),
}

// from crates/isograph_compiler/src/host_language.rs, on NetworkProtocol
    /// The compiler's kind of an entity the protocol defines.
    fn entity_kind(data: &Self::EntityData) -> EntityKind;

// from crates/isograph_compiler/src/schema.rs
impl<TEntityData> Entity<TEntityData> {
    pub fn kind<TProtocol: NetworkProtocol<EntityData = TEntityData>>(&self) -> EntityKind {
        match self.origin.reference() {
            EntityOrigin::Source => EntityKind::Object,
            EntityOrigin::Protocol(data) => TProtocol::entity_kind(data),
        }
    }
}
```

A source `entity Owner` is `Object`. The GraphQL protocol's `entity_kind` maps `Leaf` to `Scalar(scalar_kind(name))` for a scalar and `Scalar(Known(String))` for an enum (so `Leaf` carries which of the two it is when this lands, or `scalar_kind` reads the name and an enum name is never a built-in scalar's), and `InputObject`, `Object`, `Interface`, and `Union` to `Object`. An entity never stores its concrete types: an interface's implementors depend on every schema file, and storing them would make `entity_named(Node)` fail when any implementing type is broken. `link_types` reads the concrete types of an abstract target through the protocol memo `object_types_implementing` (interfaces) and the union's members, behind a `NetworkProtocol` method this change adds. `ScalarKind`, `EntityKind`, and `entity_kind` land with change 2, their first reader.

## The type layer

Generated types are built as data in no host language, as v0 builds values as `ArtifactValue`. The compiler computes them in memos; the host language prints them.

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// The type of a param type's `data`: one field per selection, in selection order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactObjectType(pub Vec<ArtifactObjectField>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactObjectField {
    /// The selection's alias, or its name when it has none.
    pub key: SelectionName,
    /// The selected selectable's description.
    pub description: Option<DescriptionValue>,
    pub type_: Wrapper<ArtifactInnerType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactInnerType {
    Scalar(ScalarKind),
    Object(ArtifactObjectType),
    /// What a client field's resolver returns: its `output_type` artifact.
    ClientFieldOutput(EntityNameAndSelectableName),
}

/// A field's variable definitions, and every input object they name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParametersType {
    pub parameters: NonEmpty<ArtifactParameter>,
    /// Every input object a parameter's type names, at any depth.
    pub input_objects: BTreeMap<EntityName, Vec<ArtifactInputField>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactParameter {
    pub name: VariableName,
    pub type_: Wrapper<ArtifactInputType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactInputField {
    pub name: SelectableName,
    pub type_: Wrapper<ArtifactInputType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactInputType {
    Scalar(ScalarKind),
    /// Named; `ParametersType.input_objects` has its fields.
    InputObject(EntityName),
}
```

`isograph_compiler` gains the `nonempty` workspace dependency.

## The type walk

### Field declarations

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// The one `field` declaration of a client field or pointer, with its source file and its
/// literal's host-language context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDeclaration<TLiteralContext> {
    pub path: RelativePathToSourceFile,
    pub context: TLiteralContext,
    pub declaration: SelectableDeclaration,
}

/// The declaration of `field` in the source files `client` reads.
#[memo]
pub fn field_declaration<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
) -> DiagnosticResult<FieldDeclaration<LiteralContextOf<TProfile>>>;

pub type LiteralContextOf<TProfile> =
    <<TProfile as CompilationProfile>::HostLanguage as HostLanguage>::LiteralContext;
```

`field_declaration` filters `source_iso_literal_items` as `source_selectable_definitions` does and returns the one match; none and two or more are the same two diagnostics `selectable_named` returns. `SourceIsoLiteralItem` gains `pub context: &'db LiteralContextOf<TProfile>`, set from the extraction in `source_iso_literal_items`.

### Field types

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// What a field's generated types say, and which client fields its selection set selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldTypes {
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    /// Every client field or pointer the selection set selects at any depth, in selection order.
    pub selected: Vec<SelectedClientField>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedClientField {
    pub field: EntityNameAndSelectableName,
    pub loadability: Loadability,
}

/// Whether a selection has `@loadable`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loadability {
    Loadable,
    Eager,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GenerationError {
    #[error("{}", .0.0.message)]
    Lookup(Diagnostic),
    #[error("`{}.{}` has a value with no fields, so it cannot have a selection set.", .0.entity, .0.selectable)]
    SelectionSetOnScalar(SelectableKey),
    #[error("`{}.{}` is a client field, so it cannot have a selection set.", .0.entity, .0.selectable)]
    SelectionSetOnClientField(SelectableKey),
    #[error("`{}.{}` points to an entity with fields, so it needs a selection set.", .0.entity, .0.selectable)]
    MissingSelectionSet(SelectableKey),
    #[error("The declaration of `{0}` did not parse completely.")]
    IncompleteParse(EntityNameAndSelectableName),
}

/// The types of `field` as `client` reads the project.
#[memo]
pub fn field_types<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
) -> Result<FieldTypes, GenerationError>;
```

`Diagnostic` is `common_lang_types::Diagnostic`, which `entity_named` and `selectable_named` return. `IncompleteParse` is a `Slot` with no item or a `None` type annotation. Those appear only in a literal with parse errors, which `artifact_sources_in_file` (v0) already rejects before any walk runs; the walk returns the error instead of unwrapping.

`field_types` reads `field_declaration(db, client, field)`, then walks:

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// The object type of `selection_set` on `entity`, and the client fields it selects.
fn object_type<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
    entity: EntityName,
    selection_set: &SelectionSet,
) -> Result<(ArtifactObjectType, Vec<SelectedClientField>), GenerationError>;

/// One selection's field of the object type, and the client fields it selects.
fn selection_field<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
    entity: EntityName,
    selection: &Selection,
) -> Result<(ArtifactObjectField, Vec<SelectedClientField>), GenerationError>;
```

`field_types` calls `object_type` with `field.parent_entity_name` and the declaration's selection set; a declaration with no selection set (`field Owner.displayName`) has the empty object type and selects nothing. `object_type` maps every slot through `selection_field` and concatenates the selected lists in order. `selection_field`:

1. `selectable_named(db, client, entity, selection.name)`; an `Err` is `Lookup`.
2. `key` is `reader_alias` if present, else `name`. `description` is the selectable's.
3. By `selectable.kind`:
   - `ClientField(_)`: a selection set is `SelectionSetOnClientField`. The type is `Inner(ClientFieldOutput(P.f))`. It selects `P.f` with the selection's loadability.
   - `ClientPointer(Parsed(target))`: no selection set is `MissingSelectionSet`. The type is `target.try_map` over each inner entity: `object_type` of the selection set on that entity, wrapped as `Object`. It selects the pointer with the selection's loadability, followed by what each nested `object_type` selects, in inner-entity order.
   - `ClientPointer(Unparsed)`: `IncompleteParse`.
   - `Server(server)`: `server.target.try_map` over each inner entity: `entity_named` of it (`Err` is `Lookup`); `Scalar(kind)` with a selection set is `SelectionSetOnScalar`, without one is `Scalar(kind)`; `Object(_)` without a selection set is `MissingSelectionSet`, with one is `Object` of the nested `object_type`. It selects what the nested sets select.

Loadability is `Loadable` when the selection's `directive_set` has a directive named `loadable`, else `Eager`. A client field selected with `@loadable` has the same `ClientFieldOutput` type as an eager one; see open decision 2.

Parameters: a declaration with no variable definitions has `parameters: None`. Otherwise each `VariableDeclaration` becomes `ArtifactParameter { name, type_ }`, where `type_` is `wrapper_of_type_annotation` of its type (`None` is `IncompleteParse`) mapped by `input_type`: `entity_named` of the inner entity; `Scalar(kind)` is `Scalar(kind)`; `Object(_)` is `InputObject(name)`. Every `InputObject` name is expanded into `input_objects` by a worklist: pop a name not yet in the map, read `selectables_for_entity(db, client, name)`, map each `Server(target)` selectable to `ArtifactInputField` the same way, insert, and push the input objects those fields name. A selectable of an input object that is not `Server` is `Lookup` with a diagnostic naming it. The worklist ends because each name is expanded once, so a recursive input object (`input Filter { and: [Filter!] }`) is finite.

`field_types` is keyed by `(client, field)`. Editing one literal re-runs the walks of fields whose lookups read it, and each backdates when its `FieldTypes` is `==`.

### Reachability and the project

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// Everything a host language needs to print every artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectArtifacts<TLiteralContext> {
    /// Sorted by field.
    pub fields: Vec<FieldArtifacts<TLiteralContext>>,
    /// The `entity` literals, sorted.
    pub entities: Vec<EntityName>,
    /// The `entrypoint` literals, sorted.
    pub entrypoints: Vec<EntityNameAndSelectableName>,
    /// One per entity some pointer points to, sorted by entity.
    pub link_types: Vec<LinkType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldArtifacts<TLiteralContext> {
    pub field: EntityNameAndSelectableName,
    pub kind: FieldArtifactKind,
    pub resolver: FieldDeclarationSource<TLiteralContext>,
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    pub output_type: Generation,
    pub resolver_reader: Generation,
    /// The refetch reader, and the entrypoint and normalization AST of the field itself.
    pub loadable_artifacts: Generation,
}

/// Where a field's resolver is: its source file and its literal's host-language context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDeclarationSource<TLiteralContext> {
    pub path: RelativePathToSourceFile,
    pub context: TLiteralContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldArtifactKind {
    ClientField(ClientFieldKind),
    Pointer(Wrapper<EntityName>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    Generated,
    NotGenerated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkType {
    pub entity: EntityName,
    /// The entity itself when concrete; its concrete types when abstract.
    pub concrete_types: Vec<EntityName>,
}

/// The artifacts of the project whose iso literals are `sources`, as `client` reads it.
pub fn project_artifacts<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    sources: &BTreeSet<ArtifactSource>,
) -> Result<ProjectArtifacts<LiteralContextOf<TProfile>>, GenerationError>;
```

`project_artifacts` is a plain function over memos, called by `generate_artifacts` in `crates/isograph_cli/src/compile.rs`. It computes, from `field_types` of every `Field` source and every `Entrypoint` source's field:

- Entrypoints `E`: the `Entrypoint` sources. An entrypoint naming no declared field is `Lookup` from `field_declaration`, so it blocks the write. v0 generated an entrypoint artifact without looking anything up; with a reader artifact to import, the field must exist.
- Reachable `R`: a worklist from `E` over `field_types(..).selected`. A loop is the shape here (AGENTS.md, Iterators: a worklist).
- Loadably selected `L`: every `SelectedClientField` with `Loadable` in the `selected` list of a field in `R`.
- Output types `O`: `E`, plus every field in any field's `selected` list (upstream's `encountered_output_types`, which also visits fields not reachable from an entrypoint).
- `output_type` is `Generated` for `O`, `resolver_reader` for `R`, `loadable_artifacts` for `L`.
- `kind`: `selectable_named` of the field's own name; `ClientField(kind)` is `ClientField(kind)`, `ClientPointer(Parsed(target))` is `Pointer(target)`, `Unparsed` is `IncompleteParse`.
- `link_types`: for each distinct inner entity of every pointer's target, `entity_named` and its `kind`; `Object` gives the protocol's concrete types of the entity (`[entity]` for a source entity and an object type, the implementors or members of an interface or union, through a `NetworkProtocol` method over `object_types_implementing` and the union's members); `Scalar(_)` is `Lookup` with a diagnostic that a pointer points to a scalar.

## The host language

v0's method becomes:

```rust
// from crates/isograph_compiler/src/host_language.rs
pub trait HostLanguage: Send + Sync + Sized + 'static {
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;
    /// A project this host language cannot print.
    type GenerationError: std::fmt::Display + std::error::Error + 'static;

    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;

    fn should_include_source_file(relative_path: &Path) -> IncludeFile;

    /// The path and text of every artifact of `project`.
    fn generate_artifacts(
        project: &ProjectArtifacts<Self::LiteralContext>,
        options: &GenerationOptions,
    ) -> Result<Vec<(ArtifactPath, String)>, Self::GenerationError>;
}
```

```rust
// from crates/isograph_compiler/src/artifact_types.rs
/// Project facts the host language reads when printing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationOptions {
    /// `<output_directory>/__isograph`, relative to the config file's directory.
    pub artifact_folder: PathBuf,
    /// The config's `custom_scalars`.
    pub custom_scalars: BTreeMap<EntityName, ScalarValueKind>,
}
```

The test hosts (`TestHostLanguage` in isograph_compiler, `InHost` in isograph_cli's watch tests) return `Vec::new().wrap_ok()`. Their `GenerationError` is their existing `Error` type (`TestHostError`), not `Infallible`, which AGENTS.md rejects; a test host is a fixture that never fails, and that is the fixture's property, not a production `Result` that cannot fail.

```rust
// from crates/isograph_extract_typescript/src/generate_artifacts.rs
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TypeScriptGenerationError {
    #[error("`{0}` is not exported with `export const`, so its type cannot import its resolver.")]
    MissingExport(EntityNameAndSelectableName),
    #[error("No relative path leads from the artifact folder to the source file of `{0}`.")]
    SourceNotRelative(EntityNameAndSelectableName),
}
```

`MissingExport` is the `TypeScriptHostError::MissingExport` diagnostic's case reaching generation; the literal already shows that error. `SourceNotRelative` is `pathdiff::diff_paths` returning `None`, which happens when the artifact folder path has a `..` that the source path does not cross.

### The TypeScript generator

`TypeScriptHostLanguage::generate_artifacts` returns, in this order: per field (in `fields` order) its param type, parameters type, output type, resolver reader, refetch reader; per entrypoint in `E ∪ L` (sorted, deduplicated) its entrypoint and normalization AST; per link type its output type; `iso.ts`; `tsconfig.json`. Order does not matter to the write (v0 diffs a map); it fixes test expectations.

```rust
// from crates/isograph_extract_typescript/src/generate_types.rs
/// `P/f/param_type.ts`.
fn param_type_file(field: &FieldArtifacts<TypeScriptLiteralContext>, options: &GenerationOptions) -> (ArtifactPath, String);

/// `P/f/parameters_type.ts`.
fn parameters_type_file(field: EntityNameAndSelectableName, parameters: &ParametersType, options: &GenerationOptions) -> (ArtifactPath, String);

/// `P/f/output_type.ts`.
fn output_type_file(field: &FieldArtifacts<TypeScriptLiteralContext>, options: &GenerationOptions) -> Result<(ArtifactPath, String), TypeScriptGenerationError>;

/// `E/__link/output_type.ts`.
fn link_type_file(link: &LinkType) -> (ArtifactPath, String);

/// `{ readonly k: T; ... }`, each property with its description as a leading JSDoc comment.
fn object_type(object: &ArtifactObjectType, options: &GenerationOptions, comment_at: &mut impl FnMut(DescriptionValue) -> Span) -> TsType;

/// `wrapper` with `inner` printed at each `Inner`: `List` is `ReadonlyArray<T>`, a `Union`
/// of two or more variants is `(A | B)`, `Null` is `null`, a one-variant union is its variant.
fn wrapped_type<T>(wrapper: &Wrapper<T>, inner: &mut impl FnMut(&T) -> TsType) -> TsType;

/// A parameter's type: like `wrapped_type`, except that a top-level nullable union prints its
/// variants then `null | void` with no parentheses (upstream's `format_parameter_type`), and
/// the property is optional.
fn parameter_type(wrapper: &Wrapper<ArtifactInputType>, options: &GenerationOptions) -> TsType;

/// The specifier that imports the source file `source` from the artifact at `from`:
/// `pathdiff::diff_paths(source, artifact_folder.join(from's folder))`, `/`-separated, the
/// extension dropped (`./../../../Pet` from `Pet/fullName/output_type.ts` to `src/Pet.ts` with
/// folder `src/__isograph`).
fn source_specifier(from: &ArtifactPath, source: RelativePathToSourceFile, options: &GenerationOptions) -> Option<String>;
```

Names: `P__f__param`, `P__f__parameters`, `P__f__output_type`, `E____link__output_type` (`__link` is upstream's field name, so the name has four underscores), each `EntityNameAndSelectableName::underscore_separated()` plus the suffix. A type import's local name is the exported name. Specifiers between artifacts are v0's `artifact_import_specifier`, generalized to `relative_artifact_specifier(from: &ArtifactPath, to: &ArtifactPath) -> String`, which `artifact_import_specifier` now calls.

Keys: an object key is a `SelectionName`, which the tokenizer limits to `[a-zA-Z_][a-zA-Z0-9_]*`, so it prints as an identifier key.

Descriptions: upstream writes `/**\n<description>\n<indent>*/` before each property whose selectable has a description (`write_optional_description`). i2 attaches the same block comment as a leading comment of the `TsPropertySignature`: `comment_at` takes the next position `BytePos(n)` (`n` from 1), adds `Comment { kind: CommentKind::Block, span: DUMMY_SP, text: format!("*\n{text}\n") }` at it in a `SingleThreadedComments`, and returns `Span::new(BytePos(n), BytePos(n))` for the signature. `*/` inside a description is written `*\/` so it cannot close the comment. The closure is created where the module is built, next to the `SingleThreadedComments` it writes.

v0's `emit(module: Module) -> String` becomes `emit(module: Module, comments: &SingleThreadedComments, positions: u32) -> String`. It creates the `SourceMap` with one anonymous file of `positions` spaces, so every `BytePos(1..=positions)` lies in a file and any source-map query the emitter makes on those positions succeeds, and passes `comments.wrap_some()` as the emitter's `comments`. `swc_ecma_codegen` 33.0.2 emits leading comments of a `TsPropertySignature`'s span (`typescript.rs`, `emit_leading_comments_of_span`) and skips `BytePos(0)`, which is why positions start at 1. A module with no descriptions passes an empty `SingleThreadedComments` and `0`.

Stubs: `ArtifactModule { path, default_export: ArtifactValue::Object(vec![]) }`, printed by v0's `print_module` as `export default {};`.

Raw files: v0's `ArtifactFile` gains the variant v0 deferred:

```rust
// from crates/isograph_compiler/src/artifacts.rs
#[derive(Clone, Debug, PartialEq)]
pub enum ArtifactFile {
    Module(ArtifactModule),
    Raw(RawArtifactFile),
}

/// A file whose text the generator writes as is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawArtifactFile {
    pub path: ArtifactPath,
    pub contents: String,
}
```

`tsconfig.json` is its first producer. Query generation's `query_text.graphql`, if it lands, is its second.

### iso.ts

v0's `Identity` is replaced by upstream's two identity types, verbatim apart from the comments the emitter does not print:

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
type IdentityWithParam<TParam extends object, TReturnConstraint = unknown> = <TClientFieldReturn extends TReturnConstraint>(clientField: (param: TParam) => TClientFieldReturn) => (param: TParam) => TClientFieldReturn;
type IdentityWithParamComponent<TParam extends object> = <TClientFieldReturn, TComponentProps = Record<PropertyKey, never>>(clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn) => (data: TParam, componentProps: TComponentProps) => TClientFieldReturn;
```

Field overloads, by `FieldArtifactKind` (upstream's `build_iso_overload_for_client_defined_type`):

- `ClientField(Plain)`: `IdentityWithParam<P__f__param>`.
- `ClientField(Component)`: `IdentityWithParamComponent<P__f__param>`.
- `Pointer(target)`: `IdentityWithParam<P__f__param, T>`, where `T` is `wrapped_type(target)` with `E____link__output_type` at each inner entity `E`. `to Pet` gives `(Pet____link__output_type | null)`; `to Owner!` gives `Owner____link__output_type`.

The matched text stays `field P.f` for pointers (upstream's pointer keyword is `pointer`; i2 writes `field P.f to T`). The implementation signature returns `IdentityWithParam<any> | IdentityWithParamComponent<any> | object | void`. Imports, in this order: `import type { P__f__param } from "./P/f/param_type"` per field overload, then `import type { E____link__output_type } from "./E/__link/output_type"` per link type, then v0's entrypoint imports. Overload order is v0's.

For `field Pet.fullName`, `field Pet.bestFriend to Pet`, `field Query.HomeRoute @component`, `entity Owner`, `entrypoint Query.HomeRoute`:

```ts
// generated by crates/isograph_extract_typescript/src/generate_artifacts.rs
import type { Pet__bestFriend__param } from "./Pet/bestFriend/param_type";
import type { Pet__fullName__param } from "./Pet/fullName/param_type";
import type { Query__HomeRoute__param } from "./Query/HomeRoute/param_type";
import type { Pet____link__output_type } from "./Pet/__link/output_type";
import type Query__HomeRoute__entrypoint from "./Query/HomeRoute/entrypoint";
type IdentityWithParam<TParam extends object, TReturnConstraint = unknown> = <TClientFieldReturn extends TReturnConstraint>(clientField: (param: TParam) => TClientFieldReturn) => (param: TParam) => TClientFieldReturn;
type IdentityWithParamComponent<TParam extends object> = <TClientFieldReturn, TComponentProps = Record<PropertyKey, never>>(clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn) => (data: TParam, componentProps: TComponentProps) => TClientFieldReturn;
type WhitespaceCharacter = " " | "\t" | "\n";
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}` ? Whitespace<In> : In;
type MatchesWhitespaceAndString<TString extends string, T> = Whitespace<T> extends `${TString}${string}` ? T : never;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.bestFriend", T>): IdentityWithParam<Pet__bestFriend__param, (Pet____link__output_type | null)>;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Pet.fullName", T>): IdentityWithParam<Pet__fullName__param>;
export function iso<T>(param: T & MatchesWhitespaceAndString<"field Query.HomeRoute", T>): IdentityWithParamComponent<Query__HomeRoute__param>;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entity Owner", T>): void;
export function iso<T>(param: T & MatchesWhitespaceAndString<"entrypoint Query.HomeRoute", T>): typeof Query__HomeRoute__entrypoint;
export function iso(_isographLiteralText: string): IdentityWithParam<any> | IdentityWithParamComponent<any> | object | void {
    throw new Error("iso: Unexpected invocation at runtime. Either the Babel transform was not set up, or it failed to identify this call site. Make sure it is being used verbatim as `iso`.");
}
```

A resolver passed to `iso(...)` is contextually typed by the overload's `clientField` parameter, so `function fullName({ data })` has `data: Pet__fullName__param["data"]` with no annotation.

## Compiling

v0's `IsographState.artifact_folder: Option<ArtifactFolder>` becomes the folder and the options, which exist only together:

```rust
// from crates/isograph_cli/src/state.rs
/// Where and how the daemon writes artifacts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtifactOutput {
    pub folder: ArtifactFolder,
    pub generation: GenerationOptions,
}

    /// `None` when the config has no `output_directory`.
    pub(crate) artifact_output: Option<ArtifactOutput>,
```

`serve` builds `GenerationOptions { artifact_folder: output_directory.join(ISOGRAPH_FOLDER), custom_scalars }`, interning each `custom_scalars` key as an `EntityName`.

v0's `generate_artifacts` after this doc:

```rust
// from crates/isograph_cli/src/compile.rs
/// The write of every artifact whose text differs from the artifact folder's. No effects when
/// the config has no `output_directory`, when any literal failed, when the project cannot be
/// typed or printed, or when nothing changed.
fn generate_artifacts<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let Some(output) = state.artifact_output.clone() else {
        return no_effects();
    };
    let input = state.compile_input();
    let Some(files) = project_artifact_sources(input.reference())
        .map_err(|error| tracing::warn!(%error, "artifacts not written"))
        .ok()
        .and_then(|sources| {
            project_artifacts(input.db, input.client, sources.reference())
                .map_err(|error| tracing::warn!(%error, "artifacts not written"))
                .ok()
        })
        .and_then(|project| {
            TProfile::HostLanguage::generate_artifacts(project.reference(), output.generation.reference())
                .map_err(|error| tracing::warn!(%error, "artifacts not written"))
                .ok()
        })
    else {
        return no_effects();
    };
    state
        .artifact_files
        .replace(ArtifactFiles(files.into_iter().collect()))
        .map(|changes| {
            IsographEffect::WriteArtifacts(
                WriteArtifactsEffect { folder: output.folder, changes }.boxed(),
            )
        })
        .into_iter()
        .collect()
}
```

`input` borrows `state` immutably; it ends before `state.artifact_files` is written.

## Pet demo

`isograph.config.json`:

```json
{
  "source_files": ["src/**/*.ts"],
  "schema": ["schema/**/*.graphql"],
  "output_directory": "src",
  "custom_scalars": { "Url": "String" }
}
```

`src/iso.ts` is deleted; v0 kept it only for `ResolverParameters`. Every source drops `import type { ResolverParameters } from './iso'` and every resolver's parameter annotation:

```ts
// from demos/pet-demo/src/HomeRoute.ts
import { iso } from './__isograph/iso';

export const HomeRoute = iso(`
  field Query.HomeRoute @component {
    pets {
      id
      PetSummaryCard
    }
  }
`)(function HomeRoute({ data }) {
  return data.pets.map((pet) => pet.PetSummaryCard);
});

console.log(iso(`entrypoint Query.HomeRoute`).name);
```

The two pointers return link objects, which their overloads require:

```ts
// from demos/pet-demo/src/Pet.ts
export const bestFriend = iso(`
  field Pet.bestFriend to Pet {
    best_friend_relationship {
      best_friend {
        id
      }
    }
  }
`)(function bestFriend({ data }) {
  const friend = data.best_friend_relationship?.best_friend;
  return friend == null ? null : { __link: friend.id, __typename: 'Pet' as const };
});
```

```ts
// from demos/pet-demo/src/Owner.ts
export const owner = iso(`
  field Pet.owner to Owner! {
    id
  }
`)(function owner({ data }) {
  return { __link: `owner-of-${data.id}`, __typename: 'Owner' as const };
});
```

The other resolvers' bodies are unchanged. `demos/pet-demo/src/__isograph/` is regenerated and committed. `tsc --noEmit --strict` over the demo's `src` passes. The README's Sources section loses the sentence about `src/iso.ts` defining `iso` and gains: "`iso` is generated into `src/__isograph/iso.ts`; resolvers are typed by their field's generated param type." The Config section names `output_directory` and `custom_scalars`.

## Dependencies

- `refactors/pending/artifact-generation.md` (v0), every change. This doc edits its types (`HostLanguage::generate_artifacts`, `ArtifactFile`, `emit`, `IsographState.artifact_folder`, the entrypoint artifact).
- The schema integration: `refactors/pending/graphql-schema-parser/integration.md` (protocol lookups) and the merge of protocol definitions into `entity_named` and `selectable_named` (`refactors/past/source-backed-lookups.md`, Later: the protocol's entities). This doc needs from it: `SelectableKind::Server(ServerSelectable { target, data })` on every GraphQL field, with the field's description (landed); `NetworkProtocol::entity_kind` over the protocol's entity data, and a protocol method for the concrete types of an abstract entity (change 2 here adds both); input objects' fields as server selectables (change 2 here adds them, with open decision 6). Change 2 and later need it.
- Query generation (the GraphQL protocol plan): `query_text.ts`, `raw_response_type.ts`, and the refetch artifacts. Upstream's raw response type (`crates/artifact_content/src/raw_response_type.rs`) is printed from the merged selection set with a nullable field's key optional (`name?:`) and no `readonly`; that plan computes the merged set, and the TypeScript host prints it with `wrapped_type`. Nothing in this doc waits on it.

## Changes

Each change is one commit.

### 1. `Wrapper` and `SelectableKind`

Status: landed with integration.md change 2: `crates/isograph_compiler/src/wrapper.rs` (`Wrapper`, `UnionWrapper`, `WrapperVariant`, `inner_values`, `wrapper_of_type_annotation`), `Selectable.kind`, `SelectableKind` with `Server`, `ClientPointerTarget`, `ServerSelectable`, `selectable_of_declaration`, `selectable_target`, the hover text's target. `ClientFieldKind`, `Nullability`, `try_map`, and `nullability` move to change 2, their first reader.

Tests (`crates/isograph_compiler`, landed): `to Pet` is `Union([Inner(Pet), Null])`; `to Pet!` is `Inner(Pet)`; `to [Pet!]!` is `List(Inner(Pet))`; `to [Pet]` is `Union([List(Union([Inner(Pet), Null])), Null])`; `to [!]` is `None`; `inner_values` of `Union([Inner(A), List(Inner(B)), Null])` is `[A, B]`. The existing hover and `selectable_target` tests keep their expectations. Change 2 adds: no `to` with `@component` is `ClientField(Component)`, with no directive `ClientField(Plain)`, with `@loadable` `ClientField(Plain)`; `nullability` of `to Pet` is `Nullable` and of `to [Pet]!` is `NonNull`; `try_map` of `[Pet]` with a function that fails returns that error.

### 2. Entity kinds and the type walk

After the schema integration. `ScalarValueKind` (`common_lang_types`), `ScalarKind`, `EntityKind`, `Entity::kind` (source entities `Object`), `NetworkProtocol::entity_kind` and the concrete-types method, `GraphQLEntityData::Leaf` split into `Scalar` and `Enum`, `scalar_kind` in `graphql_network_protocol`, `ClientFieldKind`, `Nullability`, `try_map`, `nullability`, input object fields as server selectables, `artifact_types.rs` (every type above through `GenerationError`), `field_declaration`, `field_types`, `SourceIsoLiteralItem.context`.

Tests (`crates/graphql_network_protocol`, `#[cfg(test)]`, a profile of `TypeScriptHostLanguage` and `GraphQLNetworkProtocol` with `isograph_extract_typescript` as a dev-dependency; facts about `FieldTypes` values):

- `scalar_kind`: `String` and `ID` are `Known(String)`, `Int` and `Float` `Known(Number)`, `Boolean` `Known(Boolean)`, `Url` `Custom(Url)`. `entity_kind` of an enum is `Scalar(Known(String))`, of a union is `Object`; the concrete types of a union with members `A`, `B` are `[A, B]`.
- `type Pet { firstName: String! nickname: String friends: [Pet!]! }` and `field Pet.f { firstName nickname friends { firstName } }`: `firstName` is `Inner(Scalar(Known(String)))`; `nickname` is `Union([Inner(Scalar(..)), Null])`; `friends` is `List(Inner(Object([firstName])))`.
- An alias `n: nickname` has key `n`. A selection order `b a` keeps `[b, a]`. A described schema field carries its description; an undescribed one `None`.
- `field Pet.g { f }` with `field Pet.f`: `f` is `Inner(ClientFieldOutput(Pet.f))` and `selected` is `[Pet.f Eager]`; `f @loadable` is `Loadable`. A client field nested under `friends { f }` is in `selected`.
- `field Pet.p to Pet { id }` selected as `p { firstName }`: `p` is `Union([Inner(Object([firstName])), Null])`, and `selected` lists `Pet.p`.
- `firstName { x }` is `SelectionSetOnScalar`; `friends` with no set is `MissingSelectionSet`; `f { x }` on a client field is `SelectionSetOnClientField`; `nope` is `Lookup`.
- `field Owner.d` with no selection set has the empty object type and `parameters: None`.
- `field Pet.f($id: ID!, $limit: Int = 3)`: parameters `id` `Inner(Scalar(Known(String)))` and `limit` `Union([Inner(Scalar(Known(Number))), Null])`.
- `input Filter { and: [Filter!] name: String }` and `$filter: Filter`: `input_objects` has exactly `Filter` with `and` `Union([List(Inner(InputObject(Filter))), Null])`.
- Editing an unrelated literal leaves `field_types` of `Pet.f` `==` its previous value.

### 3. The full artifact set

`ProjectArtifacts`, `FieldArtifacts`, `FieldDeclarationSource`, `FieldArtifactKind`, `Generation`, `LinkType`, `project_artifacts`, `GenerationOptions`, `HostLanguage::GenerationError` and the new `generate_artifacts` signature, the test hosts, `TypeScriptGenerationError`, `generate_types.rs`, `relative_artifact_specifier`, `emit` with comments, `ArtifactFile::Raw`, `RawArtifactFile`, the stubs, `tsconfig.json`, the link types, the typed `iso.ts`, the entrypoint's imports, `custom_scalars` in the config, `ArtifactOutput`, `serve`, `compile.rs`.

Tests for `project_artifacts` (`crates/isograph_cli/src/compile.rs`, the TypeScript test profile; source entities only, so every type is a client field or pointer over `entity` declarations):

- `entity Pet`, `field Pet.a { b }`, `field Pet.b`, `field Pet.c`, `entrypoint Pet.a`: `a` and `b` have `resolver_reader` `Generated`, `c` `NotGenerated`; `output_type` is `Generated` for `a` and `b`, `NotGenerated` for `c`.
- `field Pet.a { b @loadable }`: `b` has `loadable_artifacts` `Generated`; the same selection under a field no entrypoint reaches leaves it `NotGenerated`.
- `field Pet.p to Pet { b }`: `link_types` is `[Pet: [Pet]]` and `p`'s kind is `Pointer(Union([Inner(Pet), Null]))`.
- `entrypoint Pet.missing` is `Err(Lookup(..))`.

Tests for the TypeScript generator (`crates/isograph_extract_typescript`, hand-built `ProjectArtifacts`; facts about text):

- A field with data `{ firstName: Known(String), nickname: Union([Known(String), Null]), friends: List(Object({ id })) }` prints `readonly firstName: string`, `readonly nickname: (string | null)`, and `readonly friends: ReadonlyArray<{` with `readonly id: string` inside.
- `Custom(Url)` prints `unknown`, and `string` when `custom_scalars` maps `Url` to `String`. `Known(Number)` prints `number`, `Known(Boolean)` `boolean`.
- `ClientFieldOutput(Pet.fullName)` prints `Pet__fullName__output_type` and the file imports it with `import type { Pet__fullName__output_type } from "./../../Pet/fullName/output_type"`. The same output type named twice is imported once.
- A description `The pet's ID` prints `/**` then `The pet's ID` then `*/` immediately before that property's `readonly`; a description containing `*/` prints `*\/`; a property with no description has no comment before it.
- No parameters: `readonly parameters: Record<PropertyKey, never>` and no `parameters_type.ts`. With `$id: ID!` and `$limit: Int`: `parameters_type.ts` has `readonly id: string` and `readonly limit?: number | null | void`, and `param_type.ts` imports `Pet__f__parameters` from `"./parameters_type"`.
- An input object `Filter` prints a local `type Filter = {` in `parameters_type.ts` and the parameter's type names `Filter`.
- `output_type.ts` of a field exported as `fullName` from `src/Pet.ts` with artifact folder `src/__isograph` has `import type { fullName as resolver } from "./../../../Pet"` and `ReturnType<typeof resolver>`. A context with no export name is `Err(MissingExport(Pet.fullName))`.
- `resolver_reader.ts`, `refetch_reader.ts`, and `normalization_ast.ts` are exactly `export default {};` and a newline.
- `entrypoint.ts` has `kind: "Entrypoint"`, `name: "Query.HomeRoute"`, `normalizationAst: Query__HomeRoute__normalization_ast`, `readerArtifact: Query__HomeRoute__resolver_reader`, and imports both from `"./normalization_ast"` and `"./resolver_reader"`.
- A link type `Pet: [Pet]` prints `readonly __link: string` and `readonly __typename: "Pet"`; `Node: [A, B]` prints a union of two such objects; `Node: []` prints `never`.
- `iso.ts`: a `Plain` field's overload returns `IdentityWithParam<Pet__fullName__param>`; a `Component` field's returns `IdentityWithParamComponent<Query__HomeRoute__param>`; a pointer `to Pet` returns `IdentityWithParam<Pet__bestFriend__param, (Pet____link__output_type | null)>` and `to Owner!` `IdentityWithParam<Pet__owner__param, Owner____link__output_type>`; param-type imports precede link imports, which precede entrypoint imports.
- `tsconfig.json` is upstream's text byte for byte.
- No file's path is generated twice in one result.

Handle tests (`crates/isograph_cli/src/compile.rs`): v0's tests with the new file set (the first fire writes `iso.ts`, `tsconfig.json`, and every file of `Query/HomeRoute/`); editing a selection set that changes a param type writes only that `param_type.ts`; adding a description to a field writes the param types that select it and not its own; a config without `custom_scalars` and one with it differ only in the param types that select the custom scalar.

### 4. Pet demo

The config, the deleted `src/iso.ts`, the sources without `ResolverParameters`, the pointer resolvers, the README, and the regenerated `src/__isograph/`. `tsc --noEmit --strict` over `src` passes.

## Open decisions

### 1. Scalar type mapping

Options and consequences are in Scalars above. I hold option 3: the protocol reports `ScalarKind` (`Known(ScalarValueKind)` or `Custom(name)`), the config's `custom_scalars` maps a custom scalar's name to a `ScalarValueKind`, and the host language maps the value kind to its type, with `unknown` for an unmapped custom scalar.

### 2. Types upstream imports from `@isograph/react` and `react`

Upstream's type files import `Link`, `ExtractSecondParam`, `CombineWithIntrinsicAttributes`, `LoadableField`, `ExtractParameters` from `@isograph/react` and `React` from `react`. They appear in: component output types (`(React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>)`), link output types (`Link<"Pet">`), and param types that select a field with `@loadable` or select a pointer (`LoadableField<P__f__param, T, ...>`).

- (a) Print upstream's types and imports verbatim. The generated text matches upstream's. Every project that type-checks the generated files needs `@isograph/react` and `@types/react` installed; the pet demo becomes a pnpm workspace package with both as dev dependencies. i2 deleted `@isograph/react` from this repo (180ec76f, "Delete the libs and demos trees"), v0 dropped the `IsographEntrypoint` import so a project without it type-checks, and the planned runtime (`refactors/pending/runtime/pico-runtime.md`) has no `ComponentReaderArtifact`, no React, and no loadable fields, so each of these imports changes again when that runtime lands.
- (b) Print only types that need no package. A component's output type is `ReturnType<typeof resolver>`, as a plain field's. A link is the structural object `{ readonly __link: string; readonly __typename: "Pet" }` in `Pet/__link/output_type.ts`, with no brand. A `@loadable` selection has the field's output type, and a pointer selection has the nested object type under the pointer's wrapper, as an eager selection would. Generated files type-check with nothing installed. The component, loadable, and pointer selection types differ from upstream's, and a later framework (pluggable-compiler.md's `FrameworkBindings`) or runtime supplies its own forms.

I hold (b), for the three reasons in (a)'s consequences: the package is gone from i2, v0 already set the rule that generated files reference only the artifact folder and the user's sources, and the runtime i2 is building does not have the types (a) imports.

### 3. Which per-field files exist

- (a) Upstream's rules: `resolver_reader.ts` only for fields reachable from an entrypoint, `output_type.ts` only for entrypoints and selected fields, `refetch_reader.ts` and a field's own entrypoint only for loadably selected reachable fields. Adding an entrypoint adds reader files of every field it reaches; generation needs the reachability worklist, which query generation also walks.
- (b) Every file for every field. Each file depends on one literal, so an edit changes only that literal's files; the generated tree is larger than upstream's and has readers no runtime loads.

I hold (a): the user's direction is upstream's artifact set, and the worklist is a few lines over `field_types`.

### 4. The entrypoint artifact's content

- (a) v0's `{ kind, name }` unchanged. The stub files are imported by nothing.
- (b) v0's object plus `normalizationAst` and `readerArtifact` importing the stubs, named as pico-runtime.md's `Entrypoint`. The module graph from an entrypoint reaches its stubs, as upstream's does; when readers become real, `readerArtifact` is already the default export the runtime calls.
- (c) Upstream's shape: `networkRequestInfo: { kind: "NetworkRequestInfo", operation: { kind: "Operation", text }, normalizationAst }`, `concreteType`, `readerWithRefetchQueries: { kind: "ReaderWithRefetchQueries", nestedRefetchQueries, readerArtifact }`. It needs query text and `concreteType` from query generation, and it is the shape of the runtime i2 is not building.

I hold (b).

### 5. Enum types

- (a) `string`, upstream's. A server that adds an enum value does not break a reader's types.
- (b) The union of the enum's values (`"CAT" | "DOG" | "OTHER" | "HAMSTER"`). `ScalarKind` gains `Enum(Vec<EnumLiteralValue>)` and the protocol reports the values (and the values of every `extend enum`). A reader's `switch` is checked for exhaustiveness; a value added on the server is outside the type at runtime.

I hold (a), upstream's behavior.

### 6. Input object parameter types

- (a) Upstream's inline expansion (`format_server_field_scalar_type` prints the input object's fields in place). A recursive input object (`input Filter { and: [Filter!] }`) recurses without end; upstream has this defect.
- (b) One local `type <InputObject> = { ... }` per input object in `parameters_type.ts`, referenced by name. Recursive input objects are finite. The text differs from upstream's for every input object parameter.

I hold (b).

## Docs this doc amends

docs-website/docs/design-docs/mental-model.md, in change 1: the Wrapper section's `Wrapper`, `UnionWrapper`, and `WrapperVariant` become generic over the wrapped value (`Wrapper<T>`, `Inner(T)` where the section has `Entity(Entity)`), with the sentence "A selectable's target is a `Wrapper<EntityName>`; a generated type is the same structure over a host-language-neutral type." In change 2, the Entity section gains "Every entity has a kind: an object, read through its selectables, or a scalar, read whole, with a `ScalarKind` saying how its values arrive."

## Later

- `__link` as a selectable on every object entity (upstream's `add_link_fields`), with description `A store Link for the <E> type.`, typed `E____link__output_type`. Then pointer resolvers select `__link` instead of building link objects.
- `@updatable` selections (upstream's `startUpdate` and `StartUpdate` in the param type): they need a runtime write path.
- Arbitrary host-language type text for custom scalars (Scalars, option 5).
- `as<Member>` refinement selectables and their reader stubs, when the protocol defines them.
- `persisted_documents.json`, when the config has the option.
