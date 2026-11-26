# pico-js: implementation

This doc is the complete source and package of `@isograph/pico-js` in `libs/pico-js`. PR #1027 holds the engine as commit `6f431ec0e`, which is `libs/pico-js` of the prototype commit `f0b001a9d` without `bench/`. This doc describes the engine that the PR converges to: the prototype's engine with the API deltas listed under "Deltas from the prototype", split into five source files.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation and the value that computation last returned.
- A computation receives `(callingNode, previous)` and reads through `readSource(callingNode, source)` and `readDerived(callingNode, node)`. Each read records an edge from the calling node to the node it read. A node depends on exactly the nodes its latest computation read. A computation makes no other read.
- `createDerivedNode(computation, evaluation)` fixes the node's evaluation, `LAZY` (the default) or `EAGER`, for its lifetime.
- A node refreshes in the flush, and marking stops at it, while it is retained and was created `EAGER` or has a retain with an `onChange`. Every other node is lazy: it recomputes when it is read.
- A write runs no computation. It marks the derived nodes that read the source `Dirty` and the lazy nodes above them `Check`. Marking continues through a lazy node and stops at a node that refreshes in the flush, which it queues.
- A read of a `Dirty` node recomputes it. A read of a `Check` node reads its derived dependencies in the order its computation read them, and recomputes the node as soon as one of them changed, without reading the rest. If none changed, the node becomes `Clean` without running.
- A recompute receives `previous`, the node's last value. A recompute whose value is `===` to `previous` keeps `previous` and marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- `startBatch()` and `endBatch()` open and close a batch. Batches nest, and a nested batch joins the open one. A write inside a batch marks at once, and the outermost `endBatch` flushes. A `writeSource` outside a batch flushes before it returns. The flush refreshes every queued node, then calls the `onChange` of each retain of each node whose value the flush changed.
- `retainDerivedNode(node, onChange?)` computes the node and returns a `Retain`, which joins the node's retain list. `releaseRetain(retain)` removes it. `onChange` exists only on a retain.
- `readDerivedNode(node)` is the top-level read of a retained node. `readUnretained(node)` is a one-off top-level read of any node: it reads and links as any read does, and queues the node for cleanup if nothing holds it.
- A derived node is needed while it has a retain or a dependent. A node that becomes unneeded keeps its value and its edges and joins the cleanup queue. `disposeNext()` and `disposeAll()` dispose the queued nodes that are still unneeded: a disposed node returns to `Uncomputed`, drops its value, and unlinks its edges, and each derived dependency the disposal leaves unneeded joins the queue. Sources are never disposed.
- At the end of the outermost operation that queued a node onto an empty queue, the engine calls `scheduler.schedule()`. The default disposes during idle time. A scheduler that calls `disposeAll` disposes when the operation ends.
- A write's marking that reaches an unneeded node disposes it when the write, or the outermost batch, ends, so a write never leaves an unneeded node `Dirty` or `Check` and linked. A `Clean` dependency that this disposal leaves unneeded joins the cleanup queue with its value.
- The engine keeps one graph per process in module state: the batch depth, the batch's queue, the flush state, the cleanup queue, and the scheduler. The calling node stays a parameter of each tracked read.

### Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior.

- The caller writes a source only when its value changed.
- A scheduler does not throw. A computation or an `onChange` may throw. After a throw, the statuses and values of the nodes that the throwing operation was updating are unspecified. The engine's module state is restored (Guarantees).
- A computation is synchronous. It calls `readSource` and `readDerived` only with the `callingNode` it received, and reads nothing else of the graph: no `readDerivedNode`, no `readUnretained`, and no `source.value`.
- A computation does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`.
- An `onChange` does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`. It may read through `readDerivedNode` and `source.value`.
- A computation's result depends only on `previous` and on the values its reads return.
- The derived nodes form a directed acyclic graph, counting the edges a node keeps from its previous run.
- `readDerivedNode` is called only on a retained node, and only outside computations.
- `readUnretained` is a one-off read. A read whose value must stay current until a later point, such as a render read until its commit, retains the node until then, because a scheduler may dispose an unretained node as soon as the read returns.
- Each `retainDerivedNode` is matched by exactly one `releaseRetain` of the retain it returned. A second release of the same retain unlinks it again and corrupts the node's retain list.
- Each `startBatch` is matched by exactly one `endBatch`. Both are called outside computations and `onChange` callbacks. A caller whose code between them can throw calls `endBatch` in a `finally`.
- `startBatch` and its `endBatch` run in one synchronous stretch of code and never span an `await`. On a server that serves several requests in one process, the batch is module state, so another request's writes made during the `await` would join the open batch.
- `disposeNext` and `disposeAll` are called outside computations, batches, and `onChange` callbacks. After a call to `scheduler.schedule()`, the scheduler calls `disposeNext` until it returns `false`, or calls `disposeAll`.
- The chains of derived nodes that a first computation or a recompute of `Dirty` nodes descends stay well below the call stack limit, because each node adds frames. In Node 24.12 with its default stack and one-line computations, the first computation of a chain overflows at about 2,000 derived nodes, and a recompute through a chain of `Dirty` nodes at about 2,200.
- A caller does not mutate a value that a read returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode` and `createDerivedNode`, and writes no field of a node or a retain, except through `scheduler.schedule`. The types enforce this for every field except `computation`, a method, which a type cannot mark `readonly`.
- A caller that needs current values reads after the batch flushes. Until the flush, a queued node and the nodes that read it return their previous values.

### Guarantees

pico-js upholds these while every contract holds.

- A read made while no batch is open returns the value the node's latest computation returned, or `previous` when that computation returned a value `===` to it, and each read of that computation returned the current value of the node it read.
- A computation runs only when its node is `Uncomputed`, or when a source its latest computation read was written, or a derived node it read recomputed to a value not `===` to its previous value.
- A read of a `Check` node reads its derived dependencies in the order its latest computation read them, and recomputes the node as soon as one of them changed, without reading the rest. A read of a `Dirty` lazy node recomputes it without reading its dependencies first.
- A recompute whose value is `===` to its previous value keeps the previous value and marks nothing. `NaN` counts as changed, and `-0` over a stored `0` keeps `0`.
- `retainDerivedNode` computes the node before it returns. A node that starts to refresh in the flush is `Clean` when it starts.
- The flush calls, for each node that it recomputed to a value not `===` to the previous one, the `onChange` of each of the node's retains once, most recent retain first, even when the node recomputed more than once, in the order of each node's first changed recompute. It calls them after every queued node is refreshed and after the disposal of the unneeded nodes the write's marking reached. A first computation, a cutoff, and a lazy node call no `onChange`.
- A queued node that a release in the same batch made lazy is skipped by the flush. It stays `Dirty` or `Check`, its dependents are `Check` or `Dirty`, and a read recomputes it as a lazy node.
- When a flush returns, every node that refreshes in the flush is `Clean`. A read of a `Dirty` or `Check` node that refreshes in the flush, inside a batch, returns its previous value and runs nothing.
- A `writeSource` inside a batch runs no computation and calls no `onChange`. A write walks the dependents list of the source and of each lazy node it takes out of `Clean` at most once each. A write returns without visiting a derived node when no `readSource` of the source has run since the source was created or last written.
- After a computation, its node is in the dependents list of each node it read, and of no other node, until its next computation or its disposal. Marking unlinks no edge.
- When the outermost operation returns, every derived node that is not `Uncomputed` and has no retain and no dependent is on the cleanup queue. After `disposeAll` returns, every derived node with no retain and no dependent is `Uncomputed`.
- When a write outside a batch, or the outermost `endBatch`, returns, no node that its marking took out of `Clean` is both unneeded and `Dirty` or `Check`. Each such node is `Uncomputed`, or `Clean` because a read inside the batch recomputed it.
- A disposed node is `Uncomputed`, holds `undefined`, and has no edges. Disposal marks nothing and calls nothing. `disposeNext` and `disposeAll` skip a queued node that is needed again or already `Uncomputed`.
- A derived node that becomes unneeded and is read again before a disposal reaches it keeps its value and does not run for that read, whether the read comes from a later run in the same operation or from a later operation.
- The engine calls `scheduler.schedule()` only when an outermost operation returns, with no batch open and no flush running, and only once per time the cleanup queue went from empty to non-empty since the last call. A scheduler that calls `disposeAll` therefore disposes every unneeded node when each outermost operation returns.
- A recompute whose reads match, in order, the edges of its previous run allocates no edge and links or unlinks none.
- Marking, verification of `Check` nodes, and disposal keep their stacks in arrays. In Node 24.12, a read through a chain of 200,000 `Check` nodes, and a release followed by `disposeAll` of a chain of 100,000 nodes, do not overflow.
- If a computation or an `onChange` throws, the engine restores its module state before the exception leaves the outermost entry point: the flush state is idle, the marked list is empty, and the batch's queue is cleared by the `endBatch` or `writeSource` that started the flush. Later operations, including those of other requests on a server, are unaffected. The graph that was being updated may be partially updated. A scheduler call that the throwing operation would have made happens at the end of the next outermost operation instead.
- No module-level array keeps a node reachable after the operation that used it returns, except the cleanup queue, which holds each queued node until a disposal pops it. The marked list is emptied at the end of each outermost write or batch, and the batch's queue is dropped when its flush starts.
- `retainDerivedNode` allocates one `Retain`. `releaseRetain` and a `readDerivedNode` of a `Clean` node allocate nothing beyond growth of the cleanup queue's array. A write outside a batch that queues no node allocates no queue, and a flush that changes no node with a retain allocates no list.

## The package

### Location and files

The engine is a workspace library at `libs/pico-js`, covered by `pnpm-workspace.yaml`'s `libs/*`. Its build and test setup copies `libs/isograph-runtime`'s without the Babel plugin, which only the runtime's fixtures need.

```text
libs/pico-js/
  package.json
  README.md
  tsconfig.json             references tsconfig.package.json and tsconfig.test.json
  tsconfig.package.json     extends ../../tsconfig.build.json, includes src/**/*.ts
  tsconfig.test.json        extends ../../tsconfig.build.json, includes test/**/*.ts and test-random/**/*.ts
  tsdown.config.ts          merges ../../tsdown.config.ts, dts.build true
  vitest.config.ts          typecheck enabled with tsconfig.test.json
  vitest.random.config.ts   runs test-random/random.ts only
  src/nodes.ts              node, edge, and retain types, their constants, createSourceNode, createDerivedNode
  src/marking.ts            storeValue, marking, the marked-unneeded list
  src/pull.ts               edges, readSource, readDerived, pull, verify, recompute, trim
  src/retain.ts             retains, top-level reads, evaluation state, the cleanup queue, the scheduler, disposal
  src/batch.ts              startBatch, endBatch, writeSource, the flush, the end-of-operation check
  src/index.ts              the public exports
  test/graph.test.ts, test/collector.test.ts, test/scheduler.test.ts, test/graph.test-d.ts   Tests
  test-random/random.ts     Random differential test, run by `pnpm test:random`
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
    "test:random": "vitest run --config vitest.random.config.ts",
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

This is `6f431ec0e`'s file. Deltas from `libs/isograph-runtime/package.json`: `name`, `description`, and `repository.directory`; `dependencies` is empty; `devDependencies` drops `@babel/core` and `@isograph/babel-plugin`; `scripts` adds `test:random`. `private: true` stays, as the runtime's and `@isograph/disposable-types`'s do: publishing to npm is distribution, which `IN_PROGRESS.md` lists as not planned. The one export is `.`, built from `src/index.ts` by the root `tsdown.config.ts` (`entry: ['src/index.ts']`, `format: ['commonjs', 'esm']`).

`"sideEffects": false` holds although the engine has module state: evaluating the modules allocates two empty arrays and assigns constants, and runs no code with an effect outside the package.

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
  "include": ["test/**/*.ts", "test-random/**/*.ts"],
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

```ts
// from libs/pico-js/vitest.random.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { include: ['test-random/random.ts'], testTimeout: 3_600_000 },
});
```

Delta of `vitest.random.config.ts` from `6f431ec0e`: `include` names `test-random/random.ts` instead of `test-random/*.ts`, because the prototype's `random-retain.ts` is renamed back to `random.ts` (Tests, Random differential test).

`vitest.config.ts` keeps vitest's default `include`, which matches `*.test.ts` and `*.spec.ts`, so `pnpm test` and the `pico-js-test` hook do not run the random test. `tsconfig.test.json` includes `test-random/**/*.ts`, so `tsc -b` and the `pico-js-typecheck` hook type-check it without running it. vitest runs each test file in its own module instance, so the module state of one file does not reach another.

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`. The engine has no class and no private field. oxlint's `strict-boolean-expressions` (`.oxlintrc.json`) holds for every condition. oxlint's `switch-exhaustiveness-check` holds for every `switch`, and no `switch` has a `default` arm.

### Pre-commit hooks

`.pre-commit-config.yaml` gains two hooks, after `disposable-types-typecheck`. `6f431ec0e` already has them.

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

`.github/workflows/js.yml`'s `babel-plugin` job runs `pnpm compile-libs`, which turbo runs for every workspace package with a `compile-libs` script; turbo's `compile-libs` task depends on `tsc` (`turbo.json`), so the engine is type-checked and built there with no change. The job gains two steps after "Test @isograph/disposable-types". `6f431ec0e` already has them.

```yaml
# from .github/workflows/js.yml
      - name: Test @isograph/pico-js
        run: pnpm --filter @isograph/pico-js test
      - name: Random differential test of @isograph/pico-js
        run: pnpm --filter @isograph/pico-js test:random
        env:
          PICO_JS_RANDOM_SEEDS: 2000
