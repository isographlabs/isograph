# pico-js: independently toggled features

pico-js v0 ships `libs/pico-js/src/graph.ts` as it is at the engine commit `ba3daedf7`. This doc is later work.

## Goal

Each feature can be turned on or off on its own. A feature is a contract check, such as a double release or a read of an unretained node, or a change of engine behavior. A feature that is off costs nothing: its statements are absent from the emitted JS, and a node field that only the feature reads is absent from every node's object literal, so every node of a configuration keeps one monomorphic shape.

Upstream isograph has no debug build of its runtime. `../isograph/libs/isograph-react/src/core/check.ts:133` and `:166` hold `// TODO in __DEV__ assert linkedRecord is an object`, and upstream defines no `__DEV__`. Upstream emits its artifacts as strings built with `format!` (`../isograph/crates/artifact_content/src/import_statements.rs:33`). This repo emits its TypeScript artifacts through swc nodes and `swc_ecma_codegen` (`crates/typescript_artifacts/src/swc.rs`), and the generator follows that crate.

## Design

- The engine is written in Rust, as a generator that emits the engine's JS and its `.d.ts` through `swc_ecma_ast` and `swc_ecma_codegen`. The output is JS. It is not wasm.
- Each feature is a field of `Features`, and the generator matches on it at each place the feature touches, so adding a feature fails to compile until every construction of `Features` sets it.
- The generator emits one JS file per configuration. The package ships pregenerated configurations: all features off, all features on, and each feature on by itself. That is n + 2 files for n features, and it is exactly the set the tests run against, so the package ships no configuration that the tests did not run.
- Every configuration has the same `.d.ts`. A feature's node fields are on the engine's internal view only, so the public types do not depend on the configuration, and `declarations` takes no `Features`.
- `src/graph.ts` and `src/index.ts` are deleted once the generator reproduces them. The tests stay in TypeScript.

These were rejected:

- One debug wrapper module. It turns every check on together. Toggling each check needs 2^n modules or composition at run time, and composition costs a call or a branch on the hot path.
- Per-feature bundler defines (`if (PICO_X)` with an esbuild, Vite, or Rollup `define`). Zero cost then depends on the consumer's bundler and its dead-code elimination, an undefined flag is a `ReferenceError` at run time, and a node field cannot depend on a flag without a preprocessor, so the node shape would differ from the field list in `newDerivedNode` or carry the field in every configuration.
- `#if` comments with a preprocessing pass. `tsc` type-checks both sides of every `#if`, and the build needs two passes.
- A TypeScript AST transform that emits TypeScript. The user rejected TypeScript generating TypeScript.
- ReScript or Haxe. The compiler decides the emitted JS: ReScript wraps nested options in `Caml_option.some`, encodes exceptions in its own representation, and curried by default before v11, and Haxe emits class and runtime glue. The hot path depends on emitted-code details (numeric statuses, one monomorphic shape per node, intrusive linked lists, arrays reused and never truncated with `length = 0`, no allocation on the hot path), and each compiler upgrade would need the output checked again. ReScript has no conditional compilation in ReScript syntax: its docs limited conditional compilation to the old OCaml syntax, 12.0.0-alpha.1 removed compiling `.ml` files, and 12.0.0-beta.2 removed the `-bs-D` and `-bs-list-conditionals` flags (ReScript CHANGELOG.md). The `readonly` public `.d.ts` would be maintained by hand beside the generated JS.

These are the costs:

- The engine logic becomes JS fragments inside Rust. A change to the engine is a change to the generator.
- Reviewers read the generator and the generated output. The generated files are checked in for that reason, and a pre-commit hook regenerates them and fails on a diff.
- Stack traces and coverage point at generated JS, so the output keeps the function names, variable names, and statement structure of `graph.ts`, carries the JSDoc of each function, and is formatted with dprint as `crates/typescript_artifacts/src/format.rs` formats artifacts.
- CI runs the test suite and the random differential test n + 2 times.

## swc and `quote!`

The root workspace pins `swc_common = "=27.0.0"`, `swc_ecma_ast = "=30.0.2"`, and `swc_ecma_codegen = "=33.0.2"` in `crates/typescript_artifacts/Cargo.toml`. `crates/swc_isograph_plugin` is its own workspace on `swc_core = "84.2.0"` (the root `Cargo.toml` excludes it, because `swc_core` with the plugin features locks about 200 packages). Its `Cargo.lock` resolves `swc_ecma_ast` 30.0.2 and `swc_ecma_codegen` 33.0.2.

