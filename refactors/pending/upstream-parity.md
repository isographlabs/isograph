# Parity with upstream isograph

This doc is an inventory, not a plan. It lists everything upstream isograph supports or does that i2 does not, or does differently, as a user or a maintainer sees it. It proposes no designs.

Upstream is the checkout at `/Users/rbalicki/code/isograph` (commit `79e2a3c0`); its paths are written `isograph/<path>:<line>`. i2 paths are relative to this repo. Evidence was read from source at i2 commit `46ad7828e`.

Each bullet carries one status:

- `missing`: upstream has it, i2 has nothing and no doc plans it.
- `partial`: i2 has part of it.
- `different`: i2 has it with different behavior; the bullet says what differs.
- `planned`: a pending doc specifies it. Every runtime doc (`refactors/pending/runtime/store.md:1`, `reader-ast.md:3`, `react.md:3`, `pico-runtime.md:3`) is under discussion and not approved, so `planned` there means specified, not scheduled.
- `dropped`: i2 deliberately does not do it, by a recorded decision (a v1 scope decision in `refactors/pending/`, an AGENTS.md rule, or a commit that removed it).

Within each area, unplanned gaps (`missing`, `partial`, `different`) come first, then `planned`, then `dropped`.

## Language and parser

### Unplanned gaps

- `different`: literal keywords are `field`, `pointer`, `entrypoint` upstream (`isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:58,70,82`); i2 has `entity`, `field`, `entrypoint` and no `pointer` keyword (`crates/isograph_parser/src/parse_iso_literal.rs:213-216`).
- `different`: a client pointer is `pointer P.f to T` with `to` required upstream (`parse_iso_literal.rs:241-258`); i2 writes `field P.f to T` with `to` optional (`refactors/past/fields-pointers.md:4`, `crates/isograph_parser/src/parse_iso_literal.rs:297-321`).
- `different`: upstream requires a selection set on every field and pointer ("Selection sets are required", `isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:200-206,296-302`); in i2 the selection set is optional and a field without one is a declaration (`docs-website/docs/design-docs/mental-model.md:152`, `crates/isograph_compiler/src/compiler_errors.rs:171`).
- `different`: upstream has a dedicated message for an unknown literal keyword (`parse_iso_literal.rs:97-101`); i2 reports the generic "Expected the keyword ..." (`crates/isograph_parser/src/parse_error.rs:105`).
- `missing`: upstream reports `...` (spread or inline fragment) with a message suggesting `asT` (`isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:441-443,920-958`); i2 reports "Expected a selection, found period" (`crates/isograph_parser/src/selections.rs:419-439`).
- `different`: upstream rejects a variable inside a default value at parse time (`parse_iso_literal.rs:767-792`); i2 parses it and rejects it at merge as `VariableInDefault`, which reaches only the daemon log (`crates/isograph_compiler/src/merged_selection.rs:205`).
- `different`: argument values: upstream parses variable, string, int, boolean, null, object (`isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:593-687`); i2 adds list literals and block strings (`crates/isograph_parser/src/arguments.rs:63-71,288-293`) but query printing rejects lists ("List arguments are not supported in queries yet.", `crates/isograph_compiler/src/merged_selection.rs:211`).
- `different`: upstream panics on an integer overflowing i64 (`isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:636`); i2 reports `IntegerDoesNotFitI64` (`crates/isograph_parser/src/parse_error.rs:15-16`).
- `different`: upstream stops at the first parse error per literal; i2 recovers and reports several, with partial ASTs (`crates/isograph_parser/src/parse_iso_literal.rs:134-165`).

### Planned

- `planned`: `#` comments; neither side supports them (upstream `isograph/crates/isograph_lang_parser/src/token_kind.rs:77-79` commented out); i2 plans them in `refactors/pending/comments.md`.

### Dropped

- `dropped`: directives in every position (upstream `isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:512-549`); i2 parses each and reports "unexpected directive" (`crates/isograph_parser/src/directives.rs:31-33,95-99`); "v1 supports no directive, by user decision" (`refactors/pending/iso-directives-unexpected.md:7`).
- `dropped`: `@component` on client fields (`isograph/crates/isograph_lang_types/src/client_selectable_directive_set.rs:7-19`), per `refactors/pending/iso-directives-unexpected.md:7,73`.
- `dropped`: `@loadable(lazyLoadArtifact:)` on selections (`isograph/crates/isograph_lang_types/src/selection_directive_set.rs:9-13,28-44`); "v1 also drops everything `@loadable` drove" (`refactors/pending/iso-directives-unexpected.md:9,38`).
- `dropped`: `@updatable` on scalar and object selections (`selection_directive_set.rs:9-26`); rejected under the v1 directive decision, listed Later in `refactors/pending/artifact-set-and-types.md:1457`.
- `dropped`: `@lazyLoad(reader:, normalization:)` on entrypoints (`isograph/crates/isograph_lang_types/src/entrypoint_directive_set.rs:7-62`), per `refactors/pending/iso-directives-unexpected.md:3`.
- `dropped`: unknown directive names and arguments rejected by serde `deny_unknown_fields` with one diagnostic over the whole list (`isograph/crates/isograph_lang_types/src/isograph_directives.rs:19-31`); i2 reports one error per directive on its own span (`refactors/pending/iso-directives-unexpected.md:7`).

## Schema and GraphQL

### Unplanned gaps

- `different`: upstream defines `ID`, `String`, `Int`, `Float`, `Boolean` implicitly (`isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:50-138`); in i2 a built-in scalar exists only if a schema file declares it (`refactors/past/graphql-schema-parser/integration.md:57`, `demos/pet-demo/schema/scalars.graphql`).
- `different`: upstream reads one `schema` file plus `schema_extensions`, and extensions are allowed only in extension files (`isograph/crates/graphql_schema_parser/src/parse_schema.rs:37-90`); i2 reads every `.graphql`/`.graphqls` file the `schema` globs match (`crates/graphql_network_protocol/src/lib.rs:69-74`).
- `different`: upstream supports only `extend type` and applies only its directives, ignoring extension fields ("TODO extensions", `isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:672-678`); i2 applies `extend` for schema, scalar, type, interface, union, enum, input with merged contents (`crates/graphql_network_protocol/src/protocol_data.rs:130-150,419-445`).
- `missing`: `@exposeField(field:, as:, fieldMap:)` on SDL types creates mutation fields (`isograph/crates/isograph_schema/src/create_additional_fields/expose_field_directive.rs:8-18`); i2 parses SDL directives and never reads them. `refactors/pending/artifact-set-and-types.md:93` says the GraphQL protocol plan owns it; no `graphql-protocol/*.md` mentions it; `runtime/react.md:252` excludes the hook only.
- `missing`: `@canonicalId(fieldName:)` is deserialized upstream (`isograph/crates/isograph_schema/src/create_additional_fields/canonical_id_directive.rs:6-8`) but nothing upstream reads it; i2 lacks it.
- `different`: root operation types: upstream defaults each missing slot to `Query`/`Mutation`/`Subscription` even inside `schema {}`, and a duplicate `schema` is non-fatal (`isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:139-160`); i2 follows the spec, so a `schema {}` without `mutation` has no mutation root and a duplicate is fatal (`crates/graphql_network_protocol/src/root_operation_types.rs:50-62`, delta at `refactors/pending/graphql-protocol/entrypoints.md:392`).
- `partial`: `__typename` is user-selectable upstream, typed as the literal `"T"` or the member union (`parse_nested_server_schema.rs:331-355,472-500`); i2 adds it only as the query's discriminator (`crates/graphql_network_protocol/src/lib.rs:153-155`) and a user-written `__typename` resolves to no selectable (`protocol_data.rs:226-250,491-530`); `refactors/pending/artifact-set-and-types.md:261` plans only the literal output type.
- `different`: identity: upstream takes any field named `id` and checks `ID!` under `on_invalid_id_type` (`isograph/crates/isograph_schema/src/memoized/server_selectable_access.rs:38-62`); i2 takes `id` only when it is `id: ID!` and otherwise silently treats the type as having no identity (`crates/graphql_network_protocol/src/protocol_data.rs:88-105`).
- `different`: custom scalars are `unknown` upstream (`isograph/crates/graphql_network_protocol/src/nested_server_schema/parse_nested_server_schema.rs:538-550`); i2 maps them through the profile's `custom_scalar_types`, and the `isograph` binary maps none (`crates/ts_graphql_react_isograph_cli/src/main.rs:18-20`).
- `different`: SDL parse errors: upstream fails at the first error per file and reports it (`isograph/crates/graphql_schema_parser/src/parse_schema.rs:28-46`); i2 recovers per definition (`crates/graphql_schema_parser/src/parse_schema.rs:34-60`) but shows schema-file errors nowhere until `refactors/pending/compiler-errors.md` change 9 (`compiler-errors.md:26`), so this is also `planned` there.

