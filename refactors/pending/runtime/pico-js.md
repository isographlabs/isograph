# pico-js: implementation

This doc is the complete source and package of `@isograph/pico-js` in `libs/pico-js`. PR #1027 contains it as commit `225f893e5`: the engine of the prototype commit `f0b001a9d` with the API deltas listed under "Deltas from the prototype", split into seven source files. Every source block below is `git show 225f893e5:libs/pico-js/src/<file>` byte for byte.

## Terms

- A source node stores a value that the caller writes. A derived node stores a computation and the value that computation last returned.
- An edge connects a dependent (a derived node) to a dependency (a source node or a derived node that the dependent's computation read).
- A tracked read is a call to `readSource` or `readDerived` inside a computation. A top-level read is a call to `readDerivedNode` or `readUnretained`.
- A retain is the `Retain` object that `retainDerivedNode` returns. A node is retained while it has at least one retain. `releaseRetain` releases a retain.
- A node is needed while it has a retain or a dependent, and unneeded otherwise.
- To mark is to set a node `Dirty` or `MaybeDirty` in a write. To recompute is to run a node's computation, including the first run of an `Uncomputed` node. To verify is to walk a `MaybeDirty` node's derived dependencies to find whether one changed. To refresh is for the flush to recompute or verify a node on the flush queue.
- A node behaves eagerly while it is retained and was created `EAGER`, or while it has a retain with an `onChange`. Every other node is lazy.
- The flush queue contains the nodes that behave eagerly and that marking put there. The cleanup queue contains the nodes that became unneeded and await disposal.
- To dispose a node is to unlink every edge to its dependencies, drop its value, and set it `Uncomputed`. To unlink an edge is to remove it from both of its lists.
- An outermost operation is a call to `writeSource`, `endWriteBatch`, `retainDerivedNode`, `releaseRetain`, `readDerivedNode`, or `readUnretained` made with no batch open and no flush running, or the `endWriteBatch` that closes the outermost batch.
- The flush is the refresh of every node on the flush queue followed by the `onChange` calls. A `writeSource` outside a batch runs the flush, and so does the outermost `endWriteBatch`.
- A node is maybe-dirty (status `MaybeDirty`) when a dependency may have changed, and whether the node's value changed is not yet known.
- Direction is always named by dependency and dependent: marking moves from a source toward its dependents, and verification moves from a node toward its dependencies.

## Model

- A computation receives `(callingNode, previous)` and reads other nodes through `readSource(callingNode, source)` and `readDerived(callingNode, node)`. Each tracked read records an edge from the calling node, the dependent, to the node it read, the dependency. A node has edges to exactly the nodes that its latest computation read with a tracked read. A computation makes no other read of a node.
- `createDerivedNode(computation, evaluation)` fixes the node's evaluation, `LAZY` (the default) or `EAGER`, for its lifetime.
- Marking stops at a node that behaves eagerly and puts it on the flush queue, and the flush refreshes it. A lazy node recomputes when a tracked read or a top-level read reaches it.
- A write runs no computation. It marks the dependents of the source `Dirty`, and the nodes that depend on those through lazy nodes `MaybeDirty`. Marking continues through a lazy node and stops at a node that behaves eagerly, which it puts on the flush queue.
- A tracked read or a top-level read of a `Dirty` lazy node recomputes it. The same read of a `MaybeDirty` lazy node verifies it: it brings each derived dependency up to date in the order in which the node's latest recompute read the dependencies, and recomputes the node as soon as one dependency changed, without visiting the remaining dependencies. If no dependency changed, the node becomes `Clean` and does not recompute.
- A recompute receives `previous`, the node's last value. A recompute whose value is `===` to `previous` keeps `previous` and marks nothing. A computation whose result is structurally equal to `previous` returns `previous`.
- `startWriteBatch()` and `endWriteBatch()` open and close a batch. Batches nest, and a nested batch joins the open one. A write inside a batch marks at once, and the outermost `endWriteBatch` flushes. A `writeSource` outside a batch flushes before it returns. The flush refreshes every node on the flush queue, then calls the `onChange` of each retain of each node whose value the flush changed.
- `retainDerivedNode(node, onChange?)` brings the node up to date and returns a `Retain`, which joins the node's retain list. `releaseRetain(retain)` removes that retain from the node's retain list. `onChange` exists only on a retain.
- `readDerivedNode(node)` is the top-level read of a retained node. `readUnretained(node)` is a top-level read of any node, and `readUnretained` adds no retain. `readUnretained` links edges as a tracked read does, and puts the node on the cleanup queue if the node is unneeded.
- A derived node that becomes unneeded keeps its value and its edges and joins the cleanup queue. `disposeNext()` and `disposeAll()` dispose the nodes on the cleanup queue that are still unneeded. A disposed node becomes `Uncomputed`, drops its value, and has none of its edges to dependencies. Each derived dependency that the disposal leaves unneeded joins the cleanup queue. A source node is never disposed.
- At the end of the outermost operation that put a node on an empty cleanup queue, the engine calls `scheduler.schedule()`. The default disposes during idle time. A scheduler that calls `disposeAll` disposes when the outermost operation ends.
- When marking reaches an unneeded node, the engine disposes the node when the write, or the outermost batch, ends. A write therefore never leaves an unneeded node `Dirty` or `MaybeDirty` with its edges linked. A `Clean` dependency that this disposal leaves unneeded joins the cleanup queue with its value.
- The engine keeps one graph per process in module state: the batch depth, the flush queue of the open batch, the flush state, the cleanup queue, and the scheduler. The calling node is a parameter of each tracked read.

### Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior.

- The caller writes a source only when its value changed.
- A scheduler does not throw. A computation or an `onChange` may throw. After a throw, the statuses and values of the nodes that the throwing operation was updating are unspecified. The engine's module state is restored (Guarantees).
- A computation is synchronous. It calls `readSource` and `readDerived` only with the `callingNode` it received, and makes no other read of a node: no `readDerivedNode`, no `readUnretained`, and no `source.value`.
- A computation does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`.
- An `onChange` does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`. It may call `readDerivedNode` and read `source.value`.
- A computation's result depends only on `previous` and on the values that its tracked reads return.
- The derived nodes form a directed acyclic graph, counting the edges that a node keeps from its previous recompute.
- `readDerivedNode` is called only on a retained node, and only outside computations.
- `readUnretained` is a one-off top-level read. A caller that needs the node's value to stay current until a later point, such as a render until its commit, retains the node until then, because a scheduler may dispose an unretained node as soon as `readUnretained` returns.
- Each `retainDerivedNode` is matched by exactly one `releaseRetain` of the retain it returned. A second release of the same retain unlinks that retain again and corrupts the node's retain list.
- Each `startWriteBatch` is matched by exactly one `endWriteBatch`. Both are called outside computations and `onChange` callbacks. A caller whose code between them can throw calls `endWriteBatch` in a `finally`.
- Between `startWriteBatch` and the matching `endWriteBatch`, the caller makes no top-level read of a derived node: no `readDerivedNode` and no `readUnretained`. Inside a batch, a node whose dependency is on the flush queue can still hold a value computed from that dependency's old value, and the engine keeps no copy of the values before the batch. Reading a source's value inside a batch is allowed and returns the last value written.
- `startWriteBatch` and its `endWriteBatch` run in one synchronous stretch of code and never span an `await`. On a server that serves several requests in one process, the batch is module state, so another request's writes made during the `await` would join the open batch.
- `disposeNext` and `disposeAll` are called outside computations, batches, and `onChange` callbacks. After a call to `scheduler.schedule()`, the scheduler calls `disposeNext` until it returns `false`, or calls `disposeAll`.
- A chain of derived nodes that the first recompute of `Uncomputed` nodes, or the recompute of `Dirty` nodes, goes through stays within the call stack limit, because each node adds frames. In Node 24.12 with its default stack and one-line computations, the first recompute of a chain overflows at about 2,000 derived nodes, and the recompute of a chain of `Dirty` nodes at about 2,200.
- A caller does not mutate a value that a tracked read or a top-level read returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode` and `createDerivedNode`, and writes no field of a node or a retain, except through `scheduler.schedule`. The types make a write to a field a type error.
- A caller that needs current values makes its top-level reads after the batch flushes. Until the flush, a top-level read of a node on the flush queue, or of a node that depends on it, returns the previous value.

### Guarantees

pico-js upholds these while the caller meets every contract.

- A top-level read made while no batch is open returns the value that the node's latest recompute returned, or `previous` when that recompute returned a value `===` to `previous`. Each tracked read in that recompute returned the current value of its dependency.
- A node recomputes only when it is `Uncomputed`, or when a source dependency was written, or when a derived dependency recomputed to a value not `===` to its previous value.
- A tracked read or a top-level read of a `MaybeDirty` lazy node verifies it: it brings each derived dependency up to date in the order that the latest recompute read them, and recomputes the node as soon as one dependency changed, without visiting the remaining dependencies. The same read of a `Dirty` lazy node recomputes it without first visiting its dependencies.
- A recompute whose value is `===` to its previous value keeps the previous value and marks nothing. `NaN` counts as changed, and `-0` over a stored `0` keeps `0`.
- `retainDerivedNode` brings the node up to date before it returns. A node that starts to behave eagerly is `Clean` at that moment.
- For each node that the flush recomputed to a value not `===` to the previous value, the flush calls the `onChange` of each of the node's retains once, most recent retain first, even when the node recomputed more than once. The flush orders the nodes by each node's first recompute that changed its value. It makes these calls after it has refreshed every node on the flush queue and after it has disposed the unneeded nodes that marking reached. The first recompute of a node, a recompute that returns a value `===` to `previous`, and a lazy node cause no `onChange` call.
- The flush skips a node on the flush queue that a release in the same batch made lazy. The node stays `Dirty` or `MaybeDirty`, its dependents are `MaybeDirty` or `Dirty`, and a tracked read or a top-level read recomputes it as a lazy node.
- When a flush returns, every node that the flush refreshed is `Clean`. Inside a batch, a top-level read of a `Dirty` or `MaybeDirty` node that behaves eagerly returns its previous value and recomputes nothing.
- A `writeSource` inside a batch runs no computation and calls no `onChange`. A write walks the list of dependents of the source, and of each lazy node it takes out of `Clean`, at most once each. A write returns without visiting a derived node when no `readSource` of the source has run since the source was created or last written.
- After a recompute, the node is in the list of dependents of each dependency that the recompute read, and of no other node, until the node's next recompute or its disposal. Marking unlinks no edge.
- When an outermost operation returns, every derived node that is unneeded and not `Uncomputed` is on the cleanup queue. After `disposeAll` returns, every unneeded derived node is `Uncomputed`.
- When a write outside a batch, or the outermost `endWriteBatch`, returns, no node that its marking took out of `Clean` is both unneeded and `Dirty` or `MaybeDirty`. Each such node is `Uncomputed`, or `Clean` because a tracked read or a top-level read inside the batch recomputed it.
- A disposed node is `Uncomputed`, has the value `undefined`, and has no edges. Disposal marks nothing and calls nothing. `disposeNext` and `disposeAll` skip an entry of the cleanup queue whose node is needed again or already `Uncomputed`.
- A derived node that becomes unneeded and then receives a tracked read or a top-level read before a disposal reaches it keeps its value and does not recompute for that read. This applies whether the read comes from a later recompute in the same operation or from a later operation.
- The engine calls `scheduler.schedule()` only when an outermost operation returns, and only once for each time the cleanup queue went from empty to non-empty since the previous call. A scheduler that calls `disposeAll` therefore disposes every unneeded node when each outermost operation returns.
- A recompute whose tracked reads match, in order, the edges of the node's previous recompute allocates no edge and links or unlinks none.
- Marking, verification of `MaybeDirty` nodes, and disposal keep their stacks in arrays. In Node 24.12, the verification of a chain of 200,000 `MaybeDirty` nodes, and a release followed by `disposeAll` of a chain of 100,000 nodes, do not overflow.
- If a computation or an `onChange` throws, the engine restores its module state before the exception leaves the outermost entry point: the flush state is idle, the marked-unneeded list is empty, and the `endWriteBatch` or `writeSource` that started the flush has cleared the flush queue of the batch. Later operations, including those of other requests on a server, are unaffected. The graph that was being updated may be partially updated. A scheduler call that the throwing operation would have made happens at the end of the next outermost operation instead.
- No module-level array keeps a node reachable after the operation that used it returns, except the cleanup queue, which keeps each node on it until a disposal pops it. The marked-unneeded list is empty when each outermost operation returns, and the flush queue of the batch is dropped when its flush starts.
- `retainDerivedNode` allocates one `Retain`. `releaseRetain` and a `readDerivedNode` of a `Clean` node allocate nothing beyond growth of the cleanup queue's array. A write outside a batch that puts no node on the flush queue allocates no flush queue, and a flush that changes no node that has a retain allocates no list.

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
  src/edges.ts              linkEdge, unlinkEdge, unlinkAll
  src/marking.ts            storeValue, marking, the marked-unneeded list
  src/cleanup.ts            the cleanup queue, the scheduler, disposeNext, disposeAll, disposeMarked
  src/pull.ts               readSource, readDerived, pull, verify, recompute, trim
  src/batch.ts              startWriteBatch, endWriteBatch, writeSource, the flush, the end-of-operation check
  src/retain.ts             retainDerivedNode, releaseRetain, readDerivedNode, readUnretained, evaluation state
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

This is `225f893e5`'s file. Deltas from `libs/isograph-runtime/package.json`: `name`, `description`, and `repository.directory`; `dependencies` is empty; `devDependencies` drops `@babel/core` and `@isograph/babel-plugin`; `scripts` adds `test:random`. `private: true` stays, as the runtime's and `@isograph/disposable-types`'s do: publishing to npm is distribution, which `IN_PROGRESS.md` lists as not planned. The one export is `.`, built from `src/index.ts` by the root `tsdown.config.ts` (`entry: ['src/index.ts']`, `format: ['commonjs', 'esm']`).

`"sideEffects": false` holds although the engine has module state: evaluating the modules allocates two empty arrays and assigns constants, and runs no code with an effect outside the package.

### TypeScript, tsdown, vitest

`tsconfig.json` and `tsconfig.package.json` are the runtime's, byte for byte (`libs/isograph-runtime/tsconfig.json`, `tsconfig.package.json`). `tsdown.config.ts` is the runtime's plus `unbundle: false`, which overrides the root config's `unbundle: true` so that `dist/` holds one bundled file per format (Source layout). The engine compiles under the root `tsconfig.build.json`: `target: ES2022`, `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `noUnusedLocals`, `noUnusedParameters`.

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
  unbundle: false,
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
  test: {
    typecheck: { enabled: true, tsconfig: './tsconfig.test.json' },
    pool: 'forks',
    poolOptions: { forks: { execArgv: ['--expose-gc'] } },
  },
});
```

Delta from the runtime's `vitest.config.ts`: no Babel plugin, and the tests run in forked processes with `--expose-gc`, so the test that the engine keeps no node of a finished graph reachable can call `gc()` before it reads its `WeakRef`s.

```ts
// from libs/pico-js/vitest.random.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: { include: ['test-random/random.ts'], testTimeout: 3_600_000 },
});
```

Delta of `vitest.random.config.ts` from the prototype's: `include` names `test-random/random.ts` instead of `test-random/*.ts`, because the prototype's `random-retain.ts` is renamed back to `random.ts` (Tests, Random differential test).

`vitest.config.ts` keeps vitest's default `include`, which matches `*.test.ts` and `*.spec.ts`, so `pnpm test` and the `pico-js-test` hook do not run the random test. `tsconfig.test.json` includes `test-random/**/*.ts`, so `tsc -b` and the `pico-js-typecheck` hook type-check it without running it. vitest runs each test file in its own module instance, so the module state of one file does not reach another.

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`. The engine has no class and no private field. oxlint's `strict-boolean-expressions` (`.oxlintrc.json`) applies to every condition. oxlint's `switch-exhaustiveness-check` applies to every `switch`, and no `switch` has a `default` arm.