`swc_ecma_quote` 47.0.0 and `swc_ecma_quote_macros` 47.0.0 are in the cargo registry cache, and they depend on `swc_common` 27.0.0 and `swc_ecma_ast` 30.0.0, the versions this repo uses. `swc_core` 84.2.0 re-exports the macro under its `ecma_quote` feature (`ecma_quote = ["__common", "ecma_ast", "swc_ecma_quote_macros"]`) and pins `swc_ecma_ast` 30.0.2, `swc_ecma_codegen` 33.0.2, and `swc_ecma_quote_macros` 47.0.0. What the macro does, from its source:

- It expands to paths under `swc_core` (`swc_core::ecma::ast::…`, `swc_core::common::DUMMY_SP`, `swc_core::atoms::atom!`, `swc_core::quote::ImplicitClone`), so a crate that calls `quote!` depends on `swc_core` by that name, not on `swc_ecma_quote` alone.
- It parses its string at compile time as JavaScript (`Default::default()` syntax, `EsVersion::Es2020`). Every TypeScript node (`TsTypeAliasDecl`, `TsAsExpr`, `TsTypeAnn`, …) is `fail_todo!`, so the `.d.ts` is built from `swc_ecma_ast` nodes directly, as `crates/typescript_artifacts/src/swc.rs` builds its types.
- Its output types are `Expr`, `Pat`, `AssignTarget`, `Stmt`, and `ModuleItem`, each also in `Box` or `Option`. A `$name` variable substitutes an `Ident`, `Expr`, `Pat`, `AssignTarget`, or `Str`. A variable cannot stand for a list of statements, so a function body is a `Vec<Stmt>` assembled in Rust from quoted statements.
- Quoted nodes carry `DUMMY_SP`, so a comment cannot attach to them. JSDoc attaches to the function declarations, which the generator builds with a span from `comment_at`, as `swc.rs` attaches descriptions.

The generator crate depends on `swc_core = { version = "=84.2.0", features = ["ecma_ast", "ecma_codegen", "ecma_quote"] }` and reaches the AST and the emitter through `swc_core::ecma`. Those versions are the ones `typescript_artifacts` pins, so the root `Cargo.lock` gains no second copy of them. The macro adds `swc_ecma_quote_macros` and its build dependencies (`swc_ecma_parser` 47, `syn`, `anyhow`, `swc_macros_common`) to the root workspace. The package count that adds to the root lockfile is not measured.

## Features

```rust
// from crates/pico_js_generator/src/features.rs
/// The features of one generated configuration of the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Features {
    pub unchanged_write: Check,
    pub double_release: Check,
    pub unretained_read: Check,
    pub mutation_inside_computation: Check,
    pub mutation_inside_on_change: Check,
    pub mutators_after_batch: Check,
    pub on_change_cleared_during_flush: Check,
    pub read_outside_computation: Check,
    pub cycle: Check,
    pub source_value_mutation: Freeze,
    pub derived_value_mutation: Freeze,
    pub uncomputed_in_dependents: Check,
    pub clean_depends_on_dirty: Check,
    pub verify_descent: Check,
}

/// Whether a configuration throws when a contract or an internal invariant is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    Skip,
    Throw,
}

/// Whether a configuration deep-freezes the values that pass through a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freeze {
    Skip,
    DeepFreeze,
}

/// One field of `Features`, so a configuration can name a single feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::EnumIter, strum::Display)]
pub enum FeatureName {
    #[strum(to_string = "unchanged-write")]
    UnchangedWrite,
    #[strum(to_string = "double-release")]
    DoubleRelease,
    #[strum(to_string = "unretained-read")]
    UnretainedRead,
    #[strum(to_string = "mutation-inside-computation")]
    MutationInsideComputation,
    #[strum(to_string = "mutation-inside-on-change")]
    MutationInsideOnChange,
    #[strum(to_string = "mutators-after-batch")]
    MutatorsAfterBatch,
    #[strum(to_string = "on-change-cleared-during-flush")]
    OnChangeClearedDuringFlush,
    #[strum(to_string = "read-outside-computation")]
    ReadOutsideComputation,
    #[strum(to_string = "cycle")]
    Cycle,
    #[strum(to_string = "source-value-mutation")]
    SourceValueMutation,
    #[strum(to_string = "derived-value-mutation")]
    DerivedValueMutation,
    #[strum(to_string = "uncomputed-in-dependents")]
    UncomputedInDependents,
    #[strum(to_string = "clean-depends-on-dirty")]
    CleanDependsOnDirty,
    #[strum(to_string = "verify-descent")]
    VerifyDescent,
}

/// A configuration the package ships and the tests run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Configuration {
    AllOff,
    AllOn,
    Only(FeatureName),
}
```

