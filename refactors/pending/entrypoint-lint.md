# Entrypoint literal position rule

## Problem

An `` iso(`entrypoint Type.field`) `` literal can be assigned to a variable or exported today, and nothing reports it. The user's requirement is that an entrypoint literal is never assigned to a variable and never exported, and that the rule is easy for every Isograph user to adopt, not only this repo.

The rule's reason is not stated anywhere in i2 or upstream (see Reason). The user states it before implementation, and the docs-website text below quotes it.

## Lint tooling today

- JS/TS lint is oxlint 1.32.0 with `--type-aware` (`package.json:6,22`), config `.oxlintrc.json` (plugins `typescript`, `import`; no custom or JS plugins). It runs in pre-commit (`.pre-commit-config.yaml:52-56`, `--deny-warnings`) and CI (`.github/workflows/js.yml:16-22`).
- Prettier in pre-commit (`.pre-commit-config.yaml:47-51`) and CI (`js.yml:7-14`).
- Rust: clippy `-D warnings` on affected crates (`.pre-commit-config.yaml:27-32`), `clippy.toml` symlinked to `crates/prelude/clippy.toml`, and the `postfix-constructors` test (`.pre-commit-config.yaml:34-39`).
- `vscode-extension/.eslintrc.js` exists but `.oxlintrc.json` ignores `vscode-extension` and no script runs ESLint.
- No ESLint plugin, no custom lint rule, no oxlint `jsPlugins`.

## What restricts entrypoint literals today

Nothing restricts where an entrypoint literal appears, in i2 or upstream.

- i2 Babel plugin: an entrypoint literal is replaced by an import of its artifact wherever it appears (`libs/isograph-babel-plugin/compileTag.js:31-40`, `:134-189`). Only an `entity` literal that is called throws (`:22-26`).
- i2 SWC plugin: same replacement (`crates/swc_isograph_plugin/src/lib.rs:346-377`); errors are `IsographTransformError` (`:163-187`), none about position.
- i2 compiler: the TypeScript host extraction is a regex (`crates/isograph_extract_typescript/src/lib.rs:55`) that captures an `export const NAME =` prefix, `(`, and a following `(`. `literal_errors` (`:116-140`) reports a tagged template, an unexported field, and a field with no function. An entrypoint gets none of these.
- i2's own code assigns and exports entrypoints: `demos/pet-demo/src/HomeRoute.ts:14`, `PetDetailRoute.ts:23`, `SetTagline.ts:16`, `SpeciesRoute.ts:21` export them, and `fetchAndRead.ts:8-11` and `mockNetwork.ts:8-10` import them. Commit `0b2f2bf3f` added the exports. The Babel and SWC plugin tests assign them (`libs/isograph-babel-plugin/BabelPluginIsograph.test.js:115,127,146,157,172`, `libs/isograph-swc-plugin/__test__/wasm.test.ts:33`). The extraction tests extract them (`crates/isograph_extract_typescript/src/lib.rs:311-330`).

## Upstream

Upstream has no entrypoint position rule and no lint plugin.

- No ESLint plugin in `../isograph/libs` (packages: babel-plugin, compiler, disposable-types, react, react-disposable-state, reference-counted-pointer, swc-plugin). The demos carry ordinary ESLint configs (`../isograph/demos/vite-demo/eslint.config.js`, `demos/pet-demo/.eslintrc.json`). Upstream's repo lint is oxlint (`../isograph/.oxlintrc.json`).
- Babel plugin: replaces the entrypoint call anywhere (`../isograph/libs/isograph-babel-plugin/compileTag.js:22-31`); its only usage-shape error is a field or pointer literal called with other than one argument (`:38-40`).
- SWC plugin: errors at `../isograph/crates/swc_isograph_plugin/src/lib.rs:91-106`, none about position.
- Compiler: the extraction regex is the same (`../isograph/crates/isograph_schema/src/validated_isograph_schema/isograph_literals.rs:210`). The entrypoint parse ignores `const_export_name` (`../isograph/crates/isograph_lang_parser/src/parse_iso_literal.rs:58-66`). Fields and pointers must be exported (`:201-205`, `:906-914`).
- Upstream's own code assigns entrypoints: `../isograph/demos/pet-demo/src/components/RestEndpointDemoRoute.tsx:17` (`const entrypoint = iso(...)`, then `typeof entrypoint` at `:21` and `writeData(environment, entrypoint, ...)` at `:27`), and `../isograph/libs/isograph-react/src/tests/meNameSuccessor.ts:18`, `nodeQuery.ts:15`, `garbageCollection.test.ts:56`, `normalizeData.test.ts:116`.
- Upstream docs use an entrypoint literal only as a call argument (`../isograph/docs-website/docs/quickstart.md:348`, `index.md:64`, `mutations.md:49`, `conditional-fetching.md:9`, `external-data-sources.md:26,81,124`).

