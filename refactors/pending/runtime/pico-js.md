# pico-js: implementation

This doc is the complete source and package of `@isograph/pico-js` in `libs/pico-js`.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation, its last value, a status, the source and derived nodes its computation read, and the derived nodes that read it. Each dependency is an edge object that sits in the reader's dependency list, in read order, and in the dependents list of the node it read.
- A write marks the source's direct readers `Dirty` and the nodes above them `Check`. It runs no computation. Marking continues through a lazy node, created by `createDerivedNode`, and stops at an eager node, created by `createEagerDerivedNode`. A derived node's `kind` field holds `LAZY` or `EAGER`, and `setDerivedNodeKind` changes it. A source records that a write left every node in its dependents list `Dirty`, and until a computation reads the source again, a write to it visits none of those nodes.
- A derived node has one of four statuses: `Uncomputed`, `Dirty`, `Check`, and `Clean`. The `status` field holds one of the numeric constants `UNCOMPUTED`, `DIRTY`, `CHECK`, and `CLEAN`. A new node is `Uncomputed`.
- `pull` brings a node up to date. An `Uncomputed` node computes for the first time. A `Clean` node returns its value. A `Check` node pulls its derived dependencies in the order they were read and recomputes after the first pull that leaves it `Dirty`. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node, and that other dependency changed. If no pull leaves the node `Dirty`, it becomes `Clean` without running. A `Dirty` node recomputes.
- A computation receives its node and the previous value. An `Uncomputed` node's computation receives `undefined`, as Solid's `createMemo` passes it. A recompute whose value is `===` the previous value marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- Under `===`, a `NaN` value always counts as changed. Under `===`, `0` and `-0` count as equal.
- Each read during a computation records the node it read in the calling node's dependency list and links the calling node into the dependents list of the node it read. A recompute reuses the edges of the previous computation in read order and drops the edges past its last read.
- Code outside a computation reads a source's current value as `source.value`. An untracked read inside a computation, `source.value` or `pull(node)`, records no dependency and is allowed. A later change to a node read that way does not mark the computing node.
- Marking a node `Dirty` keeps its edges. A `Dirty` node stays in the dependents lists of the nodes it read, and marking that reaches it again changes nothing. Its next computation keeps each edge whose dependency it reads at the same position.
- Disposing a node marks its readers `Dirty` and returns the node to `Uncomputed`, the state of a new node.
- `startBatch(body)` creates a batch, calls `body` with it, and flushes the batch when `body` returns. A batch is an instance of the class `Batch`. Its methods `writeSource(source, value)`, `disposeDerivedNode(node)`, and `setDerivedNodeKind(node, kind)` take the parameters of the top-level functions of the same names, and it holds two arrays in the private fields `#eagerNodes` and `#callbacks`, each in the order they were added. Marking adds a node when it takes it out of `Clean`, which happens at most once until the node is pulled or refreshed.
- The top-level `writeSource`, `disposeDerivedNode`, and `setDerivedNodeKind` are a batch of one: each acts on two arrays of its own, then flushes them before it returns. The top-level `writeSource` and `disposeDerivedNode` allocate the two arrays only when they mark a dependents list.
- A `startBatch` or a top-level call inside the `body` of another `startBatch` or inside a flush flushes its own batch before it returns. Its callbacks can run while eager nodes of the outer batch are queued, and they read those nodes' previous values. Joining the outer batch would need module state that records the running batch, and the engine holds no module state.
- A `DerivedNode` has a writable field `onDirty: (() => void) | undefined`. Marking adds a lazy node's `onDirty` to `#callbacks` when the node goes from `Clean` to `Dirty` or `Check`. Two nodes that share one callback function add it twice.
- Marking that takes an eager node out of `Clean`, to `Dirty` or, through a derived dependency, to `Check`, appends the node to `#eagerNodes` and does not mark the node's dependents.
- `setDerivedNodeKind(node, kind)` sets `node.kind`. Setting the kind a node has, or the kind of an `Uncomputed` or `Clean` node, changes only that field. A `Dirty` or `Check` node set eager is appended to `#eagerNodes`, so the flush refreshes it. Its dependents are `Check` or `Dirty`, because it was lazy, and they stay as they are. A `Dirty` or `Check` node set lazy can have `Clean` dependents, because it was eager, so the call marks those dependents `Check` and the nodes above them as marking does above a `Dirty` node, collects their callbacks, queues the eager nodes among them, and adds the node's own `onDirty` to `#callbacks`, as marking does when a lazy node leaves `Clean`. The lazy node stays in `#eagerNodes`, and the flush skips a queued node that is lazy when the flush reaches it. A node set lazy and then eager again before the flush is in `#eagerNodes` twice, and the second refresh finds it `Clean` or lazy. A lazy node whose `onDirty` marking collected and that is set eager while `Dirty` or `Check` has its `onDirty` collected again when the refresh changes its value.
- The flush refreshes the next queued eager node while one is queued, and otherwise calls the next callback, until both arrays are drained. Refreshing verifies a `Check` eager node and recomputes it if a dependency changed, and recomputes a `Dirty` one. A recompute that returns a value `===` the previous one marks nothing. A recompute whose value changed adds the eager node's own `onDirty` to `#callbacks`, marks its dependents `Dirty` and the nodes above them `Check` through `markDirty`, and so collects their callbacks and queues the eager nodes among them into the batch being flushed. So every callback runs while no eager node of the batch is queued, and callbacks run in the order they were collected.
- A callback may pull, write, dispose, and set a node's kind, either through the top-level functions, each of which flushes its own batch before it returns, or through the methods of the batch being flushed, which append to its arrays, and the same flush drains them, eager nodes first. A collected callback is called even when an earlier callback disposed its node.
- A queued eager node that was disposed before the flush reaches it stays `Uncomputed`, and the flush does not compute it. A queued eager node that is `Clean` when the flush reaches it is left as it is, and so is a queued node that is lazy when the flush reaches it.
- A pull of a `Dirty` or `Check` eager node returns its previous value and runs nothing, whether the pull is outside a computation or a `readDerived` inside one. Only the flush of its batch refreshes it. A refresh inside a running computation could take out of `Clean` a node that the computation already read, while the running node is `Dirty` and marking skips it, and the running node would end `Clean` over a `Dirty` dependency that nothing marks again.

The engine does not check these contracts:

- The caller writes a source only when its value changed.
- Between a write and the flush of its batch, a marked eager node and a node that depends on it can return its previous value. Read after flushing. A batch that a `startBatch` or a top-level call inside `body` or inside a callback starts flushes while the outer batch's eager nodes are still queued, and its callbacks can read those nodes' previous values.
- A caller calls the methods of a batch only until its `startBatch` returns.
- A callback, and the `body` of `startBatch`, do not throw.
- A callback that writes eventually stops writing.
- A computation, and any function it calls, does not throw.
- A computation does not write a source, dispose a node, set a node's kind, or call `startBatch`.
- A computation is synchronous, and every `readSource` and `readDerived` call it makes receives, as `callingNode`, the node the computation received.
- `readSource` and `readDerived` are called only while the computation of the `callingNode` they receive is running.
- A computation's result depends only on what it reads through `readSource` and `readDerived` and on `previous`.
- A caller does not write a field of a source node or a derived node, except `onDirty`.
- A caller does not mutate a value that `pull`, `readSource`, or `readDerived` returns.
- The derived nodes form a directed acyclic graph.
- The chains of derived nodes that a pull descends stay well below the call stack limit. In Node 24 with its default stack, a pull of a chain of about 3,000 derived nodes with one-line computations overflows.

## The package

### Location and files

The engine is a workspace library at `libs/pico-js`, covered by `pnpm-workspace.yaml`'s `libs/*`. Its build and test setup copies `libs/isograph-runtime`'s without the Babel plugin, which only the runtime's fixtures need.

```text
libs/pico-js/
  package.json
  README.md
  tsconfig.json             references tsconfig.package.json and tsconfig.test.json
  tsconfig.package.json     extends ../../tsconfig.build.json, includes src/**/*.ts
  tsconfig.test.json        extends ../../tsconfig.build.json, includes test/**/*.ts
  tsdown.config.ts          merges ../../tsdown.config.ts, dts.build true
  vitest.config.ts          typecheck enabled with tsconfig.test.json
  src/graph.ts
  src/index.ts
  test/graph.test.ts, test/graph.test-d.ts   Tests
```

Adding the package changes `pnpm-lock.yaml` (a new importer, written by `pnpm install`), which the same commit includes.

### `package.json`

```json
// from libs/pico-js/package.json
{
  "name": "@isograph/pico-js",
  "description": "A graph of source nodes and derived nodes, recomputed on read when an input changed.",
  "private": true,
  "main": "./dist/index.cjs",
  "types": "./dist/index.d.cts",
  "license": "MIT",
  "scripts": {
    "compile-libs": "tsdown",
    "watch-libs": "tsdown --watch",
    "test": "vitest run",
    "test-watch": "vitest watch",
    "coverage": "vitest run --coverage",
    "prepack": "pnpm run test && pnpm run compile-libs",
    "tsc": "tsc -b",
    "tsc-force": "tsc --build --clean && tsc --build --force"
  },
  "dependencies": {},
  "devDependencies": {
    "typescript": "5.6.3",
    "vitest": "^2.1.2"
  },
  "repository": {
    "type": "git",
    "url": "git+https://github.com/isographlabs/isograph.git",
    "directory": "libs/pico-js"
  },
  "sideEffects": false,
  "module": "./dist/index.mjs",
  "exports": {
    ".": {
      "require": "./dist/index.cjs",
      "import": "./dist/index.mjs"
    },
    "./package.json": "./package.json"
  }
}
```

Deltas from `libs/isograph-runtime/package.json`: `name`, `description`, and `repository.directory`; `dependencies` is empty; `devDependencies` drops `@babel/core` and `@isograph/babel-plugin`. `private: true` stays, as the runtime's and `@isograph/disposable-types`'s do: publishing to npm is distribution, which `IN_PROGRESS.md` lists as not planned. The one export is `.`, built from `src/index.ts` by the root `tsdown.config.ts` (`entry: ['src/index.ts']`, `format: ['commonjs', 'esm']`).

### TypeScript, tsdown, vitest

`tsconfig.json`, `tsconfig.package.json`, and `tsdown.config.ts` are the runtime's, byte for byte (`libs/isograph-runtime/tsconfig.json`, `tsconfig.package.json`, `tsdown.config.ts`). The engine compiles under the root `tsconfig.build.json`: `target: ES2022`, `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `noUnusedLocals`, `noUnusedParameters`.

```json
// from libs/pico-js/tsconfig.json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.package.json" },
    { "path": "./tsconfig.test.json" }
  ]
}
```

```json
// from libs/pico-js/tsconfig.package.json
{
  "extends": "../../tsconfig.build.json",
  "include": ["src/**/*.ts"]
}
```

```ts
// from libs/pico-js/tsdown.config.ts
import { mergeConfig } from 'tsdown';
import baseConfig from '../../tsdown.config.ts';

export default mergeConfig(baseConfig, {
  dts: {
    build: true,
  },
});
```