`Features::all_off` and `Features::all_on` list every field, so a new field fails to compile in both. `Features::only` matches every `FeatureName`.

```rust
// from crates/pico_js_generator/src/features.rs
impl Features {
    pub fn all_off() -> Self {
        Features {
            unchanged_write: Check::Skip,
            double_release: Check::Skip,
            unretained_read: Check::Skip,
            mutation_inside_computation: Check::Skip,
            mutation_inside_on_change: Check::Skip,
            mutators_after_batch: Check::Skip,
            on_change_cleared_during_flush: Check::Skip,
            read_outside_computation: Check::Skip,
            cycle: Check::Skip,
            source_value_mutation: Freeze::Skip,
            derived_value_mutation: Freeze::Skip,
            uncomputed_in_dependents: Check::Skip,
            clean_depends_on_dirty: Check::Skip,
            verify_descent: Check::Skip,
        }
    }

    pub fn all_on() -> Self {
        Features {
            unchanged_write: Check::Throw,
            double_release: Check::Throw,
            unretained_read: Check::Throw,
            mutation_inside_computation: Check::Throw,
            mutation_inside_on_change: Check::Throw,
            mutators_after_batch: Check::Throw,
            on_change_cleared_during_flush: Check::Throw,
            read_outside_computation: Check::Throw,
            cycle: Check::Throw,
            source_value_mutation: Freeze::DeepFreeze,
            derived_value_mutation: Freeze::DeepFreeze,
            uncomputed_in_dependents: Check::Throw,
            clean_depends_on_dirty: Check::Throw,
            verify_descent: Check::Throw,
        }
    }

    pub fn only(feature: FeatureName) -> Self {
        let off = Features::all_off();
        match feature {
            FeatureName::UnchangedWrite => Features { unchanged_write: Check::Throw, ..off },
            FeatureName::DoubleRelease => Features { double_release: Check::Throw, ..off },
            FeatureName::UnretainedRead => Features { unretained_read: Check::Throw, ..off },
            FeatureName::MutationInsideComputation => Features { mutation_inside_computation: Check::Throw, ..off },
            FeatureName::MutationInsideOnChange => Features { mutation_inside_on_change: Check::Throw, ..off },
            FeatureName::MutatorsAfterBatch => Features { mutators_after_batch: Check::Throw, ..off },
            FeatureName::OnChangeClearedDuringFlush => Features { on_change_cleared_during_flush: Check::Throw, ..off },
            FeatureName::ReadOutsideComputation => Features { read_outside_computation: Check::Throw, ..off },
            FeatureName::Cycle => Features { cycle: Check::Throw, ..off },
            FeatureName::SourceValueMutation => Features { source_value_mutation: Freeze::DeepFreeze, ..off },
            FeatureName::DerivedValueMutation => Features { derived_value_mutation: Freeze::DeepFreeze, ..off },
            FeatureName::UncomputedInDependents => Features { uncomputed_in_dependents: Check::Throw, ..off },
            FeatureName::CleanDependsOnDirty => Features { clean_depends_on_dirty: Check::Throw, ..off },
            FeatureName::VerifyDescent => Features { verify_descent: Check::Throw, ..off },
        }
    }
}
```

Each feature below names the check, the functions of `graph.ts` it goes into, and the state it needs. Every check throws `new Error(message)`.