```

### `README.md`

````markdown
<!-- from libs/pico-js/README.md -->
# pico-js

pico-js is a push-pull memoization engine. A write to a source node eagerly marks the derived nodes that depend on it, and a derived node lazily recomputes when it is read.

## Goals

pico-js aims for correctness first and performance second. Where a rule can be the caller's obligation instead of a check on the hot path, pico-js makes it a contract and checks nothing at run time.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation and the value that computation last returned.
- A computation receives `(callingNode, previous)` and reads through `readSource(callingNode, source)` and `readDerived(callingNode, node)`. Each read records an edge from the calling node to the node it read. A node depends on exactly the nodes its latest computation read. A computation makes no other read.
- `createDerivedNode(computation, evaluation)` fixes the node's evaluation, `LAZY` (the default) or `EAGER`, for its lifetime.
- A node refreshes in the flush, and marking stops at it, while it is retained and was created `EAGER` or has a retain with an `onChange`. Every other node is lazy: it recomputes when it is read.
- A write runs no computation. It marks the derived nodes that read the source `Dirty` and the lazy nodes above them `Check`. Marking continues through a lazy node and stops at a node that refreshes in the flush, which it queues.
- A read of a `Dirty` node recomputes it. A read of a `Check` node reads its derived dependencies in the order its computation read them, and recomputes the node as soon as one of them changed, without reading the rest. If none changed, the node becomes `Clean` without running.
- A recompute receives `previous`, the node's last value. A recompute whose value is `===` to `previous` keeps `previous` and marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- `startBatch()` and `endBatch()` open and close a batch. Batches nest, and a nested batch joins the open one. A write inside a batch marks at once, and the outermost `endBatch` flushes. A `writeSource` outside a batch flushes before it returns. The flush refreshes every queued node, then calls the `onChange` of each retain of each node whose value the flush changed.
- `retainDerivedNode(node, onChange?)` computes the node and returns a `Retain`, which joins the node's retain list. `releaseRetain(retain)` removes it. `onChange` exists only on a retain.
- `readDerivedNode(node)` is the top-level read of a retained node. `readUnretained(node)` is a one-off top-level read of any node: it reads and links as any read does, and queues the node for cleanup if nothing holds it.
- A derived node is needed while it has a retain or a dependent. A node that becomes unneeded keeps its value and its edges and joins the cleanup queue. `disposeNext()` and `disposeAll()` dispose the queued nodes that are still unneeded: a disposed node returns to `Uncomputed`, drops its value, and unlinks its edges, and each derived dependency the disposal leaves unneeded joins the queue. Sources are never disposed.
- At the end of the outermost operation that queued a node onto an empty queue, the engine calls `scheduler.schedule()`. The default disposes during idle time. A scheduler that calls `disposeAll` disposes when the operation ends.
- A write's marking that reaches an unneeded node disposes it when the write, or the outermost batch, ends, so a write never leaves an unneeded node `Dirty` or `Check` and linked. A `Clean` dependency that this disposal leaves unneeded joins the cleanup queue with its value.
- The engine keeps one graph per process in module state: the batch depth, the batch's queue, the flush state, the cleanup queue, and the scheduler. The calling node stays a parameter of each tracked read.

## API

- `createSourceNode(value): SourceNode<TValue>` returns a source node that holds `value`.
- `createDerivedNode(computation, evaluation = LAZY): DerivedNode<TValue>` returns an `Uncomputed` derived node. `computation(callingNode, previous)` returns its value, and `previous` is `undefined` on a first computation. `evaluation` is `LAZY` or `EAGER`, fixed for the node's lifetime.
- `readSource(callingNode, source): TValue` returns the value of `source` and records it as a dependency of `callingNode`.
- `readDerived(callingNode, node): TValue` brings `node` up to date, returns its value, and records it as a dependency of `callingNode`.
- `retainDerivedNode(node, onChange?): Retain<TValue>` computes `node` and adds a holder of it. With `onChange`, the flush calls `onChange` after it changes the node's value.
- `releaseRetain(retain): void` removes the holder. A node left with no retain and no dependent joins the cleanup queue.
- `readDerivedNode(node): TValue` brings a retained `node` up to date and returns its value.
- `readUnretained(node): TValue` brings `node` up to date and returns its value without a retain, and queues it for cleanup if nothing holds it.
- `startBatch(): void` opens a batch, and `endBatch(): void` closes it. The outermost `endBatch` flushes.
- `writeSource(source, value): void` stores `value` and marks the readers of `source`. Outside a batch it flushes before it returns.
- `disposeNext(): boolean` disposes the next queued node if it is still unneeded, and returns whether the queue holds more. `disposeAll(): void` empties the queue.
- `scheduler.schedule: () => void` is called with no arguments at the end of the outermost operation that queued a node onto an empty cleanup queue. The default disposes during `requestIdleCallback` time, or for 5 ms per `setTimeout` task where `requestIdleCallback` does not exist. Assign `disposeAll` to dispose when each operation ends.
- `UNCOMPUTED`, `DIRTY`, `CHECK`, and `CLEAN` are the values of `node.status`, of type `Status`. `LAZY` and `EAGER`, of type `DerivedNodeEvaluation`, are the creation choices. `node.evaluation`, of type `EvaluationState`, is `LAZY`, `EAGER_INACTIVE` (equal to `EAGER`), `LAZY_OBSERVED`, or `EAGER_ACTIVE`, and the node refreshes in the flush while it is `LAZY_OBSERVED` or `EAGER_ACTIVE`.
- `SourceNode<TValue>`, `DerivedNode<TValue>`, and `Retain<TValue>` are the node and retain types. Every field is `readonly`. `DependentsStatus` is the type of `source.dependentsStatus`, which is internal bookkeeping.

```ts
import {
  createDerivedNode,
  createSourceNode,
  disposeAll,
  EAGER,
  endBatch,
  readDerived,
  readDerivedNode,
  readSource,
  readUnretained,
  releaseRetain,
  retainDerivedNode,
  scheduler,
  startBatch,
  writeSource,
} from '@isograph/pico-js';

// Dispose at the end of each operation, so the statuses below hold at once.
scheduler.schedule = disposeAll;

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

const greetingRetain = retainDerivedNode(greeting); // greetingRuns === 1
readDerivedNode(greeting); // 'Hello, Ada Lovelace'

writeSource(person, { first: 'Ada', last: 'Lovelace', age: 37 });
readDerivedNode(greeting); // 'Hello, Ada Lovelace', greetingRuns === 1

startBatch();
writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
writeSource(person, { first: 'Ada', last: 'King', age: 38 });
endBatch();
readDerivedNode(greeting); // 'Hello, Ada King', greetingRuns === 2

const rendered: Array<string> = [];
const view = createDerivedNode(
  (callingNode) => readDerived(callingNode, greeting),
  EAGER,
);
const viewRetain = retainDerivedNode(view, () =>
  rendered.push(readDerivedNode(view)),
);
writeSource(person, { first: 'Augusta', last: 'King', age: 38 });
// rendered is ['Hello, Augusta King']

const initials = createDerivedNode((callingNode) => {
  const { first, last } = readDerived(callingNode, name);
  return `${first[0]}${last[0]}`;
});
readUnretained(initials); // 'AK', and initials is Uncomputed again

releaseRetain(viewRetain);
releaseRetain(greetingRetain); // view, greeting, and name are Uncomputed
```

After the first write, `name` recomputes and returns `previous`, so `greeting` becomes `Clean` without running.

## Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior.

- The caller writes a source only when its value changed.
- A scheduler does not throw. A computation or an `onChange` may throw. After a throw, the statuses and values of the nodes that the throwing operation was updating are unspecified. The engine's module state is restored (Guarantees).
- A computation is synchronous. It calls `readSource` and `readDerived` only with the `callingNode` it received, and reads nothing else of the graph: no `readDerivedNode`, no `readUnretained`, and no `source.value`.
- A computation does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`.
- An `onChange` does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`. It may read through `readDerivedNode` and `source.value`.
- A computation's result depends only on `previous` and on the values its reads return.
- The derived nodes form a directed acyclic graph, counting the edges a node keeps from its previous run.
- `readDerivedNode` is called only on a retained node, and only outside computations.
- `readUnretained` is a one-off read. A read whose value must stay current until a later point, such as a render read until its commit, retains the node until then, because a scheduler may dispose an unretained node as soon as the read returns.
- Each `retainDerivedNode` is matched by exactly one `releaseRetain` of the retain it returned. A second release of the same retain unlinks it again and corrupts the node's retain list.
- Each `startBatch` is matched by exactly one `endBatch`. Both are called outside computations and `onChange` callbacks. A caller whose code between them can throw calls `endBatch` in a `finally`.
- `startBatch` and its `endBatch` run in one synchronous stretch of code and never span an `await`. On a server that serves several requests in one process, the batch is module state, so another request's writes made during the `await` would join the open batch.
- `disposeNext` and `disposeAll` are called outside computations, batches, and `onChange` callbacks. After a call to `scheduler.schedule()`, the scheduler calls `disposeNext` until it returns `false`, or calls `disposeAll`.
- The chains of derived nodes that a first computation or a recompute of `Dirty` nodes descends stay well below the call stack limit, because each node adds frames. In Node 24.12 with its default stack and one-line computations, the first computation of a chain overflows at about 2,000 derived nodes, and a recompute through a chain of `Dirty` nodes at about 2,200.
- A caller does not mutate a value that a read returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode` and `createDerivedNode`, and writes no field of a node or a retain, except through `scheduler.schedule`. The types enforce this for every field except `computation`, a method, which a type cannot mark `readonly`.
- A caller that needs current values reads after the batch flushes. Until the flush, a queued node and the nodes that read it return their previous values.

## Guarantees

pico-js upholds these while every contract holds.

- A read made while no batch is open returns the value the node's latest computation returned, or `previous` when that computation returned a value `===` to it, and each read of that computation returned the current value of the node it read.
- A computation runs only when its node is `Uncomputed`, or when a source its latest computation read was written, or a derived node it read recomputed to a value not `===` to its previous value.
- A read of a `Check` node reads its derived dependencies in the order its latest computation read them, and recomputes the node as soon as one of them changed, without reading the rest. A read of a `Dirty` lazy node recomputes it without reading its dependencies first.
- A recompute whose value is `===` to its previous value keeps the previous value and marks nothing. `NaN` counts as changed, and `-0` over a stored `0` keeps `0`.
- `retainDerivedNode` computes the node before it returns. A node that starts to refresh in the flush is `Clean` when it starts.
- The flush calls, for each node that it recomputed to a value not `===` to the previous one, the `onChange` of each of the node's retains once, most recent retain first, even when the node recomputed more than once, in the order of each node's first changed recompute. It calls them after every queued node is refreshed and after the disposal of the unneeded nodes the write's marking reached. A first computation, a cutoff, and a lazy node call no `onChange`.
- A queued node that a release in the same batch made lazy is skipped by the flush. It stays `Dirty` or `Check`, its dependents are `Check` or `Dirty`, and a read recomputes it as a lazy node.
- When a flush returns, every node that refreshes in the flush is `Clean`. A read of a `Dirty` or `Check` node that refreshes in the flush, inside a batch, returns its previous value and runs nothing.
- A `writeSource` inside a batch runs no computation and calls no `onChange`. A write walks the dependents list of the source and of each lazy node it takes out of `Clean` at most once each. A write returns without visiting a derived node when no `readSource` of the source has run since the source was created or last written.
- After a computation, its node is in the dependents list of each node it read, and of no other node, until its next computation or its disposal. Marking unlinks no edge.
- When the outermost operation returns, every derived node that is not `Uncomputed` and has no retain and no dependent is on the cleanup queue. After `disposeAll` returns, every derived node with no retain and no dependent is `Uncomputed`.
- When a write outside a batch, or the outermost `endBatch`, returns, no node that its marking took out of `Clean` is both unneeded and `Dirty` or `Check`. Each such node is `Uncomputed`, or `Clean` because a read inside the batch recomputed it.
- A disposed node is `Uncomputed`, holds `undefined`, and has no edges. Disposal marks nothing and calls nothing. `disposeNext` and `disposeAll` skip a queued node that is needed again or already `Uncomputed`.
- A derived node that becomes unneeded and is read again before a disposal reaches it keeps its value and does not run for that read, whether the read comes from a later run in the same operation or from a later operation.
- The engine calls `scheduler.schedule()` only when an outermost operation returns, with no batch open and no flush running, and only once per time the cleanup queue went from empty to non-empty since the last call. A scheduler that calls `disposeAll` therefore disposes every unneeded node when each outermost operation returns.
- A recompute whose reads match, in order, the edges of its previous run allocates no edge and links or unlinks none.
- Marking, verification of `Check` nodes, and disposal keep their stacks in arrays. In Node 24.12, a read through a chain of 200,000 `Check` nodes, and a release followed by `disposeAll` of a chain of 100,000 nodes, do not overflow.
- If a computation or an `onChange` throws, the engine restores its module state before the exception leaves the outermost entry point: the flush state is idle, the marked list is empty, and the batch's queue is cleared by the `endBatch` or `writeSource` that started the flush. Later operations, including those of other requests on a server, are unaffected. The graph that was being updated may be partially updated. A scheduler call that the throwing operation would have made happens at the end of the next outermost operation instead.
- No module-level array keeps a node reachable after the operation that used it returns, except the cleanup queue, which holds each queued node until a disposal pops it. The marked list is emptied at the end of each outermost write or batch, and the batch's queue is dropped when its flush starts.
- `retainDerivedNode` allocates one `Retain`. `releaseRetain` and a `readDerivedNode` of a `Clean` node allocate nothing beyond growth of the cleanup queue's array. A write outside a batch that queues no node allocates no queue, and a flush that changes no node with a retain allocates no list.