### Pre-commit hooks

`.pre-commit-config.yaml` gains two hooks, after `disposable-types-typecheck`. `225f893e5` has them.

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

The engine's tests run in their own workflow, `.github/workflows/pico-js.yml`, which `ci.yml` calls as the job `pico-js` and lists among the jobs its final status job needs:

```yaml
# from .github/workflows/pico-js.yml
name: pico-js

on:
  workflow_call:

jobs:
  pico-js:
    name: pico-js
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: ./.github/actions/setup-js
      - name: Test
        run: pnpm --filter @isograph/pico-js test
      - name: Random differential test
        run: pnpm --filter @isograph/pico-js test:random
        env:
          PICO_JS_RANDOM_SEEDS: 2000
```

```yaml
# from .github/workflows/ci.yml
  pico-js:
    name: pico-js
    uses: ./.github/workflows/pico-js.yml
```

`js.yml` runs no pico-js step. Its `babel-plugin` job runs `pnpm compile-libs`, which turbo runs for every workspace package with a `compile-libs` script; turbo's `compile-libs` task depends on `tsc` (`turbo.json`), so the engine is type-checked and built there with no change. The random differential test runs seeds 1 to 2,000.

### `README.md`

````markdown
<!-- from libs/pico-js/README.md -->
# pico-js

pico-js is a push-pull memoization engine. A write to a source node eagerly marks the derived nodes that depend on it, and a derived node lazily recomputes when a tracked read or a top-level read reaches it.

## Terms