Upstream's pattern for a rule about the shape of an `iso` call is a compiler diagnostic from the extraction context: parentheses required (`isograph_literals.rs:171-180`), a field must be called with a function (`:192-200`), a field must be exported (`parse_iso_literal.rs:201-205`). The transform plugins throw only on a shape they cannot transform.

## Reason

Neither repo states a reason. The nearest stated fact is `refactors/pending/auto-export-fields.md:160`: "The entrypoint direction is user module imports generated file, so no export is needed." That says an export is unnecessary. It does not say an export is wrong.

Technical consequences checked, none of which is a stated reason:

- An exported entrypoint works. Both plugins replace the literal with an identifier bound to the artifact's default import, so `export const X = iso(...)` becomes `export const X = _Query__HomeRoute`.
- Typed entrypoint generation does not read the export name. `export_name` is read only for fields (`crates/typescript_artifacts/src/lib.rs:72-95`).
- Writing the same entrypoint literal in several files is allowed. `project_artifacts` collects entrypoints from a `BTreeSet<ArtifactSource>` (`crates/isograph_compiler/src/artifact_types.rs:757-765`), and each plugin deduplicates the import per file (`compileTag.js:166-176`, `crates/swc_isograph_plugin/src/lib.rs:358-371`).

The user supplies the reason before implementation.

## The rule

An entrypoint literal is checked at its nearest enclosing syntax, after looking through parentheses, `as`, `satisfies`, `!`, `<T>x`, and `as const`.

Permitted:

```ts
// from docs-website/docs/quickstart.md
const { fragmentReference } = useLazyReference(
  iso(`entrypoint Root.HomePage`),
  {},
);
```

```ts
// from libs/isograph-runtime/fixtures/store/src/Search.ts
iso(`entrypoint Query.Search`);
```

- A call argument or `new` argument (`f(iso(...))`, `f(a, iso(...))`, `f?.(iso(...))`).
- An expression statement (`iso(...);`).
- A JSX attribute value (`<Route entrypoint={iso(...)} />`). Decision 3.
- The object of a property access (`iso(...).queryText`, `iso(...).readerArtifact()`). Decision 2.
- An operand of `?:`, `&&`, `||`, `??`: the check moves to the enclosing expression, so `useLazyReference(c ? iso(a) : iso(b))` is a call argument. Decision 4.
- The callee of a call (`iso(...)(fn)`): this is the field form; an entrypoint literal called this way is not this rule's concern.

Forbidden as assigned:

- `const x = iso(...)`, `let`, `var`, and a destructuring initializer.
- A default parameter or destructuring default (`function f(e = iso(...))`).
- `x = iso(...)`, `obj.p = iso(...)`, `module.exports = iso(...)`, `exports.x = iso(...)`, and compound assignment.
- A class field initializer (`class C { e = iso(...) }`).
- An object property value (`{ home: iso(...) }`) and an array element (`[iso(...)]`). Decision 3.

Forbidden as exported:

- `export const x = iso(...)` (also `export let`, `export var`).
- `export default iso(...)`.

Forbidden in another position:

- `return iso(...)` and an arrow body (`() => iso(...)`). Decision 3.
- Every position not listed above: a template substitution, `await`, unary and binary operands, a spread argument (`f(...iso(...))`), a sequence expression, a JSX child.

Not detected, because the rule reads syntax and not data flow:

- `export const x = identity(iso(...))`. The literal is a call argument.
- An import of the artifact module itself (`import e from './__isograph/Query/HomeRoute/entrypoint.ts'`), which `docs-website/docs/faq.md:21` documents as an alternative to the transform. Decision 5.