```json
// from libs/pico-js/tsconfig.test.json
{
  "extends": "../../tsconfig.build.json",
  "compilerOptions": {
    "noUnusedLocals": false
  },
  "include": ["test/**/*.ts"],
  "exclude": ["**/node_modules", "**/dist"]
}
```

```ts
// from libs/pico-js/vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { typecheck: { enabled: true, tsconfig: './tsconfig.test.json' } },
});
```

Deltas of `tsconfig.test.json` from the runtime's: no `paths` entry and no `fixtures/**/*.ts`, because the tests import `../src` and there are no fixtures. Deltas of `vitest.config.ts`: no Babel plugin. With `typecheck.enabled`, vitest type-checks `graph.test-d.ts` as the runtime's `read.test-d.ts` is.

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`. Every type field is writable except `Edge.dependent`: `value`, `status`, `dependentsStatus`, and the edge-list fields change on every write, read, and recompute, a reused edge changes its `dependency`, a caller sets `onDirty`, and `setDerivedNodeKind` sets `kind`. `Batch` is the engine's one class. Its two arrays are ES private fields, `readonly` because a batch appends to them in place and never replaces them, and `index.ts` exports `Batch` as a type only, so a caller gets a batch only from `startBatch`. oxlint's `strict-boolean-expressions` (`.oxlintrc.json`) holds for every condition in the body. oxlint's `switch-exhaustiveness-check` holds for every `switch` in the body, and no `switch` has a `default` arm.

### Pre-commit hooks

`.pre-commit-config.yaml` gains two hooks, after `disposable-types-typecheck`:

```yaml
# from .pre-commit-config.yaml
      - id: pico-js-test
        name: engine test
        entry: pnpm --filter @isograph/pico-js test
        language: system
        files: '^libs/pico-js/'
        pass_filenames: false
      - id: pico-js-typecheck
        name: engine typecheck
        entry: pnpm --filter @isograph/pico-js tsc
        language: system
        files: '^(libs/pico-js/|tsconfig\.build\.json$)'
        pass_filenames: false
```

`prettier` and `oxlint` already cover every file in the workspace. When the Isograph runtime gains its dependency on the engine (pico-runtime.md core change 5), the runtime's three hooks also match the engine's directory:

```yaml
# from .pre-commit-config.yaml
      # runtime-dependencies-build and runtime-typecheck, files:
        files: '^(libs/isograph-runtime/|libs/isograph-disposable-types/|libs/pico-js/|tsconfig\.build\.json$)'
      # runtime-test, files:
        files: '^libs/(isograph-runtime|isograph-disposable-types|pico-js)/'
```

### CI

`.github/workflows/js.yml`'s `babel-plugin` job runs `pnpm compile-libs`, which turbo runs for every workspace package with a `compile-libs` script; turbo's `compile-libs` task depends on `tsc` (`turbo.json`), so the engine is type-checked and built there with no change. The job gains one step after "Test @isograph/disposable-types":

```yaml
# from .github/workflows/js.yml
      - name: Test @isograph/pico-js
        run: pnpm --filter @isograph/pico-js test
```

### `README.md`

````markdown
<!-- from libs/pico-js/README.md -->
# pico-js

pico-js is a push-pull memoization engine. A write to a source node eagerly marks the derived nodes that depend on it, and a derived node lazily recomputes when it is pulled.

## Model

The graph has two kinds of node. A source node holds a value that code outside the graph writes. A derived node holds a computation and the value that computation last returned. A computation reads other nodes through `readSource` and `readDerived`, and each read records the node it read as a dependency. A node depends on exactly the nodes its latest computation read, so a computation that reads different nodes on different runs has different dependencies after each run. A node keeps its dependencies while it is `Dirty`, and its next computation reuses them in read order, so a recompute that reads the same nodes as the previous one allocates nothing.

A write runs no computation. It marks the derived nodes that read the source `Dirty`, because one of their inputs is known to have changed, and it marks every node that depends on those through derived nodes `Check`, because one of their inputs might have changed. A write that reaches a node that is already `Dirty` changes nothing. A source records that a write left every node that read it `Dirty`, and until a computation reads the source again, a write to it visits none of those nodes. No lazy node is recomputed until it is pulled.

`startBatch(body)` calls `body` with a batch and flushes the batch when `body` returns. The batch's `writeSource`, `disposeDerivedNode`, and `setDerivedNodeKind` methods collect the `onDirty` callbacks of the observed nodes they take out of `Clean`, and the flush calls them after the last write, so a callback sees every write of its batch. The top-level `writeSource`, `disposeDerivedNode`, and `setDerivedNodeKind` are a batch of one: each acts, then flushes. A `startBatch` or a top-level call inside `body` or inside a callback flushes its own batch before it returns.

An eager derived node, created by `createEagerDerivedNode` or set eager by `setDerivedNodeKind(node, EAGER)`, stops marking. A write that marks it `Dirty` or `Check` queues it in the batch and marks none of its readers. The flush first recomputes every queued eager node, in the order they were queued. A recompute that returns the previous value marks nothing, so its readers stay `Clean` and no callback runs. A recompute whose value changed marks its readers as a write does. A selector that many readers read and that rarely changes value, such as `isSelected` for each row of a list, is eager, so a write marks only the readers whose selector changed. Until the flush, a marked eager node and the nodes that read it return their previous values.

A pull brings one node up to date and runs only the computations that requires. A `Dirty` node recomputes. A `Check` node pulls its derived dependencies in the order its computation read them. As soon as a pull leaves the node `Dirty`, the node recomputes, without pulling the rest. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node, and that other dependency changed. If no pull leaves the node `Dirty`, the node becomes `Clean` and returns its value without running. Because the pulls follow read order, a computation such as `readDerived(callingNode, flag) ? readDerived(callingNode, b) : readDerived(callingNode, c)` recomputes as soon as `flag` comes back changed, before it pulls `b`, which the new run may not read.

## Example

```ts
import {
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
  startBatch,
  writeSource,
} from '@isograph/pico-js';

const person = createSourceNode({ first: 'Ada', last: 'Lovelace', age: 36 });

const name = createDerivedNode<{
  readonly first: string;
  readonly last: string;
}>((callingNode, previous) => {
  const { first, last } = readSource(callingNode, person);
  return previous !== undefined &&
    previous.first === first &&
    previous.last === last
    ? previous
    : { first, last };
});

let greetingRuns = 0;
const greeting = createDerivedNode((callingNode) => {
  greetingRuns += 1;
  const { first, last } = readDerived(callingNode, name);
  return `Hello, ${first} ${last}`;
});

pull(greeting); // 'Hello, Ada Lovelace', greetingRuns === 1

writeSource(person, { first: 'Ada', last: 'Lovelace', age: 37 });
pull(greeting); // 'Hello, Ada Lovelace', greetingRuns === 1

startBatch((batch) => {
  batch.writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
  batch.writeSource(person, { first: 'Ada', last: 'King', age: 38 });
});
pull(greeting); // 'Hello, Ada King', greetingRuns === 2
```

After the first `writeSource`, `name` recomputes and returns `previous`, so `greeting` becomes `Clean` without running.

## API

- `createSourceNode(value)` returns a source node that holds `value`.
- `startBatch(body)` creates a batch, calls `body(batch)`, and then flushes the batch. The flush recomputes the queued eager nodes, then calls the collected callbacks in the order they were collected. Before each callback, it recomputes the eager nodes that the earlier callbacks' calls queued.
- `batch.writeSource(source, value)` stores `value` in `source`, marks the derived nodes that read it, and adds to `batch` each eager node that marking reaches and the `onDirty` of each other observed node that leaves `Clean`. It runs no computation.
- `writeSource(source, value)` is `batch.writeSource(source, value)` in a batch of its own, flushed before it returns.
- `createDerivedNode(computation)` returns an `Uncomputed` lazy derived node whose value `computation(callingNode, previous)` computes.
- `createEagerDerivedNode(computation)` returns an `Uncomputed` eager derived node.
- `readSource(callingNode, source)` returns the value of `source` and records `source` as a dependency of `callingNode`.
- `readDerived(callingNode, node)` pulls `node`, returns its value, and records `node` as a dependency of `callingNode`.
- `pull(node)` brings `node` up to date and returns its value.
- `batch.disposeDerivedNode(node)` marks the readers of `node` `Dirty`, adds their eager nodes and `onDirty` callbacks to `batch`, and returns `node` to `Uncomputed`. `disposeDerivedNode(node)` does the same in a batch of its own.
- `batch.setDerivedNodeKind(node, kind)` makes `node` lazy (`LAZY`) or eager (`EAGER`). A `Dirty` or `Check` node set eager is queued in `batch`, so the flush recomputes it. A `Dirty` or `Check` node set lazy marks its readers `Check`, as a write marks the nodes above a written source, and adds its own `onDirty` and theirs to `batch`. A node of any other status only changes kind. `setDerivedNodeKind(node, kind)` does the same in a batch of its own.
- `node.onDirty` is a callback that a write or disposal collects when the lazy node `node` leaves `Clean`, and that the flush collects when it recomputes the eager node `node` to a changed value. A collected callback is called even if its node is disposed or its `onDirty` is changed before the flush reaches it.
- `Batch` is the type of the batch that `startBatch` passes to `body`. Its eager nodes and callbacks are private fields.
- `SourceNode<TValue>` is the type of a source node whose value has type `TValue`.
- `DerivedNode<TValue>` is the type of a derived node whose value has type `TValue`.
- `Status` is the type of a derived node's `status` field, one of the exported constants `UNCOMPUTED`, `DIRTY`, `CHECK`, and `CLEAN`. A caller compares `node.status` with a constant, for example `node.status === CLEAN`.
- `DerivedNodeKind` is the type of a derived node's `kind` field and of `setDerivedNodeKind`'s `kind` parameter, one of the exported constants `LAZY` and `EAGER`.
- `DependentsStatus` is the type of a source node's `dependentsStatus` field: `1` means every derived node that read the source is `Dirty`, so a write visits none of them, and `0` means one of them may be `Check` or `Clean`, so a write marks them.

Each `readSource` and `readDerived` call during a computation records a dependency. A node's dependencies are the nodes its latest computation read. Code outside a computation reads a source's current value as `source.value`. An untracked read inside a computation, `source.value` or `pull(node)`, records no dependency and is allowed. A later change to a node read that way does not mark the computing node.

## Statuses

- An `Uncomputed` node (`status` `UNCOMPUTED`) has no value. Its next pull runs its computation with `previous` `undefined`. A computation whose value type includes `undefined` cannot tell its first run from a run after a stored `undefined`, because `previous` is `undefined` in both.
- A `Dirty` node (`status` `DIRTY`) read a source that was written, or a derived node whose value changed or that was disposed. Its next pull runs its computation with its last value as `previous`.
- A `Check` node (`status` `CHECK`) depends through derived nodes on a source that was written or on a derived node that was disposed. Its next pull pulls its derived dependencies in the order they were read and recomputes the node after the first pull that leaves it `Dirty`. If no pull leaves the node `Dirty`, the node becomes `Clean` without running.
- A `Clean` node (`status` `CLEAN`) holds an up-to-date value, which a pull returns without running anything, unless it depends on an eager node whose batch has not been flushed.
- A `Dirty` or `Check` eager node is queued in a batch. A pull returns its previous value without running anything, and the flush of that batch brings it up to date. A queued node that is set lazy before the flush is left for a pull to recompute.

## Cutoff

When a recompute returns a value `===` to its previous value, the node marks none of its readers `Dirty`, and a `Check` reader becomes `Clean` without running. The engine compares only by `===`. Structural sharing is the computation's responsibility: a computation that wants a structurally equal result to count as unchanged compares its new result with `previous` and returns `previous` when they are equal.

## Disposal

`disposeDerivedNode(node)` marks the readers of `node` `Dirty`, removes `node` from the dependents of every node it read, and discards its value. The node is `Uncomputed`, and its next pull computes it with `previous` `undefined`. Disposing an `Uncomputed` node changes nothing. A queued eager node that is disposed before the flush stays `Uncomputed`, and the flush does not compute it.

## Contracts

The engine does not check these contracts:

- The caller writes a source only when its value changed.
- Between a write and the flush of its batch, a marked eager node and a node that depends on it can return its previous value. Read after flushing. A batch that a `startBatch` or a top-level call inside `body` or inside a callback starts flushes while the outer batch's eager nodes are still queued, and its callbacks can read those nodes' previous values.
- A caller calls the methods of a batch only until its `startBatch` returns.
- A callback, and the `body` of `startBatch`, do not throw.
- A callback that writes eventually stops writing.
- A computation, and any function it calls, does not throw.
- A computation does not write a source, dispose a node, set a node's kind, or call `startBatch`.
- A computation is synchronous, and every `readSource` and `readDerived` call it makes receives, as `callingNode`, the node the computation received.
- `readSource` and `readDerived` are called only while the computation of the `callingNode` they receive is running.
- A computation's result depends only on what it reads through `readSource` and `readDerived` and on `previous`.
- A caller does not write a field of a source node or a derived node, except `onDirty`.
- A caller does not mutate a value that `pull`, `readSource`, or `readDerived` returns.
- The derived nodes form a directed acyclic graph.
- The chains of derived nodes that a pull descends stay well below the call stack limit. In Node 24 with its default stack, a pull of a chain of about 3,000 derived nodes with one-line computations overflows.
````

The contract list copies the Model section's list word for word, and the API, statuses, cutoff, and disposal sections describe `src/graph.ts` above.

### Readers before the Isograph runtime depends on it

AGENTS.md says we do not have anything we do not use, and tests-only is not a reader. Until the Isograph runtime calls it, the package's reader is the user's stated goal that the engine is usable by projects outside Isograph (pico-runtime.md, Goal), and `README.md` documents the package for those projects. pico-runtime.md core change 4 ships the package before its first workspace caller on that ground; core change 5 is the first caller of the source functions and core change 8 of the derived-node functions. The package ships alone, as its own change.

## `src/graph.ts`

```ts
// from libs/pico-js/src/graph.ts
const NOT_SET: unique symbol = Symbol('NOT_SET');