## Design choices

- Edges are objects in intrusive doubly linked lists, one in the reader's dependency list and one in the dependency's dependents list, instead of sets. Linking and unlinking are O(1) and allocate nothing.
- A recompute reuses the edge at each read position when it reads the same node there, so a stable computation allocates and relinks nothing.
- A `Dirty` node keeps its edges linked, so a second write that reaches it finds it `Dirty` and stops, and its recompute reuses them.
- A source's `dependentsStatus` records that a write left every reader `Dirty`, so the next write skips the list until a computation reads the source again.
- Marking, verification, and disposal walk explicit stacks, so the graph's depth costs no call-stack frames there.
- A computation receives an explicit `callingNode` and passes it to every read, instead of the engine tracking the running computation in module state.
- The engine keeps one graph per process in module state: the batch, the flush state, and the cleanup queue. A batch allocates no object, and no node carries a pointer to its graph.
- A retain is an object in its node's retain list, and the list replaces a retain count. A node is retained while its list is not empty, and the flush finds the `onChange` callbacks by walking it.
- Whether a node refreshes in the flush is encoded in its `evaluation` field, so marking, pull, and verification test one field.
- A node that becomes unneeded is queued, not disposed, so a node that is read again before the scheduler runs keeps its value. A write's marking disposes the unneeded nodes it reaches at once, so writes do not keep walking nodes that nothing holds.
- Each kind of node is one object shape, so property access stays monomorphic.
- Public types are `readonly`, and the engine writes through an internal `Mutable` view, so the types enforce the field contract.
- pico-js checks no contract at run time. A future version may add the checks as a compile-time option.

### Module state

- The engine keeps its batch depth, its flush state, its cleanup queue, and its scheduler in module state that the whole process shares. There is one graph per process.
- On a server, every request shares that state. A batch therefore must not span an `await`, because another request's writes would join it.
- A computation or an `onChange` that throws leaves the graph it was updating partially updated. The engine restores the shared state before the exception reaches the caller, so other requests are unaffected.
- On a server, `requestIdleCallback` does not exist, so the default scheduler disposes in `setTimeout` tasks. The cleanup queue can hold nodes of several requests at once, and each disposal re-checks that its node is still unneeded before it disposes it.
````

The Model, Contracts, and Guarantees sections at the top of this doc copy the README's sections word for word.

### Readers before the Isograph runtime depends on it

AGENTS.md says we do not have anything we do not use, and tests-only is not a reader. Until the Isograph runtime calls it, the package's reader is the user's stated goal that the engine is usable by projects outside Isograph (pico-runtime.md, Goal), and `README.md` documents the package for those projects. pico-runtime.md core change 4 ships the package before its first workspace caller on that ground; core change 5 is the first caller of the source functions and core change 8 of the derived-node functions. The package ships alone, as its own change.

## Deltas from the prototype

The engine below is `src/graph.ts` of `f0b001a9d` with these changes. Every function body not named here is copied verbatim from `f0b001a9d`.

```ts
// from libs/pico-js/src/graph.ts at f0b001a9d
export function createDerivedNode<TValue>(computation: (callingNode: DerivedNode<unknown>, previous: TValue | undefined) => TValue): DerivedNode<TValue>;
export function createEagerDerivedNode<TValue>(computation: (callingNode: DerivedNode<unknown>, previous: TValue | undefined) => TValue): DerivedNode<TValue>;
export function setDerivedNodeEvaluation(node: DerivedNode<unknown>, evaluation: DerivedNodeEvaluation): void;
export function batch(body: () => void): void;
export function readRetained<TValue>(retain: Retain<TValue>): TValue;
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue; // also an untracked read inside a computation
export function resetForTests(): void;
export function collectorContents(): ReadonlyArray<DerivedNode<unknown>>;
let schedulePending = false;
let flushing = 0;
```

```ts
// from libs/pico-js/src/nodes.ts, src/retain.ts, src/batch.ts
export function createDerivedNode<TValue>(computation: (callingNode: DerivedNode<unknown>, previous: TValue | undefined) => TValue, evaluation: DerivedNodeEvaluation = LAZY): DerivedNode<TValue>;
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue; // top-level only
let scheduleStatus: ScheduleStatus = SCHEDULE_NOT_PENDING;
let flushState: FlushState = FLUSH_IDLE;
```

- `createDerivedNode` takes the evaluation as an optional second parameter, `LAZY` by default, and `createEagerDerivedNode` is removed. `newDerivedNode` folds into `createDerivedNode`.
- `setDerivedNodeEvaluation` is removed, so the creation choice is fixed. The low bit of `evaluation` still holds it, and `evaluationState(node)` reads it from there instead of taking a `choice` parameter. `updateEvaluation` calls `evaluationState(node)`. The queue path of `setDerivedNodeEvaluation` that queued a `Dirty` or `Check` node made eager has no other caller and goes with it: a node that starts to refresh in the flush starts through a retain, which pulls it first, so it is `Clean`.
- `batch(body)` is removed. `startBatch()` and `endBatch()` remain.
- `readRetained(retain)` is removed. `readDerivedNode(node)` is the one top-level read of a retained node, and a computation no longer calls it, so there are no untracked reads.
- `resetForTests` and `collectorContents` are removed, because only tests call them (AGENTS.md, Tests: an API only tests call should not exist). Tests observe the cleanup queue through `disposeNext`'s return value and node statuses, and the random test gets fresh module state per case (Tests).
- `schedulePending: boolean` becomes `scheduleStatus: ScheduleStatus`, and the counter `flushing` becomes `flushState: FlushState`, a two-value union. A flush cannot run inside another, because an `onChange` does not write or end a batch (contract), so the count is never above 1.
- `collectorStack` is renamed `cleanupQueue`. `addToCollector` keeps its name.
- `endBatch`'s and `flushOrQueue`'s shared tail is the function `flushOrDisposeMarked`.
- `flush` wraps `flushQueue` in `try`/`finally`, which sets `flushState` to `FLUSH_IDLE` and empties `markedUnneeded`, so a computation or an `onChange` that throws leaves the module state clean for the next operation. The prototype restored nothing and needed `resetForTests` after a throw.
- `scheduleIfOutermost` tests the batch depth and the flush state in `batch.ts`, and calls `scheduleIfPending` in `retain.ts`, which owns `scheduleStatus`.
- Comments that named `setDerivedNodeEvaluation`, `readRetained`, or untracked reads are rewritten; the `flush` comment states the queue invariant without the lazy-then-eager case.

With test-only emulations of the removed calls added back (`setDerivedNodeEvaluation`, `createEagerDerivedNode`, `readRetained`, `collectorContents`, `resetForTests`), this source passes the 195 runtime tests of `6f431ec0e` and random seeds 1 to 2,000 under vitest 2.1.9, which checks that the split and the renames change no behavior.

## Source layout

`f0b001a9d`'s `graph.ts` is 1,452 lines. The engine splits it by concern into five files. tsdown bundles `src/index.ts` and everything it imports into one scope, so a call across files is a call within one module in `dist/`, and costs what it cost in one file.

An ES module cannot reassign a `let` it imports, so each `let` lives in the file whose code writes it, and the other files reach it through functions. A `const` array is imported directly, because its importers mutate it and never reassign it.

```text
state                  owner          writers                                    readers in other files
batchDepth             batch.ts       startBatch, endBatch                       currentBatchQueue(), flushOrQueue() (retain.ts)
batchQueue             batch.ts       endBatch, flushOrQueue                     currentBatchQueue() (retain.ts)
flushState             batch.ts       flush                                      none
scheduleStatus         retain.ts      addToCollector, scheduleIfPending          scheduleIfPending() (batch.ts)
cleanupQueue           retain.ts      addToCollector, disposeNext, disposeAll    addToCollector() (pull.ts)
scheduler              retain.ts      the caller                                 index.ts re-export
markedUnneeded         marking.ts     markDirty, markCheck, their queue copies   disposeMarked (retain.ts), flushOrDisposeMarked and flushQueue (batch.ts)
```

The batch depth and the flush state live in `batch.ts`, beside the flush that uses them. The cleanup queue, its schedule status, and the scheduler live in `retain.ts`, beside every function that pushes, pops, or schedules, except the two push sites in `pull.ts` (the repoint in `readDerived` and `unlinkDropped`), which call `addToCollector`. `markedUnneeded` lives in `marking.ts`, whose four marking functions push onto it.

The imports form cycles: `batch.ts` and `retain.ts` import each other, and `pull.ts` and `retain.ts` import each other. No module calls an import while it evaluates, so the cycles are safe under ES module evaluation and under vitest, which loads `src/` unbundled.

```text
nodes.ts    imports nothing
marking.ts  imports nodes.ts
pull.ts     imports nodes.ts, marking.ts (markCheckDependentsDirty), retain.ts (addToCollector)
retain.ts   imports nodes.ts, marking.ts (markCheckQueue, markedUnneeded), pull.ts (pull, unlinkAll, unlinkEdge), batch.ts (currentBatchQueue, flushOrQueue, scheduleIfOutermost)
batch.ts    imports nodes.ts, marking.ts (markDirty, markDirtyQueue, markedUnneeded, storeValue), pull.ts (rerun, verify), retain.ts (disposeMarked, scheduleIfPending)
index.ts    re-exports
```

An exported function that `index.ts` does not re-export is internal to the package: `dist/` exposes only `index.ts`'s names.

## `src/nodes.ts`