Degenerate cases:

- `const x = (iso(...))`, `const x = iso(...) as E`, `const x = iso(...) satisfies E`, `const x = iso(...)!`: assigned.
- `export default (iso(...))`: exported.
- `useLazyReference((iso(...)))`: permitted.
- A literal with a substitution: the extraction regex does not match a literal containing `` ` ``, and a `${}` literal is already a transform error (`compileTag.js:90-96`). No position error is added.
- A tagged template `` iso`entrypoint ...` `` that is assigned: the `MissingParentheses` error and the position error both report. The swc parse sees a `TaggedTpl`, not a `CallExpr`, so the position is classified at the tagged template node.
- A match inside a comment or a string (the regex matches `` /* iso(`entrypoint X.y`) */ ``): the swc tree has no call at that offset, so no position error.
- A file that does not parse: no position error for any literal in it. TypeScript reports the syntax error.
- Two literals on one line, nested calls `f(g(iso(...)))`: each literal is classified at its own parent.
- An empty file, a file with no literals: no parse is run.

## Open decisions

### 1. Where to enforce

- (a) An ESLint rule in a new package `@isograph/eslint-plugin`. ESLint sees the full AST with parents, so the rule is about 60 lines over `CallExpression[callee.name='iso']`. Editor feedback is the ESLint editor extension. Users without ESLint get nothing. oxlint 1.32 runs ESLint-API plugins through `jsPlugins`, which its schema marks experimental, not semver-stable, and "not supported in language server" (`node_modules/oxlint/configuration_schema.json:54-55`), so oxlint users get CLI only and i2's own repo gets no editor feedback. Biome plugins are GritQL, so Biome users get nothing. Cost: a new package, a release, a docs page, and a config step for every user. Breaks nothing at build time.
- (b) A Babel plugin error and an SWC plugin error. Both see the AST (`path.parentPath` in Babel, the `Fold` in `crates/swc_isograph_plugin/src/lib.rs:478-517`). Feedback is a build error only, no editor diagnostic. Users who import artifacts by hand (`faq.md:21`) get nothing. Every user of either plugin gets the rule on upgrade with no config. Cost: two implementations that must agree, and the SWC plugin is a separate workspace (`crates/swc_isograph_plugin/Cargo.toml:13`). Breaks: every existing assigned entrypoint fails the build on upgrade, including i2's pet-demo and both plugins' test suites. Upstream's plugins throw only on shapes they cannot transform; an assigned entrypoint transforms correctly.
- (c) A compiler diagnostic. Every Isograph user runs the compiler, since it generates the artifacts. The diagnostic reaches the editor through the LSP (`crates/isograph_lsp/src/file_diagnostics.rs:15-30`, via `compiler_errors_in_file` at `crates/isograph_compiler/src/compiler_errors.rs:51-80`) and the CLI through `project_errors` (`crates/isograph_cli/src/compile.rs:181-217`). It is independent of the user's linter and bundler, and versioned with the compiler. The extraction regex does not see the surrounding syntax: it sees `export const NAME =` (and misses it with two spaces before `=` or none after, `crates/isograph_extract_typescript/src/lib.rs:317-330`) and nothing else. The compiler therefore needs a JS/TS parse of the host file. `swc_ecma_parser` 47.1.0, `swc_ecma_ast` 30.0.2, `swc_common` 27.0.0, and `swc_ecma_visit` 30.0.0 are already in `Cargo.lock` through `dprint-plugin-typescript` (`cargo tree -i swc_ecma_parser`: `dprint-swc-ext` from `typescript_artifacts`). Cost: a direct dependency on those crates from `isograph_extract_typescript`, a `HostLanguage::extract_iso_literals_from_text` signature change to receive the path, and a parse of each file that has a literal on each change of that file. Breaks: i2's pet-demo and extraction tests change.
- (c') A compiler diagnostic from an extended regex (capture the token before `iso(`). It sees `=`, `(`, `,`, `return`, `export default` and is wrong on comments between tokens, on `(iso(...))` inside `const x =`, and on `c ? iso(a) : iso(b)`. It is the structurally incorrect version of (c).
- (d) (c) plus (a) or (b). The compiler already reaches every user in the editor and the CLI, so a second implementation adds a duplicate report and a second place to keep in agreement.

Held: (c). Upstream puts every `iso` usage-shape rule in the compiler (Upstream, above), and (c) is the only option that is zero-config for every user, independent of their linter, and shown in the editor. It deviates from upstream in one respect: upstream's extraction is regex only, and (c) adds a swc parse for position. The stated reason is that the regex cannot see the syntax the rule is about (c').

The swc parse is a third-party parser for the host language, used only to classify positions. AGENTS.md's parser invariant ("Parsers are tokenize and grammar consume in our crates") is about the iso and schema parsers. The user rules whether a host-language parse falls under it before implementation.

### 2. A property access on the literal

`iso(...).queryText` reads a property and does not assign or export the entrypoint. i2's pet-demo needs it (`fetchAndRead.ts:22-26`, `mockNetwork.ts:27-37`) once the exports are gone. Forbidding it leaves no way to read `name`, `queryText`, `readerArtifact`, or `normalizationAst` without passing the literal to a helper.

Held: permitted.

### 3. Object property, array element, JSX attribute, return

A permitted position must not let the literal be assigned or exported. An object property value or array element can be: `export const routes = { home: iso(...) }`. A return or arrow body hands the value to the caller, which can assign it. A JSX attribute passes the value to a component, as a call argument does.

Held: JSX attribute permitted; object property value, array element, return, and arrow body forbidden. A route table (`{ path: '/', entrypoint: iso(...) }`) is therefore an error. The user confirms.

### 4. Conditional and logical operands

`useLazyReference(c ? iso(a) : iso(b))` is a call argument once the check moves through `?:`. Without that, the user writes two calls.

Held: the check moves through `?:`, `&&`, `||`, `??` to the enclosing expression.

### 5. A direct import of an entrypoint artifact

The rule covers `iso` literals. `faq.md:21` documents importing the artifact directly, and that import is a variable binding the rule does not see. Detecting it means resolving each import specifier against `output_directory`.

Held: not covered. The user confirms.

### 6. Severity

Every compiler diagnostic is an error (`crates/isograph_lsp/src/file_diagnostics.rs:52-60`), and any error blocks artifact generation for the whole project (`crates/isograph_cli/src/compile.rs:56-74`). A host error also suppresses that file's generation errors (`crates/isograph_compiler/src/generation_errors.rs:28-48`). A position error therefore stops artifact writes until the user moves the literal, although the artifacts would be correct. A warning severity does not exist; adding one is a new field on every compiler error and a change to `compile`.

Held: error, as every other host error.

### 7. TypeScript type of an entrypoint

`RestEndpointDemoRoute.tsx:17-22` assigns the literal to write `typeof entrypoint`. With the rule, the type comes from the artifact: `import type Entrypoint from './__isograph/Query/PetNameList/entrypoint'` and `typeof Entrypoint`. That is a type-only import, erased at compile time.

Held: documented as the replacement. The user confirms that a type-only artifact import is acceptable under decision 5.

## Changes

### `TypeScriptLiteralContext` gains the literal's position

```rust
// from crates/isograph_extract_typescript/src/lib.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeScriptLiteralContext {
    pub const_export_name: Option<ConstExportName>,
    pub call: IsoCall,
    pub associated_js_function: AssociatedJsFunction,
}
```

```rust
// from crates/isograph_extract_typescript/src/lib.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeScriptLiteralContext {
    pub const_export_name: Option<ConstExportName>,
    pub call: IsoCall,
    pub associated_js_function: AssociatedJsFunction,
    pub position: LiteralPosition,
}