type Option<TValue> = TValue | typeof NOT_SET;

export const UNCOMPUTED = 0;
export const DIRTY = 1;
export const CHECK = 2;
export const CLEAN = 3;

export type Status =
  | typeof UNCOMPUTED
  | typeof DIRTY
  | typeof CHECK
  | typeof CLEAN;

/**
 * An edge from `dependent` to `dependency`. It is in the dependent's
 * dependency list, in read order, and in the dependency's dependents list.
 */
type Edge<TDependency> = {
  dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  nextDependency: Edge<TDependency> | undefined;
  previousDependent: Edge<TDependency> | undefined;
  nextDependent: Edge<TDependency> | undefined;
};

type SourceEdge = Edge<SourceNode<unknown>>;
type DerivedEdge = Edge<DerivedNode<unknown>>;

type Producer<TDependency> = {
  firstDependent: Edge<TDependency> | undefined;
  lastDependent: Edge<TDependency> | undefined;
};

/** A node in the dependents list may be `Check` or `Clean`. */
const DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY = 0;
/** Every node in the dependents list is `Dirty`, so a write skips the list. */
const DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY = 1;

export type DependentsStatus =
  | typeof DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY
  | typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;

/**
 * `dependentsStatus` is `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY` from a write that
 * marks the dependents list until the next `readSource`, because a node leaves
 * `Dirty` only by a recompute and joins the list only through `readSource`.
 */
export type SourceNode<TValue> = {
  value: TValue;
  firstDependent: SourceEdge | undefined;
  lastDependent: SourceEdge | undefined;
  dependentsStatus: DependentsStatus;
};

/** Recomputes when pulled. Marking continues through it. */
export const LAZY = 0;
/** Recomputes in the flush of its batch. Marking stops at it. */
export const EAGER = 1;

export type DerivedNodeKind = typeof LAZY | typeof EAGER;

/**
 * Invariants of a derived node whose computation is not running:
 * - `Uncomputed`: `value` is `NOT_SET`, both dependency lists are empty, and
 *   every dependent is `Dirty`. No dependents list holds the node.
 * - `Dirty`: `value` is the last computed value. A lazy node's dependents are
 *   `Check` or `Dirty`. An eager node is queued in an unflushed batch, and its
 *   dependents may be `Clean`.
 * - `Check`: as `Dirty`.
 * - `Clean`: `value` is up to date, unless the node depends through `Clean`
 *   nodes on a `Dirty` or `Check` eager node.
 *
 * In every status, each edge in the node's dependency lists is in its
 * dependency's dependents list. The type is not a union on `status`, because a
 * node keeps its identity and TypeScript does not narrow an object when a
 * field is assigned, so `computedValue` and marking check these at run time.
 */
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  onDirty: (() => void) | undefined;
  status: Status;
  kind: DerivedNodeKind;
  firstSourceDependency: SourceEdge | undefined;
  lastSourceDependency: SourceEdge | undefined;
  firstDerivedDependency: DerivedEdge | undefined;
  lastDerivedDependency: DerivedEdge | undefined;
  firstDependent: DerivedEdge | undefined;
  lastDependent: DerivedEdge | undefined;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return {
    value,
    firstDependent: undefined,
    lastDependent: undefined,
    dependentsStatus: DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY,
  };
}

export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return newDerivedNode(LAZY, computation);
}

export function createEagerDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return newDerivedNode(EAGER, computation);
}

function newDerivedNode<TValue>(
  kind: DerivedNodeKind,
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return {
    value: NOT_SET,
    onDirty: undefined,
    status: UNCOMPUTED,
    kind,
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
}

/**
 * The batch `startBatch` passes to its body. Each method acts as the top-level
 * function of the same name and defers its eager refreshes and `onDirty`
 * callbacks to the flush.
 */
export class Batch {
  readonly #eagerNodes: Array<DerivedNode<unknown>>;
  readonly #callbacks: Array<() => void>;

  constructor(
    eagerNodes: Array<DerivedNode<unknown>>,
    callbacks: Array<() => void>,
  ) {
    this.#eagerNodes = eagerNodes;
    this.#callbacks = callbacks;
  }

  writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void {
    const first = storeValue(source, value);
    if (first !== undefined) {
      markDirty(this.#eagerNodes, this.#callbacks, first);
    }
  }

  disposeDerivedNode(node: DerivedNode<unknown>): void {
    const first = node.firstDependent;
    resetToUncomputed(node);
    if (first !== undefined) {
      markDirty(this.#eagerNodes, this.#callbacks, first);
    }
  }

  setDerivedNodeKind(node: DerivedNode<unknown>, kind: DerivedNodeKind): void {
    setKind(this.#eagerNodes, this.#callbacks, node, kind);
  }
}

/** A nested `startBatch` flushes its own batch before it returns. */
export function startBatch(body: (batch: Batch) => void): void {
  const eagerNodes: Array<DerivedNode<unknown>> = [];
  const callbacks: Array<() => void> = [];
  body(new Batch(eagerNodes, callbacks));
  flush(eagerNodes, callbacks);
}

/**
 * Each callback runs while no eager node of the batch is queued. A node queued
 * twice, by setting it lazy and then eager, is `Clean` or lazy the second
 * time, so the eager list needs no deduplication.
 */
function flush(
  eagerNodes: Array<DerivedNode<unknown>>,
  callbacks: Array<() => void>,
): void {
  let eagerIndex = 0;
  let callbackIndex = 0;
  for (;;) {
    const eager = eagerNodes[eagerIndex];
    if (eager !== undefined) {
      eagerIndex += 1;
      refreshEager(eagerNodes, callbacks, eager);
      continue;
    }
    const callback = callbacks[callbackIndex];
    if (callback === undefined) {
      break;
    }
    callbackIndex += 1;
    callback();
  }
}

/** A batch of one write. */
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  const first = storeValue(source, value);
  if (first !== undefined) {
    const eagerNodes: Array<DerivedNode<unknown>> = [];
    const callbacks: Array<() => void> = [];
    markDirty(eagerNodes, callbacks, first);
    flush(eagerNodes, callbacks);
  }
}

/** Returns the first edge of the dependents list if the write must mark it. */
function storeValue<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): SourceEdge | undefined {
  source.value = value;
  const first = source.firstDependent;
  if (
    first === undefined ||
    source.dependentsStatus === DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY
  ) {
    return undefined;
  }
  source.dependentsStatus = DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;
  return first;
}

function linkEdge<TDependency extends Producer<TDependency>>(
  edge: Edge<TDependency>,
  dependency: Producer<TDependency>,
): void {
  const tail = dependency.lastDependent;
  edge.previousDependent = tail;
  edge.nextDependent = undefined;
  if (tail === undefined) {
    dependency.firstDependent = edge;
  } else {
    tail.nextDependent = edge;
  }
  dependency.lastDependent = edge;
}

function unlinkEdge<TDependency>(
  edge: Edge<TDependency>,
  dependency: Producer<TDependency>,
): void {
  const previous = edge.previousDependent;
  const next = edge.nextDependent;
  if (previous === undefined) {
    dependency.firstDependent = next;
  } else {
    previous.nextDependent = next;
  }
  if (next === undefined) {
    dependency.lastDependent = previous;
  } else {
    next.previousDependent = previous;
  }
}

export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  if (source.dependentsStatus === DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY) {
    source.dependentsStatus = DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY;
  }
  const last = callingNode.lastSourceDependency;
  const next =
    last === undefined
      ? callingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== source) {
      unlinkEdge(next, next.dependency);
      next.dependency = source as SourceNode<unknown>;
      linkEdge(next, source as SourceNode<unknown>);
    }
    callingNode.lastSourceDependency = next;
  } else if (last === undefined || last.dependency !== source) {
    const edge: SourceEdge = {
      dependency: source as SourceNode<unknown>,
      dependent: callingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge, source as SourceNode<unknown>);
    if (last === undefined) {
      callingNode.firstSourceDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    callingNode.lastSourceDependency = edge;
  }
  return source.value;
}

export function readDerived<TValue>(
  callingNode: DerivedNode<unknown>,
  node: DerivedNode<TValue>,
): TValue {
  const value = pull(node);
  const last = callingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? callingNode.firstDerivedDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== node) {
      unlinkEdge(next, next.dependency);
      next.dependency = node as DerivedNode<unknown>;
      linkEdge(next, node as DerivedNode<unknown>);
    }
    callingNode.lastDerivedDependency = next;
  } else if (last === undefined || last.dependency !== node) {
    const edge: DerivedEdge = {
      dependency: node as DerivedNode<unknown>,
      dependent: callingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge, node as DerivedNode<unknown>);
    if (last === undefined) {
      callingNode.firstDerivedDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    callingNode.lastDerivedDependency = edge;
  }
  return value;
}

