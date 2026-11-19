# Auto-exporting fields from the iso transforms

Question: today a `field` literal must be a named export (`export const fullName = iso(...)(...)`). Could the Babel and SWC transforms add the `export` so the user does not write it?

This doc is an options analysis. It holds option (a), the required export, and adds a quick fix for `MissingExport`. Nothing here is approved for implementation.

i2 state: i2 has no transform and no artifact generation. `435f7352` deleted the SWC plugin and `180ec76f` deleted `libs/` (the Babel plugin and `@isograph/react`). The transforms and generated files below are upstream's (`../isograph` at `79e2a3c0`). i2 owns the check: `MissingExport` and `MissingAssociatedFunction` in `crates/isograph_extract_typescript`.

## Why the export exists

The generated reader for a field imports the user's function by its export name.

```ts
// external isograph/demos/pet-demo/src/components/Pet/fullName.ts
export const fullName = iso(`
  field Pet.fullName {
    firstName
    lastName
  }
`)(({ data }) => data.firstName + ' ' + data.lastName);
```

```ts
// external isograph/demos/pet-demo/src/components/__isograph/Pet/fullName/resolver_reader.ts
import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Pet__fullName__param } from './param_type';
import { Pet__fullName__output_type } from './output_type';
import { fullName as resolver } from '../../../Pet/fullName';

const readerAst: ReaderAst<Pet__fullName__param> = [/* ... */];

const artifact = (): EagerReaderArtifact<
  Pet__fullName__param,
  Pet__fullName__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "fullName",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
```

The same import is the only source of the field's output type and of a component field's props type.

```ts
// external isograph/demos/pet-demo/src/components/__isograph/Pet/fullName/output_type.ts
import type React from 'react';
import { fullName as resolver } from '../../../Pet/fullName';
export type Pet__fullName__output_type = ReturnType<typeof resolver>;
```

```ts
// external isograph/demos/pet-demo/src/components/__isograph/Pet/PetSummaryCard/output_type.ts
import type { ExtractSecondParam, CombineWithIntrinsicAttributes } from '@isograph/react';
import type React from 'react';
import { PetSummaryCard as resolver } from '../../../Pet/PetSummaryCard';
export type Pet__PetSummaryCard__output_type = (React.FC<CombineWithIntrinsicAttributes<ExtractSecondParam<typeof resolver>>>);
```

```ts
// external isograph/demos/pet-demo/src/components/__isograph/Pet/PetSummaryCard/param_type.ts
import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';

export type Pet__PetSummaryCard__param = {
  readonly data: {
    readonly fullName: Pet__fullName__output_type,
    // ...
  },
  readonly parameters: Record<PropertyKey, never>,
};
```

The compiler writes the import from the name the extraction regex captured.

```rust
// external isograph/crates/artifact_content/src/eager_reader_artifact.rs
    let const_export_name = target_field_info.info.const_export_name;
    ClientScalarSelectableFunctionImportStatement(format!(
        "import {{ {const_export_name} as resolver }} from '{file_name}';"
    ))
```

i2 rejects a field literal without that name.