/// Where an `iso` call sits, as the entrypoint position rule classifies it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LiteralPosition {
    Permitted,
    Forbidden(ForbiddenLiteralPosition),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ForbiddenLiteralPosition {
    Assigned,
    Exported,
    Other,
}
```

`Permitted` also covers a regex match with no `iso` call at that offset in the tree and a file that does not parse. Both produce no error, so no reader distinguishes them.

### `TypeScriptHostError` gains three variants

```rust
// from crates/isograph_extract_typescript/src/lib.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum TypeScriptHostError {
    #[error("You must call the iso function with parentheses. \"iso`...`\" is not supported")]
    MissingParentheses,
    #[error(
        "This isograph field literal must be exported as a named export, for example as `export const {suggested_name}`"
    )]
    MissingExport {
        suggested_name: SelectableNameWrapper,
    },
    #[error("Isograph literals must be immediately called, and passed a function")]
    MissingAssociatedFunction,
    #[error(
        "An entrypoint literal must not be assigned. Pass iso(`entrypoint ...`) directly as an argument where it is used."
    )]
    EntrypointAssigned,
    #[error(
        "An entrypoint literal must not be exported. Write iso(`entrypoint ...`) in each module that uses it."
    )]
    EntrypointExported,
    #[error(
        "An entrypoint literal must be a call argument, a JSX attribute value, the object of a property access, or a statement."
    )]
    EntrypointInOtherPosition,
}
```

### `literal_errors` reports a forbidden entrypoint position

```rust
// from crates/isograph_extract_typescript/src/lib.rs
        let parsed_item = parsed.item.as_ref().and_then(item_of);
        if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