/** A `Dirty` or `Check` eager node returns its previous value. */
export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  switch (node.status) {
    case CLEAN:
      return computedValue(node);
    case DIRTY:
      return node.kind === LAZY ? recompute(node) : computedValue(node);
    case CHECK:
      if (node.kind === EAGER) {
        return computedValue(node);
      }
      return verify(node) === DIRTY ? recompute(node) : computedValue(node);
    case UNCOMPUTED:
      return computeFirst(node);
  }
}

/**
 * A node set lazy after it was queued is skipped, because setting it lazy
 * marked its dependents.
 */
function refreshEager(
  eagerNodes: Array<DerivedNode<unknown>>,
  callbacks: Array<() => void>,
  node: DerivedNode<unknown>,
): void {
  if (node.kind === LAZY) {
    return;
  }
  switch (node.status) {
    case DIRTY:
      recomputeEager(eagerNodes, callbacks, node);
      break;
    case CHECK:
      if (verify(node) === DIRTY) {
        recomputeEager(eagerNodes, callbacks, node);
      }
      break;
    case CLEAN:
    case UNCOMPUTED:
      break;
  }
}

/** A batch of one disposal. */
export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const first = node.firstDependent;
  resetToUncomputed(node);
  if (first !== undefined) {
    const eagerNodes: Array<DerivedNode<unknown>> = [];
    const callbacks: Array<() => void> = [];
    markDirty(eagerNodes, callbacks, first);
    flush(eagerNodes, callbacks);
  }
}

/** Leaves the dependents list for marking. */
function resetToUncomputed(node: DerivedNode<unknown>): void {
  unlinkAll(node.firstSourceDependency);
  unlinkAll(node.firstDerivedDependency);
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = UNCOMPUTED;
  node.value = NOT_SET;
}

/** A batch of one kind change. */
export function setDerivedNodeKind(
  node: DerivedNode<unknown>,
  kind: DerivedNodeKind,
): void {
  const eagerNodes: Array<DerivedNode<unknown>> = [];
  const callbacks: Array<() => void> = [];
  setKind(eagerNodes, callbacks, node, kind);
  flush(eagerNodes, callbacks);
}

/**
 * A `Dirty` or `Check` node set lazy may have `Clean` dependents, because it
 * was eager, so its dependents are marked to restore the lazy invariant.
 */
function setKind(
  eagerNodes: Array<DerivedNode<unknown>>,
  callbacks: Array<() => void>,
  node: DerivedNode<unknown>,
  kind: DerivedNodeKind,
): void {
  if (node.kind === kind) {
    return;
  }
  node.kind = kind;
  switch (node.status) {
    case DIRTY:
    case CHECK:
      switch (kind) {
        case EAGER:
          eagerNodes.push(node);
          break;
        case LAZY:
          if (node.onDirty !== undefined) {
            callbacks.push(node.onDirty);
          }
          markCheck(eagerNodes, callbacks, node, undefined);
          break;
      }
      break;
    case CLEAN:
    case UNCOMPUTED:
      break;
  }
}

function markDirty<TDependency>(
  eagerNodes: Array<DerivedNode<unknown>>,
  callbacks: Array<() => void>,
  first: Edge<TDependency>,
): void {
  let stack: Array<DerivedNode<unknown>> | undefined;
  let held: DerivedNode<unknown> | undefined;
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case CLEAN:
        node.status = DIRTY;
        if (node.kind === EAGER) {
          eagerNodes.push(node);
          break;
        }
        if (node.onDirty !== undefined) {
          callbacks.push(node.onDirty);
        }
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
        }
        break;
      case CHECK:
        node.status = DIRTY;
        break;
      case DIRTY:
        break;
      case UNCOMPUTED:
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
  if (held !== undefined) {
    markCheck(eagerNodes, callbacks, held, stack);
  }
}

/**
 * Marks `Clean` dependents of `held` and of each node on the stack `Check`,
 * and continues through each lazy node it marks.
 */
function markCheck(
  eagerNodes: Array<DerivedNode<unknown>>,
  callbacks: Array<() => void>,
  held: DerivedNode<unknown>,
  initialStack: Array<DerivedNode<unknown>> | undefined,
): void {
  let stack = initialStack;
  let marked: DerivedNode<unknown> | undefined = held;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case CLEAN:
          dependent.status = CHECK;
          if (dependent.kind === EAGER) {
            eagerNodes.push(dependent);
            break;
          }
          if (dependent.onDirty !== undefined) {
            callbacks.push(dependent.onDirty);
          }
          if (dependent.firstDependent !== undefined) {
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          }
          break;
        case CHECK:
        case DIRTY:
          break;
        case UNCOMPUTED:
          throw new Error(
            'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
          );
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
}

/**
 * Pulls the derived dependencies of the `Check` node `root` in read order,
 * descending into lazy `Check` dependencies without recursion. Returns `DIRTY`
 * when the caller must recompute `root`.
 */
function verify(root: DerivedNode<unknown>): typeof DIRTY | typeof CLEAN {
  let verifyStack: Array<DerivedEdge> | undefined;
  let node: DerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  for (;;) {
    if (edge !== undefined) {
      const dependency = edge.dependency;
      if (dependency.status === CHECK && dependency.kind === LAZY) {
        if (
          edge.previousDependent !== undefined ||
          edge.nextDependent !== undefined
        ) {
          (verifyStack ??= []).push(edge);
        }
        node = dependency;
        edge = dependency.firstDerivedDependency;
        continue;
      }
      pull(dependency);
      if (node.status !== DIRTY) {
        edge = edge.nextDependency;
        continue;
      }
      if (node === root) {
        return DIRTY;
      }
      recompute(node);
    } else {
      node.status = CLEAN;
    }
    for (;;) {
      if (node === root) {
        return CLEAN;
      }
      const only = node.firstDependent;
      if (only === undefined) {
        throw new Error(
          'A node that verify descended into has no dependents. This is indicative of a bug in pico-js.',
        );
      }
      if (only.nextDependent === undefined) {
        edge = only;
      } else {
        const popped = verifyStack?.pop();
        if (popped === undefined) {
          throw new Error(
            'A node that verify descended into has more than one dependent and no edge on the verification stack. This is indicative of a bug in pico-js.',
          );
        }
        edge = popped;
      }
      node = edge.dependent;
      if (node.status !== DIRTY) {
        break;
      }
      if (node === root) {
        return DIRTY;
      }
      recompute(node);
    }
    edge = edge.nextDependency;
  }
}

function computeFirst<TValue>(node: DerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  trim(node);
  node.status = CLEAN;
  node.value = value;
  return value;
}

function recompute<TValue>(node: DerivedNode<TValue>): TValue {
  const previous = computedValue(node);
  const value = rerun(node, previous);
  if (value === previous) {
    return previous;
  }
  node.value = value;
  markCheckDependentsDirty(node.firstDependent);
  return value;
}

/**
 * An eager node's dependents may be `Clean`, so a changed value marks them
 * through `markDirty`. Its `onDirty` runs only when its value changed.
 */
function recomputeEager(
  eagerNodes: Array<DerivedNode<unknown>>,
  callbacks: Array<() => void>,
  node: DerivedNode<unknown>,
): void {
  const previous = computedValue(node);
  const value = rerun(node, previous);
  if (value === previous) {
    return;
  }
  node.value = value;
  if (node.onDirty !== undefined) {
    callbacks.push(node.onDirty);
  }
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(eagerNodes, callbacks, first);
  }
}

function rerun<TValue>(node: DerivedNode<TValue>, previous: TValue): TValue {
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  trim(node);
  node.status = CLEAN;
  return value;
}

function markCheckDependentsDirty(first: DerivedEdge | undefined): void {
  let edge = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case CHECK:
        node.status = DIRTY;
        break;
      case DIRTY:
        break;
      case CLEAN:
        throw new Error(
          'A Clean node depends on a Dirty node. This is indicative of a bug in pico-js.',
        );
      case UNCOMPUTED:
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
}

function computedValue<TValue>(node: DerivedNode<TValue>): TValue {
  if (node.value === NOT_SET) {
    throw new Error(
      'A node that is not Uncomputed has no value. This is indicative of a bug in pico-js.',
    );
  }
  return node.value;
}

/** Unlinks and drops the edges after the last one the computation read. */
function trim(node: DerivedNode<unknown>): void {
  const lastSource = node.lastSourceDependency;
  if (lastSource === undefined) {
    unlinkAll(node.firstSourceDependency);
    node.firstSourceDependency = undefined;
  } else if (lastSource.nextDependency !== undefined) {
    unlinkAll(lastSource.nextDependency);
    lastSource.nextDependency = undefined;
  }
  const lastDerived = node.lastDerivedDependency;
  if (lastDerived === undefined) {
    unlinkAll(node.firstDerivedDependency);
    node.firstDerivedDependency = undefined;
  } else if (lastDerived.nextDependency !== undefined) {
    unlinkAll(lastDerived.nextDependency);
    lastDerived.nextDependency = undefined;
  }
}