```ts
// from libs/pico-js/src/nodes.ts
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
export type Edge<TDependency> = {
  // Repointed when a recompute reads a different node at this position.
  dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  // Set by an append and by `trim`.
  nextDependency: Edge<TDependency> | undefined;
  // Set by `linkEdge` and by `unlinkEdge` of a neighbor.
  previousDependent: Edge<TDependency> | undefined;
  nextDependent: Edge<TDependency> | undefined;
};

export type SourceEdge = Edge<SourceNode<unknown>>;
export type DerivedEdge = Edge<DerivedNode<unknown>>;

export type NodeWithDependents<TDependency> = {
  firstDependent: Edge<TDependency> | undefined;
  lastDependent: Edge<TDependency> | undefined;
};

/** The view of a node whose fields the engine writes. */
export type Mutable<T> = { -readonly [K in keyof T]: T[K] };

export type MutableSourceNode<TValue> = Mutable<SourceNode<TValue>>;

/** The engine's view of a derived node, with the fields only it reads. */
export type MutableDerivedNode<TValue> = Mutable<DerivedNode<TValue>> & {
  onChangeStatus: OnChangeStatus;
};

export type MutableRetain<TValue> = Mutable<Retain<TValue>>;

/**
 * The eager nodes of a batch, created on its first push, so a batch that
 * queues nothing allocates no queue. The flush walks it as its eager-node
 * array.
 */
export type Queue = Array<MutableDerivedNode<unknown>>;

/** A node in the dependents list may be `Check` or `Clean`. */
export const DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY = 0;
/** Every node in the dependents list is `Dirty`, so a write skips the list. */
export const DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY = 1;

export type DependentsStatus =
  | typeof DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY
  | typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;

/**
 * Internal invariant: `dependentsStatus` is `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`
 * from the source's creation or from a write until the next `readSource`,
 * because a node leaves `Dirty` only by a recompute and joins the list only
 * through `readSource`.
 *
 * Contract: a caller writes no field. `writeSource` writes `value`. The type
 * enforces it.
 */
export type SourceNode<TValue> = {
  readonly value: TValue;
  readonly firstDependent: SourceEdge | undefined;
  readonly lastDependent: SourceEdge | undefined;
  readonly dependentsStatus: DependentsStatus;
};

/** Created lazy, and no retain has an `onChange`. It recomputes when pulled. */
export const LAZY = 0;
/** Created eager. A node that is not retained behaves as lazy. */
export const EAGER = 1;
/** Created eager, and not retained, so it behaves as lazy. */
export const EAGER_INACTIVE = EAGER;
/**
 * Created lazy, and a retain has an `onChange`. Marking stops at it, and it
 * refreshes in the flush.
 */
export const LAZY_OBSERVED = 2;
/**
 * Created eager, and retained. Marking stops at it, and it refreshes in the
 * flush.
 */
export const EAGER_ACTIVE = 3;

/** The evaluation a caller chooses at creation. */
export type DerivedNodeEvaluation = typeof LAZY | typeof EAGER;

/**
 * The evaluation a node has: the creation choice in the low bit, and at least
 * `LAZY_OBSERVED` while marking stops at the node.
 */
export type EvaluationState =
  | typeof LAZY
  | typeof EAGER_INACTIVE
  | typeof LAZY_OBSERVED
  | typeof EAGER_ACTIVE;

/** The node is in no flush's list of nodes whose `onChange` calls it makes. */
export const ON_CHANGE_NOT_QUEUED = 0;
/**
 * The node is in the list of the flush that is running, so a second changed
 * recompute in that flush does not add it again.
 */
export const ON_CHANGE_QUEUED = 1;

export type OnChangeStatus =
  | typeof ON_CHANGE_NOT_QUEUED
  | typeof ON_CHANGE_QUEUED;

/**
 * Internal invariants of a derived node whose computation is not running:
 * - `Uncomputed`: `value` is `undefined`, both dependency lists are empty, and
 *   the node has no dependents.
 * - `Dirty`: `value` is the last computed value. A node below `LAZY_OBSERVED`
 *   has `Check` or `Dirty` dependents. A node at `LAZY_OBSERVED` or above is
 *   queued in the open batch or the running flush, and its dependents may be
 *   `Clean`.
 * - `Check`: as `Dirty`.
 * - `Clean`: `value` is up to date, unless the node depends through `Clean`
 *   nodes on a `Dirty` or `Check` node at `LAZY_OBSERVED` or above.
 *
 * In every status, each edge in the node's dependency lists is in its
 * dependency's dependents list. A node that is not `Uncomputed`, has an empty
 * retain list, and has no dependent is on the cleanup queue, or on the marked
 * list of the write that is running, except a node that a computation returned
 * to a `readDerived` that has not linked its edge yet.
 *
 * Contract: a caller writes no field. The type enforces it.
 */
export type DerivedNode<TValue> = {
  readonly value: TValue;
  // The head of the list of the node's retains, most recent first. The node is
  // retained while the list is not empty.
  readonly firstRetain: Retain<TValue> | undefined;
  readonly status: Status;
  readonly evaluation: EvaluationState;
  readonly firstSourceDependency: SourceEdge | undefined;
  readonly lastSourceDependency: SourceEdge | undefined;
  readonly firstDerivedDependency: DerivedEdge | undefined;
  readonly lastDerivedDependency: DerivedEdge | undefined;
  readonly firstDependent: DerivedEdge | undefined;
  readonly lastDependent: DerivedEdge | undefined;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

/**
 * One holder of a node, which `retainDerivedNode` returns and
 * `releaseRetain` drops. It is in its node's retain list.
 */
export type Retain<TValue> = {
  readonly node: DerivedNode<TValue>;
  // Guarantee: the flush calls it once for each change of the node's value
  // that the flush made, after every queued eager node is refreshed. A first
  // computation does not call it.
  //
  // Contract: the callback does not throw. It does not write, start or end a
  // batch, retain, release, dispose, or call `readUnretained`. It may read
  // through `readDerivedNode` and `source.value`.
  readonly onChange: (() => void) | undefined;
  readonly previous: Retain<TValue> | undefined;
  readonly next: Retain<TValue> | undefined;
};

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return {
    value,
    firstDependent: undefined,
    lastDependent: undefined,
    dependentsStatus: DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY,
  };
}

/**
 * Returns an `Uncomputed` derived node. `evaluation` is fixed for the node's
 * lifetime.
 *
 * Contract: `computation` is synchronous and does not throw. It reads other
 * nodes only through `readSource` and `readDerived` with the `callingNode` it
 * receives. It does not write, start or end a batch, retain, release, dispose,
 * or call `readDerivedNode` or `readUnretained`.
 */
export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
  evaluation: DerivedNodeEvaluation = LAZY,
): DerivedNode<TValue> {
  const node: MutableDerivedNode<TValue> = {
    value: undefined as unknown as TValue,
    firstRetain: undefined,
    onChangeStatus: ON_CHANGE_NOT_QUEUED,
    status: UNCOMPUTED,
    evaluation,
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
  return node;
}
```

## `src/marking.ts`

```ts
// from libs/pico-js/src/marking.ts
import {
  CHECK,
  CLEAN,
  DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY,
  DIRTY,
  LAZY_OBSERVED,
  UNCOMPUTED,
  type DerivedEdge,
  type Edge,
  type MutableDerivedNode,
  type MutableSourceNode,
  type Queue,
  type SourceEdge,
} from './nodes';

/**
 * The nodes that marking newly marked `Dirty` or `Check` while they had no
 * retain and no dependent, since the last disposal of them. One array, reused,
 * which `disposeMarked` empties at the end of the outermost write or batch.
 */
export const markedUnneeded: Array<MutableDerivedNode<unknown>> = [];

/** Returns the first edge of the dependents list if the write must mark it. */
export function storeValue<TValue>(
  source: MutableSourceNode<TValue>,
  value: TValue,
): SourceEdge | undefined {
  source.value = value;
  // A source that no computation has read, and one whose readers a write left
  // Dirty, return here.
  if (source.dependentsStatus === DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY) {
    return undefined;
  }
  // Marking leaves every node in the list Dirty, and an empty list is all
  // Dirty vacuously.
  source.dependentsStatus = DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;
  return source.firstDependent;
}

export function markDirty<TDependency>(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  first: Edge<TDependency>,
): void {
  // `held` is the latest marked lazy node with dependents, and the stack holds
  // the earlier ones, so a write that reaches only one allocates no array.
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  let held: MutableDerivedNode<unknown> | undefined;
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent as MutableDerivedNode<unknown>;
    edge = edge.nextDependent;
    switch (node.status) {
      case CLEAN:
        node.status = DIRTY;
        // Marking stops at an eager node. The flush refreshes it, and marks
        // its dependents only if its value changed.
        if (node.evaluation >= LAZY_OBSERVED) {
          eagerNodes.push(node);
          break;
        }
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
        } else if (node.firstRetain === undefined) {
          markedUnneeded.push(node);
        }
        break;
      // Internal invariant: a Check node marked its dependents, or queued
      // itself, when it left Clean.
      case CHECK:
        node.status = DIRTY;
        break;
      // Internal invariant: a Dirty node was marked earlier. An Uncomputed
      // node has no dependency, so no dependents list holds it.
      case DIRTY:
      case UNCOMPUTED:
        break;
    }
  }
  // The dependents of the nodes marked Dirty might keep their values, so they
  // become Check.
  if (held !== undefined) {
    markCheck(eagerNodes, held, stack);
  }
}

/**
 * Marks `Clean` dependents of `held` and of each node on the stack `Check`,
 * and continues through each lazy node it marks.
 */
export function markCheck(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  held: MutableDerivedNode<unknown>,
  initialStack: Array<MutableDerivedNode<unknown>> | undefined,
): void {
  // `marked` is the node whose dependents list is walked. The stack holds the
  // marked lazy nodes whose dependents are not walked yet.
  let stack = initialStack;
  let marked: MutableDerivedNode<unknown> | undefined = held;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent as MutableDerivedNode<unknown>;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case CLEAN:
          dependent.status = CHECK;
          if (dependent.evaluation >= LAZY_OBSERVED) {
            eagerNodes.push(dependent);
            break;
          }
          if (dependent.firstDependent !== undefined) {
            // After the last dependent of the list, the walk continues into
            // that dependent's list without a push.
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          } else if (dependent.firstRetain === undefined) {
            markedUnneeded.push(dependent);
          }
          break;
        // Internal invariant: a lazy node out of Clean has no Clean
        // dependents, and marking stops at an eager one.
        case CHECK:
        case DIRTY:
        case UNCOMPUTED:
          break;
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
}

/**
 * `markDirty` into a queue that is created on the first push. A copy, because
 * an optional queue in `markDirty` slows the flush's marking.
 */
export function markDirtyQueue<TDependency>(
  initialQueue: Queue | undefined,
  first: Edge<TDependency>,
): Queue | undefined {
  let queue = initialQueue;
  // `held` is the latest marked lazy node with dependents, and the stack holds
  // the earlier ones, so a write that reaches only one allocates no array.
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  let held: MutableDerivedNode<unknown> | undefined;
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent as MutableDerivedNode<unknown>;
    edge = edge.nextDependent;
    switch (node.status) {
      case CLEAN:
        node.status = DIRTY;
        // Marking stops at an eager node. The flush refreshes it, and marks
        // its dependents only if its value changed.
        if (node.evaluation >= LAZY_OBSERVED) {
          queue = queueEagerNode(queue, node);
          break;
        }
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
        } else if (node.firstRetain === undefined) {
          markedUnneeded.push(node);
        }
        break;
      // Internal invariant: a Check node marked its dependents, or queued
      // itself, when it left Clean.
      case CHECK:
        node.status = DIRTY;
        break;
      // Internal invariant: a Dirty node was marked earlier. An Uncomputed
      // node has no dependency, so no dependents list holds it.
      case DIRTY:
      case UNCOMPUTED:
        break;
    }
  }
  // The dependents of the nodes marked Dirty might keep their values, so they
  // become Check.
  return held === undefined ? queue : markCheckQueue(queue, held, stack);
}

/** `markCheck` for a batch of one. Copied as `markDirtyQueue` is. */
export function markCheckQueue(
  initialQueue: Queue | undefined,
  held: MutableDerivedNode<unknown>,
  initialStack: Array<MutableDerivedNode<unknown>> | undefined,
): Queue | undefined {
  let queue = initialQueue;
  // `marked` is the node whose dependents list is walked. The stack holds the
  // marked lazy nodes whose dependents are not walked yet.
  let stack = initialStack;
  let marked: MutableDerivedNode<unknown> | undefined = held;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent as MutableDerivedNode<unknown>;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case CLEAN:
          dependent.status = CHECK;
          if (dependent.evaluation >= LAZY_OBSERVED) {
            queue = queueEagerNode(queue, dependent);
            break;
          }
          if (dependent.firstDependent !== undefined) {
            // After the last dependent of the list, the walk continues into
            // that dependent's list without a push.
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          } else if (dependent.firstRetain === undefined) {
            markedUnneeded.push(dependent);
          }
          break;
        // Internal invariant: a lazy node out of Clean has no Clean
        // dependents, and marking stops at an eager one.
        case CHECK:
        case DIRTY:
        case UNCOMPUTED:
          break;
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
  return queue;
}

/**
 * Creates the queue with `node` in an array literal, because a push onto an
 * empty array allocates room for 17 elements. Out of line, because the same
 * branch inside `markDirtyQueue` slowed a write that queues nothing.
 */
function queueEagerNode(
  queue: Queue | undefined,
  node: MutableDerivedNode<unknown>,
): Queue {
  if (queue === undefined) {
    return [node];
  }
  queue.push(node);
  return queue;
}

export function markCheckDependentsDirty(first: DerivedEdge | undefined): void {
  let edge = first;
  while (edge !== undefined) {
    const node = edge.dependent as MutableDerivedNode<unknown>;
    edge = edge.nextDependent;
    switch (node.status) {
      case CHECK:
        node.status = DIRTY;
        break;
      case DIRTY:
      case CLEAN:
      case UNCOMPUTED:
        break;
    }
  }
}
```

## `src/pull.ts`