### Dropped

- `dropped`: entrypoints on any type with `id`, fetched as `node(id:) { ... on T }` (`isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:305-357`); "v1 does not, by the user's decision" (`refactors/pending/graphql-protocol/entrypoints.md:44`).
- `dropped`: `__refetch` client field on every object with `id` (`isograph/crates/isograph_schema/src/create_additional_fields/process_type_system_definition.rs:81-105,193-231`); "v1 has no refetch queries" (`refactors/pending/runtime/react.md:253`).
- `dropped`: imperatively loaded fields (`ClientFieldVariant::ImperativelyLoadedField`, `isograph/crates/isograph_schema/src/memoized/refetch_strategy_access.rs`), per `refactors/pending/runtime/react.md:252-253`.
- `dropped`: `__link` selectable on every object (`isograph/crates/isograph_schema/src/validated_isograph_schema/add_link_fields.rs:21`); "v1 has no `__link` selectable and no pointers" (`refactors/pending/runtime/react.md:254`), Later in `artifact-set-and-types.md:1456`.
- `dropped`: client pointers in queries: upstream merges a pointer's selections into the parent query (`isograph/crates/isograph_schema/src/create_merged_selection_set.rs:1291-1349`); i2 merges an empty map (`crates/isograph_compiler/src/merged_selection.rs:998`); "Supporting pointers is a later doc" (`refactors/pending/graphql-protocol/query-generation.md:775`).

## Validation

### Unplanned gaps

- `different`: upstream reports every validation through `validate_entire_schema` (`isograph/crates/isograph_schema/src/validate.rs:32-81`); i2 diagnostics carry only `Parse` and `Validation` (`crates/isograph_compiler/src/compiler_errors.rs:14-19`), and every merge or generation error reaches only the daemon log as `warn "artifacts not written"` (`crates/isograph_cli/src/compile.rs:85-100`).
- `missing`: "Invalid parent type. `S` is a scalar" for a field parent (`isograph/crates/isograph_schema/src/create_additional_fields/process_client_field_declaration.rs:72`).
- `missing`: pointer target "`T` is not a type that has been defined" (`process_client_field_declaration.rs:116,154`); nothing in i2 checks `to T`.
- `missing`: pointer target must be an object, interface or union (`process_client_field_declaration.rs:135`).
- `missing`: "A field with name or alias `x` has already been defined" in one selection set (`isograph/crates/isograph_schema/src/validate_selection_sets.rs:79`); i2 checks no duplicate `reader_alias` (`crates/isograph_compiler/src/artifact_types.rs:300`).
- `partial`: selected as a scalar vs an object (`isograph/crates/isograph_schema/src/memoized/client_selectable_access.rs:233`, `validate_selection_sets.rs:379-397`); i2 diagnoses only `SelectionSetOnFieldWithoutTarget` (`crates/isograph_compiler/src/validation.rs:81-82`); `SelectionSetOnLeaf`, `MissingSelectionSet`, `SelectionSetOnScalar`, `SelectionSetOnClientField` are log-only (`merged_selection.rs:219-221`, `artifact_types.rs:132-143`).
- `missing`: "The field `P.f` has unused variables" (`isograph/crates/isograph_schema/src/validate_use_of_arguments.rs:321`).
- `different`: missing arguments: upstream checks client and server selections and does not require nullable arguments (`validate_use_of_arguments.rs:381`); i2's `MissingFieldArgument` covers client fields only and requires every variable without a default, nullable included (`crates/isograph_compiler/src/validation.rs:300-304,395`).
- `missing`: server field arguments are not checked against the schema definition at all (`refactors/pending/graphql-protocol/entrypoints.md:323`, "a later validation pass", no doc).
- `partial`: extra arguments (`isograph/crates/isograph_schema/src/validate_use_of_arguments.rs:399-419`); i2's `UnknownFieldArgument` covers client fields only and is log-only (`crates/isograph_compiler/src/merged_selection.rs:1098-1112`).
- `partial`: "This variable is not defined: $x" (`isograph/crates/isograph_schema/src/validate_argument_types.rs:476`); i2 has `UndefinedVariable`, log-only (`crates/isograph_compiler/src/merged_selection.rs:203`).
- `missing`: variable type mismatch "Received $x, with type T, expected U" (`isograph/crates/isograph_schema/src/validate_argument_types.rs:162`).
- `missing`: literal type checks (int for Int/Float/ID, string for String/ID, boolean) (`validate_argument_types.rs:173-222,483-492`).
- `missing`: "Expected non null input of type T, found null" (`validate_argument_types.rs:232`).
- `missing`: list and union input matching (`validate_argument_types.rs:273,287`).
- `missing`: input object missing fields and extra fields (`validate_argument_types.rs:338,439`); i2 checks only `DuplicateObjectKey` (`crates/isograph_compiler/src/merged_selection.rs:209`).
- `missing`: variable definition type is undefined (`isograph/crates/isograph_schema/src/validate.rs:210-254`); i2 rejects only unsupported shapes (`UnsupportedVariableType`, `crates/isograph_compiler/src/merged_selection.rs:201`).
- `missing`: schema field or argument whose type is undefined (`isograph/crates/isograph_schema/src/validate.rs:100-154`).
- `missing`: `id` must be `ID!`, governed by `on_invalid_id_type` (`validate.rs:156-169`).
- `different`: entrypoint target must be a client field, not a pointer or server field (`isograph/crates/isograph_schema/src/memoized/entrypoint_access.rs:100`); i2 rejects schema fields (`ProtocolSelectable`) but accepts `field E.f to T` (`crates/isograph_compiler/src/entrypoint.rs:53-59`).
- `different`: "Type `E` is not fetchable" (`isograph/crates/graphql_network_protocol/src/graphql_network_protocol.rs:326,377`); i2 splits it into `NotRootOperationType`, `NotObject`, `UnknownEntity`, `SourceEntity`, `NotComposite` (`crates/graphql_network_protocol/src/fetch_plan.rs:21-32`).
- `missing`: the "selection does not exist" diagnostic carries a create-field code action upstream (`isograph/crates/isograph_schema/src/validate_selection_sets.rs:399-430`); i2's `UnknownSelectable` has none (`refactors/pending/auto-export-fields.md:478`).