- A source node stores a value that the caller writes. A derived node stores a computation and the value that computation last returned.
- An edge connects a dependent (a derived node) to a dependency (a source node or a derived node that the dependent's computation read).
- A tracked read is a call to `readSource` or `readDerived` inside a computation. A top-level read is a call to `readDerivedNode` or `readUnretained`.
- A retain is the `Retain` object that `retainDerivedNode` returns. A node is retained while it has at least one retain. `releaseRetain` releases a retain.
- A node is needed while it has a retain or a dependent, and unneeded otherwise.
- To mark is to set a node `Dirty` or `MaybeDirty` in a write. To recompute is to run a node's computation, including the first run of an `Uncomputed` node. To verify is to walk a `MaybeDirty` node's derived dependencies to find whether one changed. To refresh is for the flush to recompute or verify a node on the flush queue.
- A node behaves eagerly while it is retained and was created `EAGER`, or while it has a retain with an `onChange`. Every other node is lazy.
- The flush queue contains the nodes that behave eagerly and that marking put there. The cleanup queue contains the nodes that became unneeded and await disposal.
- To dispose a node is to unlink every edge to its dependencies, drop its value, and set it `Uncomputed`. To unlink an edge is to remove it from both of its lists.
- An outermost operation is a call to `writeSource`, `endWriteBatch`, `retainDerivedNode`, `releaseRetain`, `readDerivedNode`, or `readUnretained` made with no batch open and no flush running, or the `endWriteBatch` that closes the outermost batch.
- The flush is the refresh of every node on the flush queue followed by the `onChange` calls. A `writeSource` outside a batch runs the flush, and so does the outermost `endWriteBatch`.
- A node is maybe-dirty (status `MaybeDirty`) when a dependency may have changed, and whether the node's value changed is not yet known.
- Direction is always named by dependency and dependent: marking moves from a source toward its dependents, and verification moves from a node toward its dependencies.

## Goals

pico-js aims for correctness first and performance second. Where a rule can be the caller's obligation instead of a check on the hot path, pico-js makes the rule a contract and checks nothing at run time.

## Model

- A computation receives `(callingNode, previous)` and reads other nodes through `readSource(callingNode, source)` and `readDerived(callingNode, node)`. Each tracked read records an edge from the calling node, the dependent, to the node it read, the dependency. A node has edges to exactly the nodes that its latest computation read with a tracked read. A computation makes no other read of a node.
- `createDerivedNode(computation, evaluation)` fixes the node's evaluation, `LAZY` (the default) or `EAGER`, for its lifetime.
- Marking stops at a node that behaves eagerly and puts it on the flush queue, and the flush refreshes it. A lazy node recomputes when a tracked read or a top-level read reaches it.
- A write runs no computation. It marks the dependents of the source `Dirty`, and the nodes that depend on those through lazy nodes `MaybeDirty`. Marking continues through a lazy node and stops at a node that behaves eagerly, which it puts on the flush queue.
- A tracked read or a top-level read of a `Dirty` lazy node recomputes it. The same read of a `MaybeDirty` lazy node verifies it: it brings each derived dependency up to date in the order in which the node's latest recompute read the dependencies, and recomputes the node as soon as one dependency changed, without visiting the remaining dependencies. If no dependency changed, the node becomes `Clean` and does not recompute.
- A recompute receives `previous`, the node's last value. A recompute whose value is `===` to `previous` keeps `previous` and marks nothing. A computation whose result is structurally equal to `previous` returns `previous`.
- `startWriteBatch()` and `endWriteBatch()` open and close a batch. Batches nest, and a nested batch joins the open one. A write inside a batch marks at once, and the outermost `endWriteBatch` flushes. A `writeSource` outside a batch flushes before it returns. The flush refreshes every node on the flush queue, then calls the `onChange` of each retain of each node whose value the flush changed.
- `retainDerivedNode(node, onChange?)` brings the node up to date and returns a `Retain`, which joins the node's retain list. `releaseRetain(retain)` removes that retain from the node's retain list. `onChange` exists only on a retain.
- `readDerivedNode(node)` is the top-level read of a retained node. `readUnretained(node)` is a top-level read of any node, and `readUnretained` adds no retain. `readUnretained` links edges as a tracked read does, and puts the node on the cleanup queue if the node is unneeded.
- A derived node that becomes unneeded keeps its value and its edges and joins the cleanup queue. `disposeNext()` and `disposeAll()` dispose the nodes on the cleanup queue that are still unneeded. A disposed node becomes `Uncomputed`, drops its value, and has none of its edges to dependencies. Each derived dependency that the disposal leaves unneeded joins the cleanup queue. A source node is never disposed.
- At the end of the outermost operation that put a node on an empty cleanup queue, the engine calls `scheduler.schedule()`. The default disposes during idle time. A scheduler that calls `disposeAll` disposes when the outermost operation ends.
- When marking reaches an unneeded node, the engine disposes the node when the write, or the outermost batch, ends. A write therefore never leaves an unneeded node `Dirty` or `MaybeDirty` with its edges linked. A `Clean` dependency that this disposal leaves unneeded joins the cleanup queue with its value.
- The engine keeps one graph per process in module state: the batch depth, the flush queue of the open batch, the flush state, the cleanup queue, and the scheduler. The calling node is a parameter of each tracked read.

## API

- `createSourceNode(value): SourceNode<TValue>` returns a source node that stores `value`.
- `createDerivedNode(computation, evaluation = LAZY): DerivedNode<TValue>` returns an `Uncomputed` derived node. `computation(callingNode, previous)` returns the node's value, and `previous` is `undefined` on the first recompute. `evaluation` is `LAZY` or `EAGER`, fixed for the node's lifetime.
- `readSource(callingNode, source): TValue` is a tracked read. It returns the value of `source` and records `source` as a dependency of `callingNode`.
- `readDerived(callingNode, node): TValue` is a tracked read. It brings `node` up to date, returns its value, and records `node` as a dependency of `callingNode`.
- `retainDerivedNode(node, onChange?): Retain<TValue>` brings `node` up to date and returns a new retain of it. With `onChange`, the flush calls `onChange` after the flush changes the node's value.
- `releaseRetain(retain): void` releases the retain. A node left unneeded joins the cleanup queue.
- `readDerivedNode(node): TValue` is a top-level read. It brings a retained `node` up to date and returns its value.
- `readUnretained(node): TValue` is a top-level read. It brings `node` up to date, returns its value without adding a retain, and puts `node` on the cleanup queue if `node` is unneeded.
- `startWriteBatch(): void` opens a batch, and `endWriteBatch(): void` closes it. The outermost `endWriteBatch` flushes.
- `writeSource(source, value): void` stores `value` and marks the dependents of `source`. Outside a batch it flushes before it returns.
- `disposeNext(): boolean` pops the next entry off the cleanup queue and disposes its node if the node is still unneeded, and returns whether the cleanup queue is non-empty. `disposeAll(): void` empties the cleanup queue.
- `scheduler.schedule: () => void` is called with no arguments at the end of the outermost operation that put a node on an empty cleanup queue. The default disposes during `requestIdleCallback` time, or for 5 ms per `setTimeout` task where `requestIdleCallback` does not exist. Assign `disposeAll` to dispose when each operation ends.
- `LAZY` and `EAGER`, of type `DerivedNodeEvaluation`, are the creation choices.
- `SourceNode<TValue>`, `DerivedNode<TValue>`, and `Retain<TValue>` are the node and retain types. Every field is `readonly`. A node's status and computation are internal to the engine and are not fields of `DerivedNode`.

```ts
import {
  createDerivedNode,
  createSourceNode,
  disposeAll,
  EAGER,
  endWriteBatch,
  readDerived,
  readDerivedNode,
  readSource,
  readUnretained,
  releaseRetain,
  retainDerivedNode,
  scheduler,
  startWriteBatch,
  writeSource,
} from '@isograph/pico-js';

// Dispose at the end of each operation, so the statuses in the comments are true at once.
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

startWriteBatch();
writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
writeSource(person, { first: 'Ada', last: 'King', age: 38 });
endWriteBatch();
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
- A computation is synchronous. It calls `readSource` and `readDerived` only with the `callingNode` it received, and makes no other read of a node: no `readDerivedNode`, no `readUnretained`, and no `source.value`.
- A computation does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`.
- An `onChange` does not write, start or end a batch, retain, release, call `readUnretained`, `disposeNext`, or `disposeAll`. It may call `readDerivedNode` and read `source.value`.
- A computation's result depends only on `previous` and on the values that its tracked reads return.
- The derived nodes form a directed acyclic graph, counting the edges that a node keeps from its previous recompute.
- `readDerivedNode` is called only on a retained node, and only outside computations.
- `readUnretained` is a one-off top-level read. A caller that needs the node's value to stay current until a later point, such as a render until its commit, retains the node until then, because a scheduler may dispose an unretained node as soon as `readUnretained` returns.
- Each `retainDerivedNode` is matched by exactly one `releaseRetain` of the retain it returned. A second release of the same retain unlinks that retain again and corrupts the node's retain list.
- Each `startWriteBatch` is matched by exactly one `endWriteBatch`. Both are called outside computations and `onChange` callbacks. A caller whose code between them can throw calls `endWriteBatch` in a `finally`.
- Between `startWriteBatch` and the matching `endWriteBatch`, the caller makes no top-level read of a derived node: no `readDerivedNode` and no `readUnretained`. Inside a batch, a node whose dependency is on the flush queue can still hold a value computed from that dependency's old value, and the engine keeps no copy of the values before the batch. Reading a source's value inside a batch is allowed and returns the last value written.
- `startWriteBatch` and its `endWriteBatch` run in one synchronous stretch of code and never span an `await`. On a server that serves several requests in one process, the batch is module state, so another request's writes made during the `await` would join the open batch.
- `disposeNext` and `disposeAll` are called outside computations, batches, and `onChange` callbacks. After a call to `scheduler.schedule()`, the scheduler calls `disposeNext` until it returns `false`, or calls `disposeAll`.
- A chain of derived nodes that the first recompute of `Uncomputed` nodes, or the recompute of `Dirty` nodes, goes through stays within the call stack limit, because each node adds frames. In Node 24.12 with its default stack and one-line computations, the first recompute of a chain overflows at about 2,000 derived nodes, and the recompute of a chain of `Dirty` nodes at about 2,200.
- A caller does not mutate a value that a tracked read or a top-level read returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode` and `createDerivedNode`, and writes no field of a node or a retain, except through `scheduler.schedule`. The types make a write to a field a type error.
- A caller that needs current values makes its top-level reads after the batch flushes. Until the flush, a top-level read of a node on the flush queue, or of a node that depends on it, returns the previous value.

## Guarantees

pico-js upholds these while the caller meets every contract.

- A top-level read made while no batch is open returns the value that the node's latest recompute returned, or `previous` when that recompute returned a value `===` to `previous`. Each tracked read in that recompute returned the current value of its dependency.
- A node recomputes only when it is `Uncomputed`, or when a source dependency was written, or when a derived dependency recomputed to a value not `===` to its previous value.
- A tracked read or a top-level read of a `MaybeDirty` lazy node verifies it: it brings each derived dependency up to date in the order that the latest recompute read them, and recomputes the node as soon as one dependency changed, without visiting the remaining dependencies. The same read of a `Dirty` lazy node recomputes it without first visiting its dependencies.
- A recompute whose value is `===` to its previous value keeps the previous value and marks nothing. `NaN` counts as changed, and `-0` over a stored `0` keeps `0`.
- `retainDerivedNode` brings the node up to date before it returns. A node that starts to behave eagerly is `Clean` at that moment.
- For each node that the flush recomputed to a value not `===` to the previous value, the flush calls the `onChange` of each of the node's retains once, most recent retain first, even when the node recomputed more than once. The flush orders the nodes by each node's first recompute that changed its value. It makes these calls after it has refreshed every node on the flush queue and after it has disposed the unneeded nodes that marking reached. The first recompute of a node, a recompute that returns a value `===` to `previous`, and a lazy node cause no `onChange` call.
- The flush skips a node on the flush queue that a release in the same batch made lazy. The node stays `Dirty` or `MaybeDirty`, its dependents are `MaybeDirty` or `Dirty`, and a tracked read or a top-level read recomputes it as a lazy node.
- When a flush returns, every node that the flush refreshed is `Clean`. Inside a batch, a top-level read of a `Dirty` or `MaybeDirty` node that behaves eagerly returns its previous value and recomputes nothing.
- A `writeSource` inside a batch runs no computation and calls no `onChange`. A write walks the list of dependents of the source, and of each lazy node it takes out of `Clean`, at most once each. A write returns without visiting a derived node when no `readSource` of the source has run since the source was created or last written.
- After a recompute, the node is in the list of dependents of each dependency that the recompute read, and of no other node, until the node's next recompute or its disposal. Marking unlinks no edge.
- When an outermost operation returns, every derived node that is unneeded and not `Uncomputed` is on the cleanup queue. After `disposeAll` returns, every unneeded derived node is `Uncomputed`.
- When a write outside a batch, or the outermost `endWriteBatch`, returns, no node that its marking took out of `Clean` is both unneeded and `Dirty` or `MaybeDirty`. Each such node is `Uncomputed`, or `Clean` because a tracked read or a top-level read inside the batch recomputed it.
- A disposed node is `Uncomputed`, has the value `undefined`, and has no edges. Disposal marks nothing and calls nothing. `disposeNext` and `disposeAll` skip an entry of the cleanup queue whose node is needed again or already `Uncomputed`.
- A derived node that becomes unneeded and then receives a tracked read or a top-level read before a disposal reaches it keeps its value and does not recompute for that read. This applies whether the read comes from a later recompute in the same operation or from a later operation.
- The engine calls `scheduler.schedule()` only when an outermost operation returns, and only once for each time the cleanup queue went from empty to non-empty since the previous call. A scheduler that calls `disposeAll` therefore disposes every unneeded node when each outermost operation returns.
- A recompute whose tracked reads match, in order, the edges of the node's previous recompute allocates no edge and links or unlinks none.
- Marking, verification of `MaybeDirty` nodes, and disposal keep their stacks in arrays. In Node 24.12, the verification of a chain of 200,000 `MaybeDirty` nodes, and a release followed by `disposeAll` of a chain of 100,000 nodes, do not overflow.
- If a computation or an `onChange` throws, the engine restores its module state before the exception leaves the outermost entry point: the flush state is idle, the marked-unneeded list is empty, and the `endWriteBatch` or `writeSource` that started the flush has cleared the flush queue of the batch. Later operations, including those of other requests on a server, are unaffected. The graph that was being updated may be partially updated. A scheduler call that the throwing operation would have made happens at the end of the next outermost operation instead.
- No module-level array keeps a node reachable after the operation that used it returns, except the cleanup queue, which keeps each node on it until a disposal pops it. The marked-unneeded list is empty when each outermost operation returns, and the flush queue of the batch is dropped when its flush starts.
- `retainDerivedNode` allocates one `Retain`. `releaseRetain` and a `readDerivedNode` of a `Clean` node allocate nothing beyond growth of the cleanup queue's array. A write outside a batch that puts no node on the flush queue allocates no flush queue, and a flush that changes no node that has a retain allocates no list.

## Design choices

- Edges are objects in intrusive doubly linked lists, one in the dependent's list of dependencies and one in the dependency's list of dependents, instead of sets. Linking and unlinking are O(1) and allocate nothing.
- A recompute reuses the edge at each position when its tracked read at that position reads the same node as in the previous recompute, so a stable computation allocates and relinks nothing.
- A `Dirty` node keeps its edges linked, so a second write that reaches the node finds it `Dirty` and stops, and the node's recompute reuses the edges.
- A source's `dependentsStatus` records that a write left every dependent of the source `Dirty`, so the next write skips the list of dependents until a computation calls `readSource` on the source again.
- Marking, verification, and disposal walk explicit stacks, so the depth of the graph costs no call-stack frames there.
- A computation receives an explicit `callingNode` and passes `callingNode` to every tracked read, instead of the engine tracking the running computation in module state.
- The engine keeps one graph per process in module state: the batch, the flush state, and the cleanup queue. A batch allocates no object, and no node carries a pointer to its graph.
- A retain is an object in its node's retain list, and the list replaces a retain count. A node is retained while its retain list is not empty, and the flush finds the `onChange` callbacks by walking the list.
- Whether a node behaves eagerly is encoded in its `evaluation` field, so marking, tracked reads, top-level reads, and verification test one field.
- A node that becomes unneeded goes on the cleanup queue and is not disposed at once, so a node that receives a tracked read or a top-level read before the scheduler runs keeps its value. The end of a write disposes the unneeded nodes that its marking reached, so later writes do not walk them.
- Each kind of node is one object shape, so property access stays monomorphic.
- Public types are `readonly`, and the engine writes through an internal `Mutable` view, so the types enforce the field contract.
- pico-js checks no contract at run time. A future version may add the checks as a compile-time option.

### Module state

- The engine keeps its batch depth, its flush state, its cleanup queue, and its scheduler in module state that the whole process shares. There is one graph per process.
- On a server, every request shares that state. A batch therefore must not span an `await`, because another request's writes would join the batch.
- A computation or an `onChange` that throws leaves the graph that the operation was updating partially updated. The engine restores the shared state before the exception reaches the caller, so other requests are unaffected.
- On a server, `requestIdleCallback` does not exist, so the default scheduler disposes in `setTimeout` tasks. The cleanup queue can contain nodes of several requests at once, and each disposal checks again that its node is still unneeded before it disposes the node.
````

The Terms, Model, Contracts, and Guarantees sections at the top of this doc copy the README's sections word for word.

### Readers before the Isograph runtime depends on it

AGENTS.md says we do not have anything we do not use, and tests-only is not a reader. Until the Isograph runtime calls it, the package's reader is the user's stated goal that the engine is usable by projects outside Isograph (pico-runtime.md, Goal), and `README.md` documents the package for those projects. pico-runtime.md core change 4 ships the package before its first workspace caller on that ground; core change 5 is the first caller of the source functions and core change 8 of the derived-node functions. The package ships alone, as its own change.

## Deltas from the prototype

The engine below is `src/graph.ts` of `f0b001a9d` with these changes. Function bodies not named here are the prototype's, with the prototype's line comments shortened or removed.

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
// from libs/pico-js/src/nodes.ts, src/retain.ts, src/cleanup.ts, src/batch.ts
export function createDerivedNode<TValue>(computation: (callingNode: DerivedNode<unknown>, previous: TValue | undefined) => TValue, evaluation: DerivedNodeEvaluation = LAZY): DerivedNode<TValue>;
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue; // top-level only
let scheduleStatus: ScheduleStatus = SCHEDULE_NOT_PENDING;
let flushState: FlushState = FLUSH_IDLE;
const changedNodes: Array<MutableDerivedNode<unknown> | undefined> = [];
let changedCount = 0;
```

- `createDerivedNode` takes the evaluation as an optional second parameter, `LAZY` by default, and `createEagerDerivedNode` is removed. `newDerivedNode` folds into `createDerivedNode`.
- `setDerivedNodeEvaluation` is removed, so the creation choice is fixed. The low bit of `evaluation` still holds it, and `evaluationState(node)` reads it from there instead of taking a `choice` parameter. The path of `setDerivedNodeEvaluation` that put a `Dirty` or `MaybeDirty` node on the flush queue when the node became eager has no other caller and goes with it. A node starts to behave eagerly through a retain, which brings the node up to date first, so the node is `Clean`.
- `batch(body)` is removed. `startWriteBatch()` and `endWriteBatch()` remain.
- `readRetained(retain)` is removed. `readDerivedNode(node)` is the top-level read of a retained node, and a computation no longer calls it, so a computation makes no read outside `readSource` and `readDerived`. This choice is open: `readRetained(retain)` makes "the node is retained" a fact of the argument's type instead of a contract, at the cost that every caller of `readRetained` keeps the `Retain` that it passes. The engine code is the same either way (`pull(retain.node)` against `pull(node)`).
- `resetForTests` and `collectorContents` are removed, because only tests call them (AGENTS.md, Tests: an API only tests call should not exist). Tests observe the cleanup queue through `disposeNext`'s return value and node statuses, and the random test gets fresh module state per case (Tests).
- `schedulePending: boolean` becomes `scheduleStatus: ScheduleStatus`, and the counter `flushing` becomes `flushState: FlushState`, a two-value union. A flush cannot run inside another, because an `onChange` does not write or end a batch (contract), so the count never exceeds 1.
- The flush's list of changed nodes, which the prototype allocated per flush on the first changed node, is the module-level `changedNodes` with `changedCount`. Each entry is set to `undefined` before its node's `onChange` calls run, so the array keeps no node reachable after the flush.
- `flush` wraps `flushQueue` in `try`/`finally`, which sets `flushState` to `FLUSH_IDLE`, empties `markedUnneeded`, and clears the entries of `changedNodes` that a throw left, resetting their `onChangeStatus`. A computation or an `onChange` that throws therefore leaves the module state of an idle engine for the next operation. The prototype restored nothing and needed `resetForTests` after a throw.
- `collectorStack` is renamed `cleanupQueue`, and `addToCollector` is renamed `addToCleanupQueue`.
- The public API drops `Status` and the `status` field with `UNCOMPUTED`, `DIRTY`, `MAYBE_DIRTY`, and `CLEAN`, the encoded evaluation constants `EAGER_INACTIVE`, `LAZY_OBSERVED`, and `EAGER_ACTIVE`, and the types `EvaluationState` and `DependentsStatus`. `index.ts` exports `LAZY` and `EAGER` as the creation choices. `computation` is not a field of the public `DerivedNode`; it is a method of the internal `MutableDerivedNode`, which keeps `MutableDerivedNode<TValue>` assignable to `MutableDerivedNode<unknown>`. Tests read a node's status through the test helper `test/status.ts` (`statusOf(node)`), which casts to `MutableDerivedNode`.
- The public `SourceNode`, `DerivedNode`, `Retain`, and `Edge` are `readonly`. The engine works on separate internal types written out field by field: `MutableSourceNode`, `MutableDerivedNode`, `MutableRetain`, and `MutableEdge`, whose link fields (`dependency`, `nextDependency`, `previousDependent`, `nextDependent`) are mutable and whose `dependent` is `readonly`. Each exported function casts its public arguments to the internal type once, and the internal types reference each other, so edge walks need no casts. The generic `Mutable<T>` and the aliases `SourceEdge`, `DerivedEdge`, and `Queue` are removed.
- `disposeNext` returning `false` and `disposeAll` set `scheduleStatus` to `SCHEDULE_NOT_PENDING`, because the cleanup queue they leave is empty, so a later push onto the cleanup queue schedules again.
- `endWriteBatch`'s and `flushOrQueue`'s shared tail is the function `flushOrDisposeMarked`.
- `scheduleIfOutermost` tests the batch depth and the flush state in `batch.ts`, and calls `scheduleIfPending` in `cleanup.ts`, which owns `scheduleStatus`.
- Comments that named `setDerivedNodeEvaluation`, `readRetained`, or untracked reads are rewritten. A comment states a contract, a guarantee, or an internal invariant, and comments that restated the code are removed.

## Source layout

`f0b001a9d`'s `graph.ts` is 1,452 lines. The engine splits it by concern into seven files. Seven, not five, because oxlint's `import/no-cycle` (`.oxlintrc.json`, `error`) rejects a five-file split in which `batch.ts` and `retain.ts`, and `pull.ts` and `retain.ts`, import each other: `edges.ts` takes the edge functions out of `pull.ts`, and `cleanup.ts` takes the cleanup queue, the scheduler, and disposal out of `retain.ts`. The imports are acyclic in this order:

```text
nodes.ts    imports nothing
edges.ts    imports nodes.ts (types)
marking.ts  imports nodes.ts
cleanup.ts  imports edges.ts (unlinkAll, unlinkEdge), marking.ts (markedUnneeded), nodes.ts
pull.ts     imports marking.ts (markCheckDependentsDirty), nodes.ts, cleanup.ts (addToCleanupQueue), edges.ts (linkEdge, unlinkAll, unlinkEdge)
batch.ts    imports marking.ts (markDirty, markDirtyQueue, markedUnneeded, storeValue), nodes.ts, pull.ts (rerun, verify), cleanup.ts (disposeMarked, scheduleIfPending)
retain.ts   imports batch.ts (currentBatchQueue, flushOrQueue, scheduleIfOutermost), marking.ts (markCheckQueue), nodes.ts, cleanup.ts (addToCleanupQueue), pull.ts (pull)
index.ts    re-exports batch.ts, nodes.ts, pull.ts, cleanup.ts, retain.ts
```

The package's `tsdown.config.ts` sets `unbundle: false`, overriding the root config's `unbundle: true`, so tsdown bundles `src/index.ts` and everything it imports into one file per format. A call across source files is then a call within one module in `dist/`, and costs what it cost in one file. An exported function that `index.ts` does not re-export is internal to the package.

An ES module cannot reassign a `let` it imports, so each `let` lives in the file whose code writes it, and the other files reach it through functions. A `const` array is imported directly, because its importers mutate it and never reassign it.

```text
state                  owner          writers                                          files that read the state
batchDepth             batch.ts       startWriteBatch, endWriteBatch                             currentBatchQueue(), flushOrQueue() (retain.ts)
batchQueue             batch.ts       endWriteBatch, flushOrQueue                           currentBatchQueue() (retain.ts)
flushState             batch.ts       flush                                            none
changedNodes           batch.ts       flushQueue, clearChangedNodes                    none
changedCount           batch.ts       flushQueue, clearChangedNodes                    none
scheduleStatus         cleanup.ts     addToCleanupQueue, scheduleIfPending             scheduleIfPending() (batch.ts)
cleanupQueue           cleanup.ts     addToCleanupQueue, disposeNext, disposeAll       addToCleanupQueue() (pull.ts, retain.ts)
scheduler              cleanup.ts     the caller                                       index.ts re-export
markedUnneeded         marking.ts     markDirty, markCheck, their queue copies         disposeMarked (cleanup.ts), flush, flushQueue, flushOrDisposeMarked (batch.ts)
```

## `src/nodes.ts`

```ts
// from libs/pico-js/src/nodes.ts
/*
 * An edge connects a derived node, the dependent, to a node its computation
 * read, the dependency. Each edge is in two lists: the dependent's list of
 * dependencies, in the order its computation read them (a derived node keeps
 * one list of source dependencies and one of derived dependencies), and the
 * dependency's list of dependents.
 *
 * A recompute reuses edges by position: the n-th dependency the computation
 * reads takes the dependent's n-th edge. For example, d1's computation reads
 * s1 then s2, so d1's list of source dependencies is [e1, e2], where e1 points
 * at s1 and e2 at s2. If d1 recomputes and its computation reads:
 * - s1 then s2: e1 and e2 are kept, and nothing is allocated.
 * - s1 then s3: e2 is taken off s2's dependents and put on s3's.
 * - only s1: e2 is removed from d1's dependencies and from s2's dependents.
 *
 * A write to s2 walks s2's dependents to mark them. s2's list of dependents
 * contains one edge for every derived node whose computation read s2, so many
 * dependents share it, the list can be long, and e2 can be anywhere in it. The
 * list is doubly linked through the edges, so removing e2 takes two pointer
 * updates and no search through s2's other dependents. A list of dependencies
 * is singly linked, because a recompute only walks it from the start and a trim
 * cuts its end. The links live in the edges rather than in a Set or an array,
 * so recording a dependency allocates nothing beyond its edge. A node's retain
 * list is doubly linked the same way, through the `Retain` objects.
 *
 * Internal invariant: an edge is in exactly one list of dependencies and one
 * list of dependents. A node's lists of dependencies stay linked while it is
 * `Dirty`. A disposed node has no edges.
 */

export const UNCOMPUTED = 0;
export const DIRTY = 1;
export const MAYBE_DIRTY = 2;
export const CLEAN = 3;

export type Status =
  | typeof UNCOMPUTED
  | typeof DIRTY
  | typeof MAYBE_DIRTY
  | typeof CLEAN;

/** An edge as the edge-list fields of a node expose it to a caller. */
export type Edge<TDependency> = {
  readonly dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  readonly nextDependency: Edge<TDependency> | undefined;
  readonly previousDependent: Edge<TDependency> | undefined;
  readonly nextDependent: Edge<TDependency> | undefined;
};

export type MutableEdge<TDependency> = {
  dependency: TDependency;
  readonly dependent: MutableDerivedNode<unknown>;
  nextDependency: MutableEdge<TDependency> | undefined;
  previousDependent: MutableEdge<TDependency> | undefined;
  nextDependent: MutableEdge<TDependency> | undefined;
};

export type NodeWithDependents<TDependency> = {
  firstDependent: MutableEdge<TDependency> | undefined;
  lastDependent: MutableEdge<TDependency> | undefined;
};

export type MutableSourceNode<TValue> = {
  value: TValue;
  firstDependent: MutableEdge<MutableSourceNode<unknown>> | undefined;
  lastDependent: MutableEdge<MutableSourceNode<unknown>> | undefined;
  dependentsStatus: DependentsStatus;
};

/**
 * Internal invariant: `computation` is a method, which keeps
 * `MutableDerivedNode<TValue>` assignable to `MutableDerivedNode<unknown>`.
 */
export type MutableDerivedNode<TValue> = {
  value: TValue;
  firstRetain: MutableRetain<TValue> | undefined;
  status: Status;
  evaluation: EvaluationState;
  onChangeStatus: OnChangeStatus;
  firstSourceDependency: MutableEdge<MutableSourceNode<unknown>> | undefined;
  lastSourceDependency: MutableEdge<MutableSourceNode<unknown>> | undefined;
  firstDerivedDependency: MutableEdge<MutableDerivedNode<unknown>> | undefined;
  lastDerivedDependency: MutableEdge<MutableDerivedNode<unknown>> | undefined;
  firstDependent: MutableEdge<MutableDerivedNode<unknown>> | undefined;
  lastDependent: MutableEdge<MutableDerivedNode<unknown>> | undefined;
  computation(
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ): TValue;
};

export type MutableRetain<TValue> = {
  readonly node: MutableDerivedNode<TValue>;
  readonly onChange: (() => void) | undefined;
  previous: MutableRetain<TValue> | undefined;
  next: MutableRetain<TValue> | undefined;
};

export const DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY = 0;
export const DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY = 1;

export type DependentsStatus =
  | typeof DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY
  | typeof DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;

/**
 * Internal invariant: `dependentsStatus` is `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`
 * from the source's creation or from a write until the next `readSource`.
 * A node leaves `Dirty` only by a recompute, and a node joins the source's list
 * of dependents only through `readSource`.
 *
 * Contract: a caller writes no field. The type enforces the contract.
 */
export type SourceNode<TValue> = {
  readonly value: TValue;
  readonly firstDependent: Edge<SourceNode<unknown>> | undefined;
  readonly lastDependent: Edge<SourceNode<unknown>> | undefined;
  readonly dependentsStatus: DependentsStatus;
};

/** The creation choice of a derived node that is always lazy. */
export const LAZY = 0;
/**
 * The creation choice of a derived node that behaves eagerly while it is
 * retained: the flush recomputes the node after a write or a batch changed one
 * of its dependencies.
 */
export const EAGER = 1;
export const EAGER_INACTIVE = EAGER;
export const LAZY_OBSERVED = 2;
export const EAGER_ACTIVE = 3;

/** The evaluation a caller chooses at creation. */
export type DerivedNodeEvaluation = typeof LAZY | typeof EAGER;

/**
 * Internal invariant: the low bit is the creation choice. A node behaves
 * eagerly while it is retained and was created `EAGER`, or while it has a
 * retain with an `onChange`. Marking stops at a node that behaves eagerly and
 * puts it on the flush queue. Every other node is lazy. The value is at least
 * `LAZY_OBSERVED` exactly while the node behaves eagerly.
 */
export type EvaluationState =
  | typeof LAZY
  | typeof EAGER_INACTIVE
  | typeof LAZY_OBSERVED
  | typeof EAGER_ACTIVE;

export const ON_CHANGE_NOT_QUEUED = 0;
export const ON_CHANGE_QUEUED = 1;

type OnChangeStatus = typeof ON_CHANGE_NOT_QUEUED | typeof ON_CHANGE_QUEUED;

/**
 * Internal invariants of a derived node whose computation is not running:
 * - `Uncomputed`: `value` is `undefined`, both dependency lists are empty, and
 *   the node has no dependents.
 * - `Dirty`: `value` is the value of the last recompute. A lazy node has
 *   `MaybeDirty` or `Dirty` dependents. A node that behaves eagerly is on the flush
 *   queue of the open batch or of the running flush, and its dependents may be
 *   `Clean`.
 * - `MaybeDirty`: as `Dirty`.
 * - `Clean`: `value` is up to date, unless the node depends through `Clean`
 *   nodes on a `Dirty` or `MaybeDirty` node that behaves eagerly.
 *
 * In every status, each edge in the node's dependency lists is in its
 * dependency's list of dependents. A node that is not `Uncomputed` and is
 * unneeded is on the cleanup queue, or on the marked-unneeded list of the
 * running write, except a node that `readDerived` has brought up to date but
 * has not yet linked to the calling node by an edge.
 *
 * Contract: a caller writes no field. The type enforces the contract.
 */
export type DerivedNode<TValue> = {
  readonly value: TValue;
  /** The head of the node's retain list, most recent first. */
  readonly firstRetain: Retain<TValue> | undefined;
  readonly evaluation: EvaluationState;
  readonly firstSourceDependency: Edge<SourceNode<unknown>> | undefined;
  readonly lastSourceDependency: Edge<SourceNode<unknown>> | undefined;
  readonly firstDerivedDependency: Edge<DerivedNode<unknown>> | undefined;
  readonly lastDerivedDependency: Edge<DerivedNode<unknown>> | undefined;
  readonly firstDependent: Edge<DerivedNode<unknown>> | undefined;
  readonly lastDependent: Edge<DerivedNode<unknown>> | undefined;
};

/**
 * A retain of a node. `retainDerivedNode` returns a retain and
 * `releaseRetain` releases a retain. A retain is in the retain list of its
 * node.
 */
export type Retain<TValue> = {
  readonly node: DerivedNode<TValue>;
  /**
   * Guarantee: the flush calls `onChange` once for each change of the node's
   * value that the flush made, after the flush has refreshed every node on the
   * flush queue. The first recompute of a node does not call `onChange`.
   *
   * Contract: the callback does not write, start or end a batch, retain,
   * release, dispose, or call `readUnretained`. It may call `readDerivedNode`
   * and read `source.value`.
   */
  readonly onChange: (() => void) | undefined;
  readonly previous: Retain<TValue> | undefined;
  readonly next: Retain<TValue> | undefined;
};

/** Returns a source node that stores `value`. */
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
 * Contract: `computation` is synchronous. `computation` reads other nodes
 * only through `readSource` and `readDerived`, called with the `callingNode`
 * that `computation` receives. `computation` does not write, start or end a batch, retain, release, dispose,
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

## `src/edges.ts`

```ts
// from libs/pico-js/src/edges.ts
import type { MutableEdge, NodeWithDependents } from './nodes';

export function linkEdge<TDependency extends NodeWithDependents<TDependency>>(
  edge: MutableEdge<TDependency>,
): void {
  const dependency = edge.dependency;
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

/**
 * Internal invariant: the edge keeps its own links, so `unlinkAll` can follow
 * its `nextDependency` after the unlink, and `linkEdge` overwrites its
 * `previousDependent` and `nextDependent` when the edge is reused.
 */
export function unlinkEdge<TDependency extends NodeWithDependents<TDependency>>(
  edge: MutableEdge<TDependency>,
): void {
  const dependency = edge.dependency;
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

export function unlinkAll<TDependency extends NodeWithDependents<TDependency>>(
  first: MutableEdge<TDependency> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    edge = edge.nextDependency;
  }
}
```

## `src/marking.ts`

```ts
// from libs/pico-js/src/marking.ts
import {
  MAYBE_DIRTY,
  CLEAN,
  DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY,
  DIRTY,
  LAZY_OBSERVED,
  UNCOMPUTED,
  type MutableDerivedNode,
  type MutableEdge,
  type MutableSourceNode,
} from './nodes';

/**
 * Internal invariant: `markedUnneeded` contains the nodes that marking took out
 * of `Clean` while they were unneeded. `markedUnneeded` is empty when an
 * outermost operation returns.
 */
export const markedUnneeded: Array<MutableDerivedNode<unknown>> = [];

export function storeValue<TValue>(
  source: MutableSourceNode<TValue>,
  value: TValue,
): MutableEdge<MutableSourceNode<unknown>> | undefined {
  source.value = value;
  if (source.dependentsStatus === DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY) {
    return undefined;
  }
  source.dependentsStatus = DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY;
  return source.firstDependent;
}

export function markDirty<TDependency>(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  first: MutableEdge<TDependency>,
): void {
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  let nextToMark: MutableDerivedNode<unknown> | undefined;
  let edge: MutableEdge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case CLEAN:
        node.status = DIRTY;
        if (node.evaluation >= LAZY_OBSERVED) {
          eagerNodes.push(node);
          break;
        }
        if (node.firstDependent !== undefined) {
          if (nextToMark !== undefined) {
            (stack ??= []).push(nextToMark);
          }
          nextToMark = node;
        } else if (node.firstRetain === undefined) {
          markedUnneeded.push(node);
        }
        break;
      // Internal invariant: a MaybeDirty node either marked its dependents when
      // it left Clean, or behaves eagerly and went on the flush queue then.
      case MAYBE_DIRTY:
        node.status = DIRTY;
        break;
      // Internal invariant: a Dirty node was marked earlier. An Uncomputed
      // node has no edge to a dependency, so no list of dependents has it.
      case DIRTY:
      case UNCOMPUTED:
        break;
    }
  }
  if (nextToMark !== undefined) {
    markMaybeDirty(eagerNodes, nextToMark, stack);
  }
}