```

```rust
// from crates/isograph_extract_typescript/src/lib.rs
        let parsed_item = parsed.item.as_ref().and_then(item_of);
        if let Some(IsoLiteralItem::Entrypoint(_)) = parsed_item
            && let LiteralPosition::Forbidden(forbidden) = extraction.context.position
        {
            errors.push(
                match forbidden {
                    ForbiddenLiteralPosition::Assigned => TypeScriptHostError::EntrypointAssigned,
                    ForbiddenLiteralPosition::Exported => TypeScriptHostError::EntrypointExported,
                    ForbiddenLiteralPosition::Other => {
                        TypeScriptHostError::EntrypointInOtherPosition
                    }
                }
                .with_span(span),
            );
        }
        if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
```

The error's span is the literal text, as `MissingParentheses` uses.

### `extract_iso_literals_from_text` receives the path

The swc syntax depends on the extension: `.ts` is TypeScript without TSX (so `<T>x` parses), `.tsx` is TypeScript with TSX, `.js` and `.jsx` are ECMAScript with JSX.

```rust
// from crates/isograph_compiler/src/host_language.rs
    fn extract_iso_literals_from_text(contents: &str) -> Vec<IsoLiteralExtraction<Self>>;
```

```rust
// from crates/isograph_compiler/src/host_language.rs
    fn extract_iso_literals_from_text(
        relative_path: &Path,
        contents: &str,
    ) -> Vec<IsoLiteralExtraction<Self>>;
```

```rust
// from crates/isograph_compiler/src/iso_literals.rs
    let contents = file_contents(db, client, path).as_ref()?;
    TProfile::HostLanguage::extract_iso_literals_from_text(Path::new(path.lookup()), contents)
        .wrap_some()