### Dropped

- `dropped`: directive-placement errors for `@loadable` on server scalars, exposed fields, `__link`, and `@updatable` on client fields (`isograph/crates/isograph_schema/src/validate_selection_sets.rs:178,201,215,228,337`); moot under `refactors/pending/iso-directives-unexpected.md:7`.
- `dropped`: "Entrypoint declared lazy in one location and declared eager in another" (`isograph/crates/isograph_schema/src/memoized/entrypoint_access.rs:133`); `@lazyLoad` is dropped.
- `dropped`: `@exposeField` errors (`isograph/crates/isograph_schema/src/create_additional_fields/parse_type_system_document.rs:116,294,305,321,335,346`); the feature is missing (Schema and GraphQL).

## Compiler and artifacts

### Unplanned gaps

- `missing`: `raw_response_type.ts` (`isograph/crates/artifact_content/src/entrypoint_artifact.rs:290-301`, `raw_response_type.rs:12-33`); `refactors/pending/artifact-set-and-types.md:89` assigns it to the protocol plan, which does not specify it, and `runtime/react.md:481` drops `TRawResponseType`.
- `different`: `iso.ts` overloads (`isograph/crates/artifact_content/src/iso_overload_file.rs:140-290`); i2 has no `IsographEntrypoint` import, entrypoint overloads return `typeof` the default export, no comments, a different throw message (`crates/typescript_artifacts/src/iso_overloads.rs:20`; deltas at `refactors/pending/artifact-generation.md:552-562`).
- `missing`: `no_babel_transform` form of `iso.ts`, a runtime switch over literal text (`isograph/crates/artifact_content/src/iso_overload_file.rs:248-280`); "i2 has no such option" (`refactors/pending/artifact-generation.md:558`).
- `different`: `parameters_type.ts` (`isograph/crates/artifact_content/src/eager_reader_artifact.rs:170-188`); i2 makes every parameter required and emits input objects as local named types (`refactors/pending/artifact-set-and-types.md:1443-1448`, `refactors/pending/runtime/store.md` open decision 11).
- `different`: `output_type.ts` (`isograph/crates/artifact_content/src/eager_reader_artifact.rs:420-486`); i2 uses `import type`, and checks the named export at generation (`MissingExport`, `crates/typescript_artifacts/src/lib.rs:24-30`) where upstream checks at parse (`isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:906`).
- `different`: `<E>/__link/output_type.ts` is `Link<"E">` from `@isograph/react` upstream (`isograph/crates/artifact_content/src/eager_reader_artifact.rs:488-512`); i2 writes a structural `{ readonly __link: string; readonly __typename: "E" }` (`crates/typescript_artifacts/src/generate_types.rs:222-228`, decision 2b at `refactors/pending/artifact-set-and-types.md:1412-1419`).
- `missing`: `generated_file_header` comment on every artifact (`isograph/crates/artifact_content/src/generate_artifacts.rs:109-114`).
- `missing`: `.ts` extensions in generated imports under `include_file_extensions_in_import_statements` (`isograph/crates/isograph_config/src/compilation_options.rs:51-58`); i2 imports are always extensionless (`crates/typescript_artifacts/src/imports.rs:64-65`).
- `different`: write policy: upstream deletes and recreates the artifact directory on first compile, then hash-diffs (`isograph/crates/artifact_content/src/file_system_state.rs:26-165`); i2 writes only changed text, deletes the rest, prunes empty folders (`crates/isograph_cli/src/write_artifacts.rs:13-87`).
- `partial`: error printing: upstream prints every diagnostic with location and "Compilation took X" (`isograph/crates/isograph_compiler/src/batch_compile.rs:51-65`); i2 logs `compiled errors=N` (`crates/isograph_cli/src/compile.rs:59-61`); the `isograph errors` verb is planned (`refactors/pending/compiler-errors.md:5,24-30`).
- `missing`: success stats "Compiled N client fields, N client pointers, N entrypoints, wrote N files, in T" (`isograph/crates/isograph_compiler/src/batch_compile.rs:71-89`).
- `missing`: reload of config and full recompile when the config file changes (`isograph/crates/isograph_compiler/src/watch.rs:44-55`); i2 ignores config changes (`crates/isograph_cli/src/watch.rs:169`) until `isograph restart`.
- `missing`: periodic pico garbage collection every 60s (`isograph/crates/isograph_compiler/src/compiler_state.rs:12,39-44`); "i2 never runs pico garbage collection" (`refactors/pending/compiler-errors.md:1869`).
- `different`: source selection: upstream reads every `.ts/.tsx/.js/.jsx` under `project_root` and skips any path containing `__isograph` (`isograph/crates/isograph_compiler/src/read_files.rs:16-37,78-90`); i2 selects by `source_files` globs and skips only `<output_directory>/__isograph` (`crates/isograph_cli/src/watch.rs:224-226`).
- `different`: watch scope: upstream watches `project_root`, schema, extensions, config (`isograph/crates/isograph_compiler/src/watch.rs:249-296`); i2 watches the config directory recursively and filters by globs (`crates/isograph_cli/src/watch.rs:88-135`).

### Planned

- `planned`: typed `entrypoint.ts` (`IsographEntrypoint<...>` with `networkRequestInfo`, `concreteType`, `readerWithRefetchQueries`, `isograph/crates/artifact_content/src/entrypoint_artifact.rs:500-526`); i2 writes an untyped `{ kind, name, queryText, normalizationAst, readerArtifact }` (`crates/typescript_artifacts/src/entrypoint.rs:12-49`); typing planned in `refactors/pending/runtime/react.md:462-481`.
- `planned`: printed `normalization_ast.ts` (`isograph/crates/artifact_content/src/entrypoint_artifact.rs:270-288`); i2 builds the AST in the core (`crates/isograph_compiler/src/normalization_ast.rs`) but writes `export default {};` (`crates/typescript_artifacts/src/lib.rs:85`); planned in `refactors/pending/graphql-protocol/normalization-ast.md`.
- `planned`: eager `resolver_reader.ts` with a reader AST (`isograph/crates/artifact_content/src/eager_reader_artifact.rs:39-160`); i2 writes a stub (`crates/typescript_artifacts/src/lib.rs:73-76`); planned in `refactors/pending/runtime/reader-ast.md`.
- `planned`: one-shot batch compile with exit code (`isograph/crates/isograph_compiler/src/batch_compile.rs:25-39`); Later in `refactors/pending/runtime/store.md:778`.
- `planned`: CI check that committed artifacts are up to date (`isograph/.github/workflows/ci.yml:56-79`); Later in `refactors/pending/runtime/store.md:778`.

### Dropped