function unlinkAll<TDependency extends Producer<TDependency>>(
  first: Edge<TDependency> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    unlinkEdge(edge, edge.dependency);
    edge = edge.nextDependency;
  }
}
```

- `computation` is a method signature. TypeScript checks a method's parameters bivariantly, so `DerivedNode<TValue>` is assignable to `DerivedNode<unknown>`.
- `Status` is numeric because a `switch` and a comparison on small integers cost less than on strings: numeric statuses cut the upstream benchmark total by 4.6% (pico-js-performance.md, change 5). `graph.ts` exports a `const` for each number (`UNCOMPUTED`, `DIRTY`, `CHECK`, `CLEAN`, and for `DerivedNodeKind`, `LAZY` and `EAGER`), and `Status` and `DerivedNodeKind` are unions of their `typeof`s, so the code names each value and a caller compares `status` with a constant.
- A read links its edge into the dependency's dependents list at the time of the read, and `readDerived` pulls the dependency before it links the edge, so a node is linked only to a source, to a `Clean` derived node, or to a `Dirty` or `Check` eager node whose previous value it read.
- `recompute` resets `lastSourceDependency` and `lastDerivedDependency`, and each read takes the next edge of the list. An edge whose dependency is the node read stays linked. An edge whose dependency differs is unlinked from the old dependency and linked to the node read. A read past the end of the list allocates an edge, unless the previous edge already points at the node read. `trim` unlinks and drops the edges past the last read. So a recompute that reads the same nodes in the same order allocates, links, and unlinks nothing, and after a computation the dependency lists hold exactly its reads, with adjacent reads of one node sharing one edge.
- A node that reads a dependency at two non-adjacent positions (`a, b, a`) has two edges in `a`'s dependents list. Marking visits it twice, and the second visit finds it `Dirty` and changes nothing.
- Marking unlinks no edge. A `Dirty` node stays in the dependents lists of its dependencies, and marking skips it, because its dependents are already `Check` or `Dirty`. A `Check` node that marking sets `Dirty` is not pushed, for the same reason. So repeated writes to different sources of a node that is not pulled visit that node on each write and stop there.
- `writeSource` marks the dependents list only when `dependentsStatus` is `DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY`, and sets it to `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`, because marking from a source sets every direct reader `Dirty`. A direct reader leaves `Dirty` only through a recompute, which either reads the source again, and `readSource` sets `DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY`, or does not, and `trim` unlinks its edge. A node joins the list only through `readSource`, and disposal only removes nodes from it. So a write that finds `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY` would change nothing, and it visits nothing. Repeated writes to one source between reads of it visit its readers once (pico-js-losses.md, flag2).
- `DependentsStatus` is a union of two numbers with a constant for each, private to `graph.ts`, so the hot path compares small integers as it does for `Status`, and each value has a name in the code.
- Marking pushes a node onto its stack only when the node leaves `Clean` and its dependents list is not empty, so a write visits each node's dependents at most once. Marking holds the last node it must descend into in a local and allocates its stack on the first push, and it descends into a dependent that is the last entry of its list without pushing it, so a chain, or a fan-out whose readers have no readers, allocates no stack. The marking stack and the verification stack are arrays local to the call, so the depth of a marked or verified chain does not reach the call stack, and the engine holds no module state (pico-js-losses.md, fast2).
- A node becomes `Clean` only after its computation returns or after `verify` has pulled every derived dependency it is linked to. Each lazy dependency a computation links is `Clean` when it is read, and marking continues through every lazy node, so every dependent of a lazy node that is not `Clean` is `Check` or `Dirty`. A lazy recompute therefore marks only `Check` and `Dirty` nodes and takes no node out of `Clean`, so it marks through `markCheckDependentsDirty`, which walks only the node's dependents list, needs no stack, and throws on a `Clean` or `Uncomputed` dependent. A pull outside a flush recomputes only lazy nodes, so it takes no node out of `Clean` and takes no batch. A lazy node that `setDerivedNodeKind` makes lazy while `Dirty` or `Check` had its `Clean` dependents marked `Check` by that call, so this holds for it too.
- An eager node that is `Dirty` or `Check` may have `Clean` dependents, and only until the flush of the batch whose `#eagerNodes` holds it. Marking queues it when it leaves `Clean`, and `setKind` queues a `Dirty` or `Check` node it makes eager, so every `Dirty` or `Check` eager node is queued. The flush refreshes it at the top level, outside any computation, through `refreshEager`. `recomputeEager` marks a changed value through `markDirty`, which takes the arrays of the batch being flushed, takes `Clean` dependents out of `Clean`, collects their callbacks, and queues the eager nodes among them. It collects the eager node's own `onDirty` there, when its value changed, and not when marking reached the node, so an observer of an eager node is called only when the node's value changed.
- `setKind` restores the lazy invariant when it makes a `Dirty` or `Check` eager node lazy: it marks the node's `Clean` dependents `Check` through `markCheck`, the second phase of `markDirty`, which starts from the node as if marking had just taken it out of `Clean`. Leaving the node queued and its dependents `Clean` until the flush would break the invariant until then: a pull of the lazy `Dirty` node would recompute it and throw in `markCheckDependentsDirty` on a `Clean` dependent, and marking that reached the node would stop there and leave its `Clean` dependents stale. `refreshEager` skips a lazy node, because its dependents are already `Check` or `Dirty` and its `onDirty` is already collected, and a refresh would collect it again.
- `startBatch` allocates the two arrays and one `Batch` per call. The methods of `Batch` are prototype methods over its private fields, so a batch allocates no closure. The top-level `writeSource` and `disposeDerivedNode` allocate no `Batch`, and allocate their two arrays only when they mark a dependents list. On Node 24.12, a loop of 2,000,000 writes each followed by a pull of one derived node that reads the written source takes, per iteration, median of nine runs (v0, untrusted): 9.8 ns for the top-level `writeSource` with no `onDirty`, 18.8 ns with an `onDirty`, and 16.2 ns and 23.6 ns for `startBatch` with one `batch.writeSource`.
- `flush` reads both arrays by index and checks `#eagerNodes` before each callback, so the entries that a refresh or a callback appends are drained in the same flush, and no callback runs while an eager node of the batch is queued. It does not empty the arrays, because a batch is flushed once, when its `startBatch` body returns.
- While a computation runs, its node is `Uncomputed` or `Dirty` and is linked to what it has read so far. Every node it read is `Clean`, or is a `Dirty` or `Check` eager node whose previous value it read. A pull inside the computation recomputes only lazy nodes, which take no node out of `Clean`, so nothing takes a node the computation read out of `Clean` before the computation returns, and marking never reaches the running node while it is `Uncomputed`.
- `verify` walks the derived dependency lists in read order with an explicit stack of edges. It descends into a lazy `Check` dependency before its reader continues and pulls any other dependency, so a `Check` eager dependency returns its previous value. After a pull that leaves the current node `Dirty`, it recomputes that node and returns to the node's reader. It does not recompute `root`. It returns the status it leaves `root` in, `DIRTY` or `CLEAN`, and its caller recomputes a `Dirty` root: `pull` through `recompute`, and `refreshEager` through `recomputeEager`. The return value carries the status because TypeScript narrows `node.status` in the caller's `switch` and does not see `verify` change it. The order of pulls and recomputes is that of a recursive `verify`. It pushes an edge only when the dependency it descends into has more than one dependent, and it allocates the stack on the first push. A node with one dependent returns to its reader through its only dependents-list edge, which is the edge `verify` descended through. `verify` throws when a node it descended into has no dependents, or has more than one and no edge on the stack, because a broken invariant throws in this library and the type system cannot express either invariant. A first computation still recurses through `readDerived` and `pull`.
- Because a write marks the source's direct readers `Dirty`, a `Check` node has no changed source, and `verify` pulls only its derived dependencies. A dependency's `recompute` marks its readers `Dirty` exactly when its new value is not `===` its previous value, so after a pull in `verify` the node is `Dirty` exactly when a lazy dependency it read changed value. A `Dirty` or `Check` eager dependency returns its previous value and leaves the node as it is, and its flush marks the node if its value changed.
- A node's `status` is `Uncomputed` exactly when its `value` is `NOT_SET`. In the other three statuses, `value` holds a value the computation returned, which can be `undefined`, so a `Dirty` node always has a value for its next computation to receive as `previous`. `NOT_SET` and `Option` are private to `graph.ts`, so no `TValue` contains `NOT_SET`. `computedValue` throws if a node that is not `Uncomputed` holds `NOT_SET`, because a broken invariant throws in this library. The check costs one comparison per pull of a `Clean` node.
- A first computation has no cutoff and marks nothing, because every node in an `Uncomputed` node's dependents list is already `Dirty`: a new node has no readers, and disposal marked every reader `Dirty`.
- On cutoff the node keeps its stored value, so a recompute that returns `-0` where the stored value is `0` leaves `0` stored and returned, and every reader of the node reads the same value.
- Disposal marks every reader `Dirty`, unlinks the node's own edges, and discards its value, so its next computation receives `previous` `undefined`. The readers stay in the node's dependents list, and each reader's next recompute either reads the node again, which computes it first, or drops the edge. An `Uncomputed` node has empty dependency lists and only `Dirty` dependents, so its disposal changes nothing. A `Dirty` node's dependents are `Check` or `Dirty`, and its disposal marks them `Dirty`. Disposal resets the node before it marks its readers, so a callback of the batch of one that the top-level `disposeDerivedNode` flushes finds the node `Uncomputed`. A disposed eager node that is still in `#eagerNodes` is `Uncomputed` when the flush reaches it, and `refreshEager` leaves it `Uncomputed`, because computing it would link it to its sources again. A `Clean` eager node in `#eagerNodes` is one that was disposed and then pulled, or one that is in the list twice, and `refreshEager` leaves it as it is.

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export {
  CHECK,
  CLEAN,
  createDerivedNode,
  createEagerDerivedNode,
  createSourceNode,
  DIRTY,
  disposeDerivedNode,
  EAGER,
  LAZY,
  pull,
  readDerived,
  readSource,
  setDerivedNodeKind,
  startBatch,
  UNCOMPUTED,
  writeSource,
} from './graph';
export type {
  Batch,
  DependentsStatus,
  DerivedNode,
  DerivedNodeKind,
  SourceNode,
  Status,
} from './graph';
```

## Tests

```ts
// from libs/pico-js/test/graph.test.ts
import { describe, expect, test } from 'vitest';
import {
  CHECK,
  CLEAN,
  createDerivedNode,
  createEagerDerivedNode,
  createSourceNode,
  DIRTY,
  disposeDerivedNode,
  EAGER,
  LAZY,
  pull,
  readDerived,
  readSource,
  setDerivedNodeKind,
  startBatch,
  UNCOMPUTED,
  writeSource,
  type DerivedNode,
  type SourceNode,
} from '../src/index';

describe('pull', () => {
  test('a derived node runs nothing until it is pulled, and a second pull of the Clean node runs nothing', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) * 2;
    });
    expect(runs).toBe(0);
    expect(double.status).toBe(UNCOMPUTED);
    expect(pull(double)).toBe(2);
    expect(double.status).toBe(CLEAN);
    expect(pull(double)).toBe(2);
    expect(runs).toBe(1);
  });

  test('a node created inside a computation and read through readDerived is a dependency of its reader', () => {
    const source = createSourceNode(1);
    let inner: DerivedNode<number> | undefined;
    const outer = createDerivedNode((callingNode) => {
      inner ??= createDerivedNode(
        (innerCallingNode) => readSource(innerCallingNode, source) * 10,
      );
      return readDerived(callingNode, inner);
    });
    expect(pull(outer)).toBe(10);
    writeSource(source, 2);
    expect(outer.status).toBe(CHECK);
    expect(pull(outer)).toBe(20);
  });

  test('the first computation receives undefined as previous and a recompute receives the previous value', () => {
    const previousValues: Array<number | undefined> = [];
    const a = createSourceNode(1);
    const double = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, a) * 2;
    });
    pull(double);
    writeSource(a, 2);
    pull(double);
    expect(previousValues).toEqual([undefined, 2]);
  });
});

describe('marking', () => {
  test('a write marks direct readers Dirty and their readers Check and runs nothing', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs.sum += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const label = createDerivedNode((callingNode) => {
      runs.label += 1;
      return readDerived(callingNode, sum) > 2 ? 'big' : 'small';
    });
    expect(pull(label)).toBe('big');
    writeSource(a, 5);
    expect(sum.status).toBe(DIRTY);
    expect(label.status).toBe(CHECK);
    expect(runs).toEqual({ sum: 1, label: 1 });
  });

  test('in a diamond, a write marks the shared node Dirty and every node above it Check', () => {
    const source = createSourceNode(1);
    const shared = createDerivedNode(
      (callingNode) => readSource(callingNode, source) + 1,
    );
    const left = createDerivedNode(
      (callingNode) => readDerived(callingNode, shared) * 2,
    );
    const right = createDerivedNode(
      (callingNode) => readDerived(callingNode, shared) * 3,
    );
    const top = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, left) + readDerived(callingNode, right),
    );
    pull(top);
    writeSource(source, 2);
    expect([shared.status, left.status, right.status, top.status]).toEqual([
      DIRTY,
      CHECK,
      CHECK,
      CHECK,
    ]);
  });

  test('a write marks a chain of 100,000 derived nodes', () => {
    const source = createSourceNode(0);
    const bottom = createDerivedNode((callingNode) =>
      readSource(callingNode, source),
    );
    pull(bottom);
    const top = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (below) => {
        const node = createDerivedNode(
          (callingNode) => readDerived(callingNode, below) + 1,
        );
        pull(node);
        return node;
      },
      bottom,
    );
    writeSource(source, 1);
    expect(bottom.status).toBe(DIRTY);
    expect(top.status).toBe(CHECK);
  });
});