```ts
// from libs/pico-js/src/pull.ts
import { markCheckDependentsDirty } from './marking';
import {
  CHECK,
  CLEAN,
  DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY,
  DIRTY,
  LAZY_OBSERVED,
  UNCOMPUTED,
  type DerivedEdge,
  type DerivedNode,
  type Edge,
  type MutableDerivedNode,
  type MutableSourceNode,
  type NodeWithDependents,
  type SourceEdge,
  type SourceNode,
} from './nodes';
import { addToCollector } from './retain';

export function linkEdge<TDependency extends NodeWithDependents<TDependency>>(
  edge: Edge<TDependency>,
): void {
  const dependency = edge.dependency;
  // The edge becomes the tail. It may be an unlinked edge, so its
  // previousDependent and nextDependent are overwritten.
  const tail = dependency.lastDependent;
  edge.previousDependent = tail;
  edge.nextDependent = undefined;
  // An empty list has no tail, and the edge becomes its head as well.
  if (tail === undefined) {
    dependency.firstDependent = edge;
  } else {
    tail.nextDependent = edge;
  }
  dependency.lastDependent = edge;
}

export function unlinkEdge<TDependency extends NodeWithDependents<TDependency>>(
  edge: Edge<TDependency>,
): void {
  const dependency = edge.dependency;
  // The edge's own pointers stay. unlinkAll still follows its nextDependency,
  // and linkEdge overwrites its dependents pointers when the edge is reused.
  const previous = edge.previousDependent;
  const next = edge.nextDependent;
  // An edge with no previous edge is the head, and one with no next edge is
  // the tail.
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

export function unlinkAll<TDependency extends NodeWithDependents<TDependency>>(
  first: Edge<TDependency> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    edge = edge.nextDependency;
  }
}

/** Contract: called only while the computation of `callingNode` runs. */
export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableSource = source as MutableSourceNode<TValue>;
  // The calling node will be Clean in this source's dependents list, so the
  // list is no longer known to be all Dirty.
  mutableSource.dependentsStatus = DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY;
  // `lastSourceDependency` is the last edge this run has read. The edges
  // after it are left from the previous run, and `next` is the one at this
  // read's position.
  const last = mutableCallingNode.lastSourceDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    // An edge from the previous run to a different source moves to this
    // source's dependents list. An edge to the same source is reused as is.
    if (next.dependency !== mutableSource) {
      unlinkEdge(next);
      next.dependency = mutableSource as SourceNode<unknown>;
      linkEdge(next);
    }
    mutableCallingNode.lastSourceDependency = next;
  } else if (last === undefined || last.dependency !== mutableSource) {
    // No edge is left from the previous run, so this read appends one, unless
    // it repeats the run's previous `readSource`.
    const edge: SourceEdge = {
      dependency: mutableSource as SourceNode<unknown>,
      dependent: mutableCallingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge);
    if (last === undefined) {
      mutableCallingNode.firstSourceDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    mutableCallingNode.lastSourceDependency = edge;
  }
  return mutableSource.value;
}

/** Contract: called only while the computation of `callingNode` runs. */
export function readDerived<TValue>(
  callingNode: DerivedNode<unknown>,
  node: DerivedNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  // `lastDerivedDependency` is the last edge this run has read. The edges
  // after it are left from the previous run, and `next` is the one at this
  // read's position.
  const last = mutableCallingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstDerivedDependency
      : last.nextDependency;
  if (next !== undefined) {
    // An edge from the previous run to a different node moves to this
    // node's dependents list. An edge to the same node is reused as is.
    if (next.dependency !== node) {
      const previousDependency = next.dependency as MutableDerivedNode<unknown>;
      unlinkEdge(next);
      next.dependency = node as DerivedNode<unknown>;
      linkEdge(next);
      if (
        previousDependency.firstDependent === undefined &&
        previousDependency.firstRetain === undefined
      ) {
        addToCollector(previousDependency);
      }
    }
    mutableCallingNode.lastDerivedDependency = next;
  } else if (last === undefined || last.dependency !== node) {
    // No edge is left from the previous run, so this read appends one, unless
    // it repeats the run's previous `readDerived`.
    const edge: DerivedEdge = {
      dependency: node as DerivedNode<unknown>,
      dependent: mutableCallingNode,
      nextDependency: undefined,
      previousDependent: undefined,
      nextDependent: undefined,
    };
    linkEdge(edge);
    if (last === undefined) {
      mutableCallingNode.firstDerivedDependency = edge;
    } else {
      last.nextDependency = edge;
    }
    mutableCallingNode.lastDerivedDependency = edge;
  }
  return value;
}

/**
 * Guarantee: a `Dirty` or `Check` node at `LAZY_OBSERVED` or above returns its
 * previous value and runs nothing.
 */
export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  const mutableNode = node as MutableDerivedNode<TValue>;
  switch (mutableNode.status) {
    case CLEAN:
      return mutableNode.value;
    case DIRTY:
      return mutableNode.evaluation >= LAZY_OBSERVED
        ? mutableNode.value
        : recompute(mutableNode);
    case CHECK:
      if (mutableNode.evaluation >= LAZY_OBSERVED) {
        return mutableNode.value;
      }
      // A Check node recomputes only if a derived dependency changed. When
      // none did, verify leaves it Clean and its value is current.
      return verify(mutableNode) === DIRTY
        ? recompute(mutableNode)
        : mutableNode.value;
    case UNCOMPUTED:
      return computeFirst(mutableNode);
  }
}

/**
 * Pulls the derived dependencies of the `Check` node `root` in read order,
 * descending into lazy `Check` dependencies without recursion. Returns `DIRTY`
 * when the caller must recompute `root`.
 */
export function verify(
  root: MutableDerivedNode<unknown>,
): typeof DIRTY | typeof CLEAN {
  // `node` is the lazy Check node being verified, and `edge` is its next
  // derived dependency to pull. The stack holds the edges the descent took.
  let verifyStack: Array<DerivedEdge> | undefined;
  let node: MutableDerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  while (true) {
    if (edge !== undefined) {
      const dependency = edge.dependency as MutableDerivedNode<unknown>;
      // A lazy Check dependency is verified by descending into it, in place of
      // a recursive pull.
      if (
        dependency.status === CHECK &&
        dependency.evaluation < LAZY_OBSERVED
      ) {
        // A dependency with one dependent leads back up through that edge, so
        // only an edge into a dependency with several dependents is pushed.
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
      // A pull that recomputes the dependency to a new value marks `node`
      // Dirty.
      pull(dependency);
      if (node.status !== DIRTY) {
        edge = edge.nextDependency;
        continue;
      }
      if (node === root) {
        return DIRTY;
      }
      // A Dirty node below the root recomputes without pulling the rest of its
      // dependencies. A changed value marks its Check dependents Dirty.
      recompute(node);
    } else {
      // Internal invariant: a write to a source of a Check node would have
      // made it Dirty, so it is Clean once every derived dependency came back
      // unchanged.
      node.status = CLEAN;
    }
    // `node` is up to date. The ascent returns to the node that descended into
    // it, and recomputes each node on the way that came back Dirty.
    while (true) {
      if (node === root) {
        return CLEAN;
      }
      const only = node.firstDependent!;
      edge = only.nextDependent === undefined ? only : verifyStack!.pop()!;
      node = edge.dependent as MutableDerivedNode<unknown>;
      // A node that is still Check continues with its next dependency.
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

export function computeFirst<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  trim(node);
  node.status = CLEAN;
  node.value = value;
  return value;
}

function recompute<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const previous = node.value;
  const value = rerun(node, previous);
  // On cutoff the stored value stays and nothing is marked, so every reader
  // reads the same value.
  if (value === previous) {
    return previous;
  }
  node.value = value;
  // Internal invariant: a lazy node's dependents are already Check or Dirty.
  // A Check dependent now has a changed dependency.
  markCheckDependentsDirty(node.firstDependent);
  return value;
}

export function rerun<TValue>(
  node: MutableDerivedNode<TValue>,
  previous: TValue,
): TValue {
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  trim(node);
  node.status = CLEAN;
  return value;
}

/**
 * Unlinks and drops the edges after the last one the computation read, and
 * pushes each derived dependency that the drop leaves with an empty retain list and
 * no dependent onto the collector.
 */
export function trim(node: MutableDerivedNode<unknown>): void {
  // The edges after the cursor are left from the previous run, which read more
  // than this one. A run that read nothing of a kind drops the whole list.
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
    // A call with an empty list slowed a recompute of a node that reads only
    // sources.
    const first = node.firstDerivedDependency;
    if (first !== undefined) {
      unlinkDropped(first);
      node.firstDerivedDependency = undefined;
    }
  } else if (lastDerived.nextDependency !== undefined) {
    unlinkDropped(lastDerived.nextDependency);
    lastDerived.nextDependency = undefined;
  }
}

/**
 * Unlinks the derived edges from `first` on, and pushes each dependency that
 * an unlink leaves with an empty retain list and no dependent onto the collector.
 */
export function unlinkDropped(first: DerivedEdge): void {
  let edge: DerivedEdge | undefined = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    const dependency = edge.dependency as MutableDerivedNode<unknown>;
    if (
      dependency.firstDependent === undefined &&
      dependency.firstRetain === undefined
    ) {
      addToCollector(dependency);
    }
    edge = edge.nextDependency;
  }
}
```

## `src/retain.ts`

```ts
// from libs/pico-js/src/retain.ts
import { currentBatchQueue, flushOrQueue, scheduleIfOutermost } from './batch';
import { markCheckQueue, markedUnneeded } from './marking';
import {
  CHECK,
  CLEAN,
  DIRTY,
  EAGER,
  EAGER_ACTIVE,
  EAGER_INACTIVE,
  LAZY,
  LAZY_OBSERVED,
  UNCOMPUTED,
  type DerivedNode,
  type EvaluationState,
  type MutableDerivedNode,
  type MutableRetain,
  type Retain,
} from './nodes';
import { pull, unlinkAll, unlinkEdge } from './pull';

/**
 * The cleanup queue: one stack, never truncated, of derived nodes that were
 * left with an empty retain list and no dependent, once per time they were
 * left so. `disposeNext` and `disposeAll` dispose the ones that are still
 * unneeded.
 */
const cleanupQueue: Array<MutableDerivedNode<unknown>> = [];

/** A push found the cleanup queue empty, and the scheduler has not been called. */
const SCHEDULE_NOT_PENDING = 0;
const SCHEDULE_PENDING = 1;

type ScheduleStatus = typeof SCHEDULE_NOT_PENDING | typeof SCHEDULE_PENDING;

let scheduleStatus: ScheduleStatus = SCHEDULE_NOT_PENDING;

/**
 * Called by the engine, with no arguments, at the end of the outermost
 * operation after the cleanup queue went from empty to non-empty. It may call
 * `disposeAll` synchronously. Assign `scheduler.schedule` to replace it.
 *
 * Contract: after a call, the scheduler keeps calling `disposeNext` until it
 * returns false, or calls `disposeAll`.
 */
export const scheduler: { schedule: () => void } = {
  schedule: defaultSchedule,
};

/** The time a `setTimeout` task spends disposing, in milliseconds. */
const TIMEOUT_DISPOSAL_MS = 5;

/**
 * Disposes during `requestIdleCallback` time where it exists, and for a few
 * milliseconds of a `setTimeout` task elsewhere, while nodes remain.
 */
function defaultSchedule(): void {
  if (typeof requestIdleCallback === 'function') {
    requestIdleCallback(disposeWhileIdle);
  } else {
    setTimeout(disposeForTimeout, 0);
  }
}

function disposeWhileIdle(deadline: IdleDeadline): void {
  let remaining = true;
  while (remaining && deadline.timeRemaining() > 0) {
    remaining = disposeNext();
  }
  if (remaining) {
    requestIdleCallback(disposeWhileIdle);
  }
}

function disposeForTimeout(): void {
  const end = performance.now() + TIMEOUT_DISPOSAL_MS;
  let remaining = true;
  while (remaining && performance.now() < end) {
    remaining = disposeNext();
  }
  if (remaining) {
    setTimeout(disposeForTimeout, 0);
  }
}

export function addToCollector(node: MutableDerivedNode<unknown>): void {
  if (cleanupQueue.length === 0) {
    scheduleStatus = SCHEDULE_PENDING;
  }
  cleanupQueue.push(node);
}

/**
 * Calls the scheduler if a push found the cleanup queue empty since the last
 * call. Only `scheduleIfOutermost` calls it.
 */
export function scheduleIfPending(): void {
  if (scheduleStatus === SCHEDULE_PENDING) {
    scheduleStatus = SCHEDULE_NOT_PENDING;
    scheduler.schedule();
  }
}

/**
 * Pops one entry off the cleanup queue and disposes its node if it is still
 * unneeded and not `Uncomputed`. A disposal pushes each derived dependency it
 * leaves unneeded onto the queue. Returns whether entries remain.
 *
 * Contract: called outside computations, batches, and `onChange` callbacks.
 */
export function disposeNext(): boolean {
  const node = cleanupQueue.pop();
  if (
    node !== undefined &&
    node.firstRetain === undefined &&
    node.firstDependent === undefined &&
    node.status !== UNCOMPUTED
  ) {
    disposeNode(node);
  }
  return cleanupQueue.length > 0;
}

/**
 * Disposes every unneeded node on the cleanup queue, and each one that a
 * disposal leaves unneeded.
 *
 * Contract: as `disposeNext`.
 */
export function disposeAll(): void {
  while (true) {
    const node = cleanupQueue.pop();
    if (node === undefined) {
      return;
    }
    if (
      node.firstRetain === undefined &&
      node.firstDependent === undefined &&
      node.status !== UNCOMPUTED
    ) {
      disposeNode(node);
    }
  }
}

/**
 * Unlinks every edge of `node`, pushes each derived dependency it leaves with
 * an empty retain list and no dependent onto the cleanup queue, and makes
 * `node` `Uncomputed`. Sources are never disposed.
 */
function disposeNode(node: MutableDerivedNode<unknown>): void {
  unlinkAll(node.firstSourceDependency);
  let edge = node.firstDerivedDependency;
  while (edge !== undefined) {
    unlinkEdge(edge);
    const dependency = edge.dependency as MutableDerivedNode<unknown>;
    if (
      dependency.firstDependent === undefined &&
      dependency.firstRetain === undefined
    ) {
      addToCollector(dependency);
    }
    edge = edge.nextDependency;
  }
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = UNCOMPUTED;
  node.value = undefined;
}

/**
 * Disposes each node on `markedUnneeded` that is still unneeded and not
 * `Clean`. A dependency that a disposal leaves unneeded is disposed too if it
 * is not `Clean`, because this write reached it, and is pushed onto the
 * cleanup queue with its value otherwise. Called at the end of the outermost
 * write or batch, after the flush's refreshes and before its `onChange` calls.
 */
export function disposeMarked(): void {
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  for (let index = 0; index < markedUnneeded.length; index += 1) {
    let node: MutableDerivedNode<unknown> | undefined = markedUnneeded[index]!;
    while (node !== undefined) {
      if (
        node.firstRetain === undefined &&
        node.firstDependent === undefined &&
        node.status !== UNCOMPUTED &&
        node.status !== CLEAN
      ) {
        unlinkAll(node.firstSourceDependency);
        let edge = node.firstDerivedDependency;
        while (edge !== undefined) {
          unlinkEdge(edge);
          const dependency = edge.dependency as MutableDerivedNode<unknown>;
          if (
            dependency.firstDependent === undefined &&
            dependency.firstRetain === undefined
          ) {
            if (dependency.status === CLEAN) {
              addToCollector(dependency);
            } else {
              (stack ??= []).push(dependency);
            }
          }
          edge = edge.nextDependency;
        }
        node.firstSourceDependency = undefined;
        node.lastSourceDependency = undefined;
        node.firstDerivedDependency = undefined;
        node.lastDerivedDependency = undefined;
        node.status = UNCOMPUTED;
        node.value = undefined;
      }
      node = stack?.pop();
    }
  }
  markedUnneeded.length = 0;
}

/**
 * Computes `node`, adds a holder to the head of its retain list, and returns
 * it. A retained node keeps its value and its dependencies until its last
 * release. The first retain of a node created eager, and a retain with an
 * `onChange`, make the node refresh in the flush.
 *
 * Contract: called outside computations and `onChange` callbacks.
 */
export function retainDerivedNode<TValue>(
  node: DerivedNode<TValue>,
  onChange?: () => void,
): Retain<TValue> {
  const mutableNode = node as MutableDerivedNode<TValue>;
  // Retaining computes the node, so it is Clean when it starts to refresh in
  // the flush.
  pull(mutableNode);
  const head = mutableNode.firstRetain as MutableRetain<TValue> | undefined;
  const retain: MutableRetain<TValue> = {
    node,
    onChange,
    previous: undefined,
    next: head,
  };
  if (head !== undefined) {
    head.previous = retain;
  }
  mutableNode.firstRetain = retain;
  if (head === undefined || onChange !== undefined) {
    updateEvaluation(mutableNode as MutableDerivedNode<unknown>);
  }
  scheduleIfOutermost();
  return retain;
}

/**
 * Drops `retain`. A release that leaves the node computed with an empty
 * retain list and no dependent pushes it onto the cleanup queue, and the
 * node keeps its value and its edges until a disposal or a write's marking
 * reaches it.
 *
 * Contract: called once per retain, outside computations and `onChange`
 * callbacks.
 */
export function releaseRetain<TValue>(retain: Retain<TValue>): void {
  const mutableNode = retain.node as MutableDerivedNode<TValue>;
  const previous = retain.previous as MutableRetain<TValue> | undefined;
  const next = retain.next as MutableRetain<TValue> | undefined;
  if (previous === undefined) {
    mutableNode.firstRetain = next;
  } else {
    previous.next = next;
  }
  if (next !== undefined) {
    next.previous = previous;
  }
  if (mutableNode.firstRetain === undefined || retain.onChange !== undefined) {
    updateEvaluation(mutableNode as MutableDerivedNode<unknown>);
  }
  if (
    mutableNode.firstRetain === undefined &&
    mutableNode.firstDependent === undefined &&
    mutableNode.status !== UNCOMPUTED
  ) {
    addToCollector(mutableNode as MutableDerivedNode<unknown>);
  }
  scheduleIfOutermost();
}

/**
 * Brings the retained `node` up to date and returns its value.
 *
 * Contract: `node` is retained. Called outside computations.
 */
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue {
  const value = pull(node);
  scheduleIfOutermost();
  return value;
}

/**
 * Brings `node` up to date as a top-level read and returns its value, without
 * a retain. Its edges link as in any read. A node left with no retain and no
 * dependent goes onto the cleanup queue, so it and its dependencies are
 * disposable at once, and the scheduler disposes them.
 *
 * Contract: called outside computations and `onChange` callbacks.
 */
export function readUnretained<TValue>(node: DerivedNode<TValue>): TValue {
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  if (
    mutableNode.firstRetain === undefined &&
    mutableNode.firstDependent === undefined &&
    mutableNode.status !== UNCOMPUTED
  ) {
    addToCollector(mutableNode as MutableDerivedNode<unknown>);
  }
  scheduleIfOutermost();
  return value;
}

/**
 * The evaluation state of `node`: a node created eager refreshes in the flush
 * while it is retained, and a node created lazy while a retain has an
 * `onChange`.
 */
function evaluationState(node: MutableDerivedNode<unknown>): EvaluationState {
  if ((node.evaluation & EAGER) === EAGER) {
    return node.firstRetain !== undefined ? EAGER_ACTIVE : EAGER_INACTIVE;
  }
  return hasObserver(node) ? LAZY_OBSERVED : LAZY;
}

/**
 * Updates the evaluation state after a first retain, a last release, or a
 * retain or release with an `onChange`. A node that stops refreshing in the
 * flush while `Dirty` or `Check` marks its `Clean` dependents `Check`, to
 * restore the internal invariant that a node below `LAZY_OBSERVED` out of
 * `Clean` has no `Clean` dependents. A node that starts was pulled first, so
 * it is `Clean`.
 */
function updateEvaluation(node: MutableDerivedNode<unknown>): void {
  const before = node.evaluation;
  const after = evaluationState(node);
  if (after === before) {
    return;
  }
  node.evaluation = after;
  if (
    before >= LAZY_OBSERVED &&
    after < LAZY_OBSERVED &&
    (node.status === DIRTY || node.status === CHECK)
  ) {
    flushOrQueue(markCheckQueue(currentBatchQueue(), node, undefined));
  }
}

/** Whether a retain in the node's list has an `onChange`. */
function hasObserver(node: MutableDerivedNode<unknown>): boolean {
  let retain = node.firstRetain;
  while (retain !== undefined) {
    if (retain.onChange !== undefined) {
      return true;
    }
    retain = retain.next;
  }
  return false;
}
```