function markMaybeDirty(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  nextToMark: MutableDerivedNode<unknown>,
  initialStack: Array<MutableDerivedNode<unknown>> | undefined,
): void {
  let stack = initialStack;
  let marked: MutableDerivedNode<unknown> | undefined = nextToMark;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case CLEAN:
          dependent.status = MAYBE_DIRTY;
          if (dependent.evaluation >= LAZY_OBSERVED) {
            eagerNodes.push(dependent);
            break;
          }
          if (dependent.firstDependent !== undefined) {
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          } else if (dependent.firstRetain === undefined) {
            markedUnneeded.push(dependent);
          }
          break;
        // Internal invariant: a lazy node that is not Clean has no Clean
        // dependents, and marking stops at a node that behaves eagerly.
        case MAYBE_DIRTY:
        case DIRTY:
        case UNCOMPUTED:
          break;
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
}

export function markDirtyQueue<TDependency>(
  initialQueue: Array<MutableDerivedNode<unknown>> | undefined,
  first: MutableEdge<TDependency>,
): Array<MutableDerivedNode<unknown>> | undefined {
  let queue = initialQueue;
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  let nextToMark: MutableDerivedNode<unknown> | undefined;
  let edge: MutableEdge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case CLEAN:
        node.status = DIRTY;
        if (node.evaluation >= LAZY_OBSERVED) {
          queue = queueEagerNode(queue, node);
          break;
        }
        if (node.firstDependent !== undefined) {
          if (nextToMark !== undefined) {
            (stack ??= []).push(nextToMark);
          }
          nextToMark = node;
        } else if (node.firstRetain === undefined) {
          markedUnneeded.push(node);
        }
        break;
      // Internal invariant: a MaybeDirty node either marked its dependents when
      // it left Clean, or behaves eagerly and went on the flush queue then.
      case MAYBE_DIRTY:
        node.status = DIRTY;
        break;
      // Internal invariant: a Dirty node was marked earlier. An Uncomputed
      // node has no edge to a dependency, so no list of dependents has it.
      case DIRTY:
      case UNCOMPUTED:
        break;
    }
  }
  return nextToMark === undefined
    ? queue
    : markMaybeDirtyQueue(queue, nextToMark, stack);
}