describe('cutoff', () => {
  test('a dependency that recomputes to the same value leaves its reader Clean without running it', () => {
    const runs = { sum: 0, label: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs.sum += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const label = createDerivedNode((callingNode) => {
      runs.label += 1;
      return readDerived(callingNode, sum) > 2 ? 'big' : 'small';
    });
    pull(label);
    writeSource(a, 0);
    writeSource(b, 3);
    expect(pull(label)).toBe('big');
    expect(label.status).toBe(CLEAN);
    expect(runs).toEqual({ sum: 2, label: 1 });
  });

  test('a computation that returns previous leaves its readers Clean without running them', () => {
    let readerRuns = 0;
    const a = createSourceNode(1);
    const parity = createDerivedNode<{ readonly even: boolean }>(
      (callingNode, previous) => {
        const even = readSource(callingNode, a) % 2 === 0;
        return previous !== undefined && previous.even === even
          ? previous
          : { even };
      },
    );
    const reader = createDerivedNode((callingNode) => {
      readerRuns += 1;
      return readDerived(callingNode, parity).even;
    });
    const first = pull(parity);
    pull(reader);
    writeSource(a, 3);
    expect(pull(reader)).toBe(false);
    expect(reader.status).toBe(CLEAN);
    expect(pull(parity)).toBe(first);
    expect(readerRuns).toBe(1);
  });

  test('a node whose computation returns undefined is Clean, and a recompute that returns undefined again leaves its reader Clean without running it', () => {
    const runs = { node: 0, reader: 0 };
    const previousValues: Array<undefined> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode<undefined>((callingNode, previous) => {
      runs.node += 1;
      previousValues.push(previous);
      readSource(callingNode, a);
      return undefined;
    });
    const reader = createDerivedNode((callingNode) => {
      runs.reader += 1;
      return readDerived(callingNode, node) === undefined;
    });
    expect(pull(reader)).toBe(true);
    expect(node.status).toBe(CLEAN);
    expect(pull(node)).toBeUndefined();
    expect(runs).toEqual({ node: 1, reader: 1 });
    writeSource(a, 2);
    expect(node.status).toBe(DIRTY);
    expect(pull(reader)).toBe(true);
    expect(reader.status).toBe(CLEAN);
    expect(runs).toEqual({ node: 2, reader: 1 });
    expect(previousValues).toEqual([undefined, undefined]);
  });

  test('a projection of a new object that recomputes to the same value leaves its reader Clean without running it', () => {
    const runs = { d1: 0, d2: 0, d3: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const d1 = createDerivedNode((callingNode) => {
      runs.d1 += 1;
      return { a: readSource(callingNode, a), b: readSource(callingNode, b) };
    });
    const d2 = createDerivedNode((callingNode) => {
      runs.d2 += 1;
      return readDerived(callingNode, d1).b;
    });
    const d3 = createDerivedNode((callingNode) => {
      runs.d3 += 1;
      return readDerived(callingNode, d2) * 2;
    });
    expect(pull(d3)).toBe(4);
    writeSource(a, 10);
    expect(pull(d3)).toBe(4);
    expect(runs).toEqual({ d1: 2, d2: 2, d3: 1 });
  });
});

describe('verification', () => {
  test('in a diamond each node runs once per change', () => {
    const source = createSourceNode(1);
    const runs = { shared: 0, left: 0, right: 0, top: 0 };
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readSource(callingNode, source) + 1;
    });
    const left = createDerivedNode((callingNode) => {
      runs.left += 1;
      return readDerived(callingNode, shared) * 2;
    });
    const right = createDerivedNode((callingNode) => {
      runs.right += 1;
      return readDerived(callingNode, shared) * 3;
    });
    const top = createDerivedNode((callingNode) => {
      runs.top += 1;
      return readDerived(callingNode, left) + readDerived(callingNode, right);
    });
    expect(pull(top)).toBe(10);
    writeSource(source, 2);
    expect(pull(top)).toBe(15);
    expect(runs).toEqual({ shared: 2, left: 2, right: 2, top: 2 });
  });

  test('verification stops at the first derived dependency whose value changed', () => {
    const gateSource = createSourceNode(1);
    const otherSource = createSourceNode(1);
    const runs = { gate: 0, other: 0, node: 0 };
    const gate = createDerivedNode((callingNode) => {
      runs.gate += 1;
      return readSource(callingNode, gateSource);
    });
    const other = createDerivedNode((callingNode) => {
      runs.other += 1;
      return readSource(callingNode, otherSource);
    });
    const node = createDerivedNode((callingNode) => {
      runs.node += 1;
      return readDerived(callingNode, gate) === 1
        ? readDerived(callingNode, other)
        : 0;
    });
    expect(pull(node)).toBe(1);
    writeSource(gateSource, 2);
    writeSource(otherSource, 2);
    expect(pull(node)).toBe(0);
    expect(runs).toEqual({ gate: 2, other: 1, node: 2 });
  });
});

describe('source writes and verification', () => {
  test('a node whose source changed recomputes without pulling the derived node it read before', () => {
    const runs = { b: 0, c: 0 };
    const a = createSourceNode(true);
    const bInput = createSourceNode(1);
    const b = createDerivedNode((callingNode) => {
      runs.b += 1;
      return readSource(callingNode, bInput);
    });
    const c = createDerivedNode(() => {
      runs.c += 1;
      return 0;
    });
    const foo = createDerivedNode((callingNode) =>
      readSource(callingNode, a)
        ? readDerived(callingNode, b)
        : readDerived(callingNode, c),
    );
    expect(pull(foo)).toBe(1);
    writeSource(bInput, 2);
    writeSource(a, false);
    expect(foo.status).toBe(DIRTY);
    expect(pull(foo)).toBe(0);
    expect(runs).toEqual({ b: 1, c: 1 });
  });
});

describe('dependencies', () => {
  test('a source that a recompute stopped reading does not mark the node', () => {
    let runs = 0;
    const useA = createSourceNode(true);
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const pick = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, useA)
        ? readSource(callingNode, a)
        : readSource(callingNode, b);
    });
    expect(pull(pick)).toBe(1);
    writeSource(useA, false);
    expect(pull(pick)).toBe(2);
    writeSource(a, 10);
    expect(pick.status).toBe(CLEAN);
    expect(pull(pick)).toBe(2);
    expect(runs).toBe(2);
  });

  test('a recompute that reads the same nodes stays marked by each of them', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
    );
    pull(sum);
    writeSource(a, 3);
    expect(pull(sum)).toBe(5);
    writeSource(b, 4);
    expect(sum.status).toBe(DIRTY);
    expect(pull(sum)).toBe(7);
    writeSource(a, 5);
    expect(sum.status).toBe(DIRTY);
    expect(pull(sum)).toBe(9);
  });

  test('a Dirty node stays in the dependents list of its other sources, and a second write leaves it Dirty', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const label = createDerivedNode(
      (callingNode) => `${readDerived(callingNode, sum)}`,
    );
    pull(label);
    writeSource(a, 3);
    expect(hasDependent(b, sum)).toBe(true);
    writeSource(b, 4);
    expect(sum.status).toBe(DIRTY);
    expect(label.status).toBe(CHECK);
    expect(pull(label)).toBe('7');
    expect(runs).toBe(2);
  });

  test('a computation that reads a, b, a recomputes once per write of a and of b', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(10);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return (
        readSource(callingNode, a) +
        readSource(callingNode, b) +
        readSource(callingNode, a)
      );
    });
    expect(pull(node)).toBe(12);
    writeSource(a, 2);
    expect(pull(node)).toBe(14);
    writeSource(b, 20);
    expect(pull(node)).toBe(24);
    writeSource(a, 3);
    writeSource(b, 30);
    expect(pull(node)).toBe(36);
    expect(runs).toBe(4);
  });

  test('a recompute that reads fewer and then more sources is marked by exactly the sources it read last', () => {
    let runs = 0;
    const count = createSourceNode(3);
    const first = createSourceNode(0);
    const second = createSourceNode(1);
    const third = createSourceNode(2);
    const sources = [first, second, third];
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return sources
        .slice(0, readSource(callingNode, count))
        .reduce((sum, source) => sum + readSource(callingNode, source), 0);
    });
    expect(pull(node)).toBe(3);
    writeSource(count, 1);
    expect(pull(node)).toBe(0);
    writeSource(third, 100);
    expect(node.status).toBe(CLEAN);
    writeSource(count, 3);
    expect(pull(node)).toBe(101);
    writeSource(third, 200);
    expect(node.status).toBe(DIRTY);
    expect(pull(node)).toBe(201);
    expect(runs).toBe(4);
  });

  test('a computation that reads a source twice recomputes once per change', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const twice = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, a);
    });
    expect(pull(twice)).toBe(2);
    writeSource(a, 2);
    expect(pull(twice)).toBe(4);
    expect(runs).toBe(2);
  });
});

describe('disposal', () => {
  test('disposing a node returns it to Uncomputed, marks its readers Dirty, and its next pull computes it with previous undefined', () => {
    const runs = { inner: 0, outer: 0 };
    const innerPreviousValues: Array<number | undefined> = [];
    const source = createSourceNode(1);
    const inner = createDerivedNode<number>((callingNode, previous) => {
      runs.inner += 1;
      innerPreviousValues.push(previous);
      return readSource(callingNode, source);
    });
    const outer = createDerivedNode((callingNode) => {
      runs.outer += 1;
      return readDerived(callingNode, inner) + 1;
    });
    expect(pull(outer)).toBe(2);
    disposeDerivedNode(inner);
    expect(inner.status).toBe(UNCOMPUTED);
    expect(outer.status).toBe(DIRTY);
    expect(pull(outer)).toBe(2);
    expect(runs).toEqual({ inner: 2, outer: 2 });
    writeSource(source, 2);
    expect(outer.status).toBe(CHECK);
    expect(pull(outer)).toBe(3);
    expect(innerPreviousValues).toEqual([undefined, undefined, 1]);
  });

  test('a disposed node is in no dependents list of the sources it read', () => {
    const source = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, source),
    );
    pull(inner);
    disposeDerivedNode(inner);
    expect(inner.status).toBe(UNCOMPUTED);
    expect(hasDependent(source, inner)).toBe(false);
    writeSource(source, 2);
    expect(pull(inner)).toBe(2);
  });

  test('disposing a Dirty node marks its Check readers Dirty, and its next pull computes it with previous undefined', () => {
    const innerPreviousValues: Array<number | undefined> = [];
    const source = createSourceNode(1);
    const inner = createDerivedNode<number>((callingNode, previous) => {
      innerPreviousValues.push(previous);
      return readSource(callingNode, source);
    });
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    writeSource(source, 2);
    expect(inner.status).toBe(DIRTY);
    expect(outer.status).toBe(CHECK);
    disposeDerivedNode(inner);
    expect(inner.status).toBe(UNCOMPUTED);
    expect(outer.status).toBe(DIRTY);
    expect(pull(outer)).toBe(3);
    expect(innerPreviousValues).toEqual([undefined, undefined]);
  });

  test('disposing a node that was never computed changes nothing', () => {
    const previousValues: Array<number | undefined> = [];
    const source = createSourceNode(1);
    const node = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, source);
    });
    expect(node.status).toBe(UNCOMPUTED);
    disposeDerivedNode(node);
    expect(node.status).toBe(UNCOMPUTED);
    expect(pull(node)).toBe(1);
    expect(previousValues).toEqual([undefined]);
  });
});