```rust
// from crates/isograph_extract_typescript/src/lib.rs
const EXTRACT_ISO_LITERAL_PATTERN: &str = r"(?<comment>// )?(export const (?<export_name>[^ ]+) =\s+)?iso(?<open_paren>\()?\s*`(?<literal>[^`]+)`,?\s*(?<close_paren>\))?(?<associated>\()?";

pub fn host_errors_for_extraction(
    extraction: &IsoLiteralExtraction<TypeScriptHostLanguage>,
    parsed: &ParsedIsoLiteral,
) -> Vec<span::WithSpan<TypeScriptHostError>> {
    let span = extraction.span();
    let mut errors = Vec::new();
    if let IsoCall::TaggedTemplate = extraction.context.call {
        errors.push(TypeScriptHostError::MissingParentheses.with_span(span));
    }
    let parsed_item = parsed.item.as_ref().and_then(item_of);
    if let Some(IsoLiteralItem::Selectable(selectable)) = parsed_item {
        if extraction.context.const_export_name.is_none() {
            errors.push(
                TypeScriptHostError::MissingExport {
                    suggested_name: selectable.name.item,
                }
                .with_span(span),
            );
        }
        if let AssociatedJsFunction::Absent = extraction.context.associated_js_function {
            errors.push(TypeScriptHostError::MissingAssociatedFunction.with_span(span));
        }
    }
    errors
}
```

The reader's default export is a thunk. Upstream `0365bd75` ("convert reader artifacts to function to prevent cycles") made it one: `HomeRoute.tsx` imports its entrypoint, the entrypoint imports `resolver_reader.ts`, and `resolver_reader.ts` imports `HomeRoute.tsx`. Inside that cycle `resolver` is uninitialized during the first evaluation, so the reader reads it when the runtime calls `readerArtifact()` (`libs/isograph-react/src/core/read.ts:553`, `IsographEnvironment.ts:267`).

## What the transforms do today

Both transforms rewrite the `iso` call expression only. The enclosing declaration and any `export` keyword are left as written.

```js
// external isograph/libs/isograph-babel-plugin/compileTag.js
    } else if (keyword === 'field' || keyword === 'pointer') {
      if (t.isCallExpression(path.parentPath.node)) {
        const firstArg = path.parentPath.node.arguments[0];
        if (path.parentPath.node.arguments.length === 1 && firstArg != null) {
          path.parentPath.replaceWith(firstArg);
        } else {
          throw new Error(
            'Invalid iso tag usage. The iso function should be passed at most one argument.',
          );
        }
      } else {
        path.replaceWith(
          t.arrowFunctionExpression([t.identifier('x')], t.identifier('x')),
        );
      }
    }
```

```rust
// external isograph/crates/swc_isograph_plugin/src/lib.rs
            ArtifactType::Field => {
                match fn_args {
                    Some(fn_args) => {
                        if let Some((first, [])) = fn_args.split_first() {
                            return first.expr.as_ref().clone().wrap_ok();
                        }
                        return IsographTransformError::IsoFnCallRequiresOneArg.wrap_err();
                    }
                    None => build_arrow_identity_expr().wrap_ok(),
                }
            }
```

`export const fullName = iso(...)(fn)` becomes `export const fullName = fn`. An `entrypoint` literal becomes an import of `__isograph/<Type>/<field>/entrypoint.ts` (Babel `addDefault` or `require(...).default`; SWC a hoisted default import or `require(...).default`). The entrypoint direction is user module imports generated file, so no export is needed. The field direction is generated file imports user module, so the export is needed.

Babel runs its whole traversal at `Program` enter (`Program(path, state) { path.traverse(visitor, state); }` in `BabelPluginIsograph.js`), so every `iso` call is rewritten before later plugins visit the program.

Without a transform, upstream's `options.no_babel_transform: true` makes the generated `iso.ts` work at runtime: it switches over entrypoint literal texts and returns `(clientFieldResolver: any) => clientFieldResolver` for fields (`crates/artifact_content/src/iso_overload_file.rs`). Fields still work in that mode only because the user wrote `export`.

## 1. What auto-export would do in each transform

The compiler (Rust regex over text) and the two transforms (Babel AST, SWC AST) share no code. The name the reader imports must be computable by all three from the literal alone. The field key is the only such input, so auto-export uses a reserved name derived from it.

```ts
// from __isograph/Pet/fullName/resolver_reader.ts (proposed, option b)
import { __isograph_resolver__Pet__fullName as resolver } from '../../../Pet/fullName';
```

Source shapes and the transform output:

```ts
// from src/Pet/fullName.ts (source shapes)

// A. top-level const, not exported
const fullName = iso(`field Pet.fullName { firstName }`)(fn);
// output
const fullName = fn;
export { fullName as __isograph_resolver__Pet__fullName };

// B. already exported
export const fullName = iso(`field Pet.fullName { firstName }`)(fn);
// output
export const fullName = fn;
export { fullName as __isograph_resolver__Pet__fullName };

// C. top-level, argument position
export const Card = memo(iso(`field Pet.Card @component { id }`)(fn));
// output: a binding is inserted immediately before the statement
const __isograph_resolver__Pet__Card = fn;
export { __isograph_resolver__Pet__Card };
export const Card = memo(__isograph_resolver__Pet__Card);

// D. default export
export default iso(`field Pet.fullName { firstName }`)(fn);
// output
const __isograph_resolver__Pet__fullName = fn;
export { __isograph_resolver__Pet__fullName };
export default __isograph_resolver__Pet__fullName;

// E. not at module top level
function make() {
  return iso(`field Pet.fullName { firstName }`)(fn);
}
// no output possible: `export` is only legal at module top level, and
// hoisting `fn` out of `make` breaks every closure over `make`'s locals.
// The transform must reject E, and the compiler must reject it too.

// F. let / var
let fullName = iso(`field Pet.fullName { firstName }`)(fn);
fullName = other;
// output as A; the export is a live binding, so the reader gets `other`
// after the reassignment. The compiler would have to reject `let`/`var`
// or accept that the resolver can change after module evaluation.
```

Consequences:

- C and D change nothing at runtime ordering: `fn` is a function expression, so evaluating it one statement earlier has no observable effect. If `fn` is an identifier that refers to a later `const`, the inserted binding is in the temporal dead zone, which is a new `ReferenceError` the user's source did not have.
- B gains a second export. That is harmless at runtime. It is visible to anything that enumerates exports (Fast Refresh, below).
- Collision with a user export: `export { fullName }` plus an added `export { fullName }` is a duplicate export, a `SyntaxError` at parse. The reserved name avoids it unless the user writes `__isograph_resolver__...` themselves. The transform must still check for an existing export of the reserved name and fail with an error, because two exports of one name are a parse error in the output.
- The compiler must know which shape it saw to reject E and F. The regex extractor cannot tell top level from nested (`EXTRACT_ISO_LITERAL_PATTERN` has no scope information). Today E with `export const` in front is impossible because `export` inside a function is a TypeScript error, so the regex check is sound by accident of the required export. Drop the required export and the regex accepts E silently, and the transform is the first place to see it.

## 2. TypeScript

tsc type-checks the untransformed source. After auto-export, the generated reader imports a name the source does not export. Checked with TypeScript 5.6.3 (the version in `.prettierrc.json` and upstream `node_modules`).

```ts
// from scratch/Pet.ts (experiment, not checked in)
const fullName = (p: { data: { firstName: string } }) => p.data.firstName.length;
export const other = 1;
```

```ts
// from scratch/a_plain.ts (experiment)
import { fullName } from './Pet';
// error TS2459: Module '"./Pet"' declares 'fullName' locally, but it is not exported.
```

```ts
// from scratch/b_expect.ts (experiment)
// @ts-expect-error
import { fullName } from './Pet';
export const wrong: ReturnType<typeof fullName> = 'a string, not a number'; // no error
export const n: typeof fullName = 5; // no error
export const deep = fullName.foo.bar; // no error
const local = (p: { data: { firstName: string } }) => p.data.firstName.length;
export const control: ReturnType<typeof local> = 'a string, not a number';
// error TS2322: Type 'string' is not assignable to type 'number'. (control only)
```

The suppressed import binding is `any`. `ReturnType<any>` is `any`, `ExtractSecondParam<any>` is `any`.

```ts
// from scratch/c_typeof_import.ts (experiment)
export type T = typeof import('./Pet')['fullName'];
// error TS2339: Property 'fullName' does not exist on type 'typeof import(".../Pet")'.
```

```ts
// from scratch/e_namespace.ts (experiment)
import * as Pet from './Pet';
export type T = typeof Pet.fullName;
// error TS2339: Property 'fullName' does not exist on type 'typeof import(".../Pet")'.
```

```ts
// from scratch/d_augment.ts (experiment)
declare module './Pet' {
  export const fullNameAug: typeof fullName;
}
// error TS2304: Cannot find name 'fullName'.
```

A module augmentation can add a typed export to `./Pet` (a hand-written `export const fullName: (p: ...) => number` inside `declare module './Pet'` compiles), but its body cannot name the module's locals, so the generator would have to write the type out. The generator knows the param type and does not know the return type or a component's props type; those are TypeScript inference over the user's function.

```ts
// from scratch/Pet.ts (experiment, second version)
const fullName = (p: { data: { firstName: string } }) => p.data.firstName.length;
declare global {
  interface IsographResolvers {
    'Pet.fullName': typeof fullName;
  }
}
```

```ts
// from scratch/f_global.ts (experiment)
export type T = ReturnType<IsographResolvers['Pet.fullName']>;
export const wrong: T = 'a string, not a number';
// error TS2322: Type 'string' is not assignable to type 'number'.
```

`declare global` written inside the user's module can name a local, and the type escapes. It is user-written text, longer than `export`, so it does not answer the question.

A file can name a type declared in another module only through an exported declaration of that module or a global augmentation written in that module. Both are source text that tsc reads. A transform changes only the code the bundler receives, never the text tsc checks, so it cannot add either. A TypeScript language service plugin affects the editor only, not `tsc`. A patched compiler (ts-patch) is out of scope.

Consequence for (b): `Pet__fullName__output_type` is `any`. Every parent whose param type selects `fullName` gets `readonly fullName: any`. A component field's props are `any`, so `<PetSummaryCard wrongProp />` type-checks. The loss is silent and propagates through the selection graph.

`@ts-expect-error` also requires the generator to know whether the user did export (option B above): if the name is exported, the directive is unused, which is error TS2578. With the reserved name it is never exported in source, so the directive is always needed, and always suppresses everything on that line, including a wrong path.

## 3. Bundler and runtime consequences

Tree shaking: under (a) and (b) the reader imports a named export, which bundlers keep exactly when the reader is reachable. No change.

Missing export without the transform (Jest without `babel-jest`, Vitest or Vite without the Babel or SWC plugin, `no_babel_transform: true`): under (b) the source has no export.

- Native ESM: a named import of a missing export is a link-time `SyntaxError` ("does not provide an export named"). The whole module graph fails to load, not one field.
- Rollup and Vite production builds: the bundler reports that the name is not exported by the module.
- webpack: a warning, and `resolver` is `undefined` at runtime, so the read throws when it calls the resolver.
- CommonJS: `_Pet.__isograph_resolver__Pet__fullName` is `undefined`, same runtime failure.

`no_babel_transform` exists so that fields work without a transform. (b) removes that mode for fields, or keeps two modes where `MissingExport` is an error only when `no_babel_transform` is set.

ESM vs CJS for (b): Babel's own pass runs at `Program` enter, before `@babel/plugin-transform-modules-commonjs` (which rewrites exports at `Program` exit), so the added export is converted like a user-written one. SWC: plugin order relative to SWC's module transform was not verified here. Babel CJS reads `_Pet.name` at use time, so the thunk in the reader still protects the cycle.

Fast Refresh: Vite's `@vitejs/plugin-react` (`dist/refresh-runtime.js`, 4.7.0) accepts an update only when every export is a likely component (`isLikelyComponentType`: a function whose `name` starts with a capital letter, or a React class) or is the same value as before the edit. Otherwise it reports `Could not Fast Refresh ("<key>" export is incompatible)` and reloads.

- (a) today: `export const fullName = (...) => ...` has `name === 'fullName'`. Editing a module that exports a lowercase field already forces a reload. Component fields named in PascalCase pass.
- (b): adds one export per field. For the A shape the exported value is the same function, `name` unchanged, same verdict as (a). For C and D the inserted binding names the function `__isograph_resolver__Pet__Card`, lowercase, so a component-only module that Fast Refreshed under (a) now reloads on every edit.
- (c): adds no export. A module whose only exports are components stays a refresh boundary even if it defines a lowercase field. Re-evaluation re-registers the field.

Module side effects: under (a) and (b), evaluating the user module has no isograph side effect. Under (c) it does, below.

## 4. Alternatives

### (a) Keep the required export (status quo)

Mechanism: the user writes `export const name = iso(...)(fn)`. The reader imports `name`.

Typing: `typeof resolver` is the user's inferred function type. Output types and component props are exact.

Runtime: ESM or CJS named import; works with and without a transform (`no_babel_transform`).

Failure modes: the user forgets `export`. `MissingExport` reports it with the suggested name before any tsc or bundler run. Shapes E and F are already rejected because `export` is only accepted on `export const` at the start of the match.

### (b) Auto-export, generated imports under `@ts-expect-error`

Mechanism: section 1. Generated reader and output type import the reserved name under `// @ts-expect-error`.

Typing: section 2. Output types and props become `any` and propagate to every parent.

Runtime: section 3. Fails at link time or at read time without the transform. Changes Fast Refresh for shapes C and D.

Failure modes: silent `any`; whole-graph `SyntaxError` under native ESM without the transform; shape E accepted by the regex and rejected only by the transform; `let` reassignment changes the resolver.

### (c) Registry keyed by field

Mechanism: the transform rewrites the field call to register the function when the module evaluates.

```ts
// from src/Pet/fullName.ts (transform output, option c)
import { registerResolver } from '@isograph/react';
const fullName = registerResolver('Pet.fullName', fn);
```

```ts
// from __isograph/Pet/fullName/resolver_reader.ts (proposed, option c)
import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { readResolver } from '@isograph/react';
import { Pet__fullName__param } from './param_type';
import { Pet__fullName__output_type } from './output_type';
import '../../../Pet/fullName';

const artifact = (): EagerReaderArtifact<
  Pet__fullName__param,
  Pet__fullName__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "fullName",
  resolver: readResolver('Pet.fullName'),
  readerAst,
  hasUpdatable: false,
});
```

Without a transform, the runtime `iso` in `no_babel_transform` mode can do the registration itself, since it already receives the literal text: `iso(text)` returns `(fn) => registerResolver(keyOf(text), fn)`. That is the one advantage (c) has over (b): it works in both modes.

Typing: the iso overload's generic identity (`IdentityWithParam<TParam>`) types `fn` at the call site, and `registerResolver` returns `fn` unchanged. No other file can name the type of `fn`, because the module exports nothing that refers to it (section 2). `output_type.ts` has nothing to name, so it is `any` exactly as in (b). Generated types cannot replace it because the return type is inference the compiler does not perform.

Runtime: the registry is a module-level `Map<string, Function>` in `@isograph/react`.

Failure modes:

- Module not evaluated. A bare `import '../../../Pet/fullName'` is dropped by bundlers when the app's `package.json` declares `"sideEffects": false` (or a list that does not include the module), and by `jest.mock('../Pet/fullName')`. `readResolver` then misses at read time, far from the cause.
- Ordering. In the `HomeRoute` cycle the reader module evaluates before `HomeRoute.tsx`. The thunk defers `readResolver` to read time, which is after the initial evaluation tick; the same assumption `0365bd75` makes.
- Duplicate registration. HMR re-evaluates the module and registers again; that must overwrite. A genuine duplicate (two literals for `Pet.fullName`) is a compiler error before runtime, so overwrite is correct at runtime. Two copies of `@isograph/react` in one bundle give two registries; registration and lookup then hit different maps.
- SSR: the registry lives for the server process. Resolvers are functions of their arguments, so sharing across requests is harmless.

### (d) Generated reader carries types only; runtime receives resolvers some other way

The runtime needs the resolver wherever the field is read, which is any parent's reader, not only inside the user's module. The user's module is the only holder of `fn`. The ways out of that module at runtime are an export (a, b) or a side effect on shared state (c). (d) reduces to (c) for runtime. For types, the reader cannot name the user's type without an export (section 2), so "types only" has no type to carry.

### (e) Precedent

- Upstream entrypoints: the transform replaces `iso(entrypoint ...)` with an import of the generated artifact. The user module imports generated code, so no export is needed. Fields reverse the direction.
- Upstream `no_babel_transform`: the generated `iso.ts` dispatches on literal text at runtime. That is a lookup keyed by literal text, the same key (c) would use.
- Upstream `0365bd75`: readers are thunks so a cycle through the user module does not observe an uninitialized `resolver`.
- Relay Resolvers (docblock `@RelayResolver`) require the resolver function to be a named export that the generated code imports. This is from memory of Relay's docs; `relay-crates/` in i2 does not contain the docblock crates, so it is not verified here.

### (f) Quick fix for `MissingExport`

The diagnostic already carries `suggested_name`. An LSP code action inserts the export. Typing, runtime, and failure modes are those of (a); the change is the cost of satisfying the check.

## 5. Held option: (a) plus a quick fix

Every option without a user-written export loses the field's output type and a component field's props type, and the loss is `any` that propagates to every parent selection. A transform cannot restore it, because tsc checks the source before the transform (section 2). (b) also breaks fields without a transform and changes Fast Refresh for C and D. (c) fixes the no-transform case and Fast Refresh, and keeps the `any`, and adds a side-effect import that `sideEffects: false` deletes. Under (a) the user writes one keyword, and the compiler already reports the missing keyword with the exact name to write. A quick fix makes writing it one editor action.

What (a) plus (f) requires:

Transforms: no change.

Generated files: no change.

TypeScript checks: `MissingExport` stays an error. Today the extractor records `const_export_name: Option<ConstExportName>`, and `None` covers both `const fullName = iso(...)` and a bare `iso(...)`. The quick fix inserts different text in those two cases, so the extractor records which one it saw.

```rust
// from crates/isograph_extract_typescript/src/lib.rs (proposed)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeScriptLiteralContext {
    pub binding: IsoLiteralBinding,
    pub call: IsoCall,
    pub associated_js_function: AssociatedJsFunction,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IsoLiteralBinding {
    /// `export const name = iso(...)`
    ExportedConst(ConstExportName),
    /// `const name = iso(...)`
    UnexportedConst(UnexportedConst),
    /// `iso(...)` preceded on its line by whitespace only.
    UnboundStatement(UnboundStatement),
    /// `iso(...)` inside a larger expression, such as `memo(iso(...)(fn))`.
    UnboundExpression,
}

/// The offset of the `const` keyword.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UnexportedConst(pub SourceOffset);

/// The offset of `iso`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UnboundStatement(pub SourceOffset);

/// A byte offset into the host file.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SourceOffset(pub usize);

const EXTRACT_ISO_LITERAL_PATTERN: &str = r"(?<comment>// )?(?<line_start>^[ \t]*)?((?<export>export )?(?<const>const) (?<binding_name>[^ ]+) =\s+)?(?<iso>iso)(?<open_paren>\()?\s*`(?<literal>[^`]+)`,?\s*(?<close_paren>\))?(?<associated>\()?";
```

The regex runs with multi-line mode (`(?m)`) so `^` matches at each line start. The name of an unexported `const` is not recorded, because nothing reads it: the quick fix inserts only `export `, and the compiler reads a name only from `ExportedConst`. Delta from today's pattern: `export` and `const` are separate optional groups, and the `line_start` and `iso` groups are new.

`host_errors_for_extraction` reports `MissingExport` for `UnexportedConst`, `UnboundStatement`, and `UnboundExpression` on a field literal. The quick fix edits:

```ts
// from src/Pet/fullName.ts (UnexportedConst: insert `export ` at the `const` offset)
const fullName = iso(`field Pet.fullName { firstName }`)(fn);
// after
export const fullName = iso(`field Pet.fullName { firstName }`)(fn);
```

```ts
// from src/Pet/fullName.ts (UnboundStatement: insert `export const {suggested_name} = ` at the `iso` offset)
iso(`field Pet.fullName { firstName }`)(fn);
// after
export const fullName = iso(`field Pet.fullName { firstName }`)(fn);
```

`UnboundExpression` (shape C) has no single insertion that produces an exported binding of the field function, so it gets the diagnostic and no quick fix.

LSP: i2 routes no `textDocument/codeAction` request today (`crates/isograph_cli/src/adapter.rs` handles semantic tokens, hover, definition, document highlight, formatting, and will-save; `lsp_request_dispatch.rs` mentions `CodeActionRequest` only in a test of parameter extraction). `IsographCodeAction` in `crates/common_lang_types/src/diagnostic.rs` has two upstream variants and no reader in i2. The quick fix therefore needs its own plan: a code action handler, an `IsographCodeAction::ExportIsoLiteral` variant or a host-specific action type, and the edit computation above. That plan is not written.