```

`crates/isograph_compiler/src/test_profile.rs:31` and `crates/isograph_cli/src/watch.rs:712` take `_relative_path: &Path` and are otherwise unchanged.

### Position classification

```rust
// from crates/isograph_extract_typescript/src/literal_position.rs
/// The position of every `iso` call in `contents`, keyed by the byte offset of its template's
/// text. Empty when the file does not parse.
pub(crate) fn literal_positions(
    relative_path: &Path,
    contents: &str,
) -> BTreeMap<usize, LiteralPosition>
```

`extract_iso_literals_from_text` calls it once when the regex found at least one match, and sets each extraction's `position` to `positions.get(&literal.start()).copied().unwrap_or(LiteralPosition::Permitted)`.

`literal_positions` parses with `swc_ecma_parser::parse_file_as_program` over a `swc_common::SourceFile` whose start position is recorded, so a span's `lo` minus that start is a byte offset into `contents`. A `swc_ecma_visit::Visit` implementation records, for each parent node kind in The rule, the position of a child that is an `` iso` call after unwrapping `Paren ``, `TsAs`, `TsSatisfies`, `TsNonNull`, `TsTypeAssertion`, and `TsConstAssertion`, and moving through `Cond` and logical `Bin` operands to the enclosing node. `visit_var_decl` inside `visit_export_decl` yields `Exported`. `visit_call_expr` and `visit_tagged_tpl` insert `Forbidden(Other)` with `entry(offset).or_insert(...)`, so a parent's classification, recorded first, wins. The key is the `lo` of the template's first quasi, which equals the regex's `literal` start.

```toml
# from crates/isograph_extract_typescript/Cargo.toml
swc_common = "=27.0.0"
swc_ecma_ast = "=30.0.2"
swc_ecma_parser = "=47.1.0"
swc_ecma_visit = "=30.0.0"
```

The versions are pinned to the ones `dprint-plugin-typescript` already locks, so the workspace compiles one copy.

### pet-demo

```ts
// from demos/pet-demo/src/HomeRoute.ts
export const HomeRouteEntrypoint = iso(`entrypoint Query.HomeRoute`);
```

```ts
// from demos/pet-demo/src/HomeRoute.ts
iso(`entrypoint Query.HomeRoute`);
```

`PetDetailRoute.ts:23`, `SetTagline.ts:16`, and `SpeciesRoute.ts:21` change the same way.

```ts
// from demos/pet-demo/src/fetchAndRead.ts
import { HomeRouteEntrypoint as homeRoute } from './HomeRoute';
import { mockNetworkFunction } from './mockNetwork';
import { PetDetailRouteEntrypoint as petDetailRoute } from './PetDetailRoute';
import { SetTaglineEntrypoint as setTagline } from './SetTagline';
// ...
  const [homeRouteRequest, releaseHomeRoute] = fetchEntrypoint(
    environment,
    homeRoute,
    {},
    null,
  );
  await homeRouteRequest.promise;
  console.log(homeRoute.name, readHomeRoute());
```

```ts
// from demos/pet-demo/src/fetchAndRead.ts
import { iso } from './__isograph/iso';
import { mockNetworkFunction } from './mockNetwork';
// ...
  const [homeRouteRequest, releaseHomeRoute] = fetchEntrypoint(
    environment,
    iso(`entrypoint Query.HomeRoute`),
    {},
    null,
  );
  await homeRouteRequest.promise;
  console.log(iso(`entrypoint Query.HomeRoute`).name, readHomeRoute());