## `src/batch.ts`

```ts
// from libs/pico-js/src/batch.ts
import { markDirty, markDirtyQueue, markedUnneeded, storeValue } from './marking';
import {
  CHECK,
  CLEAN,
  DIRTY,
  LAZY_OBSERVED,
  ON_CHANGE_NOT_QUEUED,
  ON_CHANGE_QUEUED,
  UNCOMPUTED,
  type MutableDerivedNode,
  type MutableSourceNode,
  type Queue,
  type SourceNode,
} from './nodes';
import { rerun, verify } from './pull';
import { disposeMarked, scheduleIfPending } from './retain';

/** The number of `startBatch` calls not yet matched by an `endBatch`. */
let batchDepth = 0;

/**
 * The eager nodes the open batch queued, created on its first push, so a batch
 * that queues nothing allocates nothing.
 */
let batchQueue: Queue | undefined;

/** No flush is running. */
const FLUSH_IDLE = 0;
/** A flush is running, its `onChange` calls included. */
const FLUSH_RUNNING = 1;

type FlushState = typeof FLUSH_IDLE | typeof FLUSH_RUNNING;

let flushState: FlushState = FLUSH_IDLE;

/**
 * Opens a batch. Writes until the matching `endBatch` mark at once and defer
 * eager refreshes to the flush of the outermost batch. A nested batch joins
 * the open one.
 *
 * Contract: each `startBatch` is matched by one `endBatch`, called outside
 * computations and `onChange` callbacks. A read inside the batch returns the
 * previous value of a queued node and of the nodes that read it.
 */
export function startBatch(): void {
  batchDepth += 1;
}

/** Closes a batch. The outermost `endBatch` flushes the batch's eager nodes. */
export function endBatch(): void {
  batchDepth -= 1;
  if (batchDepth === 0) {
    const queue = batchQueue;
    batchQueue = undefined;
    flushOrDisposeMarked(queue);
  }
  scheduleIfOutermost();
}

/** Writes `source`, and outside a batch flushes as a batch of one write. */
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  const first = storeValue(source as MutableSourceNode<TValue>, value);
  if (first === undefined) {
    return;
  }
  flushOrQueue(markDirtyQueue(currentBatchQueue(), first));
  scheduleIfOutermost();
}

/** The open batch's queue, or `undefined` outside a batch. */
export function currentBatchQueue(): Queue | undefined {
  return batchDepth > 0 ? batchQueue : undefined;
}

/**
 * Stores `queue` as the open batch's queue, or outside a batch flushes it, or
 * disposes what marking reached when there is nothing to flush.
 */
export function flushOrQueue(queue: Queue | undefined): void {
  if (batchDepth > 0) {
    batchQueue = queue;
  } else {
    flushOrDisposeMarked(queue);
  }
}

function flushOrDisposeMarked(queue: Queue | undefined): void {
  if (queue !== undefined) {
    flush(queue);
  } else if (markedUnneeded.length > 0) {
    disposeMarked();
  }
}

/**
 * Called at the end of each public operation. The outermost one, which ends
 * with no batch open and no flush running, calls the scheduler if a push found
 * the cleanup queue empty, so a scheduler may dispose synchronously. A
 * top-level read never runs inside a computation (its contract), so a read
 * inside an `onChange` is the only nesting, and the flush covers it.
 */
export function scheduleIfOutermost(): void {
  if (batchDepth === 0 && flushState === FLUSH_IDLE) {
    scheduleIfPending();
  }
}

/**
 * Refreshes every queued eager node, then calls the `onChange` of each one
 * whose value a refresh changed, in the order of their first changed
 * recompute.
 *
 * Guarantee: every `onChange` runs while no eager node of the batch is queued.
 *
 * Internal invariant: marking queues a node only when it takes the node out
 * of `Clean`, so the list holds a node again only after a refresh made it
 * `Clean`, and the list needs no deduplication. An `onChange` writes nothing,
 * so it queues no eager node.
 */
function flush(eagerNodes: Queue): void {
  flushState = FLUSH_RUNNING;
  // Guarantee: a computation or an `onChange` that throws leaves the flush
  // state idle and the marked list empty, so later operations, including
  // another request's, start from clean module state. The nodes left on the
  // marked list are on the cleanup queue already.
  try {
    flushQueue(eagerNodes);
  } finally {
    flushState = FLUSH_IDLE;
    markedUnneeded.length = 0;
  }
}

function flushQueue(eagerNodes: Queue): void {
  // Created on the first changed node with an observer, so a flush that
  // calls nothing allocates nothing.
  let changed: Array<MutableDerivedNode<unknown>> | undefined;
  // An index walks the array, because a refresh appends to it during the
  // walk.
  let index = 0;
  while (true) {
    const node = eagerNodes[index];
    if (node === undefined) {
      break;
    }
    index += 1;
    const result = refreshEager(eagerNodes, node);
    if (
      result === CHANGED &&
      node.firstRetain !== undefined &&
      node.onChangeStatus === ON_CHANGE_NOT_QUEUED
    ) {
      node.onChangeStatus = ON_CHANGE_QUEUED;
      (changed ??= []).push(node);
    }
  }
  if (markedUnneeded.length > 0) {
    disposeMarked();
  }
  if (changed === undefined) {
    return;
  }
  for (index = 0; index < changed.length; index += 1) {
    const node = changed[index]!;
    node.onChangeStatus = ON_CHANGE_NOT_QUEUED;
    // Contract: no computation or `onChange` retains or releases, so the list
    // holds the retains it held when the refresh changed the value.
    let retain = node.firstRetain;
    while (retain !== undefined) {
      if (retain.onChange !== undefined) {
        retain.onChange();
      }
      retain = retain.next;
    }
  }
}

/** The refresh recomputed the node to a value not `===` its previous one. */
const CHANGED = 0;
/** The refresh left the node's value as it was, or did not refresh it. */
const UNCHANGED = 1;

type RefreshResult = typeof CHANGED | typeof UNCHANGED;

/**
 * A node whose evaluation dropped below `LAZY_OBSERVED` after it was queued is
 * skipped: it stays `Dirty` or `Check`, the drop marked its dependents, and a
 * read recomputes it as lazy.
 */
function refreshEager(
  eagerNodes: Queue,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  if (node.evaluation < LAZY_OBSERVED) {
    return UNCHANGED;
  }
  switch (node.status) {
    case DIRTY:
      return recomputeEager(eagerNodes, node);
    case CHECK:
      return verify(node) === DIRTY
        ? recomputeEager(eagerNodes, node)
        : UNCHANGED;
    // Internal invariant: a Clean node is in the list twice, or was pulled
    // before the flush reached it. An Uncomputed node was disposed after it
    // was queued, and computing it would relink its sources.
    case CLEAN:
    case UNCOMPUTED:
      return UNCHANGED;
  }
}

/**
 * Internal invariant: an eager node's dependents may be `Clean`, so a changed
 * value marks them through `markDirty`.
 */
function recomputeEager(
  eagerNodes: Queue,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  const previous = node.value;
  const value = rerun(node, previous);
  // On cutoff the dependents stay Clean.
  if (value === previous) {
    return UNCHANGED;
  }
  node.value = value;
  // Eager dependents that this marks join the list the flush is walking.
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(eagerNodes, first);
  }
  return CHANGED;
}
```

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export { endBatch, startBatch, writeSource } from './batch';
export {
  CHECK,
  CLEAN,
  createDerivedNode,
  createSourceNode,
  DIRTY,
  EAGER,
  EAGER_ACTIVE,
  EAGER_INACTIVE,
  LAZY,
  LAZY_OBSERVED,
  UNCOMPUTED,
} from './nodes';
export type {
  DependentsStatus,
  DerivedNode,
  DerivedNodeEvaluation,
  EvaluationState,
  Retain,
  SourceNode,
  Status,
} from './nodes';
export { readDerived, readSource } from './pull';
export {
  disposeAll,
  disposeNext,
  readDerivedNode,
  readUnretained,
  releaseRetain,
  retainDerivedNode,
  scheduler,
} from './retain';
```

## Implementation notes

- `Status` is a union of four numbers with a constant for each, so the hot path compares small integers and each value has a name in the code. `DependentsStatus`, `OnChangeStatus`, `ScheduleStatus`, `FlushState`, and `RefreshResult` follow the same pattern. `ScheduleStatus` and `FlushState` replace the prototype's `boolean` and counter (AGENTS.md, Booleans).
- `evaluation` holds four states. Bit 0 is the creation choice (`LAZY` 0, `EAGER` 1). The node refreshes in the flush at 2 or more: `LAZY_OBSERVED` 2 (created lazy, a retain has an `onChange`) and `EAGER_ACTIVE` 3 (created eager, retained). Marking, `pull`, `verify`, and `refreshEager` test `evaluation >= LAZY_OBSERVED`. `updateEvaluation` recomputes the state after the first retain, the last release, and any retain or release that carries an `onChange`, and `evaluationState` reads the creation choice from bit 0. A release of an `onChange` retain walks the retain list in `hasObserver`, because there is no count of `onChange` retains. `hasObserver` returns a `boolean`: it answers whether one exists, with no domain names for the two answers.
- Marking pushes a node onto its stack only when the node leaves `Clean` and its dependents list is not empty, so a write visits each node's dependents at most once. Marking holds the last node it must descend into in a local and allocates its stack on the first push, and it descends into a dependent that is the last entry of its list without pushing it, so a chain, or a fan-out whose readers have no readers, allocates no stack. The marking stack and the verification stack are arrays local to the call, so the depth of a marked or verified chain does not reach the call stack.
- Marking pushes a lazy node it takes out of `Clean` onto `markedUnneeded` when the node has no dependent and no retain. `disposeMarked` runs at the end of the outermost write or batch: after the flush's refreshes and before its `onChange` calls, or alone when the write queued nothing. It disposes each listed node that is still unneeded and not `Clean`, and walks its derived dependencies with a stack allocated on the first push: a dependency left unneeded that is not `Clean` is disposed too, because the write reached it, and a `Clean` one is pushed onto the cleanup queue with its value. A listed node that a read inside the batch recomputed is `Clean` and is kept, and it is on the cleanup queue already, because it became unneeded before the write.
- A node becomes unneeded in five places, and each pushes it onto the cleanup queue: `releaseRetain`, `readUnretained`, the repoint in `readDerived`, `unlinkDropped` (the drop in `trim`), and the cascade in `disposeNode` and `disposeMarked`. A push onto an empty queue sets `scheduleStatus` to `SCHEDULE_PENDING`. The queue holds one entry per push and no flag on the node, so a node can be on it more than once, and `disposeNext` and `disposeAll` re-check each entry: an entry whose node has a retain, a dependent, or is `Uncomputed` is dropped.
- Every public operation ends with `scheduleIfOutermost`. It calls `scheduleIfPending` only when `batchDepth === 0` and `flushState === FLUSH_IDLE`. A top-level read never runs inside a computation (its contract), so the only nesting is a read inside an `onChange`, which the flush state covers. A scheduler may therefore call `disposeAll` synchronously: no computation, batch, or flush is running when it is called. With that scheduler the engine disposes when ba3daedf7 disposed, at the end of each outermost operation.
- The default scheduler uses `requestIdleCallback` where it exists, and disposes while `deadline.timeRemaining() > 0`. Elsewhere, including Node, it uses `setTimeout(…, 0)` and disposes for `TIMEOUT_DISPOSAL_MS` (5 ms) per task. Both reschedule while entries remain. On a server, the queue holds the nodes of every request in the process, and the re-check in `disposeNext` keeps a node that another request retained or read again.
- `flush` reads the eager-node array by index, so the entries that a refresh appends are refreshed in the same walk. It creates its list of changed nodes on the first changed node that has a retain, and walks that list by index after the eager-node array is drained and the marked nodes are disposed, so every `onChange` runs while no node of the batch is queued. For each changed node it walks the retain list from the head and calls each `onChange`. No `onChange` writes, retains, or releases, so the list is the one it was when the refresh changed the value, and no flush runs inside another.
- `flush` restores `flushState` and empties `markedUnneeded` in a `finally`. The batch's queue is already detached: `endBatch` sets `batchQueue` to `undefined` before it calls `flush`, and a `writeSource` outside a batch passes a local queue. The cost of this `finally` was not measured at this site. In the ambient variant's microbenchmark (Measurements), removing its outermost `try`/`finally` moved an event from 62 to 64 ns to 61 to 62 ns.
- `refreshEager` skips a node whose evaluation fell below `LAZY_OBSERVED` after it was queued, because a release in the same batch removed its last retain or its last `onChange` retain. `updateEvaluation` marked that node's `Clean` dependents `Check` through `markCheckQueue` at the release, so the lazy invariant holds and a read recomputes the node. A `Clean` node in the list was refreshed or pulled before the flush reached the entry. An `Uncomputed` node in the list was disposed after it was queued, and computing it would relink its sources, so it is skipped too.
- `writeSource` inside a batch marks at once into the batch's queue through `markDirtyQueue`, which creates the queue on the first push through `queueEagerNode`. `markDirtyQueue` and `markCheckQueue` are copies of `markDirty` and `markCheck` that create the queue on the first push, because one function with an optional queue slowed `startBatch` in PR #1027's review (a batch of one write 24.9 ns against 22.8 ns on Node 24.12). `markDirty` and `markCheck` take the array of the flush that is running, because `recomputeEager` marks into it.
- A node becomes `Clean` only after its computation returns or after `verify` has pulled every derived dependency it is linked to. A lazy recompute therefore marks only `Check` and `Dirty` nodes and takes no node out of `Clean`, so it marks through `markCheckDependentsDirty`, which walks only the node's dependents list and needs no stack. A node at `LAZY_OBSERVED` or above that is `Dirty` or `Check` may have `Clean` dependents, and only until the flush that refreshes it: `recomputeEager` marks a changed value through `markDirty`.
- `verify` walks the derived dependency lists in read order with an explicit stack of edges. It descends into a lazy `Check` dependency before its reader continues and pulls any other dependency, so a `Check` dependency at `LAZY_OBSERVED` or above returns its previous value. After a pull that leaves the current node `Dirty`, it recomputes that node and returns to the node's reader. It does not recompute `root`, and returns `DIRTY` or `CLEAN` for its caller, because TypeScript narrows `node.status` in the caller's `switch` and does not see `verify` change it. It pushes an edge only when the dependency it descends into has more than one dependent, and allocates the stack on the first push.
- `trim` calls `unlinkDropped` only when the node has a derived edge to drop. A call with an empty list, on every recompute of a node that reads only sources, slowed the write-then-read loop of `bench/single`'s read case from 4.45 ns to 6.39 ns, and the guard brought it to 4.0 ns.
- `status === UNCOMPUTED` alone records that a node has no value. An `Uncomputed` node's `value` is `undefined`, cast to `TValue` once in `createDerivedNode` and assigned in `disposeNode` and `disposeMarked`, where `value` is `unknown`. In the other three statuses, `value` holds a value the computation returned, which can be `undefined`.
- The exported `SourceNode`, `DerivedNode`, and `Retain` mark every field `readonly`, so a caller's write to any field is a type error. `computation` is a method, which a type cannot mark `readonly`, and stays a method so that `DerivedNode<TValue>` is assignable to `DerivedNode<unknown>`. The engine writes through `MutableSourceNode`, `MutableDerivedNode`, and `MutableRetain`, and casts once per exported function. `Edge` holds the public node types, so an edge reached through `node.firstDependent` does not let a caller write a node's fields. `onChangeStatus` is not a field of the exported `DerivedNode`: `MutableDerivedNode` adds it, and `index.ts` does not export `OnChangeStatus`.
- A comment in the source that states a caller's obligation starts with `Contract:`, and one that states what the engine ensures starts with `Guarantee:` or `Internal invariant:`. The `DerivedNode` doc comment lists the internal invariants, which the README does not list.

### Checks for a future debug build

A `__DEV__` build can restore these checks. Each throws `new Error(message)`.

- `markDirty`, `markCheck`, and `markCheckDependentsDirty`: a dependent's `status === UNCOMPUTED`; "An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js."
- `markCheckDependentsDirty`: a dependent's `status === CLEAN`; "A Clean node depends on a Dirty node. This is indicative of a bug in pico-js."
- `verify`: `node.firstDependent === undefined` for a node it descended into; "A node that verify descended into has no dependents. This is indicative of a bug in pico-js."
- `verify`: `verifyStack?.pop() === undefined` for a node it descended into with more than one dependent; "A node that verify descended into has more than one dependent and no edge on the verification stack. This is indicative of a bug in pico-js."
- `releaseRetain`: the retain is not in its node's list (`retain.previous === undefined ? retain.node.firstRetain !== retain : retain.previous.next !== retain`); "A retain was released twice."
- `readDerivedNode`: `node.firstRetain === undefined`; "A node was read without being retained."
- `readSource`, `readDerived`: the calling node's computation is not running; "readSource or readDerived was called outside the computation of its calling node." This needs a module-level running node in the debug build only.
- `retainDerivedNode`, `releaseRetain`, `readUnretained`, `startBatch`, `endBatch`, `writeSource`, `disposeNext`, `disposeAll`: called while a computation runs; "<name> was called inside a computation." The same running node serves.
- `writeSource`, `startBatch`, `endBatch`, `retainDerivedNode`, `releaseRetain`, `readUnretained`, `disposeNext`, `disposeAll`: called while `flushState === FLUSH_RUNNING`; "<name> was called inside an onChange."
- `startBatch`: when `batchDepth` goes from 0 to 1, `queueMicrotask` a check that `batchDepth === 0` when control returns to the event loop; "A batch was open when control returned to the event loop. startBatch and endBatch must not span an await."
- `endBatch`: `batchDepth === 0` before the decrement; "endBatch was called without a matching startBatch."
- `pull`, `recompute`, and `recomputeEager`: a node whose `status !== UNCOMPUTED` holds no computed value, which needs a private `NOT_SET` sentinel in `value` while `Uncomputed`; "A node that is not Uncomputed has no value. This is indicative of a bug in pico-js."

## Tests

`6f431ec0e` has four runtime test files, two type test files, three shims, and the random test. They are the prototype's, and they reach the engine through the shims:

```text
test/api.ts             the earlier roots API emulated on retains; graph.test.ts and graph.test-d.ts import it
test/api-retain.ts      ba3daedf7's API (retain count, onChange field, startBatch(body)) on retains, with scheduler.schedule = disposeAll; retain.test.ts, retain.test-d.ts, and random-retain.ts import it
test/api-collector.ts   the variant's API with an explicit collect(budget); collector.test.ts imports it
test/graph.test.ts      75 tests, the roots-era tests
test/retain.test.ts     105 tests, ba3daedf7's tests
test/collector.test.ts  13 tests, the cleanup queue, observers, readUnretained, marked disposal
test/scheduler.test.ts  2 tests, the default scheduler
```

The shims are folded into the tests, so every test calls the engine's exported functions and no file under `test/` re-exports or emulates an API:

- `graph.test.ts` and `retain.test.ts` merge into `test/graph.test.ts`. A test that both files hold in two versions is kept once, in the version that calls the engine API. The `describe` blocks keep the names of `retain.test.ts` (ba3daedf7's), and the roots-era blocks `roots and the collector` and `setDerivedNodeEvaluation with the evaluation the node has` move into `test/collector.test.ts` or are removed, by the rules below.
- `graph.test-d.ts` and `retain.test-d.ts` merge into `test/graph.test-d.ts`, which checks that every field of `SourceNode`, `DerivedNode`, and `Retain` is `readonly`, that `onChangeStatus` is not a field of `DerivedNode`, and that `createDerivedNode`'s second parameter accepts `LAZY` and `EAGER` and rejects `LAZY_OBSERVED`.
- The shim's `startBatch(body)` becomes `startBatch()`, the body's statements, and `endBatch()`. `mutators.writeSource` becomes `writeSource`. `createEagerDerivedNode(c)` becomes `createDerivedNode(c, EAGER)`. `x.onChange = cb` and the shim's `setOnChange` become a `retainDerivedNode(x, cb)` that the test keeps and releases. `releaseDerivedNode(node)` becomes `releaseRetain(retain)` on the retain the test kept. `readRetained(retain)` becomes `readDerivedNode(retain.node)`.
- The shim's `collect(budget)` becomes a test-local helper in `collector.test.ts` that calls `disposeNext` up to `budget` times. A test that read `collectorContents()` asserts instead what the queue does: the statuses after `disposeNext` and `disposeAll`, and `disposeNext`'s return value.
- A file sets `scheduler.schedule` in `beforeEach`: `disposeAll` in the tests ported from `retain.test.ts`, so they keep ba3daedf7's immediate cleanup, and a function that records its calls in the collector and scheduler tests. `afterEach` calls `disposeAll`, so no queued node reaches the next test. vitest gives each file its own module instance.
- Tests of `setDerivedNodeEvaluation`, of untracked reads, of nested `startBatch(body)` semantics, and of an `onChange` on a lazy node without a retain are removed with the API they test. The prototype's `// Removed under this variant:` comments are removed too (AGENTS.md: no comment that describes what was not done).
- A test of a node that refreshes in the flush creates it `EAGER` and retains it, or retains a `LAZY` node with an `onChange`.
- `collector.test.ts` gains tests for the server guarantees: a computation that throws inside the flush of `endBatch` leaves a later `startBatch`, `writeSource`, `endBatch` of other nodes flushing and calling their `onChange`; an `onChange` that throws leaves the same; after a throwing flush, `scheduler.schedule` is called at the end of the next outermost operation.
- The `README example` test runs the README's example and asserts the values and run counts its comments state.