export function markMaybeDirtyQueue(
  initialQueue: Array<MutableDerivedNode<unknown>> | undefined,
  nextToMark: MutableDerivedNode<unknown>,
  initialStack: Array<MutableDerivedNode<unknown>> | undefined,
): Array<MutableDerivedNode<unknown>> | undefined {
  let queue = initialQueue;
  let stack = initialStack;
  let marked: MutableDerivedNode<unknown> | undefined = nextToMark;
  while (marked !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case CLEAN:
          dependent.status = MAYBE_DIRTY;
          if (dependent.evaluation >= LAZY_OBSERVED) {
            queue = queueEagerNode(queue, dependent);
            break;
          }
          if (dependent.firstDependent !== undefined) {
            if (dependentEdge === undefined) {
              dependentEdge = dependent.firstDependent;
            } else {
              (stack ??= []).push(dependent);
            }
          } else if (dependent.firstRetain === undefined) {
            markedUnneeded.push(dependent);
          }
          break;
        // Internal invariant: a lazy node that is not Clean has no Clean
        // dependents, and marking stops at a node that behaves eagerly.
        case MAYBE_DIRTY:
        case DIRTY:
        case UNCOMPUTED:
          break;
      }
    }
    marked = stack === undefined ? undefined : stack.pop();
  }
  return queue;
}

function queueEagerNode(
  queue: Array<MutableDerivedNode<unknown>> | undefined,
  node: MutableDerivedNode<unknown>,
): Array<MutableDerivedNode<unknown>> {
  if (queue === undefined) {
    return [node];
  }
  queue.push(node);
  return queue;
}