```

Every other `homeRoute`, `petDetailRoute`, and `setTagline` in `fetchAndRead.ts` becomes the literal, including the two in the template at `fetchAndRead.ts:69` (`` `${iso(`entrypoint Query.HomeRoute`).name} after ...` ``). Each is the object of a property access, so each is permitted.

```ts
// from demos/pet-demo/src/mockNetwork.ts
  if (operation.text === homeRoute.queryText) {
```

```ts
// from demos/pet-demo/src/mockNetwork.ts
  if (operation.text === iso(`entrypoint Query.HomeRoute`).queryText) {
```

`mockNetwork.ts` imports `iso` from `./__isograph/iso` and drops its three entrypoint imports.

### Extraction tests

`const_without_export_has_no_name`, `extra_space_before_equals_drops_the_export_name`, and `no_space_after_equals_drops_the_export_name` (`crates/isograph_extract_typescript/src/lib.rs:310-330`) use an entrypoint literal to test `const_export_name`. They switch to a `field` literal, so the tests keep asserting the export name only. The Babel and SWC plugin tests are unchanged, since the plugins do not change.

## End-user experience

A user writes `` export const HomeRouteEntrypoint = iso(`entrypoint Query.HomeRoute`); ``. In the editor, the literal text is underlined with an error from source `isograph`: `` An entrypoint literal must not be exported. Write iso(`entrypoint ...`) in each module that uses it. `` The CLI prints the same message with the file path and range, and writes no artifacts until it is fixed. `const e = iso(...)``  reports `` An entrypoint literal must not be assigned. Pass iso(`entrypoint ...`) directly as an argument where it is used. ``  ``return iso(...)` reports "An entrypoint literal must be a call argument, a JSX attribute value, the object of a property access, or a statement."

### Adoption

- Install and config: none. The rule ships in the `isograph` compiler binary, which every project already runs, and is on for every project.
- Linters: independent of ESLint, oxlint, and Biome. A project with no JS linter gets it.
- Bundlers: independent of Babel and SWC. A project that imports artifacts by hand (`faq.md:21`) gets it, since the compiler extracts every literal.
- Editors: every LSP client of the compiler, which includes i2's VS Code and Zed extensions.
- Versioning: the rule's version is the compiler's version. An upgrade that adds it fails a project with assigned entrypoints until each literal moves; the release notes name the three messages and the migration below.
- Docs: `docs-website/docs/quickstart.md`, after the paragraph at `:342`, gains a section "Where an entrypoint literal may appear" listing the permitted positions, the three messages, the user's stated reason, and the `import type` replacement for `typeof` (decision 7). `docs-website/docs/isograph-rules.md` gains the bullet "Never assign an entrypoint literal to a variable and never export it. Pass ``iso(`entrypoint ...`)`` directly to the function that uses it."
- Migration: replace each use of the variable with the literal, and replace an exported entrypoint's imports with the literal in the importing module.

## Tests

In `crates/isograph_extract_typescript` (`literal_positions` and `literal_errors` through `TypeScriptHostLanguage`):

- `` useLazyReference(iso(`entrypoint Query.HomeRoute`), {}) `` in a `.ts` file: no host error.
- `` iso(`entrypoint Query.HomeRoute`); ``: no host error.
- `` const e = iso(`entrypoint Query.HomeRoute`); ``: exactly `EntrypointAssigned`, at the literal text's span.
- `export const e = iso(...)`, with one space, two spaces, and no space around `=`: `EntrypointExported` in each.
- `export default iso(...)`: `EntrypointExported`.
- `module.exports = iso(...)`, `exports.e = iso(...)`, `obj.e = iso(...)`, `e += iso(...)`: `EntrypointAssigned`.
- `function f(e = iso(...)) {}`, `const { a = iso(...) } = o`, `class C { e = iso(...) }`: `EntrypointAssigned`.
- `({ e: iso(...) })`, `[iso(...)]`: `EntrypointAssigned`.
- `() => iso(...)`, `function f() { return iso(...); }`: `EntrypointInOtherPosition`.
- `const e = (iso(...))`, `const e = iso(...) as E`, `const e = iso(...) satisfies E`, `const e = iso(...)!`: `EntrypointAssigned`. In a `.ts` file, `const e = <E>iso(...)`: `EntrypointAssigned`.
- `f((iso(...)))`, `f(iso(...) as E)`: no host error.
- `f(c ? iso(...) : iso(...))`, `f(a ?? iso(...))`: no host error. `const e = c ? iso(...) : iso(...)`: `EntrypointAssigned` for each literal.
- `iso(...).queryText`, `iso(...).readerArtifact()`: no host error.
- In a `.tsx` file, `<R e={iso(...)} />`: no host error. `<div>{iso(...)}</div>`: `EntrypointInOtherPosition`.
- `f(...iso(...))`, `await iso(...)`: `EntrypointInOtherPosition`.
- `` const e = iso`entrypoint Query.HomeRoute`; ``: `MissingParentheses` and `EntrypointAssigned`.
- `` /* const e = iso(`entrypoint Query.HomeRoute`) */ ``: no position error.
- A file with a syntax error and `const e = iso(...)`: no position error.
- `` export const f = iso(`field Query.f { id }`)(fn) ``: no position error (the rule reads entrypoints only).
- `` const f = iso(`field Query.f { id }`)(fn) ``: `MissingExport` only.
- Two entrypoint literals on one line, one assigned and one a call argument: exactly one `EntrypointAssigned`, at the assigned literal's span.
- An empty `.ts` file: no extraction and no error.

In `crates/isograph_lsp/src/file_diagnostics.rs`: a file with `` export const e = iso(`entrypoint Query.HomeRoute`) `` has one diagnostic whose message is `EntrypointExported`'s text and whose range is the literal text.

In `crates/isograph_cli/src/compile.rs`: a project whose only error is an assigned entrypoint reports it in `project_errors` and writes no artifacts.

pet-demo: `pnpm --filter pet-demo typecheck` and `pnpm --filter pet-demo fetch-and-read` pass after its change, and the compiler reports no error for `demos/pet-demo`.