- `unchanged_write`. Contract: the caller writes a source only when its value changed. `storeValue`, which `writeSource` and `Mutators.writeSource` call, throws "A source was written with the value it already holds." when `value === source.value`, before the assignment. State: none.
- `double_release`. Contract: each retain is matched by one release. `releaseDerivedNode` throws "A node was released more often than it was retained." when `retainCount === 0` before the decrement. State: `retainCount`.
- `unretained_read`. Contract: `readDerivedNode` is called only on a retained node. `readDerivedNode` throws "A node was read without being retained." when `retainCount === 0`, before `pull`. pico-js.md's list of checks for a future debug build also requires no dependent. This check does not, because the contract requires a retain and a dependent does not satisfy it. State: `retainCount`.
- `mutation_inside_computation`. Contract: a computation does not write, start a batch, set an evaluation, retain, or release. `computeFirst` and `rerun` add 1 to a counter before `node.computation(…)` and subtract 1 after it. `writeSource`, `Mutators.writeSource`, `startBatch`, `setDerivedNodeEvaluation`, `Mutators.setDerivedNodeEvaluation`, `retainDerivedNode`, and `releaseDerivedNode` throw "A computation wrote, started a batch, set an evaluation, retained, or released." when the counter is above 0. State: a counter that no node and no call argument can hold, because a top-level `writeSource` receives only the source and the value (see Undecided).
- `mutation_inside_on_change`. Contract: an `onChange` does not write, start a batch, set an evaluation, retain, or release. `flush` sets a flag before its walk over the changed list and clears it after. The seven functions of `mutation_inside_computation` throw "An onChange wrote, started a batch, set an evaluation, retained, or released." while the flag is set. State: as `mutation_inside_computation`.
- `mutators_after_batch`. Contract: a caller calls the methods of `mutators` only until its `startBatch` returns. `Mutators` gains a private field `#batchState`, `BODY_RUNNING` or `BODY_RETURNED`, and a method `bodyReturned()` that sets it. `startBatch` calls `bodyReturned()` after `body` returns and before `flush`. `Mutators.writeSource` and `Mutators.setDerivedNodeEvaluation` throw "A method of mutators was called after its startBatch body returned." when it is `BODY_RETURNED`. State: `#batchState` on the `Mutators` object. The `.d.ts` does not declare `bodyReturned`.
- `on_change_cleared_during_flush`. Contract: no computation or `onChange` sets an `onChange`. `flush` throws "An onChange was cleared during a flush." when `node.onChange === undefined` for a node in its changed list, in place of `node.onChange!()`. State: `onChange`. A write of `onChange` to another function is not detected, because `onChange` is a plain field.
- `read_outside_computation`. Contract: a computation calls `readSource` and `readDerived` only with the `callingNode` it received. Each derived node gains a field `runState`. `computeFirst` and `rerun` set `runState = RUNNING` before `node.computation(…)` and `NOT_RUNNING` after it. `readSource` and `readDerived` throw "readSource or readDerived received a callingNode whose computation is not running." when `callingNode.runState !== RUNNING`. It catches a `callingNode` used after its run returned. It does not catch a computation that passes another running node, such as its own reader. State: `runState` on every derived node.
- `cycle`. Contract: the derived nodes form a directed acyclic graph. It uses `runState`, with a third value `VERIFYING`. `pull` throws "A derived node was read while its computation or its verification runs. The derived nodes form a cycle." when `node.runState !== NOT_RUNNING`. `verify` sets `root.runState = VERIFYING` on entry, sets a dependency's `runState = VERIFYING` when it descends into it, throws the same error when the dependency it would descend into is already `VERIFYING`, sets `NOT_RUNNING` on each node its ascent leaves, and sets `root.runState = NOT_RUNNING` before each return. `recomputeBelowRoot` and the caller's `recompute` of `root` set `RUNNING` through `rerun`. State: `runState` on every derived node. The node literal in `newDerivedNode` has `runState` when `cycle` or `read_outside_computation` is `Throw`.
- `source_value_mutation`. Contract: a caller does not mutate a value it passed to `createSourceNode` or `writeSource`. `createSourceNode` and `storeValue` call `deepFreeze(value)`. State: none.
- `derived_value_mutation`. Contract: a caller does not mutate a value that a read returns. `computeFirst` and `rerun` call `deepFreeze(value)` on the value the computation returned, before the cutoff comparison. State: none.
- `uncomputed_in_dependents`. Internal invariant: no dependents list holds an `Uncomputed` node. `markDirty`, `markCheck`, `markDirtyQueue`, `markCheckQueue`, and `markCheckDependentsDirty` throw "An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js." in their `UNCOMPUTED` arm. State: `status`.
- `clean_depends_on_dirty`. Internal invariant: a lazy node out of `Clean` has no `Clean` dependents. `markCheckDependentsDirty` throws "A Clean node depends on a Dirty node. This is indicative of a bug in pico-js." in its `CLEAN` arm. State: `status`.
- `verify_descent`. Internal invariant: a node that `verify` descended into has a dependent, and one with more than one dependent has an edge on the verification stack. `verify`'s ascent throws "A node that verify descended into has no dependents. This is indicative of a bug in pico-js." when `node.firstDependent === undefined`, and "A node that verify descended into has more than one dependent and no edge on the verification stack. This is indicative of a bug in pico-js." when `verifyStack?.pop()` returns `undefined`. State: none.