export function markMaybeDirtyDependentsDirty(
  first: MutableEdge<MutableDerivedNode<unknown>> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case MAYBE_DIRTY:
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

## `src/cleanup.ts`

```ts
// from libs/pico-js/src/cleanup.ts
import { unlinkAll, unlinkEdge } from './edges';
import { markedUnneeded } from './marking';
import { CLEAN, UNCOMPUTED, type MutableDerivedNode } from './nodes';

/**
 * Internal invariant: the cleanup queue contains a derived node once for each
 * time the node became unneeded while it was not `Uncomputed`, until
 * `disposeNext` or `disposeAll` pops that entry. An entry whose node became
 * needed again stays on the cleanup queue until `disposeNext` or `disposeAll`
 * pops the entry.
 */
const cleanupQueue: Array<MutableDerivedNode<unknown>> = [];

const SCHEDULE_NOT_PENDING = 0;
const SCHEDULE_PENDING = 1;

type ScheduleStatus = typeof SCHEDULE_NOT_PENDING | typeof SCHEDULE_PENDING;

let scheduleStatus: ScheduleStatus = SCHEDULE_NOT_PENDING;

/**
 * `scheduler.schedule` is called with no arguments at the end of an outermost
 * operation after the cleanup queue went from empty to non-empty. It may call
 * `disposeAll` synchronously. The default disposes during
 * `requestIdleCallback` time where that exists, and for 5 ms per `setTimeout`
 * task elsewhere, while the cleanup queue is non-empty. Assigning
 * `scheduler.schedule` replaces the default.
 *
 * Contract: after a call, the scheduler calls `disposeNext` until it returns
 * false, or calls `disposeAll`.
 */
export const scheduler: { schedule: () => void } = {
  schedule: defaultSchedule,
};

const TIMEOUT_DISPOSAL_MS = 5;

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

export function addToCleanupQueue(node: MutableDerivedNode<unknown>): void {
  if (cleanupQueue.length === 0) {
    scheduleStatus = SCHEDULE_PENDING;
  }
  cleanupQueue.push(node);
}

export function scheduleIfPending(): void {
  if (scheduleStatus === SCHEDULE_PENDING) {
    scheduleStatus = SCHEDULE_NOT_PENDING;
    scheduler.schedule();
  }
}

/**
 * Pops one entry off the cleanup queue and disposes its node if the node is
 * still unneeded and not `Uncomputed`. Disposing a node puts each of its
 * derived dependencies that becomes unneeded on the cleanup queue. Returns
 * whether the cleanup queue is non-empty.
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
  if (cleanupQueue.length > 0) {
    return true;
  }
  scheduleStatus = SCHEDULE_NOT_PENDING;
  return false;
}

/**
 * Disposes every unneeded node on the cleanup queue, and each node that a
 * disposal leaves unneeded.
 *
 * Guarantee: after `disposeNext` returns false, and after `disposeAll`
 * returns, the cleanup queue is empty and no scheduler call is pending.
 *
 * Contract: as `disposeNext`.
 */
export function disposeAll(): void {
  while (true) {
    const node = cleanupQueue.pop();
    if (node === undefined) {
      scheduleStatus = SCHEDULE_NOT_PENDING;
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

function disposeNode(node: MutableDerivedNode<unknown>): void {
  unlinkAll(node.firstSourceDependency);
  let edge = node.firstDerivedDependency;
  while (edge !== undefined) {
    unlinkEdge(edge);
    const dependency = edge.dependency;
    if (
      dependency.firstDependent === undefined &&
      dependency.firstRetain === undefined
    ) {
      addToCleanupQueue(dependency);
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
 * Internal invariant: a dependency that a disposal here leaves unneeded is
 * disposed too if it is not `Clean`, because the same write marked it. If it
 * is `Clean`, it goes on the cleanup queue with its value.
 */
export function disposeMarked(): void {
  let stack: Array<MutableDerivedNode<unknown>> | undefined;
  for (const marked of markedUnneeded) {
    let node: MutableDerivedNode<unknown> | undefined = marked;
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
          const dependency = edge.dependency;
          if (
            dependency.firstDependent === undefined &&
            dependency.firstRetain === undefined
          ) {
            if (dependency.status === CLEAN) {
              addToCleanupQueue(dependency);
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
```

## `src/pull.ts`

```ts
// from libs/pico-js/src/pull.ts
import { markMaybeDirtyDependentsDirty } from './marking';
import {
  MAYBE_DIRTY,
  CLEAN,
  DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY,
  DIRTY,
  LAZY_OBSERVED,
  UNCOMPUTED,
  type DerivedNode,
  type MutableDerivedNode,
  type MutableEdge,
  type MutableSourceNode,
  type SourceNode,
} from './nodes';
import { addToCleanupQueue } from './cleanup';
import { linkEdge, unlinkAll, unlinkEdge } from './edges';

/**
 * Returns the value of `source` and records it as a dependency of
 * `callingNode`.
 *
 * Contract: called only while the computation of `callingNode` runs.
 */
export function readSource<TValue>(
  callingNode: DerivedNode<unknown>,
  source: SourceNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableSource = source as MutableSourceNode<TValue>;
  mutableSource.dependentsStatus = DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY;
  const last = mutableCallingNode.lastSourceDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstSourceDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== mutableSource) {
      unlinkEdge(next);
      next.dependency = mutableSource;
      linkEdge(next);
    }
    mutableCallingNode.lastSourceDependency = next;
  } else if (last === undefined || last.dependency !== mutableSource) {
    const edge: MutableEdge<MutableSourceNode<unknown>> = {
      dependency: mutableSource,
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

/**
 * Brings `node` up to date, returns its value, and records it as a dependency
 * of `callingNode`.
 *
 * Contract: called only while the computation of `callingNode` runs.
 */
export function readDerived<TValue>(
  callingNode: DerivedNode<unknown>,
  node: DerivedNode<TValue>,
): TValue {
  const mutableCallingNode = callingNode as MutableDerivedNode<unknown>;
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  const last = mutableCallingNode.lastDerivedDependency;
  const next =
    last === undefined
      ? mutableCallingNode.firstDerivedDependency
      : last.nextDependency;
  if (next !== undefined) {
    if (next.dependency !== mutableNode) {
      const previousDependency = next.dependency;
      unlinkEdge(next);
      next.dependency = mutableNode;
      linkEdge(next);
      if (
        previousDependency.firstDependent === undefined &&
        previousDependency.firstRetain === undefined
      ) {
        addToCleanupQueue(previousDependency);
      }
    }
    mutableCallingNode.lastDerivedDependency = next;
  } else if (last === undefined || last.dependency !== mutableNode) {
    const edge: MutableEdge<MutableDerivedNode<unknown>> = {
      dependency: mutableNode,
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
 * Guarantee: a `Dirty` or `MaybeDirty` node that behaves eagerly returns its
 * previous value and recomputes nothing.
 */
export function pull<TValue>(node: MutableDerivedNode<TValue>): TValue {
  switch (node.status) {
    case CLEAN:
      return node.value;
    case DIRTY:
      return node.evaluation >= LAZY_OBSERVED ? node.value : recompute(node);
    case MAYBE_DIRTY:
      if (node.evaluation >= LAZY_OBSERVED) {
        return node.value;
      }
      return verify(node) === DIRTY ? recompute(node) : node.value;
    case UNCOMPUTED:
      return computeFirst(node);
  }
}

export function verify(
  root: MutableDerivedNode<unknown>,
): typeof DIRTY | typeof CLEAN {
  let verifyStack: Array<MutableEdge<MutableDerivedNode<unknown>>> | undefined;
  let node: MutableDerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  while (true) {
    if (edge !== undefined) {
      const dependency = edge.dependency;
      if (
        dependency.status === MAYBE_DIRTY &&
        dependency.evaluation < LAZY_OBSERVED
      ) {
        // Internal invariant: a dependency with one dependent has one edge back
        // to that dependent, so only an edge into a dependency with several
        // dependents is pushed onto the verify stack.
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
      // Internal invariant: a write to a source dependency of a MaybeDirty node
      // makes the node Dirty, so a MaybeDirty node whose derived dependencies are
      // all unchanged becomes Clean.
      node.status = CLEAN;
    }
    while (true) {
      if (node === root) {
        return CLEAN;
      }
      const only = node.firstDependent!;
      edge = only.nextDependent === undefined ? only : verifyStack!.pop()!;
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

function computeFirst<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  trim(node);
  node.status = CLEAN;
  node.value = value;
  return value;
}

function recompute<TValue>(node: MutableDerivedNode<TValue>): TValue {
  const previous = node.value;
  const value = rerun(node, previous);
  if (value === previous) {
    return previous;
  }
  node.value = value;
  // Internal invariant: the dependents of a lazy node are already MaybeDirty or
  // Dirty. A MaybeDirty dependent now has a dependency whose value changed, so
  // markMaybeDirtyDependentsDirty makes that dependent Dirty.
  markMaybeDirtyDependentsDirty(node.firstDependent);
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

function trim(node: MutableDerivedNode<unknown>): void {
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

function unlinkDropped(first: MutableEdge<MutableDerivedNode<unknown>>): void {
  let edge: MutableEdge<MutableDerivedNode<unknown>> | undefined = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    const dependency = edge.dependency;
    if (
      dependency.firstDependent === undefined &&
      dependency.firstRetain === undefined
    ) {
      addToCleanupQueue(dependency);
    }
    edge = edge.nextDependency;
  }
}
```

## `src/batch.ts`

```ts
// from libs/pico-js/src/batch.ts
import {
  markDirty,
  markDirtyQueue,
  markedUnneeded,
  storeValue,
} from './marking';
import {
  MAYBE_DIRTY,
  CLEAN,
  DIRTY,
  LAZY_OBSERVED,
  ON_CHANGE_NOT_QUEUED,
  ON_CHANGE_QUEUED,
  UNCOMPUTED,
  type MutableDerivedNode,
  type MutableSourceNode,
  type SourceNode,
} from './nodes';
import { rerun, verify } from './pull';
import { disposeMarked, scheduleIfPending } from './cleanup';

let batchDepth = 0;

/**
 * Internal invariant: the flush queue of the open batch is created on the
 * first push, so a batch that adds nothing to the flush queue allocates
 * nothing.
 */
let batchQueue: Array<MutableDerivedNode<unknown>> | undefined;

const FLUSH_IDLE = 0;
const FLUSH_RUNNING = 1;

type FlushState = typeof FLUSH_IDLE | typeof FLUSH_RUNNING;

let flushState: FlushState = FLUSH_IDLE;

/**
 * Opens a batch. Each write until the matching `endWriteBatch` marks at once, and
 * the flush of the outermost batch refreshes the nodes that behave eagerly. A
 * nested batch joins the open batch.
 *
 * Contract: each `startWriteBatch` is matched by one `endWriteBatch`, called outside
 * computations and `onChange` callbacks. Inside the batch, a top-level read of
 * a node on the flush queue, or of a node that depends on it, returns the
 * previous value.
 */
export function startWriteBatch(): void {
  batchDepth += 1;
}

/**
 * Closes a batch. The outermost `endWriteBatch` flushes: it refreshes the nodes on
 * the flush queue, then calls the `onChange` of each retain of each node whose
 * value a refresh changed.
 *
 * Contract: as `startWriteBatch`.
 */
export function endWriteBatch(): void {
  batchDepth -= 1;
  if (batchDepth === 0) {
    const queue = batchQueue;
    batchQueue = undefined;
    flushOrDisposeMarked(queue);
  }
  scheduleIfOutermost();
}

/**
 * Stores `value` and marks the dependents of `source`. Outside a batch it
 * flushes before it returns, as a batch of one write does.
 *
 * Contract: called outside computations and `onChange` callbacks.
 */
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

export function currentBatchQueue():
  | Array<MutableDerivedNode<unknown>>
  | undefined {
  return batchDepth > 0 ? batchQueue : undefined;
}

export function flushOrQueue(
  queue: Array<MutableDerivedNode<unknown>> | undefined,
): void {
  if (batchDepth > 0) {
    batchQueue = queue;
  } else {
    flushOrDisposeMarked(queue);
  }
}

function flushOrDisposeMarked(
  queue: Array<MutableDerivedNode<unknown>> | undefined,
): void {
  if (queue !== undefined) {
    flush(queue);
  } else if (markedUnneeded.length > 0) {
    disposeMarked();
  }
}

/**
 * An outermost operation is a call to `writeSource`, `endWriteBatch`,
 * `retainDerivedNode`, `releaseRetain`, `readDerivedNode`, or
 * `readUnretained` made with no batch open and no flush running, or the
 * `endWriteBatch` that closes the outermost batch.
 *
 * Internal invariant: a top-level read never runs inside a computation (its
 * contract), so a top-level read inside an `onChange` is the only nesting of
 * operations outside a batch, and the flush state covers it.
 */
export function scheduleIfOutermost(): void {
  if (batchDepth === 0 && flushState === FLUSH_IDLE) {
    scheduleIfPending();
  }
}

/**
 * Internal invariant: no flush runs inside another, because no `onChange`
 * writes or ends a batch. One module-level array therefore serves every
 * flush. The flush clears each entry of the array before it calls the
 * `onChange` callbacks of the entry's node, and `changedCount` is 0 outside a
 * flush.
 */
const changedNodes: Array<MutableDerivedNode<unknown> | undefined> = [];
let changedCount = 0;

/**
 * Guarantee: every `onChange` runs after the flush has refreshed every node
 * on the flush queue, in the order of each node's first recompute that
 * changed its value. A computation or an `onChange` that throws leaves the
 * flush state idle, the marked-unneeded list empty, and no node waiting for
 * its `onChange` callbacks. Later operations, including another request's,
 * therefore start from the module state of an idle engine.
 *
 * Internal invariant: marking puts a node on the flush queue each time it
 * takes the node out of `Clean`. A node is on the flush queue twice when a
 * recompute made it `Clean` after its first entry and a later write marked it
 * again. `refreshEager` skips the second entry because the node is `Clean`.
 * The list of changed nodes has no duplicates, because a node on it has
 * `onChangeStatus` `ON_CHANGE_QUEUED`. An `onChange` writes nothing, so it
 * puts no node on the flush queue. The nodes left on the marked-unneeded list
 * after a throw are on the cleanup queue already.
 */
function flush(eagerNodes: Array<MutableDerivedNode<unknown>>): void {
  flushState = FLUSH_RUNNING;
  try {
    flushQueue(eagerNodes);
  } finally {
    flushState = FLUSH_IDLE;
    markedUnneeded.length = 0;
    if (changedCount > 0) {
      clearChangedNodes();
    }
  }
}

function clearChangedNodes(): void {
  for (let index = 0; index < changedCount; index += 1) {
    const node = changedNodes[index];
    if (node !== undefined) {
      node.onChangeStatus = ON_CHANGE_NOT_QUEUED;
      changedNodes[index] = undefined;
    }
  }
  changedCount = 0;
}

function flushQueue(eagerNodes: Array<MutableDerivedNode<unknown>>): void {
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
      changedNodes[changedCount] = node;
      changedCount += 1;
    }
  }
  if (markedUnneeded.length > 0) {
    disposeMarked();
  }
  for (index = 0; index < changedCount; index += 1) {
    const node = changedNodes[index]!;
    changedNodes[index] = undefined;
    node.onChangeStatus = ON_CHANGE_NOT_QUEUED;
    // Contract: no computation or `onChange` retains or releases, so the
    // retain list is the one the node had when the refresh changed its value.
    let retain = node.firstRetain;
    while (retain !== undefined) {
      if (retain.onChange !== undefined) {
        retain.onChange();
      }
      retain = retain.next;
    }
  }
  changedCount = 0;
}

const CHANGED = 0;
const UNCHANGED = 1;

type RefreshResult = typeof CHANGED | typeof UNCHANGED;

/**
 * Internal invariant: a node whose evaluation dropped to a value less than `LAZY_OBSERVED`
 * after marking put it on the flush queue is skipped by the flush. The node
 * stays `Dirty` or `MaybeDirty`, and the drop marked its dependents, so a
 * tracked read or a top-level read recomputes the node as a lazy node.
 */
function refreshEager(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  if (node.evaluation < LAZY_OBSERVED) {
    return UNCHANGED;
  }
  switch (node.status) {
    case DIRTY:
      return recomputeEager(eagerNodes, node);
    case MAYBE_DIRTY:
      return verify(node) === DIRTY
        ? recomputeEager(eagerNodes, node)
        : UNCHANGED;
    // Internal invariant: a Clean node on the flush queue was recomputed after
    // marking put it there, by a tracked read, a top-level read, or an earlier
    // entry of the same node. An Uncomputed node was disposed after marking put
    // it on the flush queue, and recomputing it would link its edges again.
    case CLEAN:
    case UNCOMPUTED:
      return UNCHANGED;
  }
}

/**
 * Internal invariant: the dependents of a node that behaves eagerly may be
 * `Clean`, so a changed value marks them through `markDirty`. Each dependent
 * that behaves eagerly and that `markDirty` marks goes on the flush queue the
 * flush is walking.
 */
function recomputeEager(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  const previous = node.value;
  const value = rerun(node, previous);
  if (value === previous) {
    return UNCHANGED;
  }
  node.value = value;
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(eagerNodes, first);
  }
  return CHANGED;
}
```

## `src/retain.ts`

```ts
// from libs/pico-js/src/retain.ts
import { currentBatchQueue, flushOrQueue, scheduleIfOutermost } from './batch';
import { markMaybeDirtyQueue } from './marking';
import {
  MAYBE_DIRTY,
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
import { addToCleanupQueue } from './cleanup';
import { pull } from './pull';

/**
 * Brings `node` up to date, adds a retain to the head of its retain list, and
 * returns the retain. A retained node keeps its value and its edges to its
 * dependencies until its last retain is released. The first retain of a node
 * created `EAGER`, and each retain with an `onChange`, make the node behave
 * eagerly.
 *
 * Contract: called outside computations and `onChange` callbacks.
 */
export function retainDerivedNode<TValue>(
  node: DerivedNode<TValue>,
  onChange?: () => void,
): Retain<TValue> {
  const mutableNode = node as MutableDerivedNode<TValue>;
  // Internal invariant: retaining brings the node up to date, so it is Clean
  // when it starts to behave eagerly.
  pull(mutableNode);
  const head = mutableNode.firstRetain;
  const retain: MutableRetain<TValue> = {
    node: mutableNode,
    onChange,
    previous: undefined,
    next: head,
  };
  if (head !== undefined) {
    head.previous = retain;
  }
  mutableNode.firstRetain = retain;
  if (head === undefined || onChange !== undefined) {
    updateEvaluation(mutableNode);
  }
  scheduleIfOutermost();
  return retain;
}

/**
 * Releases `retain`. A release that leaves the node unneeded and not
 * `Uncomputed` puts the node on the cleanup queue. The node keeps its value and
 * its edges until a disposal, or the marking of a write, reaches it.
 *
 * Contract: called once per retain, outside computations and `onChange`
 * callbacks.
 */
export function releaseRetain<TValue>(retain: Retain<TValue>): void {
  const mutableRetain = retain as MutableRetain<TValue>;
  const mutableNode = mutableRetain.node;
  const previous = mutableRetain.previous;
  const next = mutableRetain.next;
  if (previous === undefined) {
    mutableNode.firstRetain = next;
  } else {
    previous.next = next;
  }
  if (next !== undefined) {
    next.previous = previous;
  }
  if (mutableNode.firstRetain === undefined || retain.onChange !== undefined) {
    updateEvaluation(mutableNode);
  }
  if (
    mutableNode.firstRetain === undefined &&
    mutableNode.firstDependent === undefined &&
    mutableNode.status !== UNCOMPUTED
  ) {
    addToCleanupQueue(mutableNode);
  }
  scheduleIfOutermost();
}

/**
 * Top-level read of the retained `node`: brings `node` up to date and returns
 * its value.
 *
 * Contract: `node` is retained. Called outside computations.
 */
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue {
  const value = pull(node as MutableDerivedNode<TValue>);
  scheduleIfOutermost();
  return value;
}

/**
 * Top-level read of `node` without a retain: brings `node` up to date and
 * returns its value. A recompute links edges as in a tracked read. A node left
 * unneeded and not `Uncomputed` goes on the cleanup queue, so the scheduler can
 * dispose it and each dependency that its disposal leaves unneeded.
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
    addToCleanupQueue(mutableNode);
  }
  scheduleIfOutermost();
  return value;
}

function evaluationState(node: MutableDerivedNode<unknown>): EvaluationState {
  if ((node.evaluation & EAGER) === EAGER) {
    return node.firstRetain !== undefined ? EAGER_ACTIVE : EAGER_INACTIVE;
  }
  return hasOnChangeRetain(node) ? LAZY_OBSERVED : LAZY;
}

/**
 * Internal invariant: a node that stops behaving eagerly while `Dirty` or
 * `MaybeDirty` may have `Clean` dependents, so the node marks them `MaybeDirty`. A lazy
 * node that is not `Clean` has no `Clean` dependents. A node that starts
 * behaving eagerly was brought up to date first, so it is `Clean`.
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
    (node.status === DIRTY || node.status === MAYBE_DIRTY)
  ) {
    flushOrQueue(markMaybeDirtyQueue(currentBatchQueue(), node, undefined));
  }
}

function hasOnChangeRetain(node: MutableDerivedNode<unknown>): boolean {
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

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export { endWriteBatch, startWriteBatch, writeSource } from './batch';
export { disposeAll, disposeNext, scheduler } from './cleanup';
export { createDerivedNode, createSourceNode, EAGER, LAZY } from './nodes';
export type {
  DerivedNode,
  DerivedNodeEvaluation,
  Retain,
  SourceNode,
} from './nodes';
export { readDerived, readSource } from './pull';
export {
  readDerivedNode,
  readUnretained,
  releaseRetain,
  retainDerivedNode,
} from './retain';
```

## Implementation notes

- `Status` is a union of four numbers with a constant for each, so the hot path compares small integers and each value has a name in the code. `DependentsStatus`, `OnChangeStatus`, `ScheduleStatus`, `FlushState`, and `RefreshResult` follow the same pattern. `ScheduleStatus` and `FlushState` replace the prototype's `boolean` and counter (AGENTS.md, Booleans).
- `evaluation` takes one of four values. Bit 0 is the creation choice (`LAZY` 0, `EAGER` 1). A node behaves eagerly at 2 or more: `LAZY_OBSERVED` 2 (created `LAZY`, has a retain with an `onChange`) and `EAGER_ACTIVE` 3 (created `EAGER`, retained). Marking, `pull`, `verify`, and `refreshEager` test `evaluation >= LAZY_OBSERVED`. `updateEvaluation` recomputes the value after the first retain, the last release, and any retain or release that has an `onChange`. `evaluationState` reads the creation choice from bit 0. A release of a retain with an `onChange` walks the retain list in `hasOnChangeRetain`, because the engine keeps no count of retains with an `onChange`. `hasOnChangeRetain` returns a `boolean`, which answers whether a retain with an `onChange` exists and has no domain names for the two answers.
- Marking pushes a node onto its stack only when the node leaves `Clean` and its list of dependents is not empty, so a write visits each node's dependents at most once. Marking holds the last node whose dependents it must mark in a local, and allocates the marking stack on the first push. Marking continues into a dependent that is the last entry of a list of dependents without pushing the dependent, so a chain, or a fan-out whose dependents have no dependents, allocates no stack. The marking stack and the verification stack are arrays local to the call, so the depth of a marked or verified chain does not reach the call stack.
- Marking pushes a lazy node it takes out of `Clean` onto `markedUnneeded` when the node has no dependent and no retain. `disposeMarked` runs at the end of the outermost write or batch: after the flush's refreshes and before its `onChange` calls, or alone when the write put nothing on the flush queue. `disposeMarked` disposes each listed node that is still unneeded and not `Clean`, and walks the derived dependencies of each such node with a stack allocated on the first push: a dependency left unneeded that is not `Clean` is disposed too, because the write marked it, and a `Clean` one is pushed onto the cleanup queue with its value. A listed node that a top-level read inside the batch recomputed is `Clean` and is kept. That node is on the cleanup queue already, because the node became unneeded before the write.
- A node becomes unneeded in five places, and each puts the node on the cleanup queue: `releaseRetain`, `readUnretained`, the repoint in `readDerived`, `unlinkDropped` (the drop in `trim`), and the cascade in `disposeNode` and `disposeMarked`, all through `addToCleanupQueue`. A push onto an empty cleanup queue sets `scheduleStatus` to `SCHEDULE_PENDING`. The cleanup queue holds one entry per push and the node has no flag, so a node can be on the cleanup queue more than once. `disposeNext` and `disposeAll` check each entry again, and drop an entry whose node has a retain, has a dependent, or is `Uncomputed`.
- Every public operation ends with `scheduleIfOutermost`. `scheduleIfOutermost` calls `scheduleIfPending` only when `batchDepth === 0` and `flushState === FLUSH_IDLE`. A top-level read never runs inside a computation (its contract), so the only nesting is a top-level read inside an `onChange`, which the flush state covers. A scheduler may therefore call `disposeAll` synchronously: no computation, batch, or flush is running when the scheduler is called. With that scheduler the engine disposes when ba3daedf7 disposed, at the end of each outermost operation.
- The default scheduler uses `requestIdleCallback` where it exists, and disposes while `deadline.timeRemaining() > 0`. Elsewhere, including Node, it uses `setTimeout(…, 0)` and disposes for `TIMEOUT_DISPOSAL_MS` (5 ms) per task. Both reschedule while entries remain. On a server, the cleanup queue holds the nodes of every request in the process, and the check of each entry in `disposeNext` keeps a node that another request retained, or that a later tracked read or top-level read reached.
- `flush` reads the flush queue by index, so the entries that a refresh appends are refreshed in the same walk. `flush` appends each changed node that has a retain to the module-level `changedNodes`, and walks those entries by index after the flush queue is drained and the marked-unneeded nodes are disposed, so every `onChange` runs after every node on the flush queue is refreshed. `flush` sets each entry to `undefined` before it calls the `onChange` of each retain of that node, and `changedCount` is 0 when the flush returns. For each changed node, `flush` walks the retain list from the head and calls each `onChange`. No `onChange` writes, retains, or releases, so the retain list is the same as when the refresh changed the value. No flush runs inside another.
- `flush` restores `flushState`, empties `markedUnneeded`, and clears the remaining `changedNodes` entries (resetting their `onChangeStatus`) in a `finally`. The flush queue of the batch is already detached: `endWriteBatch` sets `batchQueue` to `undefined` before it calls `flush`, and a `writeSource` outside a batch passes a local flush queue. The cost of this `finally` was not measured at this site. In the ambient variant's microbenchmark (Measurements), removing the outermost `try`/`finally` of that variant moved an event from 62 to 64 ns to 61 to 62 ns.
- `refreshEager` skips a node whose evaluation fell below `LAZY_OBSERVED` after marking put it on the flush queue, because a release in the same batch removed the last retain of the node or the last retain with an `onChange`. `updateEvaluation` marked that node's `Clean` dependents `MaybeDirty` through `markMaybeDirtyQueue` at the release, so the lazy invariant holds and a tracked read or a top-level read recomputes the node. A `Clean` node on the flush queue was refreshed or recomputed by a tracked read or a top-level read before the flush reached the entry. An `Uncomputed` node on the flush queue was disposed after marking put it there, and recomputing it would link its edges again, so the flush skips it too.
- `writeSource` inside a batch marks at once into the flush queue of the batch through `markDirtyQueue`, which creates the flush queue on the first push through `queueEagerNode`. `markDirtyQueue` and `markMaybeDirtyQueue` are copies of `markDirty` and `markMaybeDirty` that create the flush queue on the first push, because one function with an optional queue slowed `startWriteBatch` in PR #1027's review (a batch of one write 24.9 ns against 22.8 ns on Node 24.12). `markDirty` and `markMaybeDirty` take the flush queue of the flush that is running, because `recomputeEager` marks into it.
- A node becomes `Clean` only after its computation returns or after `verify` has brought every derived dependency it is linked to up to date. A lazy recompute therefore marks only `MaybeDirty` and `Dirty` nodes and takes no node out of `Clean`, so the recompute marks through `markMaybeDirtyDependentsDirty`, which walks only the node's dependents list and needs no stack. A node that behaves eagerly and is `Dirty` or `MaybeDirty` may have `Clean` dependents until the flush refreshes the node, and `recomputeEager` marks a changed value through `markDirty`.
- `verify` walks the lists of derived dependencies in the order of the tracked reads with an explicit stack of edges. `verify` moves into a lazy `MaybeDirty` dependency before its dependent continues, and brings any other dependency up to date with `pull`, so a `MaybeDirty` dependency that behaves eagerly returns its previous value. After a `pull` that leaves the current node `Dirty`, `verify` recomputes that node and moves back to the node's dependent. `verify` does not recompute `root`. `verify` returns `DIRTY` or `CLEAN` for its caller, because TypeScript narrows `node.status` in the caller's `switch` and does not see `verify` change the status. `verify` pushes an edge only when the dependency it moves into has more than one dependent, and allocates the verify stack on the first push.
- `trim` calls `unlinkDropped` only when the node has a derived edge to drop. A call with an empty list, on every recompute of a node that reads only sources, slowed the write-then-read loop of `bench/single`'s read case from 4.45 ns to 6.39 ns, and the guard brought it to 4.0 ns.
- `status === UNCOMPUTED` alone records that a node has no value. An `Uncomputed` node's `value` is `undefined`, cast to `TValue` once in `createDerivedNode` and assigned in `disposeNode` and `disposeMarked`, where `value` is `unknown`. In the other three statuses, `value` is the value that a recompute returned, which can be `undefined`.
- The exported `SourceNode`, `DerivedNode`, `Retain`, and `Edge` mark every field `readonly`, so a caller's write to any field is a type error. The public `DerivedNode` has no `status`, `onChangeStatus`, or `computation` field. `DerivedNode` still has `evaluation`, typed `EvaluationState`, which `index.ts` does not export. The engine works on `MutableSourceNode`, `MutableDerivedNode`, `MutableRetain`, and `MutableEdge`, each written out field by field in `nodes.ts`, and casts once per exported function that receives a public node or retain. A `MutableEdge` holds `MutableDerivedNode` and `MutableSourceNode` values, so marking, verification, and disposal reach nodes through edges with no cast, and a caller reaching an edge through the public `node.firstDependent` sees only the `readonly` `Edge`.
- A block comment at the top of `nodes.ts` describes the edge lists. A comment in the source that states a caller's obligation starts with `Contract:`, and one that states what the engine ensures starts with `Guarantee:` or `Internal invariant:`. The `DerivedNode` doc comment lists the internal invariants, which the README does not list.

### Checks for a future debug build

A `__DEV__` build can restore these checks. Each throws `new Error(message)`.

- `readDerivedNode`, `readUnretained`: the batch depth is 0; "a derived node was read between startWriteBatch and endWriteBatch."
- `markDirty`, `markMaybeDirty`, and `markMaybeDirtyDependentsDirty`: a dependent's `status === UNCOMPUTED`; "An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js."
- `markMaybeDirtyDependentsDirty`: a dependent's `status === CLEAN`; "A Clean node depends on a Dirty node. This is indicative of a bug in pico-js."
- `verify`: `node.firstDependent === undefined` for a node it moved into; "A node that verify moved into has no dependents. This is indicative of a bug in pico-js."
- `verify`: `verifyStack?.pop() === undefined` for a node it moved into with more than one dependent; "A node that verify moved into has more than one dependent and no edge on the verification stack. This is indicative of a bug in pico-js."
- `releaseRetain`: the retain is not in its node's list (`retain.previous === undefined ? retain.node.firstRetain !== retain : retain.previous.next !== retain`); "A retain was released twice."
- `readDerivedNode`: `node.firstRetain === undefined`; "A node was read without being retained."
- `readSource`, `readDerived`: the calling node's computation is not running; "readSource or readDerived was called outside the computation of its calling node." This needs a module-level running node in the debug build only.
- `retainDerivedNode`, `releaseRetain`, `readUnretained`, `startWriteBatch`, `endWriteBatch`, `writeSource`, `disposeNext`, `disposeAll`: called while a computation runs; "<name> was called inside a computation." The same running node serves.
- `writeSource`, `startWriteBatch`, `endWriteBatch`, `retainDerivedNode`, `releaseRetain`, `readUnretained`, `disposeNext`, `disposeAll`: called while `flushState === FLUSH_RUNNING`; "<name> was called inside an onChange."
- `startWriteBatch`: when `batchDepth` goes from 0 to 1, `queueMicrotask` a check that `batchDepth === 0` when control returns to the event loop; "A batch was open when control returned to the event loop. startWriteBatch and endWriteBatch must not span an await."
- `endWriteBatch`: `batchDepth === 0` before the decrement; "endWriteBatch was called without a matching startWriteBatch."
- `pull`, `recompute`, and `recomputeEager`: a node whose `status !== UNCOMPUTED` has no value, which needs a private `NOT_SET` sentinel in `value` while `Uncomputed`; "A node that is not Uncomputed has no value. This is indicative of a bug in pico-js."

## Tests

The prototype (`f0b001a9d`) has four runtime test files, two type test files, three shims, and the random test, and they reach the engine through the shims:

```text
test/api.ts             the earlier roots API emulated on retains; graph.test.ts and graph.test-d.ts import it
test/api-retain.ts      ba3daedf7's API (retain count, onChange field, startWriteBatch(body)) on retains, with scheduler.schedule = disposeAll; retain.test.ts, retain.test-d.ts, and random-retain.ts import it
test/api-collector.ts   the variant's API with an explicit collect(budget); collector.test.ts imports it
test/graph.test.ts      75 tests, the roots-era tests
test/retain.test.ts     105 tests, ba3daedf7's tests
test/collector.test.ts  13 tests, the cleanup queue, observers, readUnretained, marked disposal
test/scheduler.test.ts  2 tests, the default scheduler
```

`225f893e5` folds the shims into the tests, so every test calls the engine's exported functions and no file under `test/` re-exports or emulates an API. Its files are `test/graph.test.ts` (108 tests), `test/collector.test.ts` (31 tests), `test/scheduler.test.ts` (4 tests), `test/graph.test-d.ts`, and the helper `test/status.ts`, which reads a node's status for assertions because the public `DerivedNode` has no `status` field. The folding follows these rules:

- `graph.test.ts` and `retain.test.ts` merge into `test/graph.test.ts`. A test that both files hold in two versions is kept once, in the version that calls the engine API. The `describe` blocks keep the names of `retain.test.ts` (ba3daedf7's), and the roots-era blocks `roots and the collector` and `setDerivedNodeEvaluation with the evaluation the node has` move into `test/collector.test.ts` or are removed, by the rules below.
- `graph.test-d.ts` and `retain.test-d.ts` merge into `test/graph.test-d.ts`, which checks the value types of the constructors and reads, that `createDerivedNode`'s second parameter accepts `LAZY` and `EAGER` and rejects `LAZY_OBSERVED`, and that a caller cannot write a field of a source, a derived node, or a retain.
- The shim's `startWriteBatch(body)` becomes `startWriteBatch()`, the body's statements, and `endWriteBatch()`. `mutators.writeSource` becomes `writeSource`. `createEagerDerivedNode(c)` becomes `createDerivedNode(c, EAGER)`. `x.onChange = cb` and the shim's `setOnChange` become a `retainDerivedNode(x, cb)` that the test keeps and releases. `releaseDerivedNode(node)` becomes `releaseRetain(retain)` on the retain the test kept. `readRetained(retain)` becomes `readDerivedNode(retain.node)`.
- The shim's `collect(budget)` becomes a test-local helper in `collector.test.ts` that calls `disposeNext` up to `budget` times. A test that read `collectorContents()` asserts instead what the cleanup queue does: the statuses after `disposeNext` and `disposeAll`, and `disposeNext`'s return value.
- A file sets `scheduler.schedule` in `beforeEach`: `disposeAll` in the tests ported from `retain.test.ts`, so they keep ba3daedf7's immediate cleanup, and a function that records its calls in the collector and scheduler tests. `afterEach` calls `disposeAll`, so no node on the cleanup queue reaches the next test. vitest gives each file its own module instance.
- Tests of `setDerivedNodeEvaluation`, of untracked reads, of nested `startWriteBatch(body)` semantics, and of an `onChange` on a lazy node without a retain are removed with the API they test. The prototype's `// Removed under this variant:` comments are removed too (AGENTS.md: no comment that describes what was not done).
- A test of a node that behaves eagerly creates the node `EAGER` and retains it, or retains a `LAZY` node with an `onChange`.
- `collector.test.ts` gains tests for the server guarantees: a computation that throws inside the flush of `endWriteBatch` leaves a later `startWriteBatch`, `writeSource`, `endWriteBatch` of other nodes flushing and calling their `onChange`; an `onChange` that throws leaves the same; after a throwing flush, `scheduler.schedule` is called at the end of the next outermost operation.
- The `README example` test runs the README's example and asserts the values and run counts its comments state.

Top-level reads go through a test-local `read(node)`. `read(node)` retains the node on its first call for that node and never releases the retain, so a test that does not test lifetime never has a node disposed.

### Random differential test

`test-random/random-retain.ts` is renamed `test-random/random.ts`, and its import of `../test/api-retain` becomes the engine's own exports. `pnpm test:random` reads the seed count from `PICO_JS_RANDOM_SEEDS` (default 2,000) and the first seed from `PICO_JS_RANDOM_BASE_SEED` (default 1).

A case is a graph and a list of operations, generated from the seed by mulberry32. The graph has 2 to 5 sources with values 0 to 3 and 3 to 14 derived nodes, 30% of them created `EAGER`. Each derived node reads a selector, then one of three branches of up to three tracked reads that the selector's value picks, so the tracked reads of a node change from run to run. A derived node reads only sources and derived nodes of a smaller index, so the graph is acyclic. A node returns the sum of the values that its tracked reads return, modulo 2, 3, or 1000, as a number, as an object it reuses from `previous` when the number is unchanged, or as a fresh object, so that cutoff applies to the first two and never to the third. An operation is a write, a retain with or without an `onChange`, a release of an outstanding retain, a top-level read of a retained node, a `readUnretained` of any node, a collect step (`disposeAll`), or a batch of these. An `onChange` calls `readDerivedNode` on up to two retained nodes. After the operations, the case releases every outstanding retain and calls `disposeAll`.

Deltas from `f0b001a9d`'s generator: no evaluation-change operation, no untracked read inside a computation, and the evaluation is drawn once per node at creation.

Each case runs under two scheduler settings, `sync` (`scheduler.schedule = disposeAll`) and `deferred` (a scheduler that does nothing, so only `disposeAll` steps and the disposal of marked-unneeded nodes dispose), and in three batching modes:

- `single`: every write is a top-level `writeSource`.
- `one-op-batch`: every write is a `startWriteBatch`, one `writeSource`, and an `endWriteBatch`.
- `grouped`: each batch is one `startWriteBatch` and `endWriteBatch`, and its nested batches join it.

The `single` and `one-op-batch` runs log every recompute, `onChange` call, top-level read value, and, after each operation, every node's status and value. The two logs must be equal entry for entry, which compares the outside-a-batch path of `writeSource` with the inside-a-batch path. Each run also checks itself against `Model`, which keeps statuses, values, retain lists, and derived dependency lists in arrays and follows the Model section and Guarantees step by step, including the cleanup queue, the check of each entry on disposal, and the disposal of marked-unneeded nodes at the end of the outermost operation.

The prototype cleared module state between cases with `resetForTests`. The random test instead calls `vi.resetModules()` and imports `../src/index` again for each case and each shrink attempt, so a case that failed inside a computation or an `onChange` leaves no state for the next one.

A failing case shrinks by removing runs of operations (halves, then quarters, down to single operations), then single batch items and callback reads, while any check still fails. The thrown error names the seed, the first failed check, the shrunk case's failed check, and the shrunk case as JSON.

## Measurements

The figures come from the prototype chain on the detached worktree `../i2-worktrees/pico-js-globals` (`ba3daedf7` … `f0b001a9d`), the roots chain on `../i2-worktrees/pico-js-roots`, the experiments chain on `../i2-worktrees/pico-js-experiments`, and the trace replay on `../i2-worktrees/pico-js-traces`. App scenarios are `bench/app`: each ratio is p50 (or p95 where stated) against `ba3daedf7` within a block, the median over 5 blocks, with a bootstrap 95% interval; a figure counts only when its interval excludes 0. Issue cases are `bench/cases.ts`, in ms per test, median over 5 processes with the range over processes. Trace replays are computation counts over the four recorded pet-demo sessions, browse / mutate / paginate / refetch. Upstream suite figures explain causes only.

Scenario names: initial-load, navigate, page-append, page-prepend, edit-row, edit-hot-record, optimistic-rollback, optimistic-confirm, live-update, live-update-frame, refetch-unchanged, cutoff-write, typing-no-graph, search-typing, gc-sweep, deep-first-pull, missing-data-arrives, move-between-groups, route-switch-shared, route-switch-shared-no-cleanup.

Variants named below:

```text
ba3daedf7   the engine before this work: retain counts, onChange field, startWriteBatch(body), dispose at the outermost return
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

- Explicit calling node over a module-level current node (be9840e60, 7fee0aff6). A write followed by a top-level read of a retained chain of three nodes took 53 to 54 ns on e5e462613 against 62 to 64 ns ambient (+15% to +17%), from a load and a check on every read and a save and restore on every recompute. Ambient was +4% to +5% on deep-first-pull at depth 50, and its app engine's computation closure cost about 97 bytes per recomputed node. Ambient was faster on the upstream propagation tests (-11% to -24%), part of which is the upstream adapter's own tracking wrapper.
- Module-level state (one batch, one cleanup queue) over per-node or per-batch objects. On e5e462613, 1cf22dd6a, and 58f634f2a: missing-data-arrives -12.8% to -13.2%; route-switch-shared -20% to -30%; a derived node 124.4 to 132.5 bytes against 139.7 for ba3daedf7. Costs outside noise in that run: cutoff-write +3.0% (e5e462613) and +4.2% (58f634f2a), which the later 1cf22dd6a run put within noise (+1.2% [-2.2, +5.2]).
- Retain list with no count (1cf22dd6a) over a count with a list of retains with an `onChange` (e5e462613) or objects for retains with an `onChange` (58f634f2a). A derived node is 124.4 bytes against 132.5 and 132.2. A retain object is 56.6 bytes; an app's retain with its `onChange` callback and node is 306 bytes against 306 and 356 (755 for ba3daedf7). edit-row was +13.1% [+6.5, +51.3] in session 2; the later run with the same build put it at +3.5% [-3.0, +12.9].
- Rejected: roots as derived nodes (1156aa177 … 5d2439a18, c1e497e0e). The upstream suite total was +8.3% to +12.6%; createComputations +215% to +257% (+100% for V5, which dropped the root closure); fanOutRepeat +51% to +144%, dynamicSwitch +47% to +56%, writeThenDispose +55% to +81%; optimistic-rollback +30.3% [+10.5, +44.6] (FN). A root is 192 to 305 bytes against a 56.6-byte retain, and the heap after gc-sweep was 231.5 to 234.5 MB against 213.1 MB.
- `startWriteBatch()` and `endWriteBatch()` over `batch(fn)`. With 64 call sites of 1 to 3 writes each, a batch took 35.4 ns against 44.9 ns (paired median +22%, range +15.7% to +39.7%). The global batch alone on ba3daedf7 (isobatch) was within noise on edit-row, cutoff-write, and deep-first-pull.
- Rejected: marking at the outermost `endWriteBatch` (e1, 6263c959f; e1s, 0b02da84a). In the interim suite, noObservers +139%, fanOutRepeat +17% to +50%, coalescedWrites +21% and +25%, sameRecordWrites +29%, isograph read10% +11% to +35%; it won revertInBatch (-44%, -99%) and writeThenDispose (-22%). The per-batch bookkeeping of the written sources costs more than the immediate mark that it defers. Its app runs are among the paused experiments.
- Evaluation encoded in the `evaluation` field, over a separate chosen-evaluation field (370894bbd). On the full variant, the separate field changed nothing outside noise: edit-row +4.3% [-2.8, +17.9] against +3.5% for 1cf22dd6a, cutoff-write -0.1% against +1.2%, deep-first-pull at depth 50 +6.7% against +6.9%. The encoding alone on ba3daedf7 (isoenc) measured edit-row +7.6% [+1.1, +17.3], which the full variant does not reproduce.
- No per-computation counter (c0fda7a73). A top-level read never runs inside a computation (contract), so `batchDepth` and the flush state decide the outermost operation. Against 1cf22dd6a in one run: edit-row +3.1% against +3.5% and cutoff-write -0.2% against +1.2%, both within noise; deep-first-pull at depth 50 +9.4% [+3.8, +13.4] against +6.9% [+5.3, +9.9], with overlapping intervals.
- End-of-operation scheduling, which lets a synchronous scheduler reproduce immediate cleanup. With `schedule = disposeAll`, route-switch-shared computed 2,243 per event, as ba3daedf7 does, and the trace replays computed 392 / 285 / 669 / 116, as ba3daedf7 does.
- Deferred cleanup through the cleanup queue and the idle scheduler. route-switch-shared computes 786 per event against 2,243, and runs -21.8% [-24.2, -14.9] on f0b001a9d; route-switch-shared-no-cleanup -86% to -88%. The trace replays compute 104 / 120 / 534 / 55, which matches Preact (104 / 120 / 534 / 55), against 392 / 285 / 669 / 116 for ba3daedf7 (alien: 392 / 287 / 669 / 116). Issue cases, f0b001a9d idle against ba3daedf7 with render retains: hideAndShow 1.34 against 1.89; replaceEffect dispose first 0.82 against 1.45; moveBetweenEffects 1.05 against 1.17; unobservedComputed 100 0.54 against 1.49, and 10000 0.01 against 1.59.
- Rejected: disposal at the last release (8b9a2537b). With render retains and the idle scheduler, the trace replays compute 334 / 247 / 669 / 106 against 104 / 120 / 534 / 55, because a node released and read again later is recomputed.
- Marked disposal (f0b001a9d over d56dc5ede). effectChurn 100 with the idle scheduler is 1.19 [1.1, 1.3] against 22.89 [22.8, 24.0] without it and 1.16 for ba3daedf7. Without marked disposal, a node that the case leaves unneeded stays linked until the idle scheduler runs, and every write in the timed loop marks it again. With `disposeAll` before the loop: 1.83 against 32.58. effectChurn 10000 is 1.65 against 1.61. In the app, navigate (-1.0%), gc-sweep (-2.2%), and the trace replay counts are unchanged, and route-switch-shared is -21.8%.
- `readUnretained` links edges and puts the node on the cleanup queue (d56dc5ede) over read-and-dispose (6c963e2ae). repeatedUnretainedReads diamond R10 is 3.26 against 23.30 (explicit) and 25.10 (idle); flat R10 1.48 against 5.00. The trace replays with the idle scheduler compute 104 / 120 / 534 / 55 against 334 / 258 / 954 / 148.
- A render retains its nodes until commit, and `readUnretained` is not for a render. Without render retains, the synchronous scheduler gives 574 / 372 / 1085 / 235 against 392 / 285 / 669 / 116 with them, because each node that a render reads is disposed before the app retains the node. With the idle scheduler both give 104 / 120 / 534 / 55, which depends on the scheduler not running before commit.

### Remaining known costs

- repeatedUnretainedReads R1 (one `readUnretained` per node) is about +25%: flat 5.25 against 4.23 (d56dc5ede) and 4.14 (ba3daedf7); diamond 27.81 against 22.34 and 22.77. The pushes onto `markedUnneeded` and the disposal at the end of each write cost this. At R10 and R100 f0b001a9d is 1.59 and 1.11 (flat), against 1.01 and 0.63 for ba3daedf7 with render retains.
- deep-first-pull at depth 50 is +4.7% [+0.5, +7.1] on f0b001a9d. With a warm-up pass it is within noise (+3.9% [-3.4, +5.8]; 1cf22dd6a +0.6%), and without one +6.7%, so the cost is a one-time compile of the first-retain path.
- The heap after the gc-sweep sweep is 226.3 to 226.7 MB against 213.1 MB for ba3daedf7, because unneeded nodes stay computed until the scheduler disposes them.
- `retainDerivedNode` allocates a 56.6-byte `Retain`, where ba3daedf7's retain allocated nothing.
- A chain of n nodes that behave eagerly, each reading the one before and the same source, recomputes n(n+1)/2 times per write (5,050 at n = 100, against 100 for alien-signals and Preact). The flush refreshes in flush-queue order, and a refresh of a dependent reads the old value of a dependency that is still on the flush queue, so the dependent is marked and refreshed again. Refreshing in dependency order makes it linear but costs 25% to 174% on every flush with two or more queued nodes (a min-heap per flush); refreshing a flush-queued dependency on read is incorrect when a lazy dependency is in between (random seed 28). The recorded app traces have no node that behaves eagerly with a dependency that behaves eagerly (0 of 344 retained nodes are read by another node), so the case does not occur in Isograph's use and the flush stays in queue order.

### Open items

- The paused experiments (`../i2-worktrees/pico-js-experiments`): the app benchmark, `bench/single`, and the fanout microbenchmark for e1 to e7 have not run, and `resume.sh` resumes them. e2 (deduplication of tracked reads by run id), e4 (inline first edge), and e5 (dependents as arrays) won targeted suite shapes and are undecided.
- The ambient variants and the reference builds (aa780509b, 703fb1f14, the `batch(fn)` and synchronous-scheduler builds) did not complete two blocks of app scenarios.
- The single-change isolations on ba3daedf7 (`iso/`) ran only on edit-row, cutoff-write, and deep-first-pull.
- 8b9a2537b ran only the trace replays, not the app scenarios or the issue cases.
- The library swaps (alien, Preact, and signal-polyfill on pico-js) must be redone on this API (`future-improvements.md`).
- The trace replays cannot support conclusions that depend on scale: their graphs hold 43 to 294 derived nodes, and their time differences between engines are within timer overhead.