describe('batches and onDirty', () => {
  test('a write calls nothing until the batch is flushed', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const double = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    pull(double);
    double.onDirty = () => calls.push('double');
    startBatch((batch) => {
      batch.writeSource(a, 2);
      expect(calls).toEqual([]);
    });
    expect(calls).toEqual(['double']);
  });

  test('several writes in one batch call each onDirty once, in the order the nodes left Clean', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const readsB = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsBoth = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, readsA) + readDerived(callingNode, readsB),
    );
    pull(readsBoth);
    readsA.onDirty = () => calls.push('readsA');
    readsB.onDirty = () => calls.push('readsB');
    readsBoth.onDirty = () => calls.push('readsBoth');
    startBatch((batch) => {
      batch.writeSource(b, 3);
      batch.writeSource(a, 4);
      batch.writeSource(b, 5);
    });
    expect(calls).toEqual(['readsB', 'readsBoth', 'readsA']);
  });

  test('a callback that pulls sees every write of the batch', () => {
    const seen: Array<number> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
    );
    pull(sum);
    sum.onDirty = () => seen.push(pull(sum));
    startBatch((batch) => {
      batch.writeSource(a, 10);
      batch.writeSource(b, 20);
    });
    expect(seen).toEqual([30]);
  });

  test('a node that is already marked does not call onDirty again until it is pulled', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    pull(double);
    double.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 2);
    writeSource(a, 3);
    expect(calls).toBe(1);
    pull(double);
    writeSource(a, 4);
    expect(calls).toBe(2);
  });

  test('a node that becomes Check calls onDirty', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    outer.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 2);
    expect(outer.status).toBe(CHECK);
    expect(calls).toBe(1);
  });

  test('a pull calls no onDirty, even when a recompute changes a value', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    startBatch((batch) => {
      batch.writeSource(a, 2);
      outer.onDirty = () => {
        calls += 1;
      };
      expect(pull(outer)).toBe(3);
    });
    expect(calls).toBe(0);
  });

  test('a callback collected in a batch is called even if an earlier callback disposed its node', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const first = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const second = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    pull(first);
    pull(second);
    first.onDirty = () => {
      calls.push('first');
      disposeDerivedNode(second);
    };
    second.onDirty = () => calls.push('second');
    writeSource(a, 2);
    expect(calls).toEqual(['first', 'second']);
  });

  test('a callback that writes through the batch being flushed has its new callbacks called in the same flush', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsB = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    pull(readsA);
    pull(readsB);
    startBatch((batch) => {
      readsA.onDirty = () => {
        calls.push('readsA');
        batch.writeSource(b, 2);
      };
      readsB.onDirty = () => calls.push('readsB');
      batch.writeSource(a, 2);
    });
    expect(calls).toEqual(['readsA', 'readsB']);
  });

  test('a callback that pulls its node and is marked again in the same flush is called again', () => {
    const calls: Array<number> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    pull(node);
    startBatch((batch) => {
      node.onDirty = () => {
        const value = pull(node);
        calls.push(value);
        if (value < 3) {
          batch.writeSource(a, value + 1);
        }
      };
      batch.writeSource(a, 2);
    });
    expect(calls).toEqual([2, 3]);
  });

  test('two nodes that share one callback call it once per node', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const left = createDerivedNode((callingNode) => readSource(callingNode, a));
    const right = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    pull(left);
    pull(right);
    const callback = () => {
      calls += 1;
    };
    left.onDirty = callback;
    right.onDirty = callback;
    writeSource(a, 2);
    expect(calls).toBe(2);
  });

  test('disposing an observed reader calls its onDirty', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    pull(outer);
    outer.onDirty = () => {
      calls += 1;
    };
    disposeDerivedNode(inner);
    expect(outer.status).toBe(DIRTY);
    expect(calls).toBe(1);
  });

  test('a batch whose callback does nothing calls nothing', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    pull(node);
    node.onDirty = () => {
      calls += 1;
    };
    startBatch(() => {});
    expect(calls).toBe(0);
    expect(node.status).toBe(CLEAN);
  });
});

describe('eager nodes', () => {
  test('an eager node whose recompute returns the previous value leaves its readers Clean, and no callback runs', () => {
    const runs = { isOne: 0, reader: 0 };
    let calls = 0;
    const a = createSourceNode(2);
    const isOne = createEagerDerivedNode((callingNode) => {
      runs.isOne += 1;
      return readSource(callingNode, a) === 1;
    });
    const reader = createDerivedNode((callingNode) => {
      runs.reader += 1;
      return readDerived(callingNode, isOne) ? 'one' : 'other';
    });
    pull(reader);
    reader.onDirty = () => {
      calls += 1;
    };
    isOne.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 3);
    writeSource(a, 4);
    expect(isOne.status).toBe(CLEAN);
    expect(reader.status).toBe(CLEAN);
    expect(calls).toBe(0);
    expect(pull(reader)).toBe('other');
    expect(runs).toEqual({ isOne: 3, reader: 1 });
  });

  test('a changed eager value marks its readers, and their callbacks run after every queued eager node is refreshed', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const eagerB = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const readsEager = createDerivedNode(
      (callingNode) => readDerived(callingNode, eagerB) + 1,
    );
    pull(readsA);
    pull(readsEager);
    readsA.onDirty = () => calls.push(`readsA, eagerB ${eagerB.status}`);
    readsEager.onDirty = () => calls.push('readsEager');
    eagerB.onDirty = () => calls.push('eagerB');
    startBatch((batch) => {
      batch.writeSource(a, 2);
      batch.writeSource(b, 2);
      expect(readsEager.status).toBe(CLEAN);
    });
    expect(calls).toEqual([`readsA, eagerB ${CLEAN}`, 'eagerB', 'readsEager']);
    expect(readsEager.status).toBe(DIRTY);
    expect(pull(readsEager)).toBe(3);
  });

  test('in a chain of eager nodes, each refreshes in the same flush, and a changed value at the top marks its reader', () => {
    const runs = { first: 0, second: 0, third: 0 };
    const a = createSourceNode(1);
    const first = createEagerDerivedNode((callingNode) => {
      runs.first += 1;
      return readSource(callingNode, a) * 2;
    });
    const second = createEagerDerivedNode((callingNode) => {
      runs.second += 1;
      return readDerived(callingNode, first) + 1;
    });
    const third = createEagerDerivedNode((callingNode) => {
      runs.third += 1;
      return readDerived(callingNode, second) * 10;
    });
    const reader = createDerivedNode((callingNode) =>
      readDerived(callingNode, third),
    );
    pull(reader);
    let calls = 0;
    reader.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 2);
    expect([first.status, second.status, third.status]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(runs).toEqual({ first: 2, second: 2, third: 2 });
    expect(calls).toBe(1);
    expect(pull(reader)).toBe(50);
  });

  test('an eager node behind a derived node becomes Check, and the flush verifies and recomputes it', () => {
    const runs = { inner: 0, eager: 0 };
    let calls = 0;
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => {
      runs.inner += 1;
      return readSource(callingNode, a) % 2;
    });
    const eager = createEagerDerivedNode((callingNode) => {
      runs.eager += 1;
      return readDerived(callingNode, inner) === 0;
    });
    const reader = createDerivedNode((callingNode) =>
      readDerived(callingNode, eager),
    );
    pull(reader);
    reader.onDirty = () => {
      calls += 1;
    };
    startBatch((batch) => {
      batch.writeSource(a, 3);
      expect(inner.status).toBe(DIRTY);
      expect(eager.status).toBe(CHECK);
      expect(reader.status).toBe(CLEAN);
    });
    expect(runs).toEqual({ inner: 2, eager: 1 });
    expect(calls).toBe(0);
    writeSource(a, 4);
    expect(runs).toEqual({ inner: 3, eager: 2 });
    expect(calls).toBe(1);
    expect(pull(reader)).toBe(true);
  });

  test('a callback that writes during the flush has the eager nodes it marks refreshed before the next callback', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsANext = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    const eagerB = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const readsEager = createDerivedNode((callingNode) =>
      readDerived(callingNode, eagerB),
    );
    pull(readsA);
    pull(readsANext);
    pull(readsEager);
    startBatch((batch) => {
      readsA.onDirty = () => {
        calls.push('readsA');
        batch.writeSource(b, 2);
      };
      readsANext.onDirty = () =>
        calls.push(`readsANext, eagerB ${eagerB.status}`);
      readsEager.onDirty = () => calls.push('readsEager');
      batch.writeSource(a, 2);
    });
    expect(calls).toEqual([
      'readsA',
      `readsANext, eagerB ${CLEAN}`,
      'readsEager',
    ]);
    expect(pull(readsEager)).toBe(2);
  });

  test('disposing an eager node marks its readers Dirty, and a queued eager node disposed before the flush stays Uncomputed', () => {
    let runs = 0;
    let calls = 0;
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 1,
    );
    pull(reader);
    reader.onDirty = () => {
      calls += 1;
    };
    startBatch((batch) => {
      batch.writeSource(a, 2);
      expect(eager.status).toBe(DIRTY);
      batch.disposeDerivedNode(eager);
    });
    expect(eager.status).toBe(UNCOMPUTED);
    expect(reader.status).toBe(DIRTY);
    expect(calls).toBe(1);
    expect(runs).toBe(1);
    expect(hasDependent(a, eager)).toBe(false);
    expect(pull(reader)).toBe(3);
    expect(runs).toBe(2);
  });

  test('before the flush, a marked eager node and its readers return their previous values', () => {
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 1,
    );
    const late = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 100,
    );
    pull(reader);
    startBatch((batch) => {
      batch.writeSource(a, 5);
      expect(pull(eager)).toBe(2);
      expect(pull(reader)).toBe(3);
      expect(pull(late)).toBe(102);
      expect(eager.status).toBe(DIRTY);
    });
    expect(pull(eager)).toBe(10);
    expect(pull(reader)).toBe(11);
    expect(pull(late)).toBe(110);
  });

  test('several writes in one batch recompute an eager node once', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    pull(eager);
    startBatch((batch) => {
      batch.writeSource(a, 10);
      batch.writeSource(b, 20);
      batch.writeSource(a, 30);
    });
    expect(runs).toBe(2);
    expect(pull(eager)).toBe(50);
  });
});