- `dropped`: `@lazyLoad` loader entrypoints (`isograph/crates/artifact_content/src/entrypoint_artifact.rs:432-497`), per `refactors/pending/iso-directives-unexpected.md:3`.
- `dropped`: `@component` artifacts (`ComponentReaderArtifact`, `React.FC` output type, `IdentityWithParamComponent` overload; `isograph/crates/artifact_content/src/eager_reader_artifact.rs:132-150,455-476`, `iso_overload_file.rs:157-168`), per `refactors/pending/iso-directives-unexpected.md:73`.
- `dropped`: `@loadable` artifacts (`refetch_reader.ts`, the field's own `entrypoint.ts` and `normalization_ast.ts`, `LoadableField` param types; `isograph/crates/artifact_content/src/generate_artifacts.rs:207-298`), per `refactors/pending/iso-directives-unexpected.md:9`.
- `dropped`: `@updatable` artifacts (`StartUpdate`, updatable data type; `isograph/crates/artifact_content/src/eager_reader_artifact.rs:345,388`), Later in `refactors/pending/artifact-set-and-types.md:1457`.
- `dropped`: imperatively loaded field and `@exposeField` refetch readers (`isograph/crates/artifact_content/src/refetch_reader_artifact.rs:24-185`), per `refactors/pending/runtime/react.md:252-253`.
- `dropped`: `__refetch__<n>.ts` and `__refetch__query_text__<n>.ts` with `nestedRefetchQueries` (`isograph/crates/artifact_content/src/imperatively_loaded_fields.rs:21-178`), per `refactors/pending/runtime/react.md:253`.
- `dropped`: `<Abstract>/as<Member>/resolver_reader.ts` refinement readers (`isograph/crates/artifact_content/src/generate_artifacts.rs:165-173`); refinements become a data condition in `refactors/pending/runtime/reader-ast.md:209-257`, Later in `artifact-set-and-types.md:1458`.
- `dropped`: persisted documents `persisted_documents.json` with md5/sha256 ids (`isograph/crates/artifact_content/src/persisted_documents.rs:17-54`, `operation_text.rs:17-80`); "i2 has no persisted operations" (`refactors/pending/runtime/react.md:262`), Later in `artifact-set-and-types.md:1459`.
- `dropped`: fixture snapshot binary `generate_isograph_fixtures` and its CI diff check (`isograph/crates/isograph_fixture_tests/src/main.rs:13-145`); AGENTS.md "No snapshot tests" and "Fixtures are inputs only".

## Config

### Unplanned gaps

- `different`: `project_root`, one directory (`isograph/crates/isograph_config/src/compilation_options.rs:125`); i2 has `source_files`, a required glob list with `!` excludes (`crates/isograph_config/src/compilation_options.rs:14-15`).
- `different`: `artifact_directory` defaults to `project_root` and is created if missing (`isograph/crates/isograph_config/src/compilation_options.rs:126-128,162-173`); i2's `output_directory` is optional and nothing is written when it is absent (`crates/isograph_config/src/compilation_options.rs:19-23`).
- `different`: `schema`, one required path, and `schema_extensions` (`isograph/crates/isograph_config/src/compilation_options.rs:129-133`); i2 has one `schema` glob list defaulting to `[]` and no extensions key (`crates/isograph_config/src/compilation_options.rs:16-18`).
- `missing`: `options.on_invalid_id_type` (`ignore|warn|error`, default `error`, `isograph/crates/isograph_config/src/compilation_options.rs:233,254-262`).
- `missing`: `options.no_babel_transform` (`compilation_options.rs:235`); tradeoffs discussed in `refactors/pending/auto-export-fields.md:164,318` without a decision.
- `missing`: `options.include_file_extensions_in_import_statements` (`compilation_options.rs:238`).
- `different`: `options.module` (`commonjs|esmodule`, `compilation_options.rs:242,264-270`); i2 has no option and both transforms pick import vs require from the file's source type (`refactors/past/iso-transforms.md:37`).
- `missing`: `options.generated_file_header` (`compilation_options.rs:244,304-311`).
- `missing`: `options.open_telemetry.{enable_tracing, collector_endpoint, service_name}` (`compilation_options.rs:288-292,376-379`, used at `isograph/crates/isograph_cli/src/main.rs:107-130`); i2 declares otel workspace dependencies (`Cargo.toml`) that no crate uses.
- `missing`: JSON Schema for the config: `build_json_schema` bin and published `isograph-config-schema.json` (`isograph/crates/isograph_config/src/main.rs`, `isograph/libs/isograph-compiler/isograph-config-schema.json`); i2 keeps `$schema` but ships no schema (`refactors/past/schema-files.md:47`).
- `different`: upstream configs are rejected by i2: both use `deny_unknown_fields` (`crates/isograph_config/src/compilation_options.rs:9`), so `project_root` or `options` is an error.
- `different`: discovery: upstream reads `./isograph.config.json` in the cwd only (`isograph/crates/isograph_cli/src/main.rs:35-37`), though its help text claims a `package.json` key search (`isograph/crates/isograph_cli/src/opt.rs:26-27`); i2 walks up for `.json`, `.js`, `.ts` (`crates/isograph_cli/src/discover.rs:13-17`).
- `different`: config errors panic upstream (`isograph/crates/isograph_config/src/compilation_options.rs:144-224`); i2 returns typed `DiscoverError`/`LoadError` and a nonzero exit (`crates/isograph_cli/src/discover.rs:39`).

### Dropped

- `dropped`: `options.persisted_documents.{file, algorithm, include_extra_info}` (`isograph/crates/isograph_config/src/compilation_options.rs:277-301`), with persisted documents (Compiler and artifacts).

## CLI

### Unplanned gaps

- `different`: a bare invocation compiles upstream (`isograph/crates/isograph_cli/src/opt.rs:5-12`); in i2 it starts the daemon (`crates/isograph_cli/src/lib.rs:50-53`).
- `missing`: `compile --watch` (`isograph/crates/isograph_cli/src/opt.rs:21-33`); i2 has no compile verb; the daemon watches.
- `different`: `lsp` serves in-process and writes no artifacts upstream (`isograph/crates/isograph_cli/src/main.rs:64-89`); i2's `lsp` is a stdio proxy to the per-config daemon, starting it if needed (`crates/isograph_cli/src/lsp_stdio.rs:40-70`).
- `different`: `--log-level` on `compile` and `lsp` plus `RUST_LOG` (`isograph/crates/isograph_cli/src/main.rs:91-105`); i2 has `logs --level` for the terminal and the `LOG_LEVEL` env var for the daemon (freddie_cli `verb.rs:94-98`).
- `missing`: exit code 1 on compile errors (`isograph/crates/isograph_cli/src/main.rs:57-61`); arrives with `isograph errors` (`refactors/pending/compiler-errors.md:28`), so also `planned`.
- `different`: Linux release binaries are static musl upstream (`isograph/.github/workflows/ci.yml:19-54`); i2 builds glibc `*-linux-gnu` (`.github/workflows/ci.yml:23-67`, `refactors/past/cli-ci-build.md:17,101`).

### Dropped

- `dropped`: `@isograph/compiler` npm package (bin `iso`, platform switch; `isograph/libs/isograph-compiler/package.json`, `index.js:6-19`, `cli.js:1-23`); removed in commit `180ec76fc`.

## LSP and editor

### Unplanned gaps

- `missing`: `isograph lsp --log-level` (`isograph/crates/isograph_cli/src/opt.rs:37-45`); i2 `LspArgs` has only `--config` and hidden `--stdio` (`crates/isograph_cli/src/lib.rs:82-90`).
- `different`: upstream server exits on config change (`isograph/crates/isograph_lsp/src/server.rs:174-194`); i2 ignores it (`crates/isograph_cli/src/watch.rs:169`).
- `different`: semantic token mapping (`isograph/crates/isograph_lang_types/src/semantic_token_legend/mod.rs`); i2 maps selections to METHOD (upstream PROPERTY), argument names and object keys to TYPE_PARAMETER (PARAMETER, PROPERTY), variables to FUNCTION (VARIABLE), descriptions to STRING (COMMENT) (`crates/isograph_lsp/src/semantic_tokens.rs:318-341`); no decision is recorded for the change.
- `different`: hover content: upstream shows "Server/Client field", arguments with types, description, parent name and description (`isograph/crates/isograph_lsp/src/hover.rs:45-175,317-364`); i2 shows "field **P.f**: **Target**" plus description, and entity hover has no description (`crates/isograph_lsp/src/hover.rs:15-78`, `crates/isograph_compiler/src/schema.rs:20-24`).
- `different`: i2 hover on a node that names no schema item returns the node's `strum::Display` name (`crates/isograph_lsp/src/hover.rs:48`); upstream returns nothing.
- `partial`: go-to-definition on `to T` (`isograph/crates/isograph_lsp/src/goto_definition.rs:211-214`); i2 returns `None` for `NamedTypeAnnotation` (`crates/isograph_lsp/src/entity_or_selectable.rs:104`).
- `different`: completion: upstream triggers on newline and inserts `name {\n}` for object selectables (`isograph/crates/isograph_lsp/src/completion.rs:92-119`); i2 triggers on letters and `_`, sets no `insert_text`, and excludes already-selected fields (`crates/isograph_lsp/src/completion.rs:66-157`).
- `different`: document highlight: upstream highlights the declaration of the name under the cursor (`isograph/crates/isograph_lsp/src/document_highlight.rs:38-124`); i2 highlights the semantic token under the cursor, punctuation included (`crates/isograph_lsp/src/document_highlight.rs:20-50`).
- `different`: formatting: upstream formats every extracted literal (`isograph/crates/isograph_lsp/src/format.rs:22-58`); i2 skips literals with parse errors (`crates/isograph_cli/src/adapter.rs:403`).
- `missing`: code action "Create new field/pointer `T.f`" as `.ts` or `@component .tsx` (`isograph/crates/isograph_lsp/src/code_action.rs:21-238`); "That plan is not written" (`refactors/pending/auto-export-fields.md:478`).
- `missing`: `workspace/executeCommand` `iso_open_file` sending `window/showDocument` (`isograph/crates/isograph_lsp/src/commands.rs:19-111`); `refactors/past/lsp-dispatch.md:162` defers it.
- `different`: diagnostics: upstream validates the whole project 100ms after any change and publishes for every file (`isograph/crates/isograph_lsp/src/diagnostic_notification.rs:15-101`); i2 publishes for the client's editor buffers only, with severity and source (`crates/isograph_cli/src/diagnostics.rs:41-127`), by decision in `refactors/past/lsp-diagnostics.md:7,19`; project-wide errors are `planned` in `refactors/pending/compiler-errors.md`.
- `different`: VS Code binary discovery: upstream searches `node_modules/@isograph/compiler` first with semver detection (`isograph/vscode-extension/src/utils/findIsographBinary.ts:15-232`); i2 checks `pathToIsograph` first and still falls back to a package i2 does not publish (`vscode-extension/src/utils/findIsographBinary.ts:130-143`), so users must set `pathToIsograph`.
- `different`: `autoformatIsoLiterals`: upstream's client sends `textDocument/formatting` on will-save (`isograph/vscode-extension/src/languageClient.ts:78-102`); i2's server answers `willSaveWaitUntil` and the client middleware returns `[]` when off (`vscode-extension/src/languageClient.ts:45-50`).
- `different`: `markdown.isTrusted` dropped (`isograph/vscode-extension/src/languageClient.ts:42-44`); the stated reason "no hover markdown" (`refactors/past/vscode-extension.md:413`) is stale, since i2 hovers are Markdown; command links in hovers do not work.
- `missing`: `build-local` (`vsce package`) script (`isograph/vscode-extension/package.json:76`), although `refactors/past/vscode-extension.md:149` lists it.

### Planned

- `planned`: multi-config workspaces; neither side supports them; `refactors/pending/vscode-config-discovery.md`.

### Dropped

- `dropped`: `GraphQL.vscode-graphql-syntax` extension dependency (`isograph/vscode-extension/package.json:23-25`); "Drop it" (`refactors/past/vscode-extension.md:35`).
- `dropped`: semver detection of the found binary (`isograph/vscode-extension/src/utils/findIsographBinary.ts`); "Semver is not a reader" (`refactors/past/vscode-extension.md:87`).
- `dropped`: `initializationFailedHandler` returning `true` (`isograph/vscode-extension/src/languageClient.ts:56-62`), per `refactors/past/vscode-extension.md:414`.
- `dropped`: tree-sitter grammar and bindings (`isograph/tree-sitter/grammar.js:10-134`); AGENTS.md "We do not add tree-sitter, a tree-sitter grammar, or a tree-sitter crate."

## Transforms (babel/swc)

### Unplanned gaps

- `different`: babel config lookup is byte-identical to upstream (cosmiconfig, `isograph.config.json` only, `libs/isograph-babel-plugin/BabelPluginIsograph.js`), while the i2 CLI also accepts `isograph.config.js` and `.ts` (`crates/isograph_cli/src/discover.rs:13-17`), so the plugin cannot find a JS or TS config.
- `different`: babel artifact directory is `artifact_directory ?? project_root` upstream (`isograph/libs/isograph-babel-plugin/compileTag.js:118-121`); i2 is `output_directory ?? '.'` (`libs/isograph-babel-plugin/compileTag.js:138-141`).
- `different`: import vs require chosen by `options.module` upstream (`isograph/libs/isograph-babel-plugin/compileTag.js:122,143-165`, `isograph/crates/swc_isograph_plugin/src/lib.rs:317-340`); i2 chooses by the file's source type in both (`refactors/past/iso-transforms.md:37`).
- `different`: swc plugin `config` option is the full `IsographProjectConfig` with `deny_unknown_fields` upstream (`isograph/crates/swc_isograph_plugin/src/lib.rs:33-43`); i2 takes `{ output_directory }` and ignores other keys (`crates/swc_isograph_plugin/src/lib.rs:32-51`).
- `different`: swc plugin panics on bad options upstream (`isograph/crates/swc_isograph_plugin/src/lib.rs:50-55`); i2 reports through `HANDLER` and leaves the program unchanged (`crates/swc_isograph_plugin/src/lib.rs:58-107`).
- `different`: swc ABI: upstream builds against `swc_core = "3.0.0"` in the main workspace (`isograph/Cargo.toml:46-52`); i2 against `swc_core = "84.2.0"` in its own workspace (`crates/swc_isograph_plugin/Cargo.toml:13-14`), so the two load in different `@swc/core` and Next.js ranges.
- `missing`: swc tracing (`swc_trace`, `debug!`, `isograph/crates/swc_isograph_plugin/src/lib.rs:24-28,59`).
- `missing`: npm publishing of `@isograph/babel-plugin` and `@isograph/plugin-swc` (`isograph/.github/workflows/ci.yml:444,525`); both packages are non-private in i2 but no job publishes them.

### Dropped

- `dropped`: babel dependencies `babel-plugin-macros` and `graphql` (`isograph/libs/isograph-babel-plugin/package.json:25,27`); nothing reads them (`libs/isograph-babel-plugin/package.json:22-25`).
- `dropped`: `@swc/counter` dependency (`isograph/libs/isograph-swc-plugin/package.json:27-29`); "which nothing reads" (`refactors/past/iso-transforms.md:11`).

## Runtime

i2's runtime today is `libs/isograph-runtime`: normalization AST types only, `types`-only export, private (`libs/isograph-runtime/src/artifacts.ts:1-58`, `package.json`). No runtime behavior exists.

### Unplanned gaps

- `different`: `ArgumentValue` has an `Enum` kind upstream (`isograph/libs/isograph-react/src/core/util.ts:13-33`); i2 has none and `Literal` is `number | boolean | null` (`libs/isograph-runtime/src/artifacts.ts:54-58`).
- `different`: normalization AST code lags its plan: the code's condition is `{ responseKey, value }` with no `root` (`libs/isograph-runtime/src/artifacts.ts:2-7,44-49`); `refactors/pending/runtime/store.md:270-272` plans `root` and `{ kind, entityName }`.

### Planned

- `planned`: store and `normalizeData` (`isograph/libs/isograph-react/src/core/cache.ts:77`) as `normalize` into a `Map` (`refactors/pending/runtime/store.md:64,276-318`); differs by throwing on any response/AST mismatch, including an absent key (`store.md:317`).
- `planned`: `check` and `CheckResult` (`isograph/libs/isograph-react/src/core/check.ts:34-70`), `store.md:320-346`.
- `planned`: mark-and-sweep `garbageCollectEnvironment` (`isograph/libs/isograph-react/src/core/garbageCollection.ts:65-226`), `store.md:348-381`.
- `planned`: `retainQuery`, `unretainQuery`, `RetainedQuery` (`garbageCollection.ts:20-63`), `refactors/pending/runtime/react.md:231`; `unretainQuery` returns nothing.
- `planned`: `readButDoNotEvaluate` (`isograph/libs/isograph-react/src/core/read.ts:60-143`) as `read` returning `ReadResult` with `MissingData` instead of throwing (`refactors/pending/runtime/reader-ast.md`, `react.md:280`).
- `planned`: `subscribe`, `callSubscriptions` (`isograph/libs/isograph-react/src/core/subscribe.ts:15-109`) as pico-runtime Step A (`refactors/pending/runtime/react.md:1594-1678`); differs: every subscription re-reads on every write (upstream filters by touched records, `subscribe.ts:74-80`), and re-read errors are rethrown (upstream logs and swallows, `subscribe.ts:47-61`).
- `planned`: suspend until change (`isograph/libs/isograph-react/src/core/read.ts:92-122`); differs: i2 resumes at the first write after which the read is no longer `MissingData` (`refactors/pending/runtime/react.md:1378`).
- `planned`: `IsographEntrypoint` type (`isograph/libs/isograph-react/src/core/entrypoint.ts:77-93`) with two type parameters (`refactors/pending/runtime/react.md:246,460-530`).
- `planned`: `PromiseWrapper` family (`isograph/libs/isograph-react/src/core/PromiseWrapper.ts:1-90`), kept with `unknown` errors (`refactors/pending/runtime/react.md:232`).
- `planned`: reader AST `Scalar`, `Linked`, `Resolver` nodes (`isograph/libs/isograph-react/src/core/reader.ts:78-159`), `refactors/pending/runtime/reader-ast.md:209-257`.

### Dropped

- `dropped`: `createIsographStore`, `ROOT_ID` (`isograph/libs/isograph-react/src/core/IsographEnvironment.ts:163,211`); the store is `new Map()` (`refactors/pending/runtime/react.md:256`).
- `dropped`: store layers and `optimisticNetworkResponse` (`isograph/libs/isograph-react/src/core/optimisticProxy.ts:100-501`); "No store layers" (`refactors/pending/runtime/react.md:275`).
- `dropped`: GC release buffer (`gcBuffer`, size 10, `isograph/libs/isograph-react/src/core/IsographEnvironment.ts:177`); "No release buffer" (`refactors/pending/runtime/react.md:276`, open decision 3).
- `dropped`: `missingFieldHandler` (`isograph/libs/isograph-react/src/core/read.ts:781-783`), Later in `refactors/pending/runtime/store.md:776`.
- `dropped`: log events: 15 `LogMessage` kinds, `logMessage`, `registerLogger` (`isograph/libs/isograph-react/src/core/logging.ts:31-112`); "No log events" (`refactors/pending/runtime/store.md:408`).
- `dropped`: `writeData` (`isograph/libs/isograph-react/src/core/writeData.ts:23-79`); "no caller in v1. Later." (`refactors/pending/runtime/react.md:266`).
- `dropped`: `startUpdate` and `@updatable` proxies (`isograph/libs/isograph-react/src/core/startUpdate.ts:41-100`), `refactors/pending/runtime/react.md:255`.
- `dropped`: `EncounteredIds`, record subscriptions (`AnyRecords`, `AnyChangesToRecord`) (`isograph/libs/isograph-react/src/core/IsographEnvironment.ts:41-68`), `refactors/pending/runtime/react.md:258-259`.
- `dropped`: `__link` reader node and `Link`, client pointer reads (`isograph/libs/isograph-react/src/core/reader.ts`), `refactors/pending/runtime/reader-ast.md:259-276`, `react.md:254`.
- `dropped`: component cache and `ComponentReaderArtifact` (`isograph/libs/isograph-react/src/core/componentCache.ts:7-27`), `refactors/pending/runtime/react.md:250`; options explored only in `components-in-userland.md:3`.
- `dropped`: refetch queries and `ReaderWithRefetchQueries` (`isograph/libs/isograph-react/src/core/entrypoint.ts:11-36`, `read.ts:1001-1065`), `refactors/pending/runtime/react.md:253`.
- `dropped`: lazily loaded entrypoints and readers (`getOrLoadIsographArtifact`, `isograph/libs/isograph-react/src/core/IsographEnvironment.ts:241-294`), `refactors/pending/runtime/react.md:251`.
- `dropped`: persisted operation types (`isograph/libs/isograph-react/src/core/entrypoint.ts:64-74`), `refactors/pending/runtime/react.md:262`.

## React

No React package exists in i2 (`libs/` holds only `isograph-babel-plugin`, `isograph-runtime`, `isograph-swc-plugin`). `refactors/pending/runtime/react.md` specifies one and is not approved.

### Unplanned gaps

- `different`: the network function returns `Promise<any>` with the GraphQL envelope and `makeNetworkRequest` checks `errors` upstream (`isograph/libs/isograph-react/src/core/makeNetworkRequest.ts:224-231`); the i2 plan resolves to the data object and the user's function owns `errors` (`refactors/pending/runtime/react.md:646-652`).
- `different`: `useReadAndSubscribe` reads once with deps `[]` upstream (`isograph/libs/isograph-react/src/react/useReadAndSubscribe.ts:25-43`); the i2 plan re-reads when the fragment reference changes (`refactors/pending/runtime/react.md:1495`).
- `different`: fragment reference stable id is `/`-joined text upstream (`isograph/libs/isograph-react/src/core/FragmentReference.ts:35-79`); the i2 plan uses stable JSON of a list (`refactors/pending/runtime/react.md:1001-1047`).
- `different`: `onComplete` runs after disposal upstream (`isograph/libs/isograph-react/src/core/makeNetworkRequest.ts`); the i2 plan runs it only when the response was normalized (`refactors/pending/runtime/react.md:995`).

### Planned

- `planned`: `createIsographEnvironment` (`isograph/libs/isograph-react/src/react/createIsographEnvironment.ts:10-23`) with store and network function only (`refactors/pending/runtime/react.md:217`).
- `planned`: `IsographEnvironmentProvider`, `useIsographEnvironment` (`isograph/libs/isograph-react/src/react/IsographEnvironmentProvider.tsx:6-34`), `react.md:216`.
- `planned`: `useLazyReference` (`isograph/libs/isograph-react/src/react/useLazyReference.ts:19-60`), `react.md:1071-1176`.
- `planned`: `useImperativeReference` (`isograph/libs/isograph-react/src/react/useImperativeReference.ts:25-100`), `react.md:1178-1246`.
- `planned`: `useResult`, eager case only (`isograph/libs/isograph-react/src/react/useResult.ts:16-67`), `react.md:1250-1296`.
- `planned`: `FragmentReader` (`isograph/libs/isograph-react/src/react/FragmentReader.tsx:10-29`), `react.md:1498-1525`.
- `planned`: `makeNetworkRequest`, request dedup by query text and variables (`isograph/libs/isograph-react/src/core/makeNetworkRequest.ts:46-340`, `getOrCreateCacheForArtifact.ts`), `react.md:798-999,1123-1176`.
- `planned`: `shouldFetch` `Yes|No|IfNecessary` and `FetchOptions` (`isograph/libs/isograph-react/src/core/check.ts:13-27`), `react.md:751-766`.
- `planned`: `NetworkRequestReaderOptions` (`suspendIfInFlight`, `throwOnNetworkError`; `isograph/libs/isograph-react/src/core/read.ts:972-984`), `react.md:229`.
- `planned`: `useRerenderOnChange` (`isograph/libs/isograph-react/src/react/useRerenderOnChange.ts:10-38`), `react.md:1462-1490`.
- `planned`: `@isograph/disposable-types` (`isograph/libs/isograph-disposable-types/src/index.ts:1-3`) and `@isograph/react-disposable-state` (`isograph/libs/isograph-react-disposable-state/src/index.ts:1-9`) as ports, `react.md:1696-1710`.

### Dropped

- `dropped`: `FragmentRenderer` (`isograph/libs/isograph-react/src/react/FragmentRenderer.tsx:11-17`), `refactors/pending/runtime/react.md:250`.
- `dropped`: `LoadableFieldReader`, `LoadableFieldRenderer`, `useClientSideDefer`, `useImperativeLoadableField` (`isograph/libs/isograph-react/src/loadable-hooks/`), `react.md:251`.
- `dropped`: `useSkipLimitPagination`, `useConnectionSpecPagination` (`isograph/libs/isograph-react/src/loadable-hooks/useSkipLimitPagination.ts:26-79`, `useConnectionSpecPagination.ts:26-90`), `react.md:251`.
- `dropped`: `useImperativeExposedMutationField` (`isograph/libs/isograph-react/src/loadable-hooks/useImperativeExposedMutationField.ts:1-8`), `react.md:252`.
- `dropped`: `useSubscribeToMultiple` (`isograph/libs/isograph-react/src/react/useReadAndSubscribe.ts:45`), `react.md:265`.
- `dropped`: `RenderAfterCommit__DO_NOT_USE` (`isograph/libs/isograph-react/src/react/RenderAfterCommit__DO_NOT_USE.tsx:9`), `react.md:267`.
- `dropped`: `createIsographEnvironment`'s `missingFieldHandler`, `logFunction`, `componentFunction` parameters, `react.md:217`.
- `dropped`: `useDisposableState` (`isograph/libs/isograph-react-disposable-state/src/useDisposableState.ts:16`), `react.md:1691,1706`.
- `dropped`: `@isograph/reference-counted-pointer` (`isograph/libs/isograph-reference-counted-pointer/src/createReferenceCountedPointer.ts:27-35`); only the pagination hooks use it (`react.md:372`).

## Tooling, CI, distribution

### Unplanned gaps

- `partial`: demo typecheck plus eslint for three demos (`isograph/.github/workflows/ci.yml:88-119`); i2 typechecks the pet demo only, no eslint (`.github/workflows/js.yml:37-38`).
- `different`: `cargo fmt` runs fmt then checks git status, edition 2021 (`isograph/.github/workflows/ci.yml:218-231`, `isograph/rustfmt.toml:1`); i2 runs `--check` with edition 2024 (`scripts/check-rustfmt.sh`, `rustfmt.toml`).
- `missing`: npm OIDC trusted publishing (`isograph/.github/workflows/ci.yml:374,442`).
- `missing`: root scripts `test`, `tsc`, `coverage`, demo `dev-*`/`build-*` (`isograph/package.json:20-53`); i2 `package.json:14-29`. `watch-libs` and `compile-libs` landed in commit `c11feea1b`.
- `partial`: `sanity-check` runs format, clippy, build-demos, git status, compile-libs, test (`isograph/scripts/sanity-check.sh:1-6`); i2 runs format, clippy, git status (`scripts/sanity-check.sh:1-3`).
- `different`: website deploy gate is identical (`.github/workflows/ci.yml:105-109`), so a push to i2 `main` would deploy i2's docs, mostly stale upstream copies, to isograph.dev.
- `partial`: pet demo: upstream is a Next.js app with a graphql-yoga backend, swc plugin, 10 entrypoints, `@component`, `@loadable`, `@updatable`, pagination, `__refetch`, `asT`, pointers (`isograph/demos/pet-demo/`); i2's is 10 `.ts` files with no React, server or bundler that logs entrypoint artifacts (`demos/pet-demo/scripts/log-entrypoints.cjs`); rendering is planned in `refactors/pending/runtime/react.md:20`.
- `missing`: `libs/isograph-react` as a compiler target for its own fixtures (`isograph/libs/isograph-react/isograph.config.json`).
- `different`: stale i2 references to removed upstream distribution: `refactors/past/cli-ci-build.md:5,184` says release drops the binary into `libs/isograph-compiler`; `refactors/pending/zed-and-vscode-extensions.md:103-121` and `blacksmith-ci.md:39-186,239` assume `@isograph/compiler` and jobs i2 lacks; `demos/pet-demo/README.md:67` lists `@component`/`@loadable` the literals no longer use.

### Planned

- `planned`: upstream vitest suites for react, disposable state (`isograph/libs/isograph-react/src/tests/*.test.ts`); ported with the packages in `refactors/pending/runtime/react.md:1696-1710`.

### Dropped

- `dropped`: config JSON schema up-to-date check (`isograph/.github/workflows/ci.yml:82-86`); removed in commits `25604bd5e`, `552db565b`.
- `dropped`: fixture up-to-date check (`isograph/.github/workflows/ci.yml:259-264`); AGENTS.md "No snapshot tests".
- `dropped`: tsdown CJS/ESM/dts build of JS packages and root vitest (`isograph/.github/workflows/ci.yml:121-153`, `tsdown.config.ts`, `vitest.workspace.ts`); commits `25604bd5e`, `552db565b`.
- `dropped`: npm canary on `main`, `deploy/*` dist-tag, and `latest` on `v*` tags (`isograph/.github/workflows/ci.yml:352-525`, `gulpfile.js:4-57`); commit `25604bd5e`; no replacement plan.
- `dropped`: `.turbo` CI cache (`isograph/.github/workflows/ci.yml:99-105`); commit `552db565b`. The turbo task graph (`isograph/turbo.json`) landed in commit `c11feea1b`.
- `dropped`: `patches/@docusaurus__tsconfig.patch` (`isograph/pnpm-workspace.yaml:21-22`); replaced by an inlined tsconfig (`docs-website/tsconfig.json:3`), commit `2e575a7f4`.
- `dropped`: `cross`, musl and the `cross` script (`isograph/package.json:20-27`), `refactors/past/cli-ci-build.md:17,101`.
- `dropped`: `github-demo` (Next.js, live GitHub API, `useConnectionSpecPagination`, S3/CloudFront deploy; `isograph/demos/github-demo/`), commit `180ec76fc`.
- `dropped`: `vite-demo` (Vite, Pokemon API, `module: "esmodule"`; `isograph/demos/vite-demo/`), commit `180ec76fc`; `refactors/pending/runtime/react.md:1821-1852` reuses its shape.
- `dropped`: `disposable-state-ajax-demo` (`isograph/demos/disposable-state-ajax-demo/`), commit `180ec76fc`.
- `dropped`: bacon jobs `watch-libs` and `build-swc` (`isograph/bacon.toml:27-31,130-138`) with the libs they built.

## Docs

Every page in `docs-website/docs/` that upstream also has, except `development-workflow.md`, is byte-identical to upstream, and so is every blog post; `design-docs/mental-model.md`, `pico.md`, `event-model.md` are i2's own. Each copy below documents behavior i2 does not have.

### Unplanned gaps

- `different`: `development-workflow.md` is edited for i2 but lines 42-46 still describe `yarn run iso` and `@isograph/compiler`, line 54 says tests are not run in CI (false in i2, `.github/workflows/rust.yml:46-57`), and lines 122-131 describe npm releases i2 does not perform.
- `missing`: `quickstart.md` documents npm install of the compiler, babel plugin and react (lines 35-37), `project_root` (line 48) and `yarn iso --watch` (lines 40, 115); none exists in i2.
- `missing`: `workflow.md` documents `yarn iso --watch --config` (line 5).
- `missing`: `isograph-config.md` documents `project_root`, `artifact_directory`, `schema_extensions`, `options.*` (lines 12-29); i2 has `source_files`, `schema`, `output_directory` (`crates/isograph_config/src/compilation_options.rs:10-24`).
- `missing`: `index.md` documents resolvers, React fetching and `@exposeField` (line 93).
- `missing`: `faq.md` documents `on_invalid_id_type` and `@component` (lines 3-43).
- `missing`: `isograph-rules.md` documents `@component` keys, loadable fields, re-exposed mutation fields (lines 83, 125-169).
- `missing`: `troubleshooting.md` documents tree-sitter install failures (lines 5-13).
- `missing`: `how-isograph-works/compiler.md` documents batch and `--watch` modes and upstream's crate list (lines 9-53).
- `partial`: `how-isograph-works/generated-artifacts.md` documents readers, typed entrypoints and `__refetch__N` (line 39); i2 generates stubs for readers and normalization ASTs.
- `partial`: `how-isograph-works/babel-plugin.md`; the plugin exists in i2 but is not on npm.
- `missing`: `how-isograph-works/one-pager.md` and `runtime.md` document a runtime i2 does not have.
- `partial`: `parameters.md`; variables and defaults work in i2, `@component` and loadable (line 36) do not.
- `missing`: `abstract-types.md` documents `asConcreteType` reads (lines 3-19); i2 has refinements in queries only.
- `missing`: `conditional-fetching.md` documents `shouldFetch` (line 17); planned in `refactors/pending/runtime/react.md`, not implemented.
- `missing`: `design-docs/incremental-compilation.md` and `isograph-data-model.md` describe upstream's compiler, not i2's.
- `missing`: no i2 blog post or release notes; `docs-website/blog/*` are upstream copies.

### Dropped

- `dropped`: `loadable-fields.md` (`@loadable`, `useClientSideDefer`, `useImperativeLoadableField`), per `refactors/pending/iso-directives-unexpected.md:9`.
- `dropped`: `pagination.md` (both pagination hooks), per `refactors/pending/runtime/react.md:251`.
- `dropped`: `mutations.md` (`@component` mutation fields, line 41), per `react.md:250`.
- `dropped`: `local-updates.md` (`@updatable`, `startUpdate`, line 35), per `react.md:255`.
- `dropped`: `external-data-sources.md` (`writeData`, lines 13-14), per `react.md:266`.
- `dropped`: `data-driven-dependencies.md` (`@loadable`, pagination, `asConcreteType` together, line 5), per `iso-directives-unexpected.md:9` and `react.md:251`.
- `dropped`: `expose-field-directives.md` and `refetching.md` (both under "Deprecated features" upstream), per `react.md:252-253`.

## i2 has, upstream lacks

- `entity Name` declarations of source-defined entities that `to Name` can target (`crates/isograph_parser/src/parse_iso_literal.rs:31-37`, `docs-website/docs/design-docs/mental-model.md:103-115`).
- A recovering iso parser with partial ASTs and several errors per literal (`crates/isograph_parser/src/parse_iso_literal.rs:134-165`).
- `extend` for every SDL type kind and `extend schema`, merged, with `ExtensionWithoutDefinition` and `ExtensionKindMismatch` (`crates/graphql_network_protocol/src/protocol_data.rs:130-150,419-445`).
- An SDL parser that recovers per definition and unescapes strings, including `\u{...}` (`crates/graphql_schema_parser/src/parse_error.rs:5-17`).
- Schema validation upstream does not do: `RefinementCollision`, `DuplicateInputField`, `MissingQuery` (`crates/graphql_network_protocol/src/protocol_data.rs:183-225`, `root_operation_types.rs:61`).
- Merge errors upstream has no equivalent for: `InlineCycle`, `DuplicateArgument`, `DuplicateObjectKey`, `ArgumentsOnRefinement`, `AliasCollision` and others (`crates/isograph_compiler/src/merged_selection.rs:129-232`), though log-only.
- One daemon per config serving many editor windows, with lifecycle verbs `start`, `restart`, `status`, `logs`, `stop`, and `config-path` (`crates/isograph_cli/src/lib.rs:64-79`, `crates/isograph_cli/src/config_path.rs:7-18`).
- `textDocument/references` (`crates/isograph_lsp/src/references.rs:38-50`) and server-side `willSaveWaitUntil` (`crates/isograph_cli/src/lsp_socket.rs:249`).
- UTF-16-correct LSP positions (`crates/isograph_lsp/src/line_index.rs`); upstream counts bytes (`isograph/crates/isograph_lsp/src/format.rs:73-91`).
- Hover ranges; completion that excludes already-selected fields (`crates/isograph_lsp/src/completion.rs:139`).
- Glob-based `source_files` and `schema`, and JS/TS configs (`crates/isograph_config/src/compilation_options.rs:14-18`, `crates/isograph_cli/src/discover.rs:225-270`).
- Artifact writes that keep unchanged files' mtimes and prune empty folders (`crates/isograph_cli/src/write_artifacts.rs:57-87`).
- `query_text.graphql` beside `query_text.ts` (`crates/graphql_network_protocol/src/query_text.rs:9-20`).
- `iso(\`entity X\`)` accepted by both transforms and the `iso.ts` overloads (`libs/isograph-babel-plugin/compileTag.js:21-29`, `crates/swc_isograph_plugin/src/lib.rs:392-397`, `crates/typescript_artifacts/src/iso_overloads.rs:45`).
- swc plugin dedupes entrypoint imports by path (`crates/swc_isograph_plugin/src/lib.rs:365-371`); upstream pushes one per literal (`isograph/crates/swc_isograph_plugin/src/lib.rs:325`).
- A `CompilationProfile` seam over host language and network protocol (`crates/ts_graphql_react_isograph_cli/src/main.rs:14-21`).
- CI: e2e tests against each release binary (`.github/workflows/build-cli.yml:53-75`), clippy `-D warnings` (`.github/workflows/rust.yml:20-44`), a full swc plugin job with the wasm test under `@swc/core` that upstream skips (`rust.yml:59-80`), and a VS Code extension lint job (`ci.yml:69-84`).
- Pre-commit running the full check suite in a dedicated worktree (`.pre-commit-config.yaml:17-89`, `scripts/precommit-in-worktree.sh`).
- Design docs `mental-model.md`, `pico.md`, `event-model.md` (`docs-website/sidebars.ts:44-46`).