Reads outside computations go through a test-local `read(node)`, which retains the node on its first call and never releases it, so a test that does not test lifetime keeps every node it reads.

### Random differential test

`test-random/random-retain.ts` is renamed `test-random/random.ts`, and its import of `../test/api-retain` becomes the engine's own exports. `pnpm test:random` reads the seed count from `PICO_JS_RANDOM_SEEDS` (default 2,000) and the first seed from `PICO_JS_RANDOM_BASE_SEED` (default 1).

A case is a graph and a list of operations, generated from the seed by mulberry32. The graph has 2 to 5 sources with values 0 to 3 and 3 to 14 derived nodes, 30% of them created `EAGER`. Each derived node reads a selector, then one of three branches of up to three reads that the selector's value picks, so its reads change from run to run. A derived node reads only sources and derived nodes of a lower index, so the graph is acyclic. A node returns the sum of its reads modulo 2, 3, or 1000, as a number, as an object it reuses from `previous` when the number is unchanged, or as a fresh object, so that cutoff applies to the first two and never to the third. An operation is a write, a retain with or without an `onChange`, a release of an outstanding retain, a read of a retained node, a `readUnretained` of any node, a collect step (`disposeAll`), or a batch of these. An `onChange` reads up to two retained nodes. After the operations, the case releases every outstanding retain and calls `disposeAll`.