describe('batches of one and nested batches', () => {
  test('an unbatched write calls the same callbacks and leaves the same statuses and values as a batch of one write', () => {
    function build(): {
      readonly a: SourceNode<number>;
      readonly calls: Array<string>;
      readonly nodes: ReadonlyArray<DerivedNode<number>>;
    } {
      const calls: Array<string> = [];
      const a = createSourceNode(1);
      const inner = createDerivedNode((callingNode) =>
        readSource(callingNode, a),
      );
      const eager = createEagerDerivedNode(
        (callingNode) => readDerived(callingNode, inner) * 2,
      );
      const reader = createDerivedNode(
        (callingNode) => readDerived(callingNode, eager) + 1,
      );
      pull(reader);
      inner.onDirty = () => calls.push(`inner ${eager.status}`);
      eager.onDirty = () => calls.push('eager');
      reader.onDirty = () => calls.push('reader');
      return { a, calls, nodes: [inner, eager, reader] };
    }
    const unbatched = build();
    const batched = build();
    writeSource(unbatched.a, 2);
    startBatch((batch) => batch.writeSource(batched.a, 2));
    expect(unbatched.calls).toEqual([`inner ${CLEAN}`, 'eager', 'reader']);
    expect(batched.calls).toEqual(unbatched.calls);
    expect(batched.nodes.map((node) => node.status)).toEqual(
      unbatched.nodes.map((node) => node.status),
    );
    expect(batched.nodes.map(pull)).toEqual([2, 4, 5]);
    expect(unbatched.nodes.map(pull)).toEqual([2, 4, 5]);
  });

  test('an unbatched write inside a callback flushes before the callback continues', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsB = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const eagerB = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, b) * 10,
    );
    const readsEager = createDerivedNode((callingNode) =>
      readDerived(callingNode, eagerB),
    );
    const readsANext = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    pull(readsA);
    pull(readsB);
    pull(readsEager);
    pull(readsANext);
    readsA.onDirty = () => {
      calls.push('readsA');
      writeSource(b, 2);
      calls.push(`readsA after write, eagerB ${eagerB.status}`);
    };
    readsB.onDirty = () => calls.push('readsB');
    readsEager.onDirty = () => calls.push(`readsEager ${pull(readsEager)}`);
    readsANext.onDirty = () => calls.push('readsANext');
    writeSource(a, 2);
    expect(calls).toEqual([
      'readsA',
      'readsB',
      'readsEager 20',
      `readsA after write, eagerB ${CLEAN}`,
      'readsANext',
    ]);
    expect([pull(readsA), pull(readsB), pull(readsANext)]).toEqual([2, 2, 3]);
  });

  test('a startBatch inside the callback of another startBatch flushes its own batch before it returns', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsB = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    pull(readsA);
    pull(readsB);
    readsA.onDirty = () => calls.push('readsA');
    readsB.onDirty = () => calls.push('readsB');
    startBatch((outer) => {
      outer.writeSource(a, 2);
      startBatch((inner) => {
        inner.writeSource(b, 2);
      });
      expect(calls).toEqual(['readsB']);
    });
    expect(calls).toEqual(['readsB', 'readsA']);
  });

  test('a nested batch leaves an eager node queued in the outer batch for the outer flush', () => {
    let eagerRuns = 0;
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const eager = createEagerDerivedNode((callingNode) => {
      eagerRuns += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    const reader = createDerivedNode((callingNode) =>
      readDerived(callingNode, eager),
    );
    pull(reader);
    reader.onDirty = () => calls.push(`reader ${pull(reader)}`);
    startBatch((outer) => {
      outer.writeSource(a, 2);
      startBatch((inner) => {
        inner.writeSource(b, 2);
      });
      expect(eagerRuns).toBe(1);
      expect(calls).toEqual([]);
    });
    expect(eagerRuns).toBe(2);
    expect(calls).toEqual(['reader 4']);
  });
});

describe('setDerivedNodeKind', () => {
  test('setting the kind a node has, or the kind of a Clean or Uncomputed node, runs nothing and calls nothing', () => {
    let runs = 0;
    let calls = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    node.onDirty = () => {
      calls += 1;
    };
    setDerivedNodeKind(node, EAGER);
    expect(node.kind).toBe(EAGER);
    expect(node.status).toBe(UNCOMPUTED);
    pull(node);
    setDerivedNodeKind(node, EAGER);
    setDerivedNodeKind(node, LAZY);
    setDerivedNodeKind(node, LAZY);
    expect(node.kind).toBe(LAZY);
    expect(node.status).toBe(CLEAN);
    expect(runs).toBe(1);
    expect(calls).toBe(0);
  });

  test('an eager node set lazy, written, and set eager again while Dirty is refreshed, and its readers return current values', () => {
    const s = createSourceNode(1);
    const n = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, s) * 10,
    );
    const top = createDerivedNode(
      (callingNode) => readDerived(callingNode, n) + 1,
    );
    expect(pull(top)).toBe(11);
    setDerivedNodeKind(n, LAZY);
    writeSource(s, 2);
    expect(n.status).toBe(DIRTY);
    expect(top.status).toBe(CHECK);
    setDerivedNodeKind(n, EAGER);
    expect(n.status).toBe(CLEAN);
    expect(pull(n)).toBe(20);
    expect(pull(top)).toBe(21);
    writeSource(s, 3);
    expect(n.status).toBe(CLEAN);
    expect(pull(n)).toBe(30);
    expect(pull(top)).toBe(31);
  });

  test('a lazy node set eager while Dirty is recomputed in the flush, and a changed value marks its readers and calls its onDirty', () => {
    let runs = 0;
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) > 0;
    });
    const reader = createDerivedNode((callingNode) =>
      readDerived(callingNode, node) ? 'positive' : 'not positive',
    );
    pull(reader);
    writeSource(a, -1);
    node.onDirty = () => calls.push('node');
    reader.onDirty = () => calls.push('reader');
    startBatch((batch) => {
      batch.setDerivedNodeKind(node, EAGER);
      expect(node.status).toBe(DIRTY);
      expect(runs).toBe(1);
    });
    expect(runs).toBe(2);
    expect(node.status).toBe(CLEAN);
    expect(reader.status).toBe(DIRTY);
    expect(calls).toEqual(['node']);
    expect(pull(reader)).toBe('not positive');
    writeSource(a, -2);
    expect(runs).toBe(3);
    expect(reader.status).toBe(CLEAN);
    expect(calls).toEqual(['node']);
  });

  test('a lazy node set eager while Check is verified in the flush', () => {
    const runs = { inner: 0, node: 0 };
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => {
      runs.inner += 1;
      return readSource(callingNode, a) % 2;
    });
    const node = createDerivedNode((callingNode) => {
      runs.node += 1;
      return readDerived(callingNode, inner) * 10;
    });
    pull(node);
    writeSource(a, 3);
    expect(node.status).toBe(CHECK);
    setDerivedNodeKind(node, EAGER);
    expect(runs).toEqual({ inner: 2, node: 1 });
    expect(node.status).toBe(CLEAN);
    expect(pull(node)).toBe(10);
  });

  test('an eager node set lazy while queued marks its readers Check, calls its onDirty and theirs, and is not recomputed by the flush', () => {
    let runs = 0;
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) * 2;
    });
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 1,
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, reader) + 1,
    );
    pull(outer);
    eager.onDirty = () => calls.push('eager');
    reader.onDirty = () => calls.push('reader');
    outer.onDirty = () => calls.push('outer');
    startBatch((batch) => {
      batch.writeSource(a, 2);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CLEAN,
        CLEAN,
      ]);
      batch.setDerivedNodeKind(eager, LAZY);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CHECK,
        CHECK,
      ]);
    });
    expect(calls).toEqual(['eager', 'reader', 'outer']);
    expect(runs).toBe(1);
    expect(eager.status).toBe(DIRTY);
    expect(pull(outer)).toBe(6);
    expect(runs).toBe(2);
  });

  test('a pull of an eager node set lazy while queued recomputes it before the flush', () => {
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 1,
    );
    pull(reader);
    startBatch((batch) => {
      batch.writeSource(a, 2);
      batch.setDerivedNodeKind(eager, LAZY);
      expect(pull(reader)).toBe(5);
      expect([eager.status, reader.status]).toEqual([CLEAN, CLEAN]);
    });
    expect(pull(reader)).toBe(5);
  });

  test('a node set lazy and eager again in one batch is refreshed once', () => {
    let runs = 0;
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    const reader = createDerivedNode((callingNode) =>
      readDerived(callingNode, eager),
    );
    pull(reader);
    reader.onDirty = () => calls.push('reader');
    startBatch((batch) => {
      batch.writeSource(a, 2);
      batch.setDerivedNodeKind(eager, LAZY);
      batch.setDerivedNodeKind(eager, EAGER);
    });
    expect(runs).toBe(2);
    expect(eager.status).toBe(CLEAN);
    expect(calls).toEqual(['reader']);
    expect(pull(reader)).toBe(2);
  });

  test('a node toggled between lazy and eager across several batches returns current values after each', () => {
    let runs = 0;
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const node = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) % 3;
    });
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, node) + 100,
    );
    pull(reader);
    reader.onDirty = () => calls.push(`reader ${reader.status}`);

    startBatch((batch) => {
      batch.setDerivedNodeKind(node, LAZY);
      batch.writeSource(a, 2);
    });
    expect([node.kind, node.status, reader.status]).toEqual([
      LAZY,
      DIRTY,
      CHECK,
    ]);
    expect(calls).toEqual([`reader ${CHECK}`]);

    startBatch((batch) => batch.setDerivedNodeKind(node, EAGER));
    expect([node.status, reader.status, runs]).toEqual([CLEAN, DIRTY, 2]);
    expect(pull(reader)).toBe(102);

    startBatch((batch) => batch.writeSource(a, 5));
    expect([node.status, reader.status, runs]).toEqual([CLEAN, CLEAN, 3]);
    expect(calls).toEqual([`reader ${CHECK}`]);

    startBatch((batch) => {
      batch.writeSource(a, 6);
      batch.setDerivedNodeKind(node, LAZY);
    });
    expect([node.kind, node.status, reader.status, runs]).toEqual([
      LAZY,
      DIRTY,
      CHECK,
      3,
    ]);
    expect(calls).toEqual([`reader ${CHECK}`, `reader ${CHECK}`]);
    expect(pull(reader)).toBe(100);

    startBatch((batch) => {
      batch.setDerivedNodeKind(node, EAGER);
      batch.writeSource(a, 7);
    });
    expect([node.status, reader.status, runs]).toEqual([CLEAN, DIRTY, 5]);
    expect(pull(reader)).toBe(101);
  });
});

function hasDependent(
  dependency: SourceNode<unknown> | DerivedNode<unknown>,
  node: DerivedNode<unknown>,
): boolean {
  let edge = dependency.firstDependent;
  while (edge !== undefined) {
    if (edge.dependent === node) {
      return true;
    }
    edge = edge.nextDependent;
  }
  return false;
}
```

```ts
// from libs/pico-js/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import { createDerivedNode, readDerived, type DerivedNode } from '../src/index';

test('a derived node that reads another derived node has the type of the value its computation returns', () => {
  const count = createDerivedNode(() => 1);
  const label = createDerivedNode(
    (callingNode) => `${readDerived(callingNode, count)}`,
  );
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
});
```

All 55 tests and the type test pass under vitest 2.1.9, and `tsc -b` reports no errors.