`deepFreeze` is emitted when `source_value_mutation` or `derived_value_mutation` is `DeepFreeze`. It walks with an explicit stack, because a value can be deeper than the call stack allows, and it freezes plain objects and arrays. It skips a frozen value, so a value that the caller froze shallowly keeps its unfrozen children.

```js
// generated by crates/pico_js_generator/src/freeze.rs
function deepFreeze(value) {
  if (typeof value !== 'object' || value === null || Object.isFrozen(value)) {
    return;
  }
  const stack = [value];
  while (stack.length > 0) {
    const object = stack.pop();
    if (ArrayBuffer.isView(object) || object instanceof Map || object instanceof Set) {
      continue;
    }
    Object.freeze(object);
    for (const key of Object.keys(object)) {
      const child = object[key];
      if (typeof child === 'object' && child !== null && !Object.isFrozen(child)) {
        stack.push(child);
      }
    }
  }
}
```

A typed array is skipped because `Object.freeze` throws a `TypeError` on a typed array with elements. A `Map` or a `Set` is skipped because freezing one does not stop `set`, `add`, or `delete`.

## Generator

```rust
// from crates/pico_js_generator/src/lib.rs
/// The engine's JS for one configuration.
pub struct GeneratedEngine(pub String);

/// The JS module of the engine with `features`, formatted.
pub fn generate(features: &Features) -> GeneratedEngine;

/// The `.d.ts` that every configuration shares.
pub fn declarations() -> String;

/// The configurations the package ships, in the order the tests run them.
pub fn configurations() -> impl Iterator<Item = Configuration> {
    [Configuration::AllOff, Configuration::AllOn]
        .into_iter()
        .chain(FeatureName::iter().map(Configuration::Only))
}

impl Configuration {
    pub fn features(self) -> Features {
        match self {
            Configuration::AllOff => Features::all_off(),
            Configuration::AllOn => Features::all_on(),
            Configuration::Only(feature) => Features::only(feature),
        }
    }

    /// The directory under `libs/pico-js/generated` and the package subpath.
    pub fn name(self) -> String {
        match self {
            Configuration::AllOff => "all-off".to_string(),
            Configuration::AllOn => "all-on".to_string(),
            Configuration::Only(feature) => feature.to_string(),
        }
    }
}
```

`crates/pico_js_generator/src/main.rs` calls `generate` for each of `configurations()` and writes `libs/pico-js/generated/<name>/graph.js`, writes `declarations()` to `libs/pico-js/generated/graph.d.ts`, and writes `libs/pico-js/generated/configurations.json`, a list of `{ "name": string, "features": string[] }` whose `features` are the `FeatureName` strings that are on. `pnpm generate` in `libs/pico-js` runs `cargo run -p pico_js_generator -- generated`.

`generate` builds one `ModuleItem` per function of `graph.ts`, each from a Rust function that takes `&Features`, and prints the module through the emitter setup of `crates/typescript_artifacts/src/swc.rs` (`SingleThreadedComments` with a synthetic `BytePos` per comment, `OwnLineBlockComments`), then formats it with dprint. A prefactor moves that setup out of `typescript_artifacts` into a new crate, `crates/swc_printing`, that both depend on, because the generator does not depend on `isograph_compiler`. A feature's statements are a `Vec<Stmt>` that is empty when the feature is off:

```rust
// from crates/pico_js_generator/src/lifetime.rs
fn release_derived_node(features: &Features, comment_at: CommentAt) -> ModuleItem {
    let double_release: Vec<Stmt> = match features.double_release {
        Check::Skip => Vec::new(),
        Check::Throw => quote!(
            "if (mutableNode.retainCount === 0) { throw new Error('A node was released more often than it was retained.'); }"
                as Stmt
        )
        .wrap_vec(),
    };
    let body = [quote!("const mutableNode = node;" as Stmt)]
        .into_iter()
        .chain(double_release)
        .chain([
            quote!("mutableNode.retainCount -= 1;" as Stmt),
            quote!(
                "if (mutableNode.retainCount === 0 && mutableNode.firstDependent === undefined) { disposeUnneeded(mutableNode, undefined); }"
                    as Stmt
            ),
        ])
        .collect();
    exported_function(
        "releaseDerivedNode",
        &["node"],
        body,
        comment_at(RELEASE_DERIVED_NODE_DOC),
    )
}
```

A node field that exists only under a feature is decided once, from the features that read it, and `new_derived_node` and every check that reads the field call the same function:

```rust
// from crates/pico_js_generator/src/node.rs
/// Whether derived nodes carry `runState`.
enum RunStateField {
    Absent,
    Present,
}

fn run_state_field(features: &Features) -> RunStateField {
    match (features.cycle, features.read_outside_computation) {
        (Check::Skip, Check::Skip) => RunStateField::Absent,
        (Check::Throw, _) | (_, Check::Throw) => RunStateField::Present,
    }
}
```

`runState` is the last property of the node literal, so every node of a configuration has one shape and the all-off literal is the literal of `graph.ts`.

## Tests

Tests import the engine from `pico-js-engine`. Each vitest workspace project aliases it to one configuration's `graph.js`, and `tsconfig.test.json` maps it to `generated/graph.d.ts`:

```ts
// from libs/pico-js/vitest.workspace.ts
import { fileURLToPath } from 'node:url';
import { defineWorkspace } from 'vitest/config';
import configurations from './generated/configurations.json';

export default defineWorkspace(
  configurations.map(({ name, features }) => ({
    resolve: {
      alias: {
        'pico-js-engine': fileURLToPath(
          new URL(`./generated/${name}/graph.js`, import.meta.url),
        ),
      },
    },
    test: {
      name,
      include: [
        'test/graph.test.ts',
        ...features.map((feature) => `test/features/${feature}.test.ts`),
      ],
    },
  })),
);
```

```json
// from libs/pico-js/tsconfig.test.json
{
  "compilerOptions": {
    "paths": { "pico-js-engine": ["./generated/graph.d.ts"] }
  }
}
```

- `test/graph.test.ts` runs against every configuration: all off, all on, and each feature on by itself. Its cases keep every contract, so every configuration passes them unchanged.
- `test/features/<name>.test.ts` breaks the feature's contract and asserts the error message. It runs in every configuration where that feature is on: all on and the feature on by itself.
- `test-random/random.ts` runs against every configuration through a second workspace file with the same aliases. Its generated cases keep every contract.
- `test/graph.test-d.ts` runs once, against `generated/graph.d.ts`.

## Verification of the port

- The first generator commit reproduces `graph.ts` at `ba3daedf7`, including the `const mutableNode = node;` aliases that exist only for TypeScript's casts. tsdown prints `dist/graph.mjs` and `dist/graph.cjs` from `generated/all-off/graph.js` byte for byte equal to the files it prints from `src/graph.ts`. The comparison runs on tsdown's output, because tsdown reprints its input: built from `graph.ts`, it drops line comments, keeps JSDoc, writes `undefined` as `void 0`, drops the braces of a one-statement `if`, and indents with tabs. `swc_ecma_codegen` prints none of those the same way, so the generated file itself cannot equal `graph.ts`'s output.
- A later commit drops the aliases and any other TypeScript-only statement. `pnpm bench`, `pnpm bench single`, and `pnpm bench app` show the same numbers for all off as for `ba3daedf7`.

## Package

- `pnpm compile-libs` runs tsdown with one entry per configuration, `generated/<name>/graph.js`, into `dist/<name>/graph.mjs` and `dist/<name>/graph.cjs`, and copies `generated/graph.d.ts` to `dist/graph.d.mts` and `dist/graph.d.cts`.
- `package.json` exports `.` as `all-off` and `./<name>` for each other configuration, each with `types` pointing to the shared declarations.
- A consumer picks a configuration by its subpath, or by aliasing `@isograph/pico-js` to `@isograph/pico-js/all-on` in its development build.

## Undecided

- `mutation_inside_computation` and `mutation_inside_on_change` need module state: a counter and a flag declared at the top of the generated module, present only in configurations with the feature on. `writeSource(source, value)` and the other top-level functions receive no node or batch that could hold it. In those configurations the Guarantee that pico-js holds no module state does not hold: an `onChange` of one graph that writes a source of an unrelated graph throws. AGENTS.md requires raising module state before it goes into a plan. The other option is to leave these two contracts unchecked.
- pico-js.md lists a check that a node out of `Uncomputed` holds a computed value, through a private `NOT_SET` sentinel in `value` while `Uncomputed`. It is not a feature here, because `value` is a public field and the Guarantee says a disposed node holds `undefined`, so the sentinel would change what a caller reads in that configuration.
- No feature changes engine behavior yet. A behavior feature is a field whose enum has a variant per behavior, matched the same way, and tested with `test/graph.test.ts` in each configuration that has it on.