Deltas from `f0b001a9d`'s generator: no evaluation-change operation, no untracked read inside a computation, and the evaluation is drawn once per node at creation.

Each case runs under two scheduler settings, `sync` (`scheduler.schedule = disposeAll`) and `deferred` (a scheduler that does nothing, so only collect steps and a write's marked disposal dispose), and in three batching modes:

- `single`: every write is a top-level `writeSource`.
- `one-op-batch`: every write is a `startBatch`, one `writeSource`, and an `endBatch`.
- `grouped`: each batch is one `startBatch` and `endBatch`, and its nested batches join it.

The `single` and `one-op-batch` runs log every computation run, `onChange` call, read value, and, after each operation, every node's status and value. The two logs must be equal entry for entry, which compares the outside-a-batch path of `writeSource` with the inside-a-batch path. Each run also checks itself against `Model`, which keeps statuses, values, retain lists, and derived dependency lists in arrays and follows the Model section and Guarantees step by step, including the cleanup queue, its re-check on disposal, and marked disposal at the end of the outermost write or batch.

The prototype cleared module state between cases with `resetForTests`. The random test instead calls `vi.resetModules()` and imports `../src/index` again for each case and each shrink attempt, so a case that failed inside a computation or an `onChange` leaves no state for the next one.

A failing case shrinks by removing runs of operations (halves, then quarters, down to single operations), then single batch items and callback reads, while any check still fails. The thrown error names the seed, the first failed check, the shrunk case's failed check, and the shrunk case as JSON.

## Measurements

The figures come from the prototype chain on the detached worktree `../i2-worktrees/pico-js-globals` (`ba3daedf7` … `f0b001a9d`), the roots chain on `../i2-worktrees/pico-js-roots`, the experiments chain on `../i2-worktrees/pico-js-experiments`, and the trace replay on `../i2-worktrees/pico-js-traces`. App scenarios are `bench/app`: each ratio is p50 (or p95 where stated) against `ba3daedf7` within a block, the median over 5 blocks, with a bootstrap 95% interval; a figure counts only when its interval excludes 0. Issue cases are `bench/cases.ts`, in ms per test, median over 5 processes with the range over processes. Trace replays are computation counts over the four recorded pet-demo sessions, browse / mutate / paginate / refetch. Upstream suite figures explain causes only.

Scenario names: initial-load, navigate, page-append, page-prepend, edit-row, edit-hot-record, optimistic-rollback, optimistic-confirm, live-update, live-update-frame, refetch-unchanged, cutoff-write, typing-no-graph, search-typing, gc-sweep, deep-first-pull, missing-data-arrives, move-between-groups, route-switch-shared, route-switch-shared-no-cleanup.

Variants named below:

```text
ba3daedf7   the engine before this work: retain counts, onChange field, startBatch(body), dispose at the outermost return
e5e462613   retain objects with a count and an observer list, module-level state, end-of-operation scheduling
1cf22dd6a   retain list with no count, module-level state (the base of the chosen design)
58f634f2a   retain count that allocates nothing, separate observer objects, module-level state
370894bbd   1cf22dd6a with a separate chosen-evaluation field (fix 1)
c0fda7a73   1cf22dd6a without the per-computation counter (fix 2)
d56dc5ede   1cf22dd6a with readUnretained, link and queue
6c963e2ae   d56dc5ede with readUnretained, read and dispose
8b9a2537b   cf59009dc (fixes 1 and 2) with disposal at the last release
f0b001a9d   c0fda7a73 with readUnretained (link and queue) and marked disposal (the chosen design)
```

### Decisions

- Explicit calling node over a module-level current node (be9840e60, 7fee0aff6). A write followed by a top-level read of a retained three-level chain took 53 to 54 ns on e5e462613 against 62 to 64 ns ambient (+15% to +17%), from a load and a check on every read and a save and restore on every recompute. Ambient was +4% to +5% on deep-first-pull at depth 50, and its app engine's computation closure cost about 97 bytes per pulled node. Ambient was faster on the upstream propagation tests (-11% to -24%), part of which is the upstream adapter's own tracking wrapper.
- Module-level state (one batch, one cleanup queue) over per-node or per-batch objects. On e5e462613, 1cf22dd6a, and 58f634f2a: missing-data-arrives -12.8% to -13.2%; route-switch-shared -20% to -30%; a derived node 124.4 to 132.5 bytes against 139.7 for ba3daedf7. Costs outside noise in that run: cutoff-write +3.0% (e5e462613) and +4.2% (58f634f2a), which the later 1cf22dd6a run put within noise (+1.2% [-2.2, +5.2]).
- Retain list with no count (1cf22dd6a) over a count with an observer list (e5e462613) or observer objects (58f634f2a). A derived node is 124.4 bytes against 132.5 and 132.2. A retain object is 56.6 bytes; an app subscription is 306 bytes against 306 and 356 (755 for ba3daedf7). edit-row was +13.1% [+6.5, +51.3] in session 2; the later run with the same build put it at +3.5% [-3.0, +12.9].
- Rejected: roots as derived nodes (1156aa177 … 5d2439a18, c1e497e0e). The upstream suite total was +8.3% to +12.6%; createComputations +215% to +257% (+100% for V5, which dropped the root closure); fanOutRepeat +51% to +144%, dynamicSwitch +47% to +56%, writeThenDispose +55% to +81%; optimistic-rollback +30.3% [+10.5, +44.6] (FN). A root is 192 to 305 bytes against a 56.6-byte retain, and the heap after gc-sweep was 231.5 to 234.5 MB against 213.1 MB.
- `startBatch()` and `endBatch()` over `batch(fn)`. With 64 call sites of 1 to 3 writes each, a batch took 35.4 ns against 44.9 ns (paired median +22%, range +15.7% to +39.7%). The global batch alone on ba3daedf7 (isobatch) was within noise on edit-row, cutoff-write, and deep-first-pull.
- Rejected: marking at the outermost `endBatch` (e1, 6263c959f; e1s, 0b02da84a). In the interim suite, noObservers +139%, fanOutRepeat +17% to +50%, coalescedWrites +21% and +25%, sameRecordWrites +29%, isograph read10% +11% to +35%; it won revertInBatch (-44%, -99%) and writeThenDispose (-22%). The per-batch bookkeeping of the written sources costs more than the immediate mark that it defers. Its app runs are among the paused experiments.
- Evaluation encoded in the `evaluation` field, over a separate chosen-evaluation field (370894bbd). On the full variant, the separate field changed nothing outside noise: edit-row +4.3% [-2.8, +17.9] against +3.5% for 1cf22dd6a, cutoff-write -0.1% against +1.2%, deep-first-pull at depth 50 +6.7% against +6.9%. The encoding alone on ba3daedf7 (isoenc) measured edit-row +7.6% [+1.1, +17.3], which the full variant does not reproduce.
- No per-computation counter (c0fda7a73). A top-level read never runs inside a computation (contract), so `batchDepth` and the flush state decide the outermost operation. Against 1cf22dd6a in one run: edit-row +3.1% against +3.5% and cutoff-write -0.2% against +1.2%, both within noise; deep-first-pull at depth 50 +9.4% [+3.8, +13.4] against +6.9% [+5.3, +9.9], with overlapping intervals.
- End-of-operation scheduling, which lets a synchronous scheduler reproduce immediate cleanup. With `schedule = disposeAll`, route-switch-shared computed 2,243 per event, as ba3daedf7 does, and the trace replays computed 392 / 285 / 669 / 116, as ba3daedf7 does.
- Deferred cleanup through the queue and the idle scheduler. route-switch-shared computes 786 per event against 2,243, and runs -21.8% [-24.2, -14.9] on f0b001a9d; route-switch-shared-no-cleanup -86% to -88%. The trace replays compute 104 / 120 / 534 / 55, which matches Preact (104 / 120 / 534 / 55), against 392 / 285 / 669 / 116 for ba3daedf7 (alien: 392 / 287 / 669 / 116). Issue cases, f0b001a9d idle against ba3daedf7 held: hideAndShow 1.34 against 1.89; replaceEffect dispose first 0.82 against 1.45; moveBetweenEffects 1.05 against 1.17; unobservedComputed 100 0.54 against 1.49, and 10000 0.01 against 1.59.
- Rejected: disposal at the last release (8b9a2537b). With render holds and the idle scheduler, the trace replays compute 334 / 247 / 669 / 106 against 104 / 120 / 534 / 55, because a node released and read again later is recomputed.
- Marked disposal (f0b001a9d over d56dc5ede). effectChurn 100 with the idle scheduler is 1.19 [1.1, 1.3] against 22.89 [22.8, 24.0] without it and 1.16 for ba3daedf7. Without marked disposal, a node that the case leaves unneeded stays linked until the idle scheduler runs, and every write in the timed loop marks it again. With `disposeAll` before the loop: 1.83 against 32.58. effectChurn 10000 is 1.65 against 1.61. In the app, navigate (-1.0%), gc-sweep (-2.2%), and the trace replay counts are unchanged, and route-switch-shared is -21.8%.
- `readUnretained` links and queues (d56dc5ede) over read-and-dispose (6c963e2ae). repeatedUnretainedReads diamond R10 is 3.26 against 23.30 (explicit) and 25.10 (idle); flat R10 1.48 against 5.00. The trace replays with the idle scheduler compute 104 / 120 / 534 / 55 against 334 / 258 / 954 / 148.
- Render reads retain until commit, and `readUnretained` is not for them. Without render holds, the synchronous scheduler gives 574 / 372 / 1085 / 235 against 392 / 285 / 669 / 116 with them, because each render read is disposed before the subscription retains the node. With the idle scheduler both give 104 / 120 / 534 / 55, which depends on the scheduler not running before commit.

### Remaining known costs

- repeatedUnretainedReads R1 (one read per unretained node) is about +25%: flat 5.25 against 4.23 (d56dc5ede) and 4.14 (ba3daedf7); diamond 27.81 against 22.34 and 22.77. The marked-list pushes and the disposal at the end of each write cost this. At R10 and R100 f0b001a9d is 1.59 and 1.11 (flat), against 1.01 and 0.63 for ba3daedf7 held.
- deep-first-pull at depth 50 is +4.7% [+0.5, +7.1] on f0b001a9d. With a warm-up pass it is within noise (+3.9% [-3.4, +5.8]; 1cf22dd6a +0.6%), and without one +6.7%, so the cost is a one-time compile of the first-retain path.
- The heap after the gc-sweep sweep is 226.3 to 226.7 MB against 213.1 MB for ba3daedf7, because unneeded nodes stay computed until the scheduler disposes them.
- `retainDerivedNode` allocates a 56.6-byte `Retain`, where ba3daedf7's retain allocated nothing.

### Open items

- The paused experiments (`../i2-worktrees/pico-js-experiments`): the app benchmark, `bench/single`, and the fanout microbenchmark for e1 to e7 have not run, and `resume.sh` resumes them. e2 (read deduplication by run id), e4 (inline first edge), and e5 (dependents as arrays) won targeted suite shapes and are undecided.
- The ambient variants and the reference builds (aa780509b, 703fb1f14, the `batch(fn)` and synchronous-scheduler builds) did not complete two blocks of app scenarios.
- The single-change isolations on ba3daedf7 (`iso/`) ran only on edit-row, cutoff-write, and deep-first-pull.
- 8b9a2537b ran only the trace replays, not the app scenarios or the issue cases.
- The library swaps (alien, Preact, and signal-polyfill on pico-js) must be redone on this API (`future-improvements.md`).
- The trace replays cannot support conclusions that depend on scale: their graphs hold 43 to 294 derived nodes, and their time differences between engines are within timer overhead.
