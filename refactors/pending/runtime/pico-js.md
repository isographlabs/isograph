# pico-js: implementation

This doc is the complete source and package of `@isograph/pico-js` in `libs/pico-js`.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation and the value that computation last returned.
- A computation reads through `readSource(callingNode, source)` and `readDerived(callingNode, node)`. Each read records an edge from the calling node to the node it read. A node depends on exactly the nodes its latest computation read.
- A lazy derived node recomputes when it is read. An eager derived node recomputes in the flush of the batch whose write marked it.
- A write runs no computation. It marks the derived nodes that read the source `Dirty` and the lazy nodes above them `Check`. Marking continues through a lazy node and stops at an eager node, which it queues in the batch.
- A read of a `Dirty` node recomputes it. A read of a `Check` node reads its derived dependencies in the order its computation read them, and recomputes the node as soon as one of them changed, without reading the rest. If none changed, the node becomes `Clean` without running.
- A recompute receives `previous`, the node's last value. A recompute whose value is `===` to `previous` keeps `previous` and marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- `startBatch(body)` calls `body` with `mutators` and flushes the batch when `body` returns. The flush recomputes every queued eager node, then calls the `onChange` of each eager node whose value the flush changed. The top-level `writeSource` and `setDerivedNodeEvaluation` are a batch of one call.
- `onChange` is a field of every derived node. The flush calls it only for eager nodes, so on a lazy node it is unused until the node is set eager.
- A derived node is needed while its `retainCount` is above 0 or another node reads it. `retainDerivedNode` and `releaseDerivedNode` count holders. A node that becomes unneeded is disposed: it returns to `Uncomputed`, drops its value, and unlinks its edges, and each derived dependency that the disposal leaves unneeded is disposed in turn. A node becomes unneeded when a release brings its count to 0, or when a recompute of its last reader stops reading it. The check waits until the `readDerivedNode` or flush that started the outermost run returns, so a node that one run stops reading and a later run of the same call reads keeps its value. A `readDerivedNode` inside a computation starts its own outermost run. Sources are never disposed.
- A computation whose branch flips, such as `readDerived(callingNode, flag) ? readDerived(callingNode, b) : readDerived(callingNode, c)`, disposes the branch it stopped reading and computes it from scratch when the flag flips back. A caller that retains both `b` and `c` keeps both computed.

### Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior.

- The caller writes a source only when its value changed.
- A computation, an `onChange`, and the `body` of `startBatch` do not throw.
- A computation is synchronous. It calls `readSource` and `readDerived` only with the `callingNode` it received. It may read untracked, through `readDerivedNode` and `source.value`. An untracked read records no dependency, so the result can be stale with respect to what it read.
- A computation does not write a source, set an evaluation, retain, release, set an `onChange`, start a batch, or call a method of `mutators`.
- An `onChange` does not write, start a batch, set an evaluation, retain, release, or set an `onChange`. It may read through `readDerivedNode` and `source.value`.
- A computation's result depends only on `previous` and on the values its reads return.
- The derived nodes form a directed acyclic graph, counting untracked reads and the edges a node keeps from its previous run.
- `readDerivedNode` is called only on a node that is retained, including an untracked read inside a computation.
- Each `retainDerivedNode` is matched by exactly one `releaseDerivedNode`. A second release decrements again and can dispose a node that another holder still counts.
- The chains of derived nodes that a first computation or a recompute of `Dirty` nodes descends stay well below the call stack limit, because each node adds frames. In Node 24.12 with its default stack and one-line computations, the first computation of a chain overflows at about 2,000 derived nodes, and a recompute through a chain of `Dirty` nodes at about 2,200.
- A caller does not mutate a value that a read returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode`, `createDerivedNode`, and `createEagerDerivedNode`, and writes no field except `onChange`. The types enforce this for every field except `computation`, a method, which a type cannot mark `readonly`.
- A caller calls each method of `mutators` through `mutators`, and only until its `startBatch` returns. A detached method throws a `TypeError`, because it cannot reach the private field of `mutators`.
- A caller that needs current values reads after the batch flushes. Until the flush, a marked eager node and the nodes that read it return their previous values.

### Guarantees

pico-js upholds these while every contract holds.

- A read made while no batch has an eager node queued returns the value the node's latest computation returned, or `previous` when that computation returned a value `===` to it, and each read of that computation returned the current value of the node it read.
- A computation runs only when its node is `Uncomputed`, or when a source its latest computation read was written, or a derived node it read recomputed to a value not `===` to its previous value.
- A read of a `Check` node reads its derived dependencies in the order its latest computation read them, and recomputes the node as soon as one of them changed, without reading the rest. A read of a `Dirty` lazy node recomputes it without reading its dependencies first.
- A recompute whose value is `===` to its previous value keeps the previous value and marks nothing. `NaN` counts as changed, and `-0` over a stored `0` keeps `0`.
- The flush calls `onChange` once for each eager node that it recomputed to a value not `===` to the previous one, even when the node recomputed more than once, in the order of each node's first changed recompute. It calls them after every queued eager node is refreshed. A first computation, a cutoff, and a lazy node call no `onChange`.
- When a flush returns, every eager node is `Clean` or `Uncomputed`, except one queued in a batch whose flush has not returned. Only a read computes an eager node for the first time. A read of a `Dirty` or `Check` eager node returns its previous value and runs nothing.
- `mutators.writeSource` and `mutators.setDerivedNodeEvaluation` run no computation and call no `onChange`. A write walks the dependents list of the source and of each lazy node it takes out of `Clean` at most once each. A write returns without visiting a derived node when no `readSource` of the source has run since the source was created or last written.
- After a computation, its node is in the dependents list of each node it read through `readSource` and `readDerived`, and of no other node, until its next computation or its disposal. Marking unlinks no edge.
- A node that is not `Uncomputed` has `retainCount` above 0 or a dependent once the outermost `readDerivedNode` or flush that is running, if any, returns. A disposed node is `Uncomputed`, holds `undefined`, and has no edges. Disposal marks nothing and calls nothing.
- A derived node that a run stops reading, and that a later run started by the same `readDerivedNode` or flush reads, keeps its value and does not run for that read, at any nesting depth and whether or not the later run's node read it before. A queued eager node that no node reads and no holder retains when the flush reaches it is disposed then instead of refreshed, and a later read in that flush computes it again.
- A recompute whose reads match, in order, the edges of its previous run allocates no edge and links or unlinks none.
- Marking, verification of `Check` nodes, and disposal keep their stacks in arrays. In Node 24.12, a read through a chain of 200,000 `Check` nodes, and a release that disposes a chain of 100,000 nodes, do not overflow.
- `retainDerivedNode`, a `releaseDerivedNode` that leaves no derived dependency unneeded, and a `readDerivedNode` of a `Clean` node allocate nothing. A single-call write that queues no eager node allocates no queue, and a flush that changes no node with an `onChange` allocates no list. A write still allocates one marking-stack array when it takes out of `Clean` two or more lazy nodes that have readers, or one that has readers and is not the last entry of the list it was reached through.
- pico-js holds no module state, so two graphs that share no node do not affect each other.

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
  vitest.random.config.ts   runs test-random/random.ts only
  src/graph.ts
  src/index.ts
  test/graph.test.ts, test/graph.test-d.ts   Tests
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

Deltas from `libs/isograph-runtime/package.json`: `name`, `description`, and `repository.directory`; `dependencies` is empty; `devDependencies` drops `@babel/core` and `@isograph/babel-plugin`; `scripts` adds `test:random`, which runs the random differential test (Tests, Random differential test). `private: true` stays, as the runtime's and `@isograph/disposable-types`'s do: publishing to npm is distribution, which `IN_PROGRESS.md` lists as not planned. The one export is `.`, built from `src/index.ts` by the root `tsdown.config.ts` (`entry: ['src/index.ts']`, `format: ['commonjs', 'esm']`).

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

`vitest.config.ts` keeps vitest's default `include`, which matches `*.test.ts` and `*.spec.ts`, so `pnpm test` and the `pico-js-test` hook do not run `test-random/random.ts`. `vitest.random.config.ts` includes only that file, with a one-hour timeout, because a run of 20,000 seeds takes about 30 seconds and a failure shrinks its case before it reports. `tsconfig.test.json` includes `test-random/**/*.ts`, so `tsc -b` and the `pico-js-typecheck` hook type-check the random test without running it. The random test reads `process.env`, whose type comes from `@types/node` through vitest's types.

Deltas of `tsconfig.test.json` from the runtime's: no `paths` entry and no `fixtures/**/*.ts`, because the tests import `../src` and there are no fixtures. Deltas of `vitest.config.ts`: no Babel plugin. With `typecheck.enabled`, vitest type-checks `graph.test-d.ts` as the runtime's `read.test-d.ts` is.

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`. Every type field is writable except `Edge.dependent`: `value`, `status`, `dependentsStatus`, and the edge-list fields change on every write, read, and recompute, a reused edge changes its `dependency`, a caller sets `onChange`, the flush sets `onChangeStatus`, `retainDerivedNode` and `releaseDerivedNode` set `retainCount`, a recompute sets `disposalCandidates`, and `setDerivedNodeEvaluation` and `mutators.setDerivedNodeEvaluation` set `evaluation`. `Mutators` is the engine's one class. Its eager-node array is an ES private field, `readonly` because a batch appends to it in place and never replaces it, and `index.ts` does not export `Mutators`, so a caller gets an instance only from `startBatch`. oxlint's `strict-boolean-expressions` (`.oxlintrc.json`) holds for every condition in the body. oxlint's `switch-exhaustiveness-check` holds for every `switch` in the body, and no `switch` has a `default` arm.

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
      - name: Random differential test of @isograph/pico-js
        run: pnpm --filter @isograph/pico-js test:random
        env:
          PICO_JS_RANDOM_SEEDS: 2000
```

The second step runs the random differential test on seeds 1 to 2,000, which took 3.1 seconds in a local run. A failure prints the seed, the first failed check, and the shrunk case.

### `README.md`

````markdown
<!-- from libs/pico-js/README.md -->
# pico-js

pico-js is a push-pull memoization engine. A write to a source node eagerly marks the derived nodes that depend on it, and a derived node lazily recomputes when it is read.

## Goals

pico-js aims for correctness first and performance second. Where a rule can be the caller's obligation instead of a check on the hot path, pico-js makes it a contract and checks nothing at run time.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation and the value that computation last returned.
- A computation reads through `readSource(callingNode, source)` and `readDerived(callingNode, node)`. Each read records an edge from the calling node to the node it read. A node depends on exactly the nodes its latest computation read.
- A lazy derived node recomputes when it is read. An eager derived node recomputes in the flush of the batch whose write marked it.
- A write runs no computation. It marks the derived nodes that read the source `Dirty` and the lazy nodes above them `Check`. Marking continues through a lazy node and stops at an eager node, which it queues in the batch.
- A read of a `Dirty` node recomputes it. A read of a `Check` node reads its derived dependencies in the order its computation read them, and recomputes the node as soon as one of them changed, without reading the rest. If none changed, the node becomes `Clean` without running.
- A recompute receives `previous`, the node's last value. A recompute whose value is `===` to `previous` keeps `previous` and marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- `startBatch(body)` calls `body` with `mutators` and flushes the batch when `body` returns. The flush recomputes every queued eager node, then calls the `onChange` of each eager node whose value the flush changed. The top-level `writeSource` and `setDerivedNodeEvaluation` are a batch of one call.
- `onChange` is a field of every derived node. The flush calls it only for eager nodes, so on a lazy node it is unused until the node is set eager.
- A derived node is needed while its `retainCount` is above 0 or another node reads it. `retainDerivedNode` and `releaseDerivedNode` count holders. A node that becomes unneeded is disposed: it returns to `Uncomputed`, drops its value, and unlinks its edges, and each derived dependency that the disposal leaves unneeded is disposed in turn. A node becomes unneeded when a release brings its count to 0, or when a recompute of its last reader stops reading it. The check waits until the `readDerivedNode` or flush that started the outermost run returns, so a node that one run stops reading and a later run of the same call reads keeps its value. A `readDerivedNode` inside a computation starts its own outermost run. Sources are never disposed.
- A computation whose branch flips, such as `readDerived(callingNode, flag) ? readDerived(callingNode, b) : readDerived(callingNode, c)`, disposes the branch it stopped reading and computes it from scratch when the flag flips back. A caller that retains both `b` and `c` keeps both computed.

## API

- `createSourceNode(value): SourceNode<TValue>` returns a source node that holds `value`.
- `createDerivedNode(computation): DerivedNode<TValue>` returns an `Uncomputed` lazy derived node. `computation(callingNode, previous)` returns its value, and `previous` is `undefined` on a first computation.
- `createEagerDerivedNode(computation): DerivedNode<TValue>` returns an `Uncomputed` eager derived node.
- `readSource(callingNode, source): TValue` returns the value of `source` and records it as a dependency of `callingNode`.
- `readDerived(callingNode, node): TValue` brings `node` up to date, returns its value, and records it as a dependency of `callingNode`.
- `retainDerivedNode(node): void` increments `node.retainCount`. It allocates nothing.
- `readDerivedNode(node): TValue` brings a retained `node` up to date and returns its value. Inside a computation it is an untracked read.
- `releaseDerivedNode(node): void` decrements `node.retainCount`, and disposes the node, and the dependencies the disposal leaves unneeded, when the count reaches 0 and no node reads it.
- `startBatch(body): void` calls `body(mutators)`, then flushes the batch.
- `mutators.writeSource(source, value): void` stores `value` and marks the readers of `source`. It runs no computation.
- `mutators.setDerivedNodeEvaluation(node, evaluation): void` sets `node` `LAZY` or `EAGER`. A `Dirty` or `Check` node set eager is queued, and a `Dirty` or `Check` node set lazy marks its readers `Check`.
- `writeSource(source, value): void` and `setDerivedNodeEvaluation(node, evaluation): void` are the `mutators` methods in a batch of their own, flushed before they return.
- `node.onChange: (() => void) | undefined` is the writable callback the flush calls after it changes the eager node's value.
- `UNCOMPUTED`, `DIRTY`, `CHECK`, and `CLEAN` are the values of `node.status`, of type `Status`. `LAZY` and `EAGER` are the values of `node.evaluation`, of type `DerivedNodeEvaluation`.
- `SourceNode<TValue>` and `DerivedNode<TValue>` are the node types. Every field is `readonly` except `onChange`. `DependentsStatus` is the type of `source.dependentsStatus`, which is internal bookkeeping.

```ts
import {
  createDerivedNode,
  createEagerDerivedNode,
  createSourceNode,
  readDerived,
  readDerivedNode,
  readSource,
  releaseDerivedNode,
  retainDerivedNode,
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

retainDerivedNode(greeting);
readDerivedNode(greeting); // 'Hello, Ada Lovelace', greetingRuns === 1

writeSource(person, { first: 'Ada', last: 'Lovelace', age: 37 });
readDerivedNode(greeting); // 'Hello, Ada Lovelace', greetingRuns === 1

startBatch((mutators) => {
  mutators.writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
  mutators.writeSource(person, { first: 'Ada', last: 'King', age: 38 });
});
readDerivedNode(greeting); // 'Hello, Ada King', greetingRuns === 2

const rendered: Array<string> = [];
const view = createEagerDerivedNode((callingNode) =>
  readDerived(callingNode, greeting),
);
retainDerivedNode(view);
readDerivedNode(view);
view.onChange = () => rendered.push(readDerivedNode(view));
writeSource(person, { first: 'Augusta', last: 'King', age: 38 });
// rendered is ['Hello, Augusta King']

releaseDerivedNode(view);
releaseDerivedNode(greeting); // greeting and name are Uncomputed
```

After the first write, `name` recomputes and returns `previous`, so `greeting` becomes `Clean` without running.

## Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior.

- The caller writes a source only when its value changed.
- A computation, an `onChange`, and the `body` of `startBatch` do not throw.
- A computation is synchronous. It calls `readSource` and `readDerived` only with the `callingNode` it received. It may read untracked, through `readDerivedNode` and `source.value`. An untracked read records no dependency, so the result can be stale with respect to what it read.
- A computation does not write a source, set an evaluation, retain, release, set an `onChange`, start a batch, or call a method of `mutators`.
- An `onChange` does not write, start a batch, set an evaluation, retain, release, or set an `onChange`. It may read through `readDerivedNode` and `source.value`.
- A computation's result depends only on `previous` and on the values its reads return.
- The derived nodes form a directed acyclic graph, counting untracked reads and the edges a node keeps from its previous run.
- `readDerivedNode` is called only on a node that is retained, including an untracked read inside a computation.
- Each `retainDerivedNode` is matched by exactly one `releaseDerivedNode`. A second release decrements again and can dispose a node that another holder still counts.
- The chains of derived nodes that a first computation or a recompute of `Dirty` nodes descends stay well below the call stack limit, because each node adds frames. In Node 24.12 with its default stack and one-line computations, the first computation of a chain overflows at about 2,000 derived nodes, and a recompute through a chain of `Dirty` nodes at about 2,200.
- A caller does not mutate a value that a read returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode`, `createDerivedNode`, and `createEagerDerivedNode`, and writes no field except `onChange`. The types enforce this for every field except `computation`, a method, which a type cannot mark `readonly`.
- A caller calls each method of `mutators` through `mutators`, and only until its `startBatch` returns. A detached method throws a `TypeError`, because it cannot reach the private field of `mutators`.
- A caller that needs current values reads after the batch flushes. Until the flush, a marked eager node and the nodes that read it return their previous values.

## Guarantees

pico-js upholds these while every contract holds.

- A read made while no batch has an eager node queued returns the value the node's latest computation returned, or `previous` when that computation returned a value `===` to it, and each read of that computation returned the current value of the node it read.
- A computation runs only when its node is `Uncomputed`, or when a source its latest computation read was written, or a derived node it read recomputed to a value not `===` to its previous value.
- A read of a `Check` node reads its derived dependencies in the order its latest computation read them, and recomputes the node as soon as one of them changed, without reading the rest. A read of a `Dirty` lazy node recomputes it without reading its dependencies first.
- A recompute whose value is `===` to its previous value keeps the previous value and marks nothing. `NaN` counts as changed, and `-0` over a stored `0` keeps `0`.
- The flush calls `onChange` once for each eager node that it recomputed to a value not `===` to the previous one, even when the node recomputed more than once, in the order of each node's first changed recompute. It calls them after every queued eager node is refreshed. A first computation, a cutoff, and a lazy node call no `onChange`.
- When a flush returns, every eager node is `Clean` or `Uncomputed`, except one queued in a batch whose flush has not returned. Only a read computes an eager node for the first time. A read of a `Dirty` or `Check` eager node returns its previous value and runs nothing.
- `mutators.writeSource` and `mutators.setDerivedNodeEvaluation` run no computation and call no `onChange`. A write walks the dependents list of the source and of each lazy node it takes out of `Clean` at most once each. A write returns without visiting a derived node when no `readSource` of the source has run since the source was created or last written.
- After a computation, its node is in the dependents list of each node it read through `readSource` and `readDerived`, and of no other node, until its next computation or its disposal. Marking unlinks no edge.
- A node that is not `Uncomputed` has `retainCount` above 0 or a dependent once the outermost `readDerivedNode` or flush that is running, if any, returns. A disposed node is `Uncomputed`, holds `undefined`, and has no edges. Disposal marks nothing and calls nothing.
- A derived node that a run stops reading, and that a later run started by the same `readDerivedNode` or flush reads, keeps its value and does not run for that read, at any nesting depth and whether or not the later run's node read it before. A queued eager node that no node reads and no holder retains when the flush reaches it is disposed then instead of refreshed, and a later read in that flush computes it again.
- A recompute whose reads match, in order, the edges of its previous run allocates no edge and links or unlinks none.
- Marking, verification of `Check` nodes, and disposal keep their stacks in arrays. In Node 24.12, a read through a chain of 200,000 `Check` nodes, and a release that disposes a chain of 100,000 nodes, do not overflow.
- `retainDerivedNode`, a `releaseDerivedNode` that leaves no derived dependency unneeded, and a `readDerivedNode` of a `Clean` node allocate nothing. A single-call write that queues no eager node allocates no queue, and a flush that changes no node with an `onChange` allocates no list. A write still allocates one marking-stack array when it takes out of `Clean` two or more lazy nodes that have readers, or one that has readers and is not the last entry of the list it was reached through.
- pico-js holds no module state, so two graphs that share no node do not affect each other.

## Design choices

- Edges are objects in intrusive doubly linked lists, one in the reader's dependency list and one in the dependency's dependents list, instead of sets. Linking and unlinking are O(1) and allocate nothing.
- A recompute reuses the edge at each read position when it reads the same node there, so a stable computation allocates and relinks nothing.
- A `Dirty` node keeps its edges linked, so a second write that reaches it finds it `Dirty` and stops, and its recompute reuses them.
- A source's `dependentsStatus` records that a write left every reader `Dirty`, so the next write skips the list until a computation reads the source again.
- Disposal waits for the outermost run. A `readDerived` moves the disposal candidates of the run its pull started to its calling node, and verification moves them to the node it verifies, so the outermost `readDerivedNode` or flush holds them without a context parameter or module state.
- Marking, verification, and disposal walk explicit stacks, so the graph's depth costs no call-stack frames there.
- Hot paths allocate nothing. A single-call write creates its queue only when it queues an eager node, the flush walks that queue by index as its eager-node array, and the flush creates its `onChange` list only when a changed node has one.
- Each kind of node is one object shape, so property access stays monomorphic.
- Public node types are `readonly`, and the engine writes through an internal `Mutable` view, so the types enforce the field contract.
- A computation receives an explicit `callingNode` and passes it to every read, instead of the engine tracking the running computation in ambient state.
- pico-js has no globals and no module state.
- Retaining is a counter on the node, so there is no handle and retaining allocates nothing.
- pico-js checks no contract at run time. A future version may add the checks as a compile-time option.
````

The Model, Contracts, and Guarantees sections at the top of this doc copy the README's sections word for word. The README's Design choices section names the choices that `src/graph.ts` below makes, and the notes after it give the measurements behind them.

### Readers before the Isograph runtime depends on it

AGENTS.md says we do not have anything we do not use, and tests-only is not a reader. Until the Isograph runtime calls it, the package's reader is the user's stated goal that the engine is usable by projects outside Isograph (pico-runtime.md, Goal), and `README.md` documents the package for those projects. pico-runtime.md core change 4 ships the package before its first workspace caller on that ground; core change 5 is the first caller of the source functions and core change 8 of the derived-node functions. The package ships alone, as its own change.

## `src/graph.ts`

```ts
// from libs/pico-js/src/graph.ts
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
  // Repointed when a recompute reads a different node at this position.
  dependency: TDependency;
  readonly dependent: DerivedNode<unknown>;
  // Set by an append and by `trim`.
  nextDependency: Edge<TDependency> | undefined;
  // Set by `linkEdge` and by `unlinkEdge` of a neighbor.
  previousDependent: Edge<TDependency> | undefined;
  nextDependent: Edge<TDependency> | undefined;
};

type SourceEdge = Edge<SourceNode<unknown>>;
type DerivedEdge = Edge<DerivedNode<unknown>>;

type NodeWithDependents<TDependency> = {
  firstDependent: Edge<TDependency> | undefined;
  lastDependent: Edge<TDependency> | undefined;
};

/** The view of a node whose fields the engine writes. */
type Mutable<T> = { -readonly [K in keyof T]: T[K] };

type MutableSourceNode<TValue> = Mutable<SourceNode<TValue>>;

/** The engine's view of a derived node, with the fields only it reads. */
type MutableDerivedNode<TValue> = Mutable<DerivedNode<TValue>> & {
  onChangeStatus: OnChangeStatus;
};

/** A node in the dependents list may be `Check` or `Clean`. */
const DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY = 0;
/** Every node in the dependents list is `Dirty`, so a write skips the list. */
const DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY = 1;

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

/** Recomputes when pulled. Marking continues through it. */
export const LAZY = 0;
/** Recomputes in the flush of its batch. Marking stops at it. */
export const EAGER = 1;

export type DerivedNodeEvaluation = typeof LAZY | typeof EAGER;

/** The node is in no flush's list of nodes whose `onChange` it calls. */
const ON_CHANGE_NOT_QUEUED = 0;
/**
 * The node is in the list of the flush that is running, so a second changed
 * recompute in that flush does not add it again.
 */
const ON_CHANGE_QUEUED = 1;

type OnChangeStatus = typeof ON_CHANGE_NOT_QUEUED | typeof ON_CHANGE_QUEUED;

/**
 * Internal invariants of a derived node whose computation is not running:
 * - `Uncomputed`: `value` is `undefined`, both dependency lists are empty, and
 *   the node has no dependents.
 * - `Dirty`: `value` is the last computed value. A lazy node's dependents are
 *   `Check` or `Dirty`. An eager node is queued in an unflushed batch, and its
 *   dependents may be `Clean`.
 * - `Check`: as `Dirty`.
 * - `Clean`: `value` is up to date, unless the node depends through `Clean`
 *   nodes on a `Dirty` or `Check` eager node.
 *
 * In every status, each edge in the node's dependency lists is in its
 * dependency's dependents list. A node that is not `Uncomputed` has
 * `retainCount > 0` or a dependent, except a node in the `disposalCandidates`
 * of a node or of a flush, until the `readDerivedNode` or flush that started
 * the outermost run returns, and a node that a computation returned to a
 * `readDerived` that has not linked its edge yet.
 *
 * Contract: a caller writes no field except `onChange`. The type enforces it.
 */
export type DerivedNode<TValue> = {
  readonly value: TValue;
  // Guarantee: the flush calls it once for each eager node that the flush
  // recomputed to a value that is not `===` its previous value, after every
  // queued eager node is refreshed. A first computation does not call it, and
  // on a lazy node it is unused.
  //
  // Contract: the callback does not throw. It does not write, start a batch,
  // set an evaluation, retain, release, or set an `onChange`. It may read
  // through `readDerivedNode` and `source.value`.
  onChange: (() => void) | undefined;
  readonly status: Status;
  readonly evaluation: DerivedNodeEvaluation;
  // The number of `retainDerivedNode` calls not yet matched by a
  // `releaseDerivedNode`.
  readonly retainCount: number;
  // The derived nodes that a run of this node, or a run nested in it, left
  // with no dependent and `retainCount` 0. The `readDerived` that ran the node
  // moves them to its calling node, and the `readDerivedNode` or flush that
  // started the outermost run disposes them, unless a run read them again.
  readonly disposalCandidates: Array<DerivedNode<unknown>> | undefined;
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

export function createSourceNode<TValue>(value: TValue): SourceNode<TValue> {
  return {
    value,
    firstDependent: undefined,
    lastDependent: undefined,
    dependentsStatus: DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY,
  };
}

/**
 * Contract: `computation` is synchronous and does not throw. It reads other
 * nodes through `readSource` and `readDerived` with the `callingNode` it
 * receives, or untracked through `readDerivedNode` and `source.value`. It does
 * not write, retain, release, set an evaluation, set an `onChange`, or start a
 * batch.
 */
export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return newDerivedNode(LAZY, computation);
}

/** Contract: as `createDerivedNode`. */
export function createEagerDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return newDerivedNode(EAGER, computation);
}

function newDerivedNode<TValue>(
  evaluation: DerivedNodeEvaluation,
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  const node: MutableDerivedNode<TValue> = {
    value: undefined as unknown as TValue,
    onChange: undefined,
    onChangeStatus: ON_CHANGE_NOT_QUEUED,
    status: UNCOMPUTED,
    evaluation,
    retainCount: 0,
    disposalCandidates: undefined,
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

/**
 * The object `startBatch` passes to its body. Its methods defer eager refreshes
 * to the flush.
 *
 * Contract: a caller calls these methods through `mutators`, and only until
 * its `startBatch` returns.
 */
class Mutators {
  readonly #eagerNodes: Array<MutableDerivedNode<unknown>>;

  constructor(eagerNodes: Array<MutableDerivedNode<unknown>>) {
    this.#eagerNodes = eagerNodes;
  }

  writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void {
    const first = storeValue(source as MutableSourceNode<TValue>, value);
    if (first !== undefined) {
      markDirty(this.#eagerNodes, first);
    }
  }

  /**
   * A `Dirty` or `Check` node set lazy may have `Clean` dependents, because it
   * was eager, so its dependents are marked to restore the internal invariant
   * that a lazy node out of `Clean` has no `Clean` dependents.
   */
  setDerivedNodeEvaluation(
    node: DerivedNode<unknown>,
    evaluation: DerivedNodeEvaluation,
  ): void {
    const mutableNode = node as MutableDerivedNode<unknown>;
    if (mutableNode.evaluation === evaluation) {
      return;
    }
    mutableNode.evaluation = evaluation;
    switch (mutableNode.status) {
      case DIRTY:
      case CHECK:
        switch (evaluation) {
          case EAGER:
            // Internal invariant: its dependents are already Check or Dirty.
            // The flush brings it up to date.
            this.#eagerNodes.push(mutableNode);
            break;
          case LAZY:
            markCheck(this.#eagerNodes, mutableNode, undefined);
            break;
        }
        break;
      // Internal invariant: a Clean node is up to date under either
      // evaluation, and an Uncomputed node has no dependents.
      case CLEAN:
      case UNCOMPUTED:
        break;
    }
  }
}

/**
 * Guarantee: a nested `startBatch` flushes its own batch before it returns.
 *
 * Contract: `body` does not throw.
 */
export function startBatch(body: (mutators: Mutators) => void): void {
  const eagerNodes: Array<MutableDerivedNode<unknown>> = [];
  body(new Mutators(eagerNodes));
  flush(eagerNodes);
}

/**
 * The eager nodes of a batch of one, created on its first push, so a single
 * call that queues nothing allocates no queue. The flush walks it as its
 * eager-node array.
 */
type Queue = Array<MutableDerivedNode<unknown>>;

/** A batch of one write. */
export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  const first = storeValue(source as MutableSourceNode<TValue>, value);
  if (first === undefined) {
    return;
  }
  const queue = markDirtyQueue(first);
  if (queue !== undefined) {
    flush(queue);
  }
}

/**
 * A batch of one evaluation change. It is
 * `mutators.setDerivedNodeEvaluation` with its arrays created on the first
 * push.
 */
export function setDerivedNodeEvaluation(
  node: DerivedNode<unknown>,
  evaluation: DerivedNodeEvaluation,
): void {
  const mutableNode = node as MutableDerivedNode<unknown>;
  if (mutableNode.evaluation === evaluation) {
    return;
  }
  mutableNode.evaluation = evaluation;
  switch (mutableNode.status) {
    case DIRTY:
    case CHECK:
      switch (evaluation) {
        case EAGER:
          flush([mutableNode]);
          break;
        case LAZY: {
          const queue = markCheckQueue(undefined, mutableNode, undefined);
          if (queue !== undefined) {
            flush(queue);
          }
          break;
        }
      }
      break;
    case CLEAN:
    case UNCOMPUTED:
      break;
  }
}

/** Returns the first edge of the dependents list if the write must mark it. */
function storeValue<TValue>(
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

/**
 * Counts a holder of `node`. A retained node keeps its value and its
 * dependencies until its last release. Allocates nothing.
 *
 * Contract: called outside computations and `onChange` callbacks.
 */
export function retainDerivedNode(node: DerivedNode<unknown>): void {
  (node as MutableDerivedNode<unknown>).retainCount += 1;
}

/**
 * Drops one holder of `node`. A release that leaves the node with
 * `retainCount` 0 and no dependent disposes it, and then each derived
 * dependency that the disposal leaves in the same state.
 *
 * Contract: called once for each `retainDerivedNode` of the node, outside
 * computations and `onChange` callbacks. A second release of the same retain
 * decrements again, and can dispose a node another holder still holds.
 */
export function releaseDerivedNode(node: DerivedNode<unknown>): void {
  const mutableNode = node as MutableDerivedNode<unknown>;
  mutableNode.retainCount -= 1;
  if (
    mutableNode.retainCount === 0 &&
    mutableNode.firstDependent === undefined
  ) {
    disposeUnneeded(mutableNode, undefined);
  }
}

/**
 * Disposes `first` and each node on the stack that is unneeded: not
 * `Uncomputed`, with `retainCount` 0 and no dependent. Each derived dependency
 * that a disposal leaves with `retainCount` 0 and no dependent joins the stack.
 * A node on the stack twice, or that was read again after it joined, is
 * skipped. Sources are never disposed.
 */
function disposeUnneeded(
  first: MutableDerivedNode<unknown>,
  initialStack: Array<DerivedNode<unknown>> | undefined,
): void {
  let stack = initialStack;
  let node: MutableDerivedNode<unknown> | undefined = first;
  while (node !== undefined) {
    if (
      node.retainCount === 0 &&
      node.firstDependent === undefined &&
      node.status !== UNCOMPUTED
    ) {
      unlinkAll(node.firstSourceDependency);
      let edge = node.firstDerivedDependency;
      while (edge !== undefined) {
        unlinkEdge(edge);
        const dependency = edge.dependency;
        if (
          dependency.firstDependent === undefined &&
          dependency.retainCount === 0
        ) {
          (stack ??= []).push(dependency);
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
    node = stack?.pop() as MutableDerivedNode<unknown> | undefined;
  }
}

/**
 * Refreshes every queued eager node, then calls the `onChange` of each one
 * whose value a refresh changed, in the order of their first changed
 * recompute.
 *
 * Guarantee: every `onChange` runs while no eager node of the batch is queued.
 *
 * Internal invariant: a node queued twice, by setting it lazy and then eager,
 * is `Clean` or lazy the second time, so the eager list needs no
 * deduplication. An `onChange` writes nothing, so it queues no eager node.
 */
function flush(eagerNodes: Array<MutableDerivedNode<unknown>>): void {
  // Created on the first changed node with an `onChange`, so a flush that
  // calls nothing allocates nothing.
  let changed: Array<MutableDerivedNode<unknown>> | undefined;
  // The nodes that the flush's runs left unneeded, disposed after the last
  // refresh, so a later refresh that reads one finds it computed.
  let candidates: Array<DerivedNode<unknown>> | undefined;
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
    if (node.disposalCandidates !== undefined) {
      candidates = takeDisposalCandidates(candidates, node);
    }
    if (
      result === CHANGED &&
      node.onChange !== undefined &&
      node.onChangeStatus === ON_CHANGE_NOT_QUEUED
    ) {
      node.onChangeStatus = ON_CHANGE_QUEUED;
      (changed ??= []).push(node);
    }
  }
  if (candidates !== undefined) {
    disposeUnneeded(
      candidates.pop() as MutableDerivedNode<unknown>,
      candidates,
    );
  }
  if (changed === undefined) {
    return;
  }
  for (index = 0; index < changed.length; index += 1) {
    const node = changed[index]!;
    node.onChangeStatus = ON_CHANGE_NOT_QUEUED;
    // Contract: no computation or `onChange` sets an `onChange`, so the field
    // still holds the function it held when the refresh changed the value.
    node.onChange!();
  }
}

function linkEdge<TDependency extends NodeWithDependents<TDependency>>(
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

function unlinkEdge<TDependency extends NodeWithDependents<TDependency>>(
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
  // A run that the pull started passes the nodes it left unneeded up to the
  // calling node, so a later read of the outermost run finds them computed.
  if (mutableNode.disposalCandidates !== undefined) {
    mutableCallingNode.disposalCandidates = takeDisposalCandidates(
      mutableCallingNode.disposalCandidates,
      mutableNode,
    );
  }
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
      const previousDependency = next.dependency;
      unlinkEdge(next);
      next.dependency = node as DerivedNode<unknown>;
      linkEdge(next);
      // The outermost run may read the previous dependency again, so its
      // disposal waits until that run ends.
      if (
        previousDependency.firstDependent === undefined &&
        previousDependency.retainCount === 0
      ) {
        (mutableCallingNode.disposalCandidates ??= []).push(previousDependency);
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
 * Brings `node` up to date and returns its value. Inside a computation it is
 * an untracked read, which records no dependency.
 *
 * Contract: `node` is retained.
 */
export function readDerivedNode<TValue>(node: DerivedNode<TValue>): TValue {
  const mutableNode = node as MutableDerivedNode<TValue>;
  const value = pull(mutableNode);
  if (mutableNode.disposalCandidates !== undefined) {
    disposeCandidates(mutableNode);
  }
  return value;
}

/**
 * Clears the disposal candidates of `from` and returns them together with
 * `into`. The shorter list is appended to the longer, because their order does
 * not matter.
 */
function takeDisposalCandidates(
  into: Array<DerivedNode<unknown>> | undefined,
  from: MutableDerivedNode<unknown>,
): Array<DerivedNode<unknown>> {
  const taken = from.disposalCandidates!;
  from.disposalCandidates = undefined;
  if (into === undefined) {
    return taken;
  }
  const longer = into.length < taken.length ? taken : into;
  const shorter = longer === into ? taken : into;
  for (let index = 0; index < shorter.length; index += 1) {
    longer.push(shorter[index]!);
  }
  return longer;
}

/** Disposes the disposal candidates of `node` that are still unneeded. */
function disposeCandidates(node: MutableDerivedNode<unknown>): void {
  const candidates = node.disposalCandidates!;
  node.disposalCandidates = undefined;
  disposeUnneeded(candidates.pop() as MutableDerivedNode<unknown>, candidates);
}

/**
 * Guarantee: a `Dirty` or `Check` eager node returns its previous value and
 * runs nothing.
 */
function pull<TValue>(node: DerivedNode<TValue>): TValue {
  const mutableNode = node as MutableDerivedNode<TValue>;
  switch (mutableNode.status) {
    case CLEAN:
      return mutableNode.value;
    case DIRTY:
      return mutableNode.evaluation === LAZY
        ? recompute(mutableNode)
        : mutableNode.value;
    case CHECK:
      if (mutableNode.evaluation === EAGER) {
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

/** The refresh recomputed the node to a value not `===` its previous one. */
const CHANGED = 0;
/** The refresh left the node's value as it was, or did not refresh it. */
const UNCHANGED = 1;

type RefreshResult = typeof CHANGED | typeof UNCHANGED;

/**
 * A node set lazy after it was queued is skipped, because setting it lazy
 * marked its dependents.
 */
function refreshEager(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  node: MutableDerivedNode<unknown>,
): RefreshResult {
  if (node.evaluation === LAZY) {
    return UNCHANGED;
  }
  // A run of this flush stopped reading the node, so it waits for disposal at
  // the end of the flush. It is disposed now instead of refreshed. A later run
  // that reads it computes it again.
  if (node.retainCount === 0 && node.firstDependent === undefined) {
    disposeUnneeded(node, undefined);
    return UNCHANGED;
  }
  switch (node.status) {
    case DIRTY:
      return recomputeEager(eagerNodes, node);
    case CHECK:
      return verify(node) === DIRTY
        ? recomputeEager(eagerNodes, node)
        : UNCHANGED;
    // Internal invariant: a Clean node is in the list twice, or was disposed
    // and computed again. An Uncomputed node was disposed by a release after
    // it was queued and retained again, and computing it would relink its
    // sources.
    case CLEAN:
    case UNCOMPUTED:
      return UNCHANGED;
  }
}

function markDirty<TDependency>(
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
        if (node.evaluation === EAGER) {
          eagerNodes.push(node);
          break;
        }
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
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
function markCheck(
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
          if (dependent.evaluation === EAGER) {
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
 * `markDirty` for a batch of one, which creates its queue on the first push. A
 * copy, because an optional queue in `markDirty` slows `startBatch`.
 */
function markDirtyQueue<TDependency>(
  first: Edge<TDependency>,
): Queue | undefined {
  let queue: Queue | undefined;
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
        if (node.evaluation === EAGER) {
          queue = queueEagerNode(queue, node);
          break;
        }
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
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
function markCheckQueue(
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
          if (dependent.evaluation === EAGER) {
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

/**
 * Pulls the derived dependencies of the `Check` node `root` in read order,
 * descending into lazy `Check` dependencies without recursion. Returns `DIRTY`
 * when the caller must recompute `root`.
 */
function verify(
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
      if (dependency.status === CHECK && dependency.evaluation === LAZY) {
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
      if (dependency.disposalCandidates !== undefined) {
        root.disposalCandidates = takeDisposalCandidates(
          root.disposalCandidates,
          dependency,
        );
      }
      if (node.status !== DIRTY) {
        edge = edge.nextDependency;
        continue;
      }
      if (node === root) {
        return DIRTY;
      }
      // A Dirty node below the root recomputes without pulling the rest of its
      // dependencies. A changed value marks its Check dependents Dirty.
      recomputeBelowRoot(root, node);
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
      recomputeBelowRoot(root, node);
    }
    edge = edge.nextDependency;
  }
}

/**
 * Recomputes a node that `verify` reached below `root`, and moves the nodes the
 * run left unneeded to `root`.
 */
function recomputeBelowRoot(
  root: MutableDerivedNode<unknown>,
  node: MutableDerivedNode<unknown>,
): void {
  recompute(node);
  if (node.disposalCandidates !== undefined) {
    root.disposalCandidates = takeDisposalCandidates(
      root.disposalCandidates,
      node,
    );
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

/**
 * Internal invariant: an eager node's dependents may be `Clean`, so a changed
 * value marks them through `markDirty`.
 */
function recomputeEager(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
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

function rerun<TValue>(
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

function markCheckDependentsDirty(first: DerivedEdge | undefined): void {
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

/**
 * Unlinks and drops the edges after the last one the computation read, and
 * adds each derived dependency that the drop leaves unneeded to the node's
 * disposal candidates.
 */
function trim(node: MutableDerivedNode<unknown>): void {
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
      unlinkDropped(node, first);
      node.firstDerivedDependency = undefined;
    }
  } else if (lastDerived.nextDependency !== undefined) {
    unlinkDropped(node, lastDerived.nextDependency);
    lastDerived.nextDependency = undefined;
  }
}

/**
 * Unlinks the derived edges of `node` from `first` on, and adds each
 * dependency that an unlink leaves unneeded to the disposal candidates of
 * `node`.
 */
function unlinkDropped(
  node: MutableDerivedNode<unknown>,
  first: DerivedEdge,
): void {
  let edge: DerivedEdge | undefined = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    const dependency = edge.dependency;
    if (
      dependency.firstDependent === undefined &&
      dependency.retainCount === 0
    ) {
      (node.disposalCandidates ??= []).push(dependency);
    }
    edge = edge.nextDependency;
  }
}

function unlinkAll<TDependency extends NodeWithDependents<TDependency>>(
  first: Edge<TDependency> | undefined,
): void {
  let edge = first;
  while (edge !== undefined) {
    unlinkEdge(edge);
    edge = edge.nextDependency;
  }
}
```

- `computation` is a method signature. TypeScript checks a method's parameters bivariantly, so `DerivedNode<TValue>` is assignable to `DerivedNode<unknown>`.
- `Status` is numeric because a `switch` and a comparison on small integers cost less than on strings: numeric statuses cut the upstream benchmark total by 4.6% (pico-js-performance.md, change 5). `graph.ts` exports a `const` for each number (`UNCOMPUTED`, `DIRTY`, `CHECK`, `CLEAN`, and for `DerivedNodeEvaluation`, `LAZY` and `EAGER`), and `Status` and `DerivedNodeEvaluation` are unions of their `typeof`s, so the code names each value and a caller compares `status` with a constant.
- A read links its edge into the dependency's dependents list at the time of the read, and `readDerived` pulls the dependency before it links the edge, so a node is linked only to a source, to a `Clean` derived node, or to a `Dirty` or `Check` eager node whose previous value it read.
- `recompute` resets `lastSourceDependency` and `lastDerivedDependency`, and each read takes the next edge of the list. An edge whose dependency is the node read stays linked. An edge whose dependency differs is unlinked from the old dependency and linked to the node read. A read past the end of the list allocates an edge, unless the previous edge already points at the node read. `trim` unlinks and drops the edges past the last read. So a recompute whose reads match the edges of the previous run in order allocates, links, and unlinks nothing, and after a computation the dependency lists hold exactly its reads. A read that repeats the read just before it shares that read's edge only on the append path, when no edge of the previous run is left at its position. The first run of `a, a, b` holds the edges `a, b`. Its second run reuses the edge to `a`, moves the edge to `b` to `a` for the second read, and appends an edge to `b`, and later runs reuse the three edges.
- A node that reads a dependency at two non-adjacent positions (`a, b, a`) has two edges in `a`'s dependents list, one per read. Marking visits it twice, and the second visit finds it `Dirty` and changes nothing. A computation that reads `a, b` 1,000 times holds 2,000 source edges, 1,000 of them in `a`'s dependents list.
- Marking unlinks no edge. A `Dirty` node stays in the dependents lists of its dependencies, and marking skips it, because its dependents are already `Check` or `Dirty`. A `Check` node that marking sets `Dirty` is not pushed, for the same reason. So repeated writes to different sources of a node that is not pulled visit that node on each write and stop there.
- `storeValue` returns the dependents list for marking only when `dependentsStatus` is `DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY`, and sets it to `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`, because marking from a source sets every direct reader `Dirty`. A direct reader leaves `Dirty` only through a recompute, which either reads the source again, and `readSource` sets `DEPENDENTS_NOT_NECESSARILY_ALL_DIRTY` on every read, without comparing it first, or does not, and `trim` unlinks its edge. A node joins the list only through `readSource`, and disposal only removes nodes from it. So a write that finds `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY` would change nothing, and it visits nothing. `createSourceNode` sets `DEPENDENTS_KNOWN_TO_BE_ALL_DIRTY`, because nothing has read a new source, so a write has no dependents to mark, and the first `readSource` resets it, so a write to a source that no computation has read returns after the flag check. A write that finds the list empty sets the flag too, for the same reason. Repeated writes to one source between reads of it visit its readers once (pico-js-losses.md, flag2).
- `DependentsStatus` is a union of two numbers with a constant for each, private to `graph.ts`, so the hot path compares small integers as it does for `Status`, and each value has a name in the code.
- Marking pushes a node onto its stack only when the node leaves `Clean` and its dependents list is not empty, so a write visits each node's dependents at most once. Marking holds the last node it must descend into in a local and allocates its stack on the first push, and it descends into a dependent that is the last entry of its list without pushing it, so a chain, or a fan-out whose readers have no readers, allocates no stack. A write allocates the stack when it takes out of `Clean` two or more lazy direct readers that have readers, or a lazy node that has readers and is not the last entry of the dependents list marking reached it through. A write to a source with 100 lazy readers that each have one reader allocates about 2,360 bytes (Node 24.12, `heapUsed` over 20,000 writes). Marking each direct reader's dependents as soon as the reader is marked allocates no stack, and that variant measured mixed in the PR #1027 performance review: 1,000 readers with one reader each went from 34.6 µs to 29.2 µs per write and pull, and a `writeSource` with one observed reader from 21.3 ns to 24.9 ns, so `markDirty` keeps `held` and the stack. The marking stack and the verification stack are arrays local to the call, so the depth of a marked or verified chain does not reach the call stack, and the engine holds no module state (pico-js-losses.md, fast2).
- A node becomes `Clean` only after its computation returns or after `verify` has pulled every derived dependency it is linked to. Each lazy dependency a computation links is `Clean` when it is read, and marking continues through every lazy node, so every dependent of a lazy node that is not `Clean` is `Check` or `Dirty`. A lazy recompute therefore marks only `Check` and `Dirty` nodes and takes no node out of `Clean`, so it marks through `markCheckDependentsDirty`, which walks only the node's dependents list, and needs no stack. A pull outside a flush recomputes only lazy nodes, so it takes no node out of `Clean` and takes no batch. A node that `setDerivedNodeEvaluation` or `mutators.setDerivedNodeEvaluation` makes lazy while `Dirty` or `Check` had its `Clean` dependents marked `Check` by that call, so this holds for it too.
- An eager node that is `Dirty` or `Check` may have `Clean` dependents, and only until the flush of the batch whose `eagerNodes` holds it. Marking queues it when it leaves `Clean`, and `setDerivedNodeEvaluation` and `mutators.setDerivedNodeEvaluation` queue a `Dirty` or `Check` node it makes eager, so every `Dirty` or `Check` eager node is queued. The flush refreshes it at the top level, outside any computation, through `refreshEager`. `recomputeEager` marks a changed value through `markDirty`, which takes the eager-node array of the batch being flushed, takes `Clean` dependents out of `Clean`, and queues the eager nodes among them, and it returns `CHANGED`. `flush` then appends the node to its list of changed nodes if the node has an `onChange` and its `onChangeStatus` is `ON_CHANGE_NOT_QUEUED`, and sets it to `ON_CHANGE_QUEUED`, so a node that recomputes twice in one flush, because it read an eager node that the flush refreshed after it, is in the list once. Marking collects nothing, so a lazy node's `onChange` is never called, and an eager node's is called only when its value changed.
- `mutators.setDerivedNodeEvaluation` and `setDerivedNodeEvaluation` restore the lazy invariant when they make a `Dirty` or `Check` eager node lazy: they mark the node's `Clean` dependents `Check` through `markCheck` or `markCheckQueue`, the second phase of `markDirty`, which starts from the node as if marking had just taken it out of `Clean`. Leaving the node queued and its dependents `Clean` until the flush would break the invariant until then: a pull of the lazy `Dirty` node would recompute it, and `markCheckDependentsDirty` would leave its `Clean` dependents `Clean` and stale, and marking that reached the node would stop there and leave its `Clean` dependents stale. `refreshEager` skips a lazy node, because its dependents are already `Check` or `Dirty`, and a read recomputes it.
- `startBatch` allocates one array and one `Mutators` per call, and the caller's `body` is a closure. The methods of `Mutators` are prototype methods over its private field. A single call allocates no queue until marking pushes an eager node, and then one array, which the flush walks as its eager-node array. `markDirtyQueue` and `markCheckQueue` are copies of `markDirty` and `markCheck` that create the queue on the first push, through `queueEagerNode`, with an array literal that holds the pushed element. A literal of one element has room for one, and a push onto an empty array allocates room for 17. The helper is a function of its own because the same branch written inside `markDirtyQueue` and `markCheckQueue` slowed a write to a source that one lazy derived node reads, which queues nothing, from 5.1 ns to 7.6 ns. One `markDirty` that took an optional queue and returned it slowed `startBatch`: on Node 24.12, median of five processes, a batch of one write took 24.9 ns against 22.8 ns, and a batch of two writes 29.2 ns against 23.7 ns. With the copies, every batched case is within 0.2 ns of the engine without single calls. In Node 24.12, `heapUsed` over 20,000 single-call writes and reads grows by 0.4 bytes per write for a source whose one reader is lazy, 4.2 bytes for one whose one reader is eager, and 31.3 bytes for one whose eager reader has an `onChange`, which is the flush's list of changed nodes.
- `flush` reads the eager-node array by index, so the entries that a refresh appends are refreshed in the same walk. It creates its list of changed nodes on the first changed node that has an `onChange`, and walks that list by index after the eager-node array is drained, so every `onChange` runs while no eager node of the batch is queued. No `onChange` writes or starts a batch, so no flush runs inside another, and the walk over the list queues nothing. Neither array is emptied, because a flush walks each once. A nested `startBatch` in a `body` allocates its own array, because without module state it cannot reach the outer batch's.
- `retainCount` counts holders. `retainDerivedNode` and a `releaseDerivedNode` that leaves the node needed write one field and allocate nothing, so there is no handle. A node is unneeded when it is not `Uncomputed`, its `retainCount` is 0, and its dependents list is empty. `disposeUnneeded` disposes such a node and walks its derived dependencies with an explicit stack, allocated on its first push, and disposes each one the disposal leaves unneeded, so a release of the head of a chain of 100,000 nodes adds no call-stack frame per node. It skips a node that is on the stack twice or that something read again. Disposal unlinks the node's own edges and marks nothing, because a disposed node has no dependents by definition.
- A run can leave a dependency unneeded in two places. `readDerived` repoints an edge whose previous dependency differs from the node it reads, and pushes the previous dependency onto `callingNode.disposalCandidates` when the repoint leaves it with no dependent and `retainCount` 0. `trim` unlinks the edges past the last read through `unlinkDropped`, which pushes each dependency an unlink leaves unneeded onto the same array. The run disposes none of them, and the call that started it takes them: `readDerived` moves the candidates of the node it pulled to its calling node, `verify` moves the candidates of each node it pulls or recomputes below `root` to `root`, `flush` collects the candidates of each eager node it refreshes and disposes them after the last refresh and before the first `onChange`, and `readDerivedNode` disposes the candidates of the node it pulled. So the check runs once per outermost `readDerivedNode` or flush, and a node that one run drops and a later run of the same call reads, whether that run's node read it before or not, is linked by then and is skipped. A first computation has no previous edges, so `computeFirst` neither repoints nor drops, and a run nested in it can still leave candidates on its node. `takeDisposalCandidates` hands the array over when the receiver has none, and otherwise appends the shorter array to the longer, so each candidate is copied at most log2 n times for n candidates. The common path pays one field load and one branch per `readDerived`, per pull in `verify`, and per refresh in `flush`. A `readDerivedNode` inside a computation is an outermost call of its own: it disposes what its runs drop when it returns, so a node that such a run drops and the calling computation reads later is computed from scratch, because `readDerivedNode` does not receive the calling node.
- `trim` calls `unlinkDropped` only when the node has a derived edge to drop. A call with an empty list, on every recompute of a node that reads only sources, slowed the write-then-read loop of `bench/single`'s read case from 4.45 ns to 6.39 ns, and the guard brought it to 4.0 ns.
- A branch that flips is disposed and recomputed: a toggle of a flag that picks between two chains of 100 lazy nodes cost 2,142 ns per toggle in the prototype of this design, against 11.3 ns when both chains stay computed, and 11.5 ns when the caller retains both heads. The README states that a caller retains both branches to keep both computed.
- While a computation runs, its node is `Uncomputed` or `Dirty` and is linked to what it has read so far. Every node it read is `Clean`, or is a `Dirty` or `Check` eager node whose previous value it read. A pull inside the computation recomputes only lazy nodes, which take no node out of `Clean`, so nothing takes a node the computation read out of `Clean` before the computation returns, and marking never reaches the running node while it is `Uncomputed`.
- `verify` walks the derived dependency lists in read order with an explicit stack of edges. It descends into a lazy `Check` dependency before its reader continues and pulls any other dependency, so a `Check` eager dependency returns its previous value. After a pull that leaves the current node `Dirty`, it recomputes that node and returns to the node's reader. It does not recompute `root`. It returns the status it leaves `root` in, `DIRTY` or `CLEAN`, and its caller recomputes a `Dirty` root: `pull` through `recompute`, and `refreshEager` through `recomputeEager`. The return value carries the status because TypeScript narrows `node.status` in the caller's `switch` and does not see `verify` change it. The order of pulls and recomputes is that of a recursive `verify`. It pushes an edge only when the dependency it descends into has more than one dependent, and it allocates the stack on the first push. A node with one dependent returns to its reader through its only dependents-list edge, which is the edge `verify` descended through. A node `verify` descended into has at least one dependent, and one with more than one has an edge on the stack. `verify` does not check either contract. A first computation still recurses through `readDerived` and `pull`.
- Because a write marks the source's direct readers `Dirty`, a `Check` node has no changed source, and `verify` pulls only its derived dependencies. A dependency's `recompute` marks its readers `Dirty` exactly when its new value is not `===` its previous value, so after a pull in `verify` the node is `Dirty` exactly when a lazy dependency it read changed value. A `Dirty` or `Check` eager dependency returns its previous value and leaves the node as it is, and its flush marks the node if its value changed.
- `status === UNCOMPUTED` alone records that a node has no value. An `Uncomputed` node's `value` is `undefined`, cast to `TValue` once in `newDerivedNode` and assigned in `disposeUnneeded`, where `value` is `unknown` and needs no cast. In the other three statuses, `value` holds a value the computation returned, which can be `undefined`, so a `Dirty` node always has a value for its next computation to receive as `previous`, and a pull of a `Clean` or `Check` node reads `node.value` directly.
- The `DerivedNode` doc comment lists internal invariants: the engine's state between computations, which the engine keeps and the README does not list. pico-js checks none of them at run time, so a bug that breaks one returns a wrong value or throws a `TypeError` instead of an error that names the invariant. The README's Contracts section lists the caller's obligations, and its Guarantees section lists what pico-js upholds while they hold. A comment in `graph.ts` that states a caller's obligation starts with `Contract:`, and one that states what the engine ensures starts with `Guarantee:` or `Internal invariant:`.
- The exported `SourceNode` and `DerivedNode` mark every field `readonly` except `onChange`, so a caller's write to any other field is a type error, and the type test checks it. `computation` is a method, which a type cannot mark `readonly`. It stays a method so that `DerivedNode<TValue>` is assignable to `DerivedNode<unknown>`. Inside `graph.ts`, `Mutable<T>` removes `readonly`, and the engine writes through `MutableSourceNode` and `MutableDerivedNode`. Each exported function and each method of `Mutators` casts its node arguments to the mutable view once. `Edge` holds the public node types, because the public edge-list fields expose edges, and an edge that held the mutable view would let a caller write a node's fields through `node.firstDependent.dependent`. So `markDirty`, `markCheck`, their queue copies, `markCheckDependentsDirty`, `verify`, `disposeUnneeded`, and `unlinkDropped` cast each node they reach through an edge. `linkEdge`, `unlinkEdge`, and `unlinkAll` write a node's dependents-list fields through the constraint `NodeWithDependents`, whose fields are mutable, and TypeScript lets a `readonly` field satisfy it. `Edge.dependent` is `readonly`. `Edge.dependency` is repointed when a recompute reads a different node at that position, and appends, `trim`, `linkEdge`, and `unlinkEdge` rewrite `nextDependency`, `previousDependent`, and `nextDependent`. `onChangeStatus` is not a field of the exported `DerivedNode`, because only the flush reads or writes it: `MutableDerivedNode` adds it to the mutable view, `newDerivedNode` builds each node at that type, and `index.ts` does not export `OnChangeStatus`. The type test checks that a read of `node.onChangeStatus` is a type error.

### Checks for a future debug build

A `__DEV__` build can restore these checks. Each throws `new Error(message)`.

- `markDirty`, `markCheck`, and `markCheckDependentsDirty`: a dependent's `status === UNCOMPUTED`; "An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js."
- `markCheckDependentsDirty`: a dependent's `status === CLEAN`; "A Clean node depends on a Dirty node. This is indicative of a bug in pico-js."
- `verify`: `node.firstDependent === undefined` for a node it descended into; "A node that verify descended into has no dependents. This is indicative of a bug in pico-js."
- `verify`: `verifyStack?.pop() === undefined` for a node it descended into with more than one dependent; "A node that verify descended into has more than one dependent and no edge on the verification stack. This is indicative of a bug in pico-js."
- `releaseDerivedNode`: `retainCount === 0` before the decrement; "A node was released more often than it was retained."
- `readDerivedNode`: a node with `retainCount === 0` and no dependent; "A node was read without being retained."
- `flush`: `node.onChange === undefined` for a node in its list of changed nodes; "An onChange was cleared during a flush."
- `pull`, `recompute`, and `recomputeEager`: a node whose `status !== UNCOMPUTED` holds no computed value, which needs a private `NOT_SET` sentinel in `value` while `Uncomputed`; "A node that is not Uncomputed has no value. This is indicative of a bug in pico-js."
- A first computation has no cutoff and marks nothing, because an `Uncomputed` node has no dependents.
- On cutoff the node keeps its stored value, so a recompute that returns `-0` where the stored value is `0` leaves `0` stored and returned, and every reader of the node reads the same value.
- Disposal discards the node's value and unlinks its own edges, so its next computation receives `previous` `undefined`. `refreshEager` disposes a queued eager node that is unneeded when the flush reaches it, because a release or an earlier refresh of the flush dropped it, instead of refreshing it, so a node that waits for disposal at the end of the flush does not run and calls no `onChange`, and a later read in the flush computes it again. An `Uncomputed` eager node that was retained again after a release is left `Uncomputed`, because computing it would link it to its sources again. A `Clean` eager node in `eagerNodes` is one that is in the list twice, or one that was disposed and computed again, and `refreshEager` leaves it as it is.

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
  EAGER,
  LAZY,
  readDerived,
  readDerivedNode,
  readSource,
  releaseDerivedNode,
  retainDerivedNode,
  setDerivedNodeEvaluation,
  startBatch,
  UNCOMPUTED,
  writeSource,
} from './graph';
export type {
  DependentsStatus,
  DerivedNode,
  DerivedNodeEvaluation,
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
  EAGER,
  LAZY,
  readDerived,
  readDerivedNode,
  readSource,
  releaseDerivedNode,
  retainDerivedNode,
  setDerivedNodeEvaluation,
  startBatch,
  UNCOMPUTED,
  writeSource,
  type DerivedNode,
  type DerivedNodeEvaluation,
  type SourceNode,
} from '../src/index';

function write<TValue>(source: SourceNode<TValue>, value: TValue): void {
  startBatch((mutators) => mutators.writeSource(source, value));
}

const retainedByRead = new WeakSet<DerivedNode<unknown>>();

/** Retains `node` on its first call, never releases it, and reads it. */
function read<TValue>(node: DerivedNode<TValue>): TValue {
  if (!retainedByRead.has(node)) {
    retainedByRead.add(node);
    retainDerivedNode(node);
  }
  return readDerivedNode(node);
}

function setEvaluation(
  node: DerivedNode<unknown>,
  evaluation: DerivedNodeEvaluation,
): void {
  startBatch((mutators) => mutators.setDerivedNodeEvaluation(node, evaluation));
}

const writers: ReadonlyArray<
  readonly [string, (source: SourceNode<number>, value: number) => void]
> = [
  ['writeSource', writeSource],
  ['mutators.writeSource', write],
];

describe('reads', () => {
  test('a derived node runs nothing until it is pulled, and a second pull of the Clean node runs nothing', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const double = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) * 2;
    });
    expect(runs).toBe(0);
    expect(double.status).toBe(UNCOMPUTED);
    expect(read(double)).toBe(2);
    expect(double.status).toBe(CLEAN);
    expect(read(double)).toBe(2);
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
    expect(read(outer)).toBe(10);
    write(source, 2);
    expect(outer.status).toBe(CHECK);
    expect(read(outer)).toBe(20);
  });

  test('the first computation receives undefined as previous and a recompute receives the previous value', () => {
    const previousValues: Array<number | undefined> = [];
    const a = createSourceNode(1);
    const double = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, a) * 2;
    });
    read(double);
    write(a, 2);
    read(double);
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
    expect(read(label)).toBe('big');
    write(a, 5);
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
    read(top);
    write(source, 2);
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
    read(bottom);
    const top = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (below) => {
        const node = createDerivedNode(
          (callingNode) => readDerived(callingNode, below) + 1,
        );
        read(node);
        return node;
      },
      bottom,
    );
    write(source, 1);
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
    read(label);
    write(a, 0);
    write(b, 3);
    expect(read(label)).toBe('big');
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
    const first = read(parity);
    read(reader);
    write(a, 3);
    expect(read(reader)).toBe(false);
    expect(reader.status).toBe(CLEAN);
    expect(read(parity)).toBe(first);
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
    expect(read(reader)).toBe(true);
    expect(node.status).toBe(CLEAN);
    expect(read(node)).toBeUndefined();
    expect(runs).toEqual({ node: 1, reader: 1 });
    write(a, 2);
    expect(node.status).toBe(DIRTY);
    expect(read(reader)).toBe(true);
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
    expect(read(d3)).toBe(4);
    write(a, 10);
    expect(read(d3)).toBe(4);
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
    expect(read(top)).toBe(10);
    write(source, 2);
    expect(read(top)).toBe(15);
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
    expect(read(node)).toBe(1);
    write(gateSource, 2);
    write(otherSource, 2);
    expect(read(node)).toBe(0);
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
    expect(read(foo)).toBe(1);
    write(bInput, 2);
    write(a, false);
    expect(foo.status).toBe(DIRTY);
    expect(read(foo)).toBe(0);
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
    expect(read(pick)).toBe(1);
    write(useA, false);
    expect(read(pick)).toBe(2);
    write(a, 10);
    expect(pick.status).toBe(CLEAN);
    expect(read(pick)).toBe(2);
    expect(runs).toBe(2);
  });

  test('a recompute that reads the same nodes stays marked by each of them', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
    );
    read(sum);
    write(a, 3);
    expect(read(sum)).toBe(5);
    write(b, 4);
    expect(sum.status).toBe(DIRTY);
    expect(read(sum)).toBe(7);
    write(a, 5);
    expect(sum.status).toBe(DIRTY);
    expect(read(sum)).toBe(9);
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
    read(label);
    write(a, 3);
    expect(hasDependent(b, sum)).toBe(true);
    write(b, 4);
    expect(sum.status).toBe(DIRTY);
    expect(label.status).toBe(CHECK);
    expect(read(label)).toBe('7');
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
    expect(read(node)).toBe(12);
    write(a, 2);
    expect(read(node)).toBe(14);
    write(b, 20);
    expect(read(node)).toBe(24);
    write(a, 3);
    write(b, 30);
    expect(read(node)).toBe(36);
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
    expect(read(node)).toBe(3);
    write(count, 1);
    expect(read(node)).toBe(0);
    write(third, 100);
    expect(node.status).toBe(CLEAN);
    write(count, 3);
    expect(read(node)).toBe(101);
    write(third, 200);
    expect(node.status).toBe(DIRTY);
    expect(read(node)).toBe(201);
    expect(runs).toBe(4);
  });

  test('a computation that reads a source twice recomputes once per change', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const twice = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, a);
    });
    expect(read(twice)).toBe(2);
    write(a, 2);
    expect(read(twice)).toBe(4);
    expect(runs).toBe(2);
  });
});

describe('batches and onChange', () => {
  test('a write calls nothing until the batch is flushed', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const double = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    read(double);
    double.onChange = () => calls.push('double');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect(calls).toEqual([]);
    });
    expect(calls).toEqual(['double']);
  });

  test('two writes in one batch call each onChange once, after both writes', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsB = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    read(readsA);
    read(readsB);
    readsA.onChange = () => calls.push('readsA');
    readsB.onChange = () => calls.push('readsB');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.writeSource(b, 2);
      expect(calls).toEqual([]);
    });
    expect(calls).toEqual(['readsA', 'readsB']);
  });

  test('an onChange that reads its node sees every write of the batch', () => {
    const seen: Array<number> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const sum = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
    );
    read(sum);
    sum.onChange = () => seen.push(readDerivedNode(sum));
    startBatch((mutators) => {
      mutators.writeSource(a, 10);
      mutators.writeSource(b, 20);
    });
    expect(seen).toEqual([30]);
  });

  test('two nodes that share one callback call it once per node', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const left = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const right = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    read(left);
    read(right);
    const callback = () => {
      calls += 1;
    };
    left.onChange = callback;
    right.onChange = callback;
    write(a, 2);
    expect(calls).toBe(2);
  });

  test('a batch that writes nothing calls nothing', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const node = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    read(node);
    node.onChange = () => {
      calls += 1;
    };
    startBatch(() => {});
    expect(calls).toBe(0);
    expect(node.status).toBe(CLEAN);
  });

  test('a node that changes in each of two batches calls onChange once per batch, and an eager node without onChange calls nothing', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const node = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const silent = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    read(node);
    read(silent);
    node.onChange = () => {
      calls += 1;
    };
    write(a, 2);
    startBatch((mutators) => {
      mutators.writeSource(a, 3);
      mutators.writeSource(a, 4);
    });
    expect(calls).toBe(2);
    expect([read(node), read(silent)]).toEqual([4, 5]);
  });
});

describe('eager nodes', () => {
  test('an eager node whose recompute returns the previous value leaves its readers Clean, and no onChange runs', () => {
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
    read(reader);
    isOne.onChange = () => {
      calls += 1;
    };
    write(a, 3);
    write(a, 4);
    expect(isOne.status).toBe(CLEAN);
    expect(reader.status).toBe(CLEAN);
    expect(calls).toBe(0);
    expect(read(reader)).toBe('other');
    expect(runs).toEqual({ isOne: 3, reader: 1 });
  });

  test('a changed eager value marks its lazy readers Dirty, and onChange runs after every queued eager node is refreshed', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const eagerA = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const eagerB = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const readsEager = createDerivedNode(
      (callingNode) => readDerived(callingNode, eagerB) + 1,
    );
    read(eagerA);
    read(readsEager);
    eagerA.onChange = () =>
      calls.push(`eagerA, eagerB ${eagerB.status} ${eagerB.value}`);
    eagerB.onChange = () => calls.push('eagerB');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.writeSource(b, 2);
      expect(readsEager.status).toBe(CLEAN);
    });
    expect(calls).toEqual([`eagerA, eagerB ${CLEAN} 2`, 'eagerB']);
    expect(readsEager.status).toBe(DIRTY);
    expect(read(readsEager)).toBe(3);
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
    read(reader);
    write(a, 2);
    expect([first.status, second.status, third.status]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(runs).toEqual({ first: 2, second: 2, third: 2 });
    expect(reader.status).toBe(DIRTY);
    expect(read(reader)).toBe(50);
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
    read(reader);
    eager.onChange = () => {
      calls += 1;
    };
    startBatch((mutators) => {
      mutators.writeSource(a, 3);
      expect(inner.status).toBe(DIRTY);
      expect(eager.status).toBe(CHECK);
      expect(reader.status).toBe(CLEAN);
    });
    expect(runs).toEqual({ inner: 2, eager: 1 });
    expect(calls).toBe(0);
    write(a, 4);
    expect(runs).toEqual({ inner: 3, eager: 2 });
    expect(calls).toBe(1);
    expect(read(reader)).toBe(true);
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
    read(reader);
    startBatch((mutators) => {
      mutators.writeSource(a, 5);
      expect(read(eager)).toBe(2);
      expect(read(reader)).toBe(3);
      expect(read(late)).toBe(102);
      expect(eager.status).toBe(DIRTY);
    });
    expect(read(eager)).toBe(10);
    expect(read(reader)).toBe(11);
    expect(read(late)).toBe(110);
  });

  test('several writes in one batch recompute an eager node once', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) + readSource(callingNode, b);
    });
    read(eager);
    startBatch((mutators) => {
      mutators.writeSource(a, 10);
      mutators.writeSource(b, 20);
      mutators.writeSource(a, 30);
    });
    expect(runs).toBe(2);
    expect(read(eager)).toBe(50);
  });
});

describe('nested batches', () => {
  test('a startBatch inside the body of another startBatch flushes its own batch before it returns', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const readsA = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const readsB = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    read(readsA);
    read(readsB);
    readsA.onChange = () => calls.push('readsA');
    readsB.onChange = () => calls.push('readsB');
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
    read(eager);
    eager.onChange = () => calls.push(`eager ${readDerivedNode(eager)}`);
    startBatch((outer) => {
      outer.writeSource(a, 2);
      startBatch((inner) => {
        inner.writeSource(b, 2);
      });
      expect(eagerRuns).toBe(1);
      expect(calls).toEqual([]);
    });
    expect(eagerRuns).toBe(2);
    expect(calls).toEqual(['eager 4']);
  });
});

describe('single calls', () => {
  type Graph = {
    readonly a: SourceNode<number>;
    readonly calls: Array<string>;
    readonly inner: DerivedNode<number>;
    readonly eager: DerivedNode<number>;
    readonly reader: DerivedNode<number>;
  };

  function build(): Graph {
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
    read(reader);
    eager.onChange = () =>
      calls.push(`eager ${readDerivedNode(eager)} ${inner.status}`);
    return { a, calls, inner, eager, reader };
  }

  function statuses(graph: Graph): ReadonlyArray<number> {
    return [graph.inner.status, graph.eager.status, graph.reader.status];
  }

  test('writeSource calls the same onChange and leaves the same statuses and values as a batch of one write', () => {
    const single = build();
    const batched = build();
    writeSource(single.a, 2);
    startBatch((mutators) => mutators.writeSource(batched.a, 2));
    expect(single.calls).toEqual([`eager 4 ${CLEAN}`]);
    expect(batched.calls).toEqual(single.calls);
    expect(statuses(single)).toEqual([CLEAN, CLEAN, DIRTY]);
    expect(statuses(batched)).toEqual(statuses(single));
    expect(
      [single.inner, single.eager, single.reader].map((node) => read(node)),
    ).toEqual([2, 4, 5]);
    expect(
      [batched.inner, batched.eager, batched.reader].map((node) => read(node)),
    ).toEqual([2, 4, 5]);
  });

  test('setDerivedNodeEvaluation calls the same onChange and leaves the same statuses and values as a batch of one evaluation change', () => {
    const single = build();
    const batched = build();
    write(single.a, 2);
    write(batched.a, 2);
    single.calls.length = 0;
    batched.calls.length = 0;
    setDerivedNodeEvaluation(single.eager, LAZY);
    write(single.a, 3);
    setDerivedNodeEvaluation(single.eager, EAGER);
    startBatch((mutators) =>
      mutators.setDerivedNodeEvaluation(batched.eager, LAZY),
    );
    write(batched.a, 3);
    startBatch((mutators) =>
      mutators.setDerivedNodeEvaluation(batched.eager, EAGER),
    );
    expect(single.calls).toEqual([`eager 6 ${CLEAN}`]);
    expect(batched.calls).toEqual(single.calls);
    expect(statuses(single)).toEqual([CLEAN, CLEAN, DIRTY]);
    expect(statuses(batched)).toEqual(statuses(single));
    expect(read(single.reader)).toBe(7);
    expect(read(batched.reader)).toBe(7);
  });

  test('a single-call write whose marking reaches only lazy nodes runs nothing and calls nothing, and a later write queues and refreshes an eager reader', () => {
    let runs = 0;
    const calls: Array<number> = [];
    const a = createSourceNode(1);
    const lazy = createDerivedNode((callingNode) => readSource(callingNode, a));
    const above = createDerivedNode((callingNode) =>
      readDerived(callingNode, lazy),
    );
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readDerived(callingNode, above) * 10;
    });
    read(above);
    writeSource(a, 2);
    expect([lazy.status, above.status]).toEqual([DIRTY, CHECK]);
    expect(read(above)).toBe(2);
    read(eager);
    eager.onChange = () => calls.push(readDerivedNode(eager));
    writeSource(a, 3);
    expect(runs).toBe(2);
    expect(calls).toEqual([30]);
  });
});

describe('setDerivedNodeEvaluation', () => {
  test('setting the evaluation a node has, or the evaluation of a Clean or Uncomputed node, runs nothing and calls nothing', () => {
    let runs = 0;
    let calls = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    node.onChange = () => {
      calls += 1;
    };
    setEvaluation(node, EAGER);
    expect(node.evaluation).toBe(EAGER);
    expect(node.status).toBe(UNCOMPUTED);
    read(node);
    setEvaluation(node, EAGER);
    setEvaluation(node, LAZY);
    setEvaluation(node, LAZY);
    expect(node.evaluation).toBe(LAZY);
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
    expect(read(top)).toBe(11);
    setEvaluation(n, LAZY);
    write(s, 2);
    expect(n.status).toBe(DIRTY);
    expect(top.status).toBe(CHECK);
    setEvaluation(n, EAGER);
    expect(n.status).toBe(CLEAN);
    expect(read(n)).toBe(20);
    expect(read(top)).toBe(21);
    write(s, 3);
    expect(n.status).toBe(CLEAN);
    expect(read(n)).toBe(30);
    expect(read(top)).toBe(31);
  });

  test('a lazy node set eager while Dirty is recomputed in the flush, and a changed value marks its readers and calls its onChange', () => {
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
    read(reader);
    write(a, -1);
    node.onChange = () => calls.push('node');
    startBatch((mutators) => {
      mutators.setDerivedNodeEvaluation(node, EAGER);
      expect(node.status).toBe(DIRTY);
      expect(runs).toBe(1);
    });
    expect(runs).toBe(2);
    expect(node.status).toBe(CLEAN);
    expect(reader.status).toBe(DIRTY);
    expect(calls).toEqual(['node']);
    expect(read(reader)).toBe('not positive');
    write(a, -2);
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
    read(node);
    write(a, 3);
    expect(node.status).toBe(CHECK);
    setEvaluation(node, EAGER);
    expect(runs).toEqual({ inner: 2, node: 1 });
    expect(node.status).toBe(CLEAN);
    expect(read(node)).toBe(10);
  });

  test('an eager node set lazy while queued marks its readers Check, calls no onChange, and is not recomputed by the flush', () => {
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
    read(outer);
    eager.onChange = () => calls.push('eager');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CLEAN,
        CLEAN,
      ]);
      mutators.setDerivedNodeEvaluation(eager, LAZY);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CHECK,
        CHECK,
      ]);
    });
    expect(calls).toEqual([]);
    expect(runs).toBe(1);
    expect(eager.status).toBe(DIRTY);
    expect(read(outer)).toBe(6);
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
    read(reader);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.setDerivedNodeEvaluation(eager, LAZY);
      expect(read(reader)).toBe(5);
      expect([eager.status, reader.status]).toEqual([CLEAN, CLEAN]);
    });
    expect(read(reader)).toBe(5);
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
    read(reader);
    eager.onChange = () => calls.push('eager');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.setDerivedNodeEvaluation(eager, LAZY);
      mutators.setDerivedNodeEvaluation(eager, EAGER);
    });
    expect(runs).toBe(2);
    expect(eager.status).toBe(CLEAN);
    expect(calls).toEqual(['eager']);
    expect(read(reader)).toBe(2);
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
    read(reader);
    node.onChange = () => calls.push(`node ${readDerivedNode(node)}`);

    startBatch((mutators) => {
      mutators.setDerivedNodeEvaluation(node, LAZY);
      mutators.writeSource(a, 2);
    });
    expect([node.evaluation, node.status, reader.status]).toEqual([
      LAZY,
      DIRTY,
      CHECK,
    ]);
    expect(calls).toEqual([]);

    startBatch((mutators) => mutators.setDerivedNodeEvaluation(node, EAGER));
    expect([node.status, reader.status, runs]).toEqual([CLEAN, DIRTY, 2]);
    expect(read(reader)).toBe(102);

    startBatch((mutators) => mutators.writeSource(a, 5));
    expect([node.status, reader.status, runs]).toEqual([CLEAN, CLEAN, 3]);
    expect(calls).toEqual(['node 2']);

    startBatch((mutators) => {
      mutators.writeSource(a, 6);
      mutators.setDerivedNodeEvaluation(node, LAZY);
    });
    expect([node.evaluation, node.status, reader.status, runs]).toEqual([
      LAZY,
      DIRTY,
      CHECK,
      3,
    ]);
    expect(calls).toEqual(['node 2']);
    expect(read(reader)).toBe(100);

    startBatch((mutators) => {
      mutators.setDerivedNodeEvaluation(node, EAGER);
      mutators.writeSource(a, 7);
    });
    expect([node.status, reader.status, runs]).toEqual([CLEAN, DIRTY, 5]);
    expect(calls).toEqual(['node 2', 'node 1']);
    expect(read(reader)).toBe(101);
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

describe('verification of chains', () => {
  test('a chain of three derived nodes returns the new value after a write, and each node runs once', () => {
    const runs = { bottom: 0, middle: 0, top: 0 };
    const source = createSourceNode(1);
    const bottom = createDerivedNode((callingNode) => {
      runs.bottom += 1;
      return readSource(callingNode, source);
    });
    const middle = createDerivedNode((callingNode) => {
      runs.middle += 1;
      return readDerived(callingNode, bottom) + 1;
    });
    const top = createDerivedNode((callingNode) => {
      runs.top += 1;
      return readDerived(callingNode, middle) + 1;
    });
    expect(read(top)).toBe(3);
    writeSource(source, 2);
    expect([bottom.status, middle.status, top.status]).toEqual([
      DIRTY,
      CHECK,
      CHECK,
    ]);
    expect(read(top)).toBe(4);
    expect([bottom.status, middle.status, top.status]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(runs).toEqual({ bottom: 2, middle: 2, top: 2 });
  });

  test('a pull of the top of a chain of 100,000 Check nodes recomputes every node without overflowing the call stack', () => {
    const source = createSourceNode(0);
    const bottom = createDerivedNode((callingNode) =>
      readSource(callingNode, source),
    );
    read(bottom);
    const top = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (below) => {
        const node = createDerivedNode(
          (callingNode) => readDerived(callingNode, below) + 1,
        );
        read(node);
        return node;
      },
      bottom,
    );
    expect(read(top)).toBe(99_999);
    writeSource(source, 1);
    expect(top.status).toBe(CHECK);
    expect(read(top)).toBe(100_000);
    expect(top.status).toBe(CLEAN);
  });

  test('a pull of the top of a chain of 100,000 Check nodes above a node that cuts off runs none of them and leaves them Clean', () => {
    let runs = 0;
    const source = createSourceNode(0);
    const bottom = createDerivedNode((callingNode) => {
      readSource(callingNode, source);
      return 0;
    });
    read(bottom);
    const nodes: Array<DerivedNode<number>> = [];
    const top = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (below) => {
        const node = createDerivedNode((callingNode) => {
          runs += 1;
          return readDerived(callingNode, below) + 1;
        });
        read(node);
        nodes.push(node);
        return node;
      },
      bottom,
    );
    expect(runs).toBe(99_999);
    writeSource(source, 1);
    expect(top.status).toBe(CHECK);
    expect(read(top)).toBe(99_999);
    expect(runs).toBe(99_999);
    expect(nodes.every((node) => node.status === CLEAN)).toBe(true);
  });

  test('a Check node whose derived dependencies all come back unchanged becomes Clean after every one of them was pulled', () => {
    const runs = { first: 0, second: 0, node: 0 };
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const first = createDerivedNode((callingNode) => {
      runs.first += 1;
      return readSource(callingNode, a) > 0;
    });
    const second = createDerivedNode((callingNode) => {
      runs.second += 1;
      return readSource(callingNode, b) > 0;
    });
    const node = createDerivedNode((callingNode) => {
      runs.node += 1;
      return (
        readDerived(callingNode, first) && readDerived(callingNode, second)
      );
    });
    expect(read(node)).toBe(true);
    writeSource(a, 2);
    writeSource(b, 2);
    expect(node.status).toBe(CHECK);
    expect(read(node)).toBe(true);
    expect([first.status, second.status, node.status]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
    expect(runs).toEqual({ first: 2, second: 2, node: 1 });
  });
});

describe('verification of diamonds below a Check node', () => {
  type Diamond = {
    readonly source: SourceNode<number>;
    readonly runs: {
      readonly bottom: number;
      readonly shared: number;
      readonly left: number;
      readonly right: number;
      readonly top: number;
    };
    readonly bottom: DerivedNode<number>;
    readonly shared: DerivedNode<number>;
    readonly left: DerivedNode<number>;
    readonly right: DerivedNode<number>;
    readonly top: DerivedNode<number>;
  };

  // source -> bottom -> shared -> left and right -> top. After a write,
  // shared is a Check node with two dependents.
  function build(bottomValue: (sourceValue: number) => number): Diamond {
    const runs = { bottom: 0, shared: 0, left: 0, right: 0, top: 0 };
    const source = createSourceNode(1);
    const bottom = createDerivedNode((callingNode) => {
      runs.bottom += 1;
      return bottomValue(readSource(callingNode, source));
    });
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readDerived(callingNode, bottom) + 1;
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
    expect(read(top)).toBe(10);
    return { source, runs, bottom, shared, left, right, top };
  }

  test('a pull of the top runs each node once when the value changed', () => {
    const graph = build((value) => value);
    writeSource(graph.source, 2);
    expect([
      graph.bottom.status,
      graph.shared.status,
      graph.left.status,
      graph.right.status,
      graph.top.status,
    ]).toEqual([DIRTY, CHECK, CHECK, CHECK, CHECK]);
    expect(read(graph.top)).toBe(15);
    expect(graph.runs).toEqual({
      bottom: 2,
      shared: 2,
      left: 2,
      right: 2,
      top: 2,
    });
  });

  test('a pull of the top runs only the bottom node when the bottom node cuts off, and leaves every node Clean', () => {
    const graph = build(() => 1);
    writeSource(graph.source, 2);
    expect(read(graph.top)).toBe(10);
    expect(graph.runs).toEqual({
      bottom: 2,
      shared: 1,
      left: 1,
      right: 1,
      top: 1,
    });
    expect([
      graph.bottom.status,
      graph.shared.status,
      graph.left.status,
      graph.right.status,
      graph.top.status,
    ]).toEqual([CLEAN, CLEAN, CLEAN, CLEAN, CLEAN]);
  });

  test('a pull of the node that read the shared node last runs each node once, and a later pull of the top runs the rest', () => {
    const graph = build((value) => value);
    writeSource(graph.source, 2);
    expect(read(graph.right)).toBe(9);
    expect(graph.runs).toEqual({
      bottom: 2,
      shared: 2,
      left: 1,
      right: 2,
      top: 1,
    });
    expect([graph.left.status, graph.top.status]).toEqual([DIRTY, DIRTY]);
    expect(read(graph.top)).toBe(15);
    expect(graph.runs).toEqual({
      bottom: 2,
      shared: 2,
      left: 2,
      right: 2,
      top: 2,
    });
  });

  test('a pull of the node that read the shared node last leaves the other reader Check when the bottom node cuts off', () => {
    const graph = build(() => 1);
    writeSource(graph.source, 2);
    expect(read(graph.right)).toBe(6);
    expect([
      graph.shared.status,
      graph.left.status,
      graph.right.status,
      graph.top.status,
    ]).toEqual([CLEAN, CHECK, CLEAN, CHECK]);
    expect(read(graph.top)).toBe(10);
    expect(graph.runs).toEqual({
      bottom: 2,
      shared: 1,
      left: 1,
      right: 1,
      top: 1,
    });
  });

  for (const order of ['cutoff node first', 'changed node first'] as const) {
    test(`a node that reads a derived node directly and through a node that cuts off recomputes once per write, ${order}`, () => {
      const runs = { base: 0, parity: 0, top: 0 };
      const source = createSourceNode(1);
      const base = createDerivedNode((callingNode) => {
        runs.base += 1;
        return readSource(callingNode, source);
      });
      const parity = createDerivedNode((callingNode) => {
        runs.parity += 1;
        return readDerived(callingNode, base) % 2;
      });
      const top = createDerivedNode((callingNode) => {
        runs.top += 1;
        if (order === 'cutoff node first') {
          const parityValue = readDerived(callingNode, parity);
          return `${parityValue}:${readDerived(callingNode, base)}`;
        }
        const baseValue = readDerived(callingNode, base);
        return `${readDerived(callingNode, parity)}:${baseValue}`;
      });
      expect(read(top)).toBe('1:1');
      writeSource(source, 3);
      expect([base.status, parity.status, top.status]).toEqual([
        DIRTY,
        CHECK,
        CHECK,
      ]);
      expect(read(top)).toBe('1:3');
      expect(runs).toEqual({ base: 2, parity: 2, top: 2 });
      expect([base.status, parity.status, top.status]).toEqual([
        CLEAN,
        CLEAN,
        CLEAN,
      ]);
    });
  }
});

describe('marking breadth', () => {
  for (const [name, writeWith] of writers) {
    test(`${name} marks the readers of three direct readers Check, and their readers Check`, () => {
      const a = createSourceNode(1);
      const direct = [0, 1, 2].map((offset) =>
        createDerivedNode((callingNode) => readSource(callingNode, a) + offset),
      );
      const middle = direct.map((below) =>
        createDerivedNode((callingNode) => readDerived(callingNode, below)),
      );
      const top = middle.map((below) =>
        createDerivedNode((callingNode) => readDerived(callingNode, below)),
      );
      top.forEach((node) => read(node));
      writeWith(a, 2);
      expect(direct.map((node) => node.status)).toEqual([DIRTY, DIRTY, DIRTY]);
      expect(middle.map((node) => node.status)).toEqual([CHECK, CHECK, CHECK]);
      expect(top.map((node) => node.status)).toEqual([CHECK, CHECK, CHECK]);
      expect(top.map((node) => read(node))).toEqual([2, 3, 4]);
    });

    test(`${name} marks every node above a node with three readers that each have readers`, () => {
      const a = createSourceNode(1);
      const base = createDerivedNode((callingNode) =>
        readSource(callingNode, a),
      );
      const first = [0, 1, 2].map((offset) =>
        createDerivedNode(
          (callingNode) => readDerived(callingNode, base) + offset,
        ),
      );
      const second = first.map((below) =>
        createDerivedNode((callingNode) => readDerived(callingNode, below)),
      );
      const third = second.map((below) =>
        createDerivedNode((callingNode) => readDerived(callingNode, below)),
      );
      third.forEach((node) => read(node));
      writeWith(a, 2);
      expect(base.status).toBe(DIRTY);
      expect(
        [...first, ...second, ...third].map((node) => node.status),
      ).toEqual([
        CHECK,
        CHECK,
        CHECK,
        CHECK,
        CHECK,
        CHECK,
        CHECK,
        CHECK,
        CHECK,
      ]);
      expect(third.map((node) => read(node))).toEqual([2, 3, 4]);
    });
  }

  test('writeSource marks a Check direct reader Dirty, and the reader recomputes once', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(10);
    const inner = createDerivedNode(
      (callingNode) => readSource(callingNode, a) > 0,
    );
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return readDerived(callingNode, inner) ? readSource(callingNode, b) : 0;
    });
    read(node);
    writeSource(a, 2);
    expect(node.status).toBe(CHECK);
    writeSource(b, 20);
    expect(node.status).toBe(DIRTY);
    expect(read(node)).toBe(20);
    expect(runs).toBe(2);
  });

  test('writeSource leaves a Check node and the Check node above it Check when a second write reaches them', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const left = createDerivedNode((callingNode) => readSource(callingNode, a));
    const right = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const top = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, left) + readDerived(callingNode, right),
    );
    const above = createDerivedNode((callingNode) =>
      readDerived(callingNode, top),
    );
    read(above);
    writeSource(a, 2);
    expect([top.status, above.status]).toEqual([CHECK, CHECK]);
    writeSource(b, 2);
    expect([top.status, above.status]).toEqual([CHECK, CHECK]);
    expect(read(above)).toBe(4);
  });

  test('writeSource queues an eager direct reader and an eager reader of a reader, and marks nothing above them', () => {
    const runs = { direct: 0, indirect: 0 };
    const a = createSourceNode(1);
    const direct = createEagerDerivedNode((callingNode) => {
      runs.direct += 1;
      return readSource(callingNode, a) > 0;
    });
    const aboveDirect = createDerivedNode((callingNode) =>
      readDerived(callingNode, direct),
    );
    const lazy = createDerivedNode(
      (callingNode) => readSource(callingNode, a) > 0,
    );
    const indirect = createEagerDerivedNode((callingNode) => {
      runs.indirect += 1;
      return readDerived(callingNode, lazy);
    });
    const aboveIndirect = createDerivedNode((callingNode) =>
      readDerived(callingNode, indirect),
    );
    read(aboveDirect);
    read(aboveIndirect);
    let calls = 0;
    direct.onChange = () => {
      calls += 1;
    };
    indirect.onChange = () => {
      calls += 1;
    };
    writeSource(a, 2);
    expect(runs).toEqual({ direct: 2, indirect: 1 });
    expect([
      direct.status,
      indirect.status,
      aboveDirect.status,
      aboveIndirect.status,
    ]).toEqual([CLEAN, CLEAN, CLEAN, CLEAN]);
    expect(calls).toBe(0);
  });

  test('writeSource refreshes two eager direct readers, and two eager readers of a lazy reader, in the order they left Clean', () => {
    const refreshed: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const direct = ['direct 0', 'direct 1'].map((name) =>
      createEagerDerivedNode((callingNode) => {
        refreshed.push(name);
        return readSource(callingNode, a);
      }),
    );
    const lazy = createDerivedNode((callingNode) => readSource(callingNode, b));
    const aboveLazy = ['above 0', 'above 1'].map((name) =>
      createEagerDerivedNode((callingNode) => {
        refreshed.push(name);
        return readDerived(callingNode, lazy);
      }),
    );
    [...direct, ...aboveLazy].forEach((node) => read(node));
    refreshed.length = 0;
    writeSource(a, 2);
    expect(refreshed).toEqual(['direct 0', 'direct 1']);
    writeSource(b, 3);
    expect(refreshed).toEqual(['direct 0', 'direct 1', 'above 0', 'above 1']);
    expect([...direct, ...aboveLazy].map((node) => node.value)).toEqual([
      2, 2, 3, 3,
    ]);
  });

  test('writeSource marks a chain of four nodes, the first Dirty and the rest Check', () => {
    const a = createSourceNode(1);
    const first = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const second = createDerivedNode((callingNode) =>
      readDerived(callingNode, first),
    );
    const third = createDerivedNode((callingNode) =>
      readDerived(callingNode, second),
    );
    const fourth = createDerivedNode((callingNode) =>
      readDerived(callingNode, third),
    );
    read(fourth);
    writeSource(a, 2);
    expect([first.status, second.status, third.status, fourth.status]).toEqual([
      DIRTY,
      CHECK,
      CHECK,
      CHECK,
    ]);
  });
});

describe('writes', () => {
  test('a write of the value the source holds marks its reader Dirty, the reader recomputes once, and its reader does not run', () => {
    const runs = { reader: 0, outer: 0 };
    const a = createSourceNode(1);
    const reader = createDerivedNode((callingNode) => {
      runs.reader += 1;
      return readSource(callingNode, a);
    });
    const outer = createDerivedNode((callingNode) => {
      runs.outer += 1;
      return readDerived(callingNode, reader) + 1;
    });
    read(outer);
    writeSource(a, 1);
    expect([reader.status, outer.status]).toEqual([DIRTY, CHECK]);
    expect(read(outer)).toBe(2);
    expect(runs).toEqual({ reader: 2, outer: 1 });
  });

  test('dependentsStatus is 1 for a new source, 0 after a read, and 1 after a write, and a write after a new reader read the source marks that reader', () => {
    const a = createSourceNode(1);
    expect(a.dependentsStatus).toBe(1);
    writeSource(a, 2);
    expect(a.dependentsStatus).toBe(1);
    const first = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    expect(read(first)).toBe(2);
    expect(a.dependentsStatus).toBe(0);
    writeSource(a, 3);
    expect(a.dependentsStatus).toBe(1);
    expect(first.status).toBe(DIRTY);
    const second = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    expect(read(second)).toBe(3);
    expect(a.dependentsStatus).toBe(0);
    startBatch((mutators) => mutators.writeSource(a, 4));
    expect(a.dependentsStatus).toBe(1);
    expect([first.status, second.status]).toEqual([DIRTY, DIRTY]);
    expect([read(first), read(second)]).toEqual([4, 4]);
  });
  for (const [name, writeWith] of writers) {
    test(`${name} that reaches a Dirty reader leaves it Dirty, marks the other reader, and the Dirty reader recomputes once`, () => {
      const runs = { both: 0, onlyB: 0 };
      const a = createSourceNode(1);
      const b = createSourceNode(10);
      const both = createDerivedNode((callingNode) => {
        runs.both += 1;
        return readSource(callingNode, a) + readSource(callingNode, b);
      });
      const onlyB = createDerivedNode((callingNode) => {
        runs.onlyB += 1;
        return readSource(callingNode, b) * 2;
      });
      read(both);
      read(onlyB);
      writeWith(a, 2);
      expect([both.status, onlyB.status]).toEqual([DIRTY, CLEAN]);
      writeWith(b, 20);
      expect([both.status, onlyB.status]).toEqual([DIRTY, DIRTY]);
      expect([read(both), read(onlyB)]).toEqual([22, 40]);
      expect(runs).toEqual({ both: 2, onlyB: 2 });
    });
  }
});

describe('values', () => {
  test('a recompute that returns -0 over a stored 0 cuts off and keeps 0', () => {
    let readerRuns = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 0,
    );
    const reader = createDerivedNode((callingNode) => {
      readerRuns += 1;
      return readDerived(callingNode, node);
    });
    expect(Object.is(read(reader), 0)).toBe(true);
    writeSource(a, -1);
    expect(Object.is(read(node), 0)).toBe(true);
    expect(Object.is(node.value, 0)).toBe(true);
    expect(Object.is(read(reader), 0)).toBe(true);
    expect(readerRuns).toBe(1);
  });
});

describe('dependency lists', () => {
  test('a computation that reads one source and one derived node 100 times in a row holds one edge to each', () => {
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const node = createDerivedNode((callingNode) => {
      let sum = 0;
      for (let index = 0; index < 100; index += 1) {
        sum += readSource(callingNode, a);
      }
      for (let index = 0; index < 100; index += 1) {
        sum += readDerived(callingNode, inner);
      }
      return sum;
    });
    expect(read(node)).toBe(200);
    expectSame(sourceDependenciesOf(node), [a]);
    expectSame(derivedDependenciesOf(node), [inner]);
    expectSame(dependentsOf(a), [node, inner]);
    expectSame(dependentsOf(inner), [node]);
    writeSource(a, 2);
    expect(read(node)).toBe(400);
    expectSame(sourceDependenciesOf(node), [a]);
    expectSame(derivedDependenciesOf(node), [inner]);
    expectSame(dependentsOf(a), [node, inner]);
    expectSame(dependentsOf(inner), [node]);
  });

  test('a recompute that reads the same sources and derived nodes in the same order keeps the same edge objects', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(2);
    const left = createDerivedNode((callingNode) => readSource(callingNode, a));
    const right = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const node = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, a) +
        readSource(callingNode, b) +
        readDerived(callingNode, left) +
        readDerived(callingNode, right),
    );
    read(node);
    const firstSourceEdge = node.firstSourceDependency;
    const lastSourceEdge = node.lastSourceDependency;
    const firstDerivedEdge = node.firstDerivedDependency;
    const lastDerivedEdge = node.lastDerivedDependency;
    writeSource(a, 3);
    expect(read(node)).toBe(10);
    expect(node.firstSourceDependency === firstSourceEdge).toBe(true);
    expect(node.lastSourceDependency === lastSourceEdge).toBe(true);
    expect(node.firstDerivedDependency === firstDerivedEdge).toBe(true);
    expect(node.lastDerivedDependency === lastDerivedEdge).toBe(true);
    expectSame(sourceDependenciesOf(node), [a, b]);
    expectSame(derivedDependenciesOf(node), [left, right]);
    expectSame(dependentsOf(a), [node, left]);
    expectSame(dependentsOf(b), [node, right]);
  });

  test('a recompute that reads a different derived node at the same position moves the edge, and the node it stopped reading does not mark it', () => {
    let runs = 0;
    const useLeft = createSourceNode(true);
    const leftInput = createSourceNode(1);
    const rightInput = createSourceNode(10);
    const left = createDerivedNode((callingNode) =>
      readSource(callingNode, leftInput),
    );
    const right = createDerivedNode((callingNode) =>
      readSource(callingNode, rightInput),
    );
    const pick = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, useLeft)
        ? readDerived(callingNode, left)
        : readDerived(callingNode, right);
    });
    expect(read(pick)).toBe(1);
    const edge = pick.firstDerivedDependency;
    writeSource(useLeft, false);
    expect(read(pick)).toBe(10);
    expect(pick.firstDerivedDependency === edge).toBe(true);
    expectSame(derivedDependenciesOf(pick), [right]);
    expectSame(dependentsOf(left), []);
    expectSame(dependentsOf(right), [pick]);
    writeSource(leftInput, 2);
    expect(pick.status).toBe(CLEAN);
    writeSource(rightInput, 20);
    expect(pick.status).toBe(CHECK);
    expect(read(pick)).toBe(20);
    expect(runs).toBe(3);
  });

  test('a recompute that reads fewer, none, and then more derived nodes is marked by exactly the derived nodes it read last', () => {
    let runs = 0;
    const count = createSourceNode(3);
    const inputs = [
      createSourceNode(1),
      createSourceNode(2),
      createSourceNode(4),
    ];
    const derived = inputs.map((input) =>
      createDerivedNode((callingNode) => readSource(callingNode, input)),
    );
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return derived
        .slice(0, readSource(callingNode, count))
        .reduce((sum, below) => sum + readDerived(callingNode, below), 0);
    });
    expect(read(node)).toBe(7);
    expectSame(derivedDependenciesOf(node), derived);
    writeSource(count, 1);
    expect(read(node)).toBe(1);
    expectSame(derivedDependenciesOf(node), [derived[0]]);
    expectSame(dependentsOf(derived[1]!), []);
    expectSame(dependentsOf(derived[2]!), []);
    writeSource(inputs[2]!, 8);
    expect(node.status).toBe(CLEAN);
    writeSource(count, 0);
    expect(read(node)).toBe(0);
    expectSame(derivedDependenciesOf(node), []);
    expect(node.lastDerivedDependency === undefined).toBe(true);
    expectSame(dependentsOf(derived[0]!), []);
    writeSource(inputs[0]!, 16);
    expect(node.status).toBe(CLEAN);
    writeSource(count, 3);
    expect(read(node)).toBe(26);
    expectSame(derivedDependenciesOf(node), derived);
    writeSource(inputs[2]!, 32);
    expect(node.status).toBe(CHECK);
    expect(read(node)).toBe(50);
    expect(runs).toBe(5);
  });

  test('a recompute that reads fewer, none, and then more sources is marked by exactly the sources it read last', () => {
    let runs = 0;
    const countInput = createSourceNode(3);
    const count = createDerivedNode((callingNode) =>
      readSource(callingNode, countInput),
    );
    const sources = [
      createSourceNode(1),
      createSourceNode(2),
      createSourceNode(4),
    ];
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return sources
        .slice(0, readDerived(callingNode, count))
        .reduce((sum, source) => sum + readSource(callingNode, source), 0);
    });
    expect(read(node)).toBe(7);
    expectSame(sourceDependenciesOf(node), sources);
    writeSource(countInput, 1);
    expect(read(node)).toBe(1);
    expectSame(sourceDependenciesOf(node), [sources[0]]);
    expectSame(dependentsOf(sources[1]!), []);
    expectSame(dependentsOf(sources[2]!), []);
    writeSource(countInput, 0);
    expect(read(node)).toBe(0);
    expectSame(sourceDependenciesOf(node), []);
    expect(node.lastSourceDependency === undefined).toBe(true);
    expectSame(dependentsOf(sources[0]!), []);
    writeSource(sources[0]!, 16);
    expect(node.status).toBe(CLEAN);
    writeSource(countInput, 3);
    expect(read(node)).toBe(22);
    expectSame(sourceDependenciesOf(node), sources);
    writeSource(sources[1]!, 32);
    expect(node.status).toBe(DIRTY);
    expect(read(node)).toBe(52);
    expect(runs).toBe(5);
  });

  test('a recompute that reads its two sources in the other order is marked by both', () => {
    const forward = createSourceNode(true);
    const a = createSourceNode(1);
    const b = createSourceNode(10);
    const node = createDerivedNode((callingNode) => {
      if (readSource(callingNode, forward)) {
        const first = readSource(callingNode, a);
        return first - readSource(callingNode, b);
      }
      const first = readSource(callingNode, b);
      return first - readSource(callingNode, a);
    });
    expect(read(node)).toBe(-9);
    writeSource(forward, false);
    expect(read(node)).toBe(9);
    expectSame(sourceDependenciesOf(node), [forward, b, a]);
    expectSame(dependentsOf(a), [node]);
    expectSame(dependentsOf(b), [node]);
    writeSource(a, 2);
    expect(node.status).toBe(DIRTY);
    expect(read(node)).toBe(8);
    writeSource(b, 20);
    expect(node.status).toBe(DIRTY);
    expect(read(node)).toBe(18);
  });

  test('removing the first, the middle, and the last reader of a source leaves the others in its dependents list in read order', () => {
    const a = createSourceNode(1);
    const readers = [0, 1, 2, 3].map((offset) =>
      createDerivedNode((callingNode) => readSource(callingNode, a) + offset),
    );
    readers.forEach((node) => {
      retainDerivedNode(node);
      readDerivedNode(node);
    });
    expectSame(dependentsOf(a), readers);
    releaseDerivedNode(readers[0]!);
    expectSame(dependentsOf(a), [readers[1], readers[2], readers[3]]);
    releaseDerivedNode(readers[2]!);
    expectSame(dependentsOf(a), [readers[1], readers[3]]);
    releaseDerivedNode(readers[3]!);
    expectSame(dependentsOf(a), [readers[1]]);
    expect(a.lastDependent?.dependent === readers[1]).toBe(true);
    retainDerivedNode(readers[0]!);
    readDerivedNode(readers[0]!);
    expectSame(dependentsOf(a), [readers[1], readers[0]]);
    writeSource(a, 2);
    expect(readers.map((node) => node.status)).toEqual([
      DIRTY,
      DIRTY,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
    releaseDerivedNode(readers[1]!);
    releaseDerivedNode(readers[0]!);
    expectSame(dependentsOf(a), []);
    expect(a.firstDependent === undefined).toBe(true);
    expect(a.lastDependent === undefined).toBe(true);
  });
});

describe('release', () => {
  test('a released node with no reader holds no value and no dependency, and the release cascades to the derived node it read', () => {
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, a) + readDerived(callingNode, inner),
    );
    retainDerivedNode(outer);
    expect(readDerivedNode(outer)).toBe(2);
    expectSame(dependentsOf(inner), [outer]);
    releaseDerivedNode(outer);
    expect([outer.status, inner.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect([outer.value, inner.value]).toEqual([undefined, undefined]);
    expectSame(dependentsOf(inner), []);
    expectSame(dependentsOf(a), []);
    expect(
      [
        outer.firstSourceDependency,
        outer.lastSourceDependency,
        outer.firstDerivedDependency,
        outer.lastDerivedDependency,
        inner.firstSourceDependency,
      ].map((edge) => edge === undefined),
    ).toEqual([true, true, true, true, true]);
  });

  test('a release cascades through a chain, and a node the chain reads that another node still reads survives with its value', () => {
    const runs = { shared: 0, left: 0, right: 0 };
    const a = createSourceNode(1);
    const shared = createDerivedNode((callingNode) => {
      runs.shared += 1;
      return readSource(callingNode, a);
    });
    const left = createDerivedNode((callingNode) => {
      runs.left += 1;
      return readDerived(callingNode, shared) + 1;
    });
    const leftTop = createDerivedNode(
      (callingNode) => readDerived(callingNode, left) + 1,
    );
    const right = createDerivedNode((callingNode) => {
      runs.right += 1;
      return readDerived(callingNode, shared) + 10;
    });
    retainDerivedNode(leftTop);
    retainDerivedNode(right);
    expect([readDerivedNode(leftTop), readDerivedNode(right)]).toEqual([3, 11]);
    releaseDerivedNode(leftTop);
    expect([leftTop.status, left.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect([shared.status, right.status]).toEqual([CLEAN, CLEAN]);
    expectSame(dependentsOf(shared), [right]);
    expectSame(dependentsOf(a), [shared]);
    write(a, 2);
    expect(readDerivedNode(right)).toBe(12);
    expect(runs).toEqual({ shared: 2, left: 1, right: 2 });
  });

  test('a recompute that switches from reading c to reading d disposes c and the node only c read, and switching back computes them again', () => {
    const runs = { c: 0, belowC: 0, d: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const b = createDerivedNode((callingNode) => readSource(callingNode, a));
    const belowC = createDerivedNode((callingNode) => {
      runs.belowC += 1;
      return readSource(callingNode, a) * 100;
    });
    const c = createDerivedNode((callingNode) => {
      runs.c += 1;
      return readDerived(callingNode, belowC) + 1;
    });
    const d = createDerivedNode((callingNode) => {
      runs.d += 1;
      return readSource(callingNode, a) * 1000;
    });
    const top = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, b) +
        (readSource(callingNode, flag)
          ? readDerived(callingNode, c)
          : readDerived(callingNode, d)),
    );
    retainDerivedNode(top);
    expect(readDerivedNode(top)).toBe(102);
    write(flag, false);
    expect(readDerivedNode(top)).toBe(1001);
    expect([c.status, belowC.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expect([b.status, d.status]).toEqual([CLEAN, CLEAN]);
    expectSame(derivedDependenciesOf(top), [b, d]);
    expectSame(dependentsOf(a), [b, d]);
    write(flag, true);
    expect(readDerivedNode(top)).toBe(102);
    expect(d.status).toBe(UNCOMPUTED);
    expect(runs).toEqual({ c: 2, belowC: 2, d: 1 });
  });

  test('a branch that flips is disposed and recomputed on each flip, and retaining both branches keeps them computed', () => {
    const runs = { left: 0, right: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const left = createDerivedNode((callingNode) => {
      runs.left += 1;
      return readSource(callingNode, a);
    });
    const right = createDerivedNode((callingNode) => {
      runs.right += 1;
      return readSource(callingNode, a) * 10;
    });
    const top = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, left)
        : readDerived(callingNode, right),
    );
    retainDerivedNode(top);
    readDerivedNode(top);
    write(flag, false);
    readDerivedNode(top);
    write(flag, true);
    readDerivedNode(top);
    expect(runs).toEqual({ left: 2, right: 1 });
    expect(right.status).toBe(UNCOMPUTED);

    retainDerivedNode(left);
    retainDerivedNode(right);
    write(flag, false);
    readDerivedNode(top);
    write(flag, true);
    expect(readDerivedNode(top)).toBe(1);
    write(flag, false);
    expect(readDerivedNode(top)).toBe(10);
    expect(runs).toEqual({ left: 2, right: 2 });
    expect([left.status, right.status]).toEqual([CLEAN, CLEAN]);
  });

  test('a recompute that reads fewer derived nodes disposes the nodes trim drops', () => {
    const count = createSourceNode(3);
    const a = createSourceNode(1);
    const nodes = [0, 1, 2].map((offset) =>
      createDerivedNode((callingNode) => readSource(callingNode, a) + offset),
    );
    const top = createDerivedNode((callingNode) =>
      nodes
        .slice(0, readSource(callingNode, count))
        .reduce((sum, node) => sum + readDerived(callingNode, node), 0),
    );
    retainDerivedNode(top);
    expect(readDerivedNode(top)).toBe(6);
    write(count, 1);
    expect(readDerivedNode(top)).toBe(1);
    expect(nodes.map((node) => node.status)).toEqual([
      CLEAN,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
    expectSame(dependentsOf(a), [nodes[0]]);
    write(count, 0);
    expect(readDerivedNode(top)).toBe(0);
    expect(nodes[0]!.status).toBe(UNCOMPUTED);
    expectSame(dependentsOf(a), []);
  });

  test('a trim that drops a node and the node it read disposes the chain, and a dropped node another dropped edge still reaches is disposed after that edge is dropped', () => {
    const count = createSourceNode(3);
    const a = createSourceNode(1);
    const bottom = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const middle = createDerivedNode(
      (callingNode) => readDerived(callingNode, bottom) + 1,
    );
    const reads = [middle, bottom];
    const top = createDerivedNode((callingNode) =>
      reads
        .slice(0, readSource(callingNode, count) - 1)
        .reduce((sum, node) => sum + readDerived(callingNode, node), 0),
    );
    retainDerivedNode(top);
    expect(readDerivedNode(top)).toBe(3);
    write(count, 1);
    expect(readDerivedNode(top)).toBe(0);
    expect([middle.status, bottom.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(a), []);
  });

  test('a node that a recompute repoints away from and reads again later in the same run survives with its value and does not rerun', () => {
    const runs = { x: 0, y: 0 };
    const order = createSourceNode<'xy' | 'yx'>('xy');
    const a = createSourceNode(1);
    const x = createDerivedNode((callingNode) => {
      runs.x += 1;
      return readSource(callingNode, a);
    });
    const y = createDerivedNode((callingNode) => {
      runs.y += 1;
      return readSource(callingNode, a) * 10;
    });
    const top = createDerivedNode((callingNode) =>
      readSource(callingNode, order) === 'xy'
        ? readDerived(callingNode, x) * 100 + readDerived(callingNode, y)
        : readDerived(callingNode, y) * 100 + readDerived(callingNode, x),
    );
    retainDerivedNode(top);
    expect(readDerivedNode(top)).toBe(110);
    write(order, 'yx');
    // The first read repoints the edge from x to y, so x has no dependent
    // until the second read relinks it.
    expect(readDerivedNode(top)).toBe(1001);
    expect([x.status, y.status]).toEqual([CLEAN, CLEAN]);
    expectSame(derivedDependenciesOf(top), [y, x]);
    expect(runs).toEqual({ x: 1, y: 1 });
    expect(top.disposalCandidates).toBeUndefined();
  });

  test('a node that a recompute repoints away from and does not read again is disposed after the run, with the node only it read', () => {
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const below = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const x = createDerivedNode(
      (callingNode) => readDerived(callingNode, below) + 1,
    );
    const y = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 10,
    );
    let statusOfXDuringRun: number | undefined;
    const top = createDerivedNode((callingNode) => {
      if (readSource(callingNode, flag)) {
        return readDerived(callingNode, x);
      }
      const value = readDerived(callingNode, y);
      statusOfXDuringRun = x.status;
      return value;
    });
    retainDerivedNode(top);
    expect(readDerivedNode(top)).toBe(2);
    write(flag, false);
    expect(readDerivedNode(top)).toBe(10);
    expect(statusOfXDuringRun).toBe(CLEAN);
    expect([x.status, below.status, y.status]).toEqual([
      UNCOMPUTED,
      UNCOMPUTED,
      CLEAN,
    ]);
    expectSame(dependentsOf(a), [y]);
  });

  test('a node retained several times is disposed only when the last retain is released', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    [0, 1, 2].forEach(() => retainDerivedNode(node));
    expect(node.retainCount).toBe(3);
    expect([0, 1, 2].map(() => readDerivedNode(node))).toEqual([1, 1, 1]);
    expect(runs).toBe(1);
    releaseDerivedNode(node);
    releaseDerivedNode(node);
    expect(node.status).toBe(CLEAN);
    expectSame(dependentsOf(a), [node]);
    releaseDerivedNode(node);
    expect(node.status).toBe(UNCOMPUTED);
    expectSame(dependentsOf(a), []);
    expect(node.retainCount).toBe(0);
  });

  test('a second release of the same retain decrements again, so it disposes a node that another holder still counts on, as the contract states', () => {
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    retainDerivedNode(node);
    retainDerivedNode(node);
    expect(readDerivedNode(node)).toBe(1);
    releaseDerivedNode(node);
    expect(node.status).toBe(CLEAN);
    releaseDerivedNode(node);
    expect(node.retainCount).toBe(0);
    expect(node.status).toBe(UNCOMPUTED);
  });

  test('releasing a node that a reader reads keeps it, and releasing the reader disposes both', () => {
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    retainDerivedNode(inner);
    retainDerivedNode(outer);
    expect([readDerivedNode(inner), readDerivedNode(outer)]).toEqual([1, 2]);
    releaseDerivedNode(inner);
    expect([inner.status, outer.status]).toEqual([CLEAN, CLEAN]);
    write(a, 2);
    expect(readDerivedNode(outer)).toBe(3);
    releaseDerivedNode(outer);
    expect([inner.status, outer.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(a), []);
  });

  test('a disposed node that is retained again computes with previous undefined and is marked by what it read', () => {
    const previousValues: Array<number | undefined> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(10);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const node = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, a) + readDerived(callingNode, inner);
    });
    retainDerivedNode(node);
    expect(readDerivedNode(node)).toBe(11);
    releaseDerivedNode(node);
    write(a, 2);
    retainDerivedNode(node);
    expect(readDerivedNode(node)).toBe(12);
    expectSame(sourceDependenciesOf(node), [a]);
    expectSame(derivedDependenciesOf(node), [inner]);
    write(a, 3);
    expect(node.status).toBe(DIRTY);
    expect(readDerivedNode(node)).toBe(13);
    write(b, 20);
    expect(node.status).toBe(CHECK);
    expect(readDerivedNode(node)).toBe(23);
    expect(previousValues).toEqual([undefined, undefined, 12, 13]);
  });

  test('releasing a Check node disposes it and the Dirty node it read', () => {
    const a = createSourceNode(1);
    const bottom = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const top = createDerivedNode(
      (callingNode) => readDerived(callingNode, bottom) + 1,
    );
    retainDerivedNode(top);
    readDerivedNode(top);
    write(a, 2);
    expect([bottom.status, top.status]).toEqual([DIRTY, CHECK]);
    releaseDerivedNode(top);
    expect([bottom.status, top.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(a), []);
  });

  test('retaining a node that was never computed holds nothing, and releasing it changes nothing', () => {
    const previousValues: Array<number | undefined> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode<number>((callingNode, previous) => {
      previousValues.push(previous);
      return readSource(callingNode, a);
    });
    retainDerivedNode(node);
    expect(node.status).toBe(UNCOMPUTED);
    expectSame(dependentsOf(a), []);
    releaseDerivedNode(node);
    expect(node.status).toBe(UNCOMPUTED);
    retainDerivedNode(node);
    expect(readDerivedNode(node)).toBe(1);
    expect(previousValues).toEqual([undefined]);
  });

  test('an eager node whose eager reader stops reading it in a flush is disposed, and a queued eager node released before the flush stays Uncomputed', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const flag = createSourceNode(true);
    const eager = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a) * 2;
    });
    const reader = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, eager) : 0,
    );
    retainDerivedNode(reader);
    expect(readDerivedNode(reader)).toBe(2);
    write(flag, false);
    expect(reader.status).toBe(CLEAN);
    expect(reader.value).toBe(0);
    expect(eager.status).toBe(UNCOMPUTED);
    expectSame(dependentsOf(a), []);

    const queued = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    retainDerivedNode(queued);
    readDerivedNode(queued);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect(queued.status).toBe(DIRTY);
      releaseDerivedNode(queued);
    });
    expect(queued.status).toBe(UNCOMPUTED);
    expect(hasDependent(a, queued)).toBe(false);
    expect(runs).toBe(2);
  });

  test('a subscriber retains an eager node and sets onChange on it, which the flush calls when the value changed, and after the release a write calls nothing and visits nothing', () => {
    const calls: Array<number> = [];
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const node = createEagerDerivedNode(
      (callingNode) => Math.floor(readDerived(callingNode, inner) / 2) + 1,
    );
    retainDerivedNode(node);
    node.onChange = () => calls.push(readDerivedNode(node));
    expect(readDerivedNode(node)).toBe(1);
    write(a, 2);
    write(a, 3);
    write(a, 4);
    expect(calls).toEqual([2, 3]);
    releaseDerivedNode(node);
    expect([inner.status, node.status]).toEqual([UNCOMPUTED, UNCOMPUTED]);
    expectSame(dependentsOf(a), []);
    write(a, 6);
    expect(calls).toEqual([2, 3]);
    expect(a.dependentsStatus).toBe(1);
  });

  test('two subscribers of one node each retain an eager node, and releasing one leaves the node read by the other', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    const subscribe = (name: string): DerivedNode<number> => {
      const subscriber = createEagerDerivedNode((callingNode) =>
        readDerived(callingNode, node),
      );
      retainDerivedNode(subscriber);
      subscriber.onChange = () =>
        calls.push(`${name} ${readDerivedNode(subscriber)}`);
      readDerivedNode(subscriber);
      return subscriber;
    };
    const first = subscribe('first');
    subscribe('second');
    write(a, 2);
    releaseDerivedNode(first);
    expect(first.status).toBe(UNCOMPUTED);
    expect(node.status).toBe(CLEAN);
    write(a, 3);
    expect(calls).toEqual(['first 2', 'second 2', 'second 3']);
  });

  test('a release cascades through a chain of 100,000 nodes without overflowing the call stack', () => {
    const a = createSourceNode(0);
    let previous = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const bottom = previous;
    const nodes: Array<DerivedNode<number>> = [previous];
    for (let i = 0; i < 100_000; i++) {
      const below = previous;
      previous = createDerivedNode(
        (callingNode) => readDerived(callingNode, below) + 1,
      );
      nodes.push(previous);
    }
    // Computing the chain top-down would overflow, so each node is computed
    // bottom-up while it is retained, and every retain but the top one is
    // released once the node above holds it.
    nodes.forEach((node) => {
      retainDerivedNode(node);
      readDerivedNode(node);
    });
    nodes.forEach((node, index) => {
      if (index !== nodes.length - 1) {
        releaseDerivedNode(node);
      }
    });
    expect(bottom.status).toBe(CLEAN);
    releaseDerivedNode(previous);
    expect(nodes.every((node) => node.status === UNCOMPUTED)).toBe(true);
    expectSame(dependentsOf(a), []);
  });
});

describe('disposal deferred to the outermost run', () => {
  /** A lazy node that reads `a` and counts its runs in `runs.x`. */
  function countedReader(
    runs: { x: number },
    a: SourceNode<number>,
  ): DerivedNode<number> {
    return createDerivedNode((callingNode) => {
      runs.x += 1;
      return readSource(callingNode, a);
    });
  }

  test('a node that an inner run drops and that the outer run read after the inner node before survives and does not rerun', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const outer = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, inner) + readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(2);
    write(flag, false);
    expect(read(outer)).toBe(1);
    expect(runs.x).toBe(1);
    expectSame(derivedDependenciesOf(outer), [inner, x]);
  });

  test('a node that an inner run drops and that the outer run read before the inner node, and now reads after it, survives and does not rerun', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const outer = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, x) + readDerived(callingNode, inner)
        : readDerived(callingNode, inner) + readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(2);
    write(flag, false);
    expect(read(outer)).toBe(1);
    expect(runs.x).toBe(1);
  });

  test('a node whose edge the outer run repointed before reading the inner node, which drops it, survives and does not rerun when the outer run reads it again', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const b = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 10,
    );
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const outer = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, x) + readDerived(callingNode, inner)
        : readDerived(callingNode, b) +
          readDerived(callingNode, inner) +
          readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(2);
    write(flag, false);
    expect(read(outer)).toBe(11);
    expect(runs.x).toBe(1);
    expectSame(derivedDependenciesOf(outer), [b, inner, x]);
  });

  test('a node that an inner run drops and that the outer run reads for the first time survives and does not rerun', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const outer = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, inner)
        : readDerived(callingNode, inner) + readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(1);
    write(flag, false);
    expect(read(outer)).toBe(1);
    expect(runs.x).toBe(1);
  });

  test('a node that a run two levels down drops survives and does not rerun when the outer run, which repointed its edge to it, reads it again', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const b = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 10,
    );
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const middle = createDerivedNode(
      (callingNode) => readDerived(callingNode, inner) + 1,
    );
    const outer = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, x) + readDerived(callingNode, middle)
        : readDerived(callingNode, b) +
          readDerived(callingNode, middle) +
          readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(3);
    write(flag, false);
    expect(read(outer)).toBe(12);
    expect(runs.x).toBe(1);
  });

  test('a node that one branch of a swap within one node stops reading at its position survives and does not rerun', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const y = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 10,
    );
    const outer = createDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, x) * 100 + readDerived(callingNode, y)
        : readDerived(callingNode, y) * 100 + readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(110);
    write(flag, false);
    expect(read(outer)).toBe(1001);
    expect(runs.x).toBe(1);
  });

  for (const [name, writesOther] of [
    ['recomputes', true],
    ['verifies', false],
  ] as const) {
    test(`a node that one node of a swap across two nodes drops and the other reads survives and does not rerun when the outer node ${name}`, () => {
      const runs = { x: 0 };
      const flag = createSourceNode(true);
      const other = createSourceNode(0);
      const a = createSourceNode(1);
      const x = countedReader(runs, a);
      const y = createDerivedNode(
        (callingNode) => readSource(callingNode, a) * 10,
      );
      const first = createDerivedNode((callingNode) =>
        readSource(callingNode, flag)
          ? readDerived(callingNode, x)
          : readDerived(callingNode, y),
      );
      const second = createDerivedNode((callingNode) =>
        readSource(callingNode, flag)
          ? readDerived(callingNode, y)
          : readDerived(callingNode, x),
      );
      const outer = createDerivedNode(
        (callingNode) =>
          readSource(callingNode, other) +
          readDerived(callingNode, first) * 100 +
          readDerived(callingNode, second),
      );
      expect(read(outer)).toBe(110);
      startBatch((mutators) => {
        mutators.writeSource(flag, false);
        if (writesOther) {
          mutators.writeSource(other, 1);
        }
      });
      expect(read(outer)).toBe(writesOther ? 1002 : 1001);
      expect(runs.x).toBe(1);
      expect([x.status, y.status]).toEqual([CLEAN, CLEAN]);
    });
  }

  test('a node that one eager node of a swap drops in a flush and a later eager node reads survives and does not rerun', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const y = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 10,
    );
    const first = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, x)
        : readDerived(callingNode, y),
    );
    const second = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, y)
        : readDerived(callingNode, x),
    );
    expect([read(first), read(second)]).toEqual([1, 10]);
    write(flag, false);
    expect([first.value, second.value]).toEqual([10, 1]);
    expect(runs.x).toBe(1);
  });

  test('a node that an eager inner node drops survives and does not rerun when the eager outer node reads it after the inner node', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const inner = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const outer = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, flag)
        ? readDerived(callingNode, x) + readDerived(callingNode, inner)
        : readDerived(callingNode, inner) + readDerived(callingNode, x),
    );
    expect(read(outer)).toBe(2);
    write(flag, false);
    expect(outer.value).toBe(1);
    expect(runs.x).toBe(1);
  });

  test('a node that a run verification reaches below the read node drops survives and does not rerun when a later run reads it', () => {
    const runs = { x: 0 };
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = countedReader(runs, a);
    const selector = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? 1 : 2,
    );
    const first = createDerivedNode((callingNode) =>
      readDerived(callingNode, selector) === 1
        ? readDerived(callingNode, x)
        : 0,
    );
    const second = createDerivedNode((callingNode) =>
      readDerived(callingNode, selector) === 1
        ? 0
        : readDerived(callingNode, x),
    );
    const outer = createDerivedNode(
      (callingNode) =>
        readDerived(callingNode, first) * 100 +
        readDerived(callingNode, second),
    );
    expect(read(outer)).toBe(100);
    write(flag, false);
    expect(read(outer)).toBe(1);
    expect(runs.x).toBe(1);
    expectSame(derivedDependenciesOf(second), [selector, x]);
  });

  for (const [outerDrops, innerDrops] of [
    [1, 2],
    [2, 1],
  ] as const) {
    test(`when the outer run leaves ${outerDrops} and an inner run leaves ${innerDrops} nodes unneeded, the read disposes each one that no later run read`, () => {
      const flag = createSourceNode(true);
      const a = createSourceNode(1);
      const nodes = (count: number) =>
        Array.from({ length: count }, () =>
          createDerivedNode((callingNode) => readSource(callingNode, a)),
        );
      const outerNodes = nodes(outerDrops);
      const innerNodes = nodes(innerDrops);
      const replacement = createDerivedNode(
        (callingNode) => readSource(callingNode, a) * 10,
      );
      const sum = (
        callingNode: DerivedNode<unknown>,
        list: ReadonlyArray<DerivedNode<number>>,
      ) =>
        list.reduce((total, node) => total + readDerived(callingNode, node), 0);
      const inner = createDerivedNode((callingNode) =>
        readSource(callingNode, flag) ? sum(callingNode, innerNodes) : 0,
      );
      const outer = createDerivedNode((callingNode) =>
        readSource(callingNode, flag)
          ? sum(callingNode, outerNodes) + readDerived(callingNode, inner)
          : readDerived(callingNode, replacement) +
            readDerived(callingNode, inner) +
            readDerived(callingNode, innerNodes[0]!),
      );
      expect(read(outer)).toBe(outerDrops + innerDrops);
      write(flag, false);
      expect(read(outer)).toBe(11);
      expect(outerNodes.map((node) => node.status)).toEqual(
        outerNodes.map(() => UNCOMPUTED),
      );
      expect(innerNodes.map((node) => node.status)).toEqual(
        innerNodes.map((_, index) => (index === 0 ? CLEAN : UNCOMPUTED)),
      );
      expectSame(dependentsOf(a), [innerNodes[0]!, replacement]);
    });
  }

  test('a queued eager node that an earlier refresh of the flush stops reading is disposed without running, and its onChange is not called', () => {
    let runs = 0;
    const calls: Array<number> = [];
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const dropped = createEagerDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    const reader = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, dropped) : 0,
    );
    expect(read(reader)).toBe(1);
    dropped.onChange = () => calls.push(dropped.value);
    startBatch((mutators) => {
      mutators.writeSource(flag, false);
      mutators.writeSource(a, 2);
    });
    expect(reader.value).toBe(0);
    expect(dropped.status).toBe(UNCOMPUTED);
    expect(runs).toBe(1);
    expect(calls).toEqual([]);
    expectSame(dependentsOf(a), []);
  });

  test('a node that an inner run drops and no later run reads is disposed when the outermost read returns', () => {
    const flag = createSourceNode(true);
    const a = createSourceNode(1);
    const x = createDerivedNode((callingNode) => readSource(callingNode, a));
    let statusOfXAfterInner: number | undefined;
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, flag) ? readDerived(callingNode, x) : 0,
    );
    const outer = createDerivedNode((callingNode) => {
      const value = readDerived(callingNode, inner);
      statusOfXAfterInner = x.status;
      return value;
    });
    expect(read(outer)).toBe(1);
    write(flag, false);
    expect(read(outer)).toBe(0);
    expect(statusOfXAfterInner).toBe(CLEAN);
    expect(x.status).toBe(UNCOMPUTED);
    expectSame(dependentsOf(a), []);
  });
});

describe('eager nodes outside the flush', () => {
  test('a pull of a Check eager node before the flush returns its previous value and runs nothing, and the flush marks its reader', () => {
    const runs = { inner: 0, eager: 0 };
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => {
      runs.inner += 1;
      return readSource(callingNode, a);
    });
    const eager = createEagerDerivedNode((callingNode) => {
      runs.eager += 1;
      return readDerived(callingNode, inner) * 2;
    });
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 1,
    );
    expect(read(reader)).toBe(3);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect(eager.status).toBe(CHECK);
      expect(read(eager)).toBe(2);
      expect(read(reader)).toBe(3);
      expect(eager.status).toBe(CHECK);
      expect(runs).toEqual({ inner: 1, eager: 1 });
    });
    expect(runs).toEqual({ inner: 2, eager: 2 });
    expect([eager.status, reader.status]).toEqual([CLEAN, DIRTY]);
    expect(read(reader)).toBe(5);
  });

  test('a lazy Check node above a queued Check eager node becomes Clean with its previous value without running the eager node, and the flush marks it again', () => {
    let eagerRuns = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const eager = createEagerDerivedNode((callingNode) => {
      eagerRuns += 1;
      return readDerived(callingNode, inner) * 2;
    });
    const lazy = createDerivedNode((callingNode) => readSource(callingNode, b));
    const middle = createDerivedNode(
      (callingNode) => readDerived(callingNode, lazy) > 0,
    );
    const top = createDerivedNode((callingNode) =>
      readDerived(callingNode, middle) ? readDerived(callingNode, eager) : 0,
    );
    expect(read(top)).toBe(2);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.writeSource(b, 2);
      expect([eager.status, top.status]).toEqual([CHECK, CHECK]);
      expect(read(top)).toBe(2);
      expect([eager.status, top.status]).toEqual([CHECK, CLEAN]);
      expect(eagerRuns).toBe(1);
    });
    expect(eagerRuns).toBe(2);
    expect(top.status).toBe(DIRTY);
    expect(read(top)).toBe(4);
  });
});

describe('setDerivedNodeEvaluation as a single call', () => {
  test('a queued eager node set lazy by the single call marks its readers Check before the call returns and calls no onChange', () => {
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
    read(outer);
    eager.onChange = () => calls.push('eager');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CLEAN,
        CLEAN,
      ]);
      setDerivedNodeEvaluation(eager, LAZY);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CHECK,
        CHECK,
      ]);
    });
    expect(calls).toEqual([]);
    expect(eager.evaluation).toBe(LAZY);
    expect(runs).toBe(1);
    expect(read(outer)).toBe(6);
    expect(runs).toBe(2);
  });

  test('a queued eager node set lazy by the single call marks its readers Check, and refreshes an eager reader of a reader and calls its onChange', () => {
    const runs = { eager: 0, above: 0 };
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode((callingNode) => {
      runs.eager += 1;
      return readSource(callingNode, a) * 2;
    });
    const reader = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 1,
    );
    const above = createEagerDerivedNode((callingNode) => {
      runs.above += 1;
      return readDerived(callingNode, reader) + 1;
    });
    const sibling = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 100,
    );
    read(above);
    read(sibling);
    above.onChange = () => calls.push(`above ${readDerivedNode(above)}`);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      setDerivedNodeEvaluation(eager, LAZY);
      expect(runs).toEqual({ eager: 2, above: 2 });
      expect(calls).toEqual(['above 6']);
      expect([
        eager.status,
        reader.status,
        above.status,
        sibling.status,
      ]).toEqual([CLEAN, CLEAN, CLEAN, DIRTY]);
    });
    expect(read(above)).toBe(6);
    expect(read(sibling)).toBe(104);
    expect(runs).toEqual({ eager: 2, above: 2 });
  });

  test('a Check lazy node set eager by the single call is verified before the call returns, and a Dirty one is recomputed', () => {
    const runs = { inner: 0, check: 0, dirty: 0 };
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => {
      runs.inner += 1;
      return readSource(callingNode, a) % 2;
    });
    const check = createDerivedNode((callingNode) => {
      runs.check += 1;
      return readDerived(callingNode, inner) * 10;
    });
    const dirty = createDerivedNode((callingNode) => {
      runs.dirty += 1;
      return readSource(callingNode, a) * 10;
    });
    read(check);
    read(dirty);
    writeSource(a, 3);
    expect([check.status, dirty.status]).toEqual([CHECK, DIRTY]);
    setDerivedNodeEvaluation(check, EAGER);
    setDerivedNodeEvaluation(dirty, EAGER);
    expect([check.evaluation, dirty.evaluation]).toEqual([EAGER, EAGER]);
    expect([check.status, dirty.status]).toEqual([CLEAN, CLEAN]);
    expect(runs).toEqual({ inner: 2, check: 1, dirty: 2 });
    expect([read(check), read(dirty)]).toEqual([10, 30]);
  });
});

describe('setDerivedNodeEvaluation with the evaluation the node has', () => {
  test('setting a Dirty lazy node lazy, by the single call and by the batch method, runs nothing and marks nothing', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return readSource(callingNode, a);
    });
    const reader = createDerivedNode((callingNode) =>
      readDerived(callingNode, node),
    );
    read(reader);
    writeSource(a, 2);
    setDerivedNodeEvaluation(node, LAZY);
    startBatch((mutators) => mutators.setDerivedNodeEvaluation(node, LAZY));
    expect([node.status, reader.status]).toEqual([DIRTY, CHECK]);
    expect(runs).toBe(1);
  });
});

describe('onChange', () => {
  test('the flush calls onChange after every queued eager node is refreshed, in the order of each node first changed recompute', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const b = createSourceNode(1);
    const first = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const second = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    read(first);
    read(second);
    first.onChange = () =>
      calls.push(`first, second ${second.status} ${second.value}`);
    second.onChange = () => calls.push(`second, first ${first.value}`);
    startBatch((mutators) => {
      mutators.writeSource(b, 2);
      mutators.writeSource(a, 2);
    });
    expect(calls).toEqual(['second, first 2', `first, second ${CLEAN} 2`]);
  });

  test('a node that the flush recomputes twice calls onChange once, with the value of its last recompute', () => {
    const runs = { first: 0, late: 0 };
    const calls: Array<string> = [];
    const a = createSourceNode(1);
    const late = createEagerDerivedNode((callingNode) => {
      runs.late += 1;
      return readSource(callingNode, a) * 10;
    });
    const first = createEagerDerivedNode((callingNode) => {
      runs.first += 1;
      return readSource(callingNode, a) + readDerived(callingNode, late);
    });
    read(first);
    first.onChange = () => calls.push(`first ${readDerivedNode(first)}`);
    late.onChange = () => calls.push(`late ${readDerivedNode(late)}`);
    expectSame(dependentsOf(a), [first, late]);
    write(a, 2);
    expect(runs).toEqual({ first: 3, late: 2 });
    expect(calls).toEqual(['first 22', 'late 20']);
    write(a, 3);
    expect(calls).toEqual(['first 22', 'late 20', 'first 33', 'late 30']);
  });

  test('a first computation calls no onChange, outside or inside a flush', () => {
    const calls: Array<string> = [];
    const a = createSourceNode(0);
    const b = createSourceNode(5);
    const late = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    late.onChange = () => calls.push('late');
    const top = createEagerDerivedNode((callingNode) =>
      readSource(callingNode, a) > 0 ? readDerived(callingNode, late) : 0,
    );
    top.onChange = () => calls.push(`top ${readDerivedNode(top)}`);
    expect(read(top)).toBe(0);
    expect(calls).toEqual([]);
    write(a, 1);
    expect(late.status).toBe(CLEAN);
    expect(calls).toEqual(['top 5']);
    write(b, 6);
    expect(calls).toEqual(['top 5', 'late', 'top 6']);
  });

  test('an onChange reads current values of lazy and eager nodes through readDerivedNode, and leaves them Clean', () => {
    const seen: Array<ReadonlyArray<number>> = [];
    const a = createSourceNode(1);
    const eager = createEagerDerivedNode(
      (callingNode) => readSource(callingNode, a) * 2,
    );
    const lazy = createDerivedNode(
      (callingNode) => readSource(callingNode, a) + 1,
    );
    const lazyAbove = createDerivedNode(
      (callingNode) => readDerived(callingNode, eager) + 100,
    );
    read(eager);
    read(lazy);
    read(lazyAbove);
    eager.onChange = () =>
      seen.push([
        readDerivedNode(eager),
        readDerivedNode(lazy),
        readDerivedNode(lazyAbove),
        a.value,
      ]);
    write(a, 5);
    expect(seen).toEqual([[10, 6, 110, 5]]);
    expect([eager.status, lazy.status, lazyAbove.status]).toEqual([
      CLEAN,
      CLEAN,
      CLEAN,
    ]);
  });

  test('onChange on a lazy node is unused until the node is set eager, and then the flush calls it when it changes the value', () => {
    const calls: Array<number> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    node.onChange = () => calls.push(readDerivedNode(node));
    read(node);
    write(a, 2);
    expect(read(node)).toBe(2);
    startBatch((mutators) => {
      mutators.writeSource(a, 3);
      expect(read(node)).toBe(3);
    });
    write(a, 4);
    expect(calls).toEqual([]);
    setDerivedNodeEvaluation(node, EAGER);
    expect(calls).toEqual([4]);
    write(a, 5);
    expect(calls).toEqual([4, 5]);
    setDerivedNodeEvaluation(node, LAZY);
    write(a, 6);
    expect(read(node)).toBe(6);
    expect(calls).toEqual([4, 5]);
  });
});

describe('untracked reads', () => {
  test('readDerivedNode and source.value inside a computation record no dependency, so the result is stale until a tracked read changes', () => {
    let runs = 0;
    const a = createSourceNode(1);
    const b = createSourceNode(100);
    const inner = createDerivedNode(
      (callingNode) => readSource(callingNode, a) * 10,
    );
    read(inner);
    const node = createDerivedNode((callingNode) => {
      runs += 1;
      return a.value + readDerivedNode(inner) + readSource(callingNode, b);
    });
    expect(read(node)).toBe(111);
    expectSame(sourceDependenciesOf(node), [b]);
    expectSame(derivedDependenciesOf(node), []);
    expectSame(dependentsOf(inner), []);
    write(a, 2);
    expect(node.status).toBe(CLEAN);
    expect(read(node)).toBe(111);
    expect(inner.status).toBe(DIRTY);
    write(b, 200);
    expect(read(node)).toBe(222);
    expect(runs).toBe(2);
    expect(inner.status).toBe(CLEAN);
  });
});

describe('queues', () => {
  for (const [name, writeWith] of writers) {
    test(`${name} refreshes every eager node it queues in queue order, and eager nodes that changed recomputes queue join the same walk`, () => {
      const refreshed: Array<string> = [];
      const calls: Array<string> = [];
      const a = createSourceNode(1);
      const direct = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((index) =>
        createEagerDerivedNode((callingNode) => {
          refreshed.push(`direct ${index}`);
          return readSource(callingNode, a) + index;
        }),
      );
      const above = [0, 9].map((index) =>
        createEagerDerivedNode((callingNode) => {
          refreshed.push(`above ${index}`);
          return readDerived(callingNode, direct[index]!) * 10;
        }),
      );
      [...direct, ...above].forEach((node) => read(node));
      [...direct, ...above].forEach((node, index) => {
        node.onChange = () => calls.push(`${index}`);
      });
      refreshed.length = 0;
      writeWith(a, 2);
      expect(refreshed).toEqual([
        ...direct.map((_, index) => `direct ${index}`),
        'above 0',
        'above 9',
      ]);
      expect(calls).toEqual([
        '0',
        '1',
        '2',
        '3',
        '4',
        '5',
        '6',
        '7',
        '8',
        '9',
        '10',
        '11',
      ]);
      expect(above.map((node) => node.value)).toEqual([20, 110]);
      refreshed.length = 0;
      writeWith(a, 3);
      expect(refreshed.length).toBe(12);
      expect(calls.length).toBe(24);
    });
  }
});

describe('README example', () => {
  test('the example returns the values and run counts its comments state', () => {
    const person = createSourceNode({
      first: 'Ada',
      last: 'Lovelace',
      age: 36,
    });
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
    retainDerivedNode(greeting);
    expect(readDerivedNode(greeting)).toBe('Hello, Ada Lovelace');
    expect(greetingRuns).toBe(1);
    writeSource(person, { first: 'Ada', last: 'Lovelace', age: 37 });
    expect(readDerivedNode(greeting)).toBe('Hello, Ada Lovelace');
    expect(greetingRuns).toBe(1);
    startBatch((mutators) => {
      mutators.writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
      mutators.writeSource(person, { first: 'Ada', last: 'King', age: 38 });
    });
    expect(readDerivedNode(greeting)).toBe('Hello, Ada King');
    expect(greetingRuns).toBe(2);
    const rendered: Array<string> = [];
    const view = createEagerDerivedNode((callingNode) =>
      readDerived(callingNode, greeting),
    );
    retainDerivedNode(view);
    readDerivedNode(view);
    view.onChange = () => rendered.push(readDerivedNode(view));
    writeSource(person, { first: 'Augusta', last: 'King', age: 38 });
    expect(rendered).toEqual(['Hello, Augusta King']);
    releaseDerivedNode(view);
    releaseDerivedNode(greeting);
    expect(view.status).toBe(UNCOMPUTED);
    expect(greeting.status).toBe(UNCOMPUTED);
    expect(name.status).toBe(UNCOMPUTED);
  });
});

function dependentsOf(
  dependency: SourceNode<unknown> | DerivedNode<unknown>,
): ReadonlyArray<DerivedNode<unknown>> {
  const dependents: Array<DerivedNode<unknown>> = [];
  let edge = dependency.firstDependent;
  while (edge !== undefined) {
    dependents.push(edge.dependent);
    edge = edge.nextDependent;
  }
  return dependents;
}

function sourceDependenciesOf(
  node: DerivedNode<unknown>,
): ReadonlyArray<SourceNode<unknown>> {
  const dependencies: Array<SourceNode<unknown>> = [];
  let edge = node.firstSourceDependency;
  while (edge !== undefined) {
    dependencies.push(edge.dependency);
    edge = edge.nextDependency;
  }
  return dependencies;
}

function derivedDependenciesOf(
  node: DerivedNode<unknown>,
): ReadonlyArray<DerivedNode<unknown>> {
  const dependencies: Array<DerivedNode<unknown>> = [];
  let edge = node.firstDerivedDependency;
  while (edge !== undefined) {
    dependencies.push(edge.dependency);
    edge = edge.nextDependency;
  }
  return dependencies;
}

// Compares by identity. toEqual on nodes walks the whole graph through the
// edge lists.
function expectSame(
  actual: ReadonlyArray<unknown>,
  expected: ReadonlyArray<unknown>,
): void {
  expect(actual.length).toBe(expected.length);
  actual.forEach((item, index) => {
    expect(item === expected[index]).toBe(true);
  });
}
```

```ts
// from libs/pico-js/test/graph.test-d.ts
import { expectTypeOf, test } from 'vitest';
import {
  type CHECK,
  type CLEAN,
  createDerivedNode,
  createEagerDerivedNode,
  createSourceNode,
  type DIRTY,
  type EAGER,
  type LAZY,
  readDerived,
  readDerivedNode,
  readSource,
  releaseDerivedNode,
  retainDerivedNode,
  startBatch,
  type UNCOMPUTED,
  writeSource,
  type DependentsStatus,
  type DerivedNode,
  type DerivedNodeEvaluation,
  type SourceNode,
  type Status,
} from '../src/index';

test('a derived node that reads another derived node has the type of the value its computation returns', () => {
  const count = createDerivedNode(() => 1);
  const label = createDerivedNode(
    (callingNode) => `${readDerived(callingNode, count)}`,
  );
  expectTypeOf(label).toEqualTypeOf<DerivedNode<string>>();
});

test('the constructors and reads carry the value type, and the exported types name the node fields', () => {
  const source = createSourceNode(1);
  expectTypeOf(source).toEqualTypeOf<SourceNode<number>>();
  expectTypeOf(source.dependentsStatus).toEqualTypeOf<DependentsStatus>();
  const lazy = createDerivedNode((callingNode) =>
    readSource(callingNode, source),
  );
  const eager = createEagerDerivedNode(
    (callingNode) => `${readDerived(callingNode, lazy)}`,
  );
  expectTypeOf(lazy).toEqualTypeOf<DerivedNode<number>>();
  expectTypeOf(eager).toEqualTypeOf<DerivedNode<string>>();
  expectTypeOf(readDerivedNode(eager)).toEqualTypeOf<string>();
  expectTypeOf(lazy.status).toEqualTypeOf<Status>();
  expectTypeOf(lazy.evaluation).toEqualTypeOf<DerivedNodeEvaluation>();
  expectTypeOf<Status>().toEqualTypeOf<
    typeof UNCOMPUTED | typeof DIRTY | typeof CHECK | typeof CLEAN
  >();
  expectTypeOf<DerivedNodeEvaluation>().toEqualTypeOf<
    typeof LAZY | typeof EAGER
  >();
  expectTypeOf<DependentsStatus>().toEqualTypeOf<0 | 1>();
  expectTypeOf(lazy.retainCount).toEqualTypeOf<number>();
  expectTypeOf(lazy.onChange).toEqualTypeOf<(() => void) | undefined>();
});

test('retain and release take any derived node and return nothing, and readDerivedNode returns the value type', () => {
  expectTypeOf(retainDerivedNode).toEqualTypeOf<
    (node: DerivedNode<unknown>) => void
  >();
  expectTypeOf(releaseDerivedNode).toEqualTypeOf<
    (node: DerivedNode<unknown>) => void
  >();
  const node = createDerivedNode(() => 'value');
  retainDerivedNode(node);
  expectTypeOf(readDerivedNode(node)).toEqualTypeOf<string>();
  releaseDerivedNode(node);
});

test('a computation receives its previous value as the value type or undefined', () => {
  createDerivedNode<number>((callingNode, previous) => {
    expectTypeOf(callingNode).toEqualTypeOf<DerivedNode<unknown>>();
    expectTypeOf(previous).toEqualTypeOf<number | undefined>();
    return 1;
  });
});

test('a write takes a value of the source type, in the single call and in the batch method', () => {
  const source = createSourceNode(1);
  // @ts-expect-error a string is not a number
  writeSource(source, 'one');
  startBatch((mutators) => {
    // @ts-expect-error a string is not a number
    mutators.writeSource(source, 'one');
    expectTypeOf(mutators.setDerivedNodeEvaluation)
      .parameter(1)
      .toEqualTypeOf<DerivedNodeEvaluation>();
  });
});

test('a caller cannot write a node field except onChange', () => {
  const source = createSourceNode(1);
  const node = createDerivedNode((callingNode) =>
    readSource(callingNode, source),
  );
  // @ts-expect-error a source's value is written through writeSource
  source.value = 2;
  // @ts-expect-error a source's fields are read-only
  source.dependentsStatus = 0;
  // @ts-expect-error a derived node's status is read-only
  node.status = 3;
  // @ts-expect-error a derived node's value is read-only
  node.value = 2;
  // @ts-expect-error a derived node's evaluation is read-only
  node.evaluation = 1;
  // @ts-expect-error a derived node's edge lists are read-only
  node.firstDependent = undefined;
  // @ts-expect-error a derived node's retain count changes only through retain and release
  node.retainCount = 1;
  // @ts-expect-error a derived node's onChange status is internal to the engine
  node.onChangeStatus;
  // @ts-expect-error a derived node's disposal candidates are read-only
  node.disposalCandidates = undefined;
  node.onChange = () => {};
});
```

All 123 runtime tests and 6 type tests pass under vitest 2.1.9, `tsc -b` reports no errors, and the runtime tests cover every line and branch of `src/graph.ts` (`@vitest/coverage-v8` 2.1.9). Reads outside computations go through a test-local `read(node)`, which retains the node on its first call and never releases it, so a test that does not test lifetime keeps every node it reads.

### Random differential test

`test-random/random.ts` generates a random case for each seed and runs it through pico-js three times. `pnpm test:random` reads the seed count from `PICO_JS_RANDOM_SEEDS` (default 2,000) and the first seed from `PICO_JS_RANDOM_BASE_SEED` (default 1). Environment variables carry them because vitest reads positional arguments as file filters.

A case is a graph and a list of operations, generated from the seed by mulberry32. The graph has 2 to 5 sources with values 0 to 3 and 3 to 14 derived nodes, 30% of them eager. Each derived node reads a selector, then one of three branches of up to three reads that the selector's value picks, so its reads change from run to run. A derived node reads only sources and derived nodes of a lower index, so the graph is acyclic, and 20% of derived nodes also make an untracked `readDerivedNode` of a lower node when that node is retained, and discard the value. A node returns the sum of its reads modulo 2, 3, or 1000, as a number, as an object it reuses from `previous` when the number is unchanged, or as a fresh object, so that cutoff applies to the first two and never to the third. An operation is a write, an evaluation change, a retain, a release of an outstanding retain, a read of a retained node, an `onChange` assignment, or a batch. A batch holds mutations, retains, releases, reads, nested batches, and single calls. An `onChange` callback reads up to two retained nodes. After the operations, the case releases every outstanding retain. A write of the value a source already holds is skipped, because the contract allows only changing writes, and a read or release of a node the run does not hold is skipped and logged, so a shrunk case keeps the contracts.

The three runs are:

- `single`: every mutation, at top level or in a batch, is a single call (`writeSource`, `setDerivedNodeEvaluation`).
- `one-op-batch`: every mutation is a `startBatch` whose body makes that one mutator call.
- `grouped`: each batch is one `startBatch`, its nested batches are nested `startBatch` calls, and its single items are single calls inside the body.

The `single` and `one-op-batch` runs log every computation run, `onChange` call, read value, and, after each operation, every node's status and value. The two logs must be equal entry for entry, which compares the single-call marking code (`markDirtyQueue`, `markCheckQueue`) with the batched code (`markDirty`, `markCheck`).

Each run also checks itself against `Model`, a model that keeps statuses, values, retain counts, and derived dependency lists in arrays and follows the README's Model section and Guarantees step by step. The model edits each node's derived dependency list by position, as the engine edits its edge list, and disposes a node that is computed, has retain count 0, and is in no list, then repeats the check on its dependencies. A release, a trim at the end of a run, and a repoint trigger the check. The check for a trim or a repoint runs when the outermost `readDerivedNode` returns, through a stack of pending lists with one list per `readDerivedNode` that has not returned, so a node that a later run of the same call reads survives and does not run again. An untracked read inside a run is a `readDerivedNode` of its own and gets its own list.

- After every mutator call and every release, every status, value, retain count, and derived dependency list equals the model's. A mutator call runs no computation and calls no `onChange`.
- Each read returns the model's value and runs exactly the computations the model runs, in the same order. A read made while no eager node is `Dirty` or `Check` returns the value recomputed from the sources alone. A second read runs nothing.
- Every recompute has a reason: a source its latest computation read was written, or a derived node it read recomputed to a value that is not `===` its previous one.
- Each flush calls exactly the `onChange` of the eager nodes it recomputed to a changed value, once per node, in the order of each node's first changed recompute, and none for a first computation or a lazy node, and it recomputes no eager node after its first `onChange`.
- After each operation, every eager node is `Clean` or `Uncomputed`, every `Clean` node holds the value recomputed from the sources, each node that is not `Uncomputed` is in the dependents list of exactly the nodes its latest computation read, an `Uncomputed` node holds no value, no dependency, and no dependent, no computed node has retain count 0 and no dependent, and a source whose `dependentsStatus` is 1 has only `Dirty` readers. After the final releases, every node is `Uncomputed` and no source has a dependent.

A flush that refreshes an eager node recomputes in the order of its eager queue, which the guarantees leave open, so the model stops predicting statuses at that flush. At the end of the operation, after the checks above pass, the model adopts the engine's statuses and the observed values and dependency lists.

A failing case shrinks by removing runs of operations (halves, then quarters, down to single operations), then single batch items and callback reads, while any check still fails. The thrown error names the seed, the first failed check, the shrunk case's failed check, and the shrunk case as JSON.

Each of these changes to `graph.ts` fails on one of seeds 1 to 130: disposing each run's candidates when the run ends instead of when the outermost call returns (seed 130); disposing a repointed dependency during the run (seed 34); dropping the push onto the candidates in `unlinkDropped`; dropping the move of the candidates in `readDerived`; dropping the disposal in `readDerivedNode`; dropping the flush's disposal of the candidates; dropping the `onChangeStatus` check in `flush`; counting a refresh of an `Uncomputed` node as changed; and dropping the cascade into dependencies in `disposeUnneeded`. Disposing an unneeded queued eager node in `refreshEager` is checked by a runtime test instead, because the model goes out of sync in a flush that refreshes an eager node. Seeds 1 to 20,000 pass.

```ts
// from libs/pico-js/test-random/random.ts
import { test } from 'vitest';
import {
  CHECK,
  CLEAN,
  createDerivedNode,
  createEagerDerivedNode,
  createSourceNode,
  DIRTY,
  EAGER,
  LAZY,
  readDerived,
  readDerivedNode,
  readSource,
  releaseDerivedNode,
  retainDerivedNode,
  setDerivedNodeEvaluation,
  startBatch,
  UNCOMPUTED,
  writeSource,
  type DerivedNode,
  type DerivedNodeEvaluation,
  type SourceNode,
  type Status,
} from '../src/index';

type Mutators = Parameters<Parameters<typeof startBatch>[0]>[0];

type Read =
  | { readonly kind: 'source'; readonly source: number }
  | { readonly kind: 'derived'; readonly node: number };

/** How a computation turns its number into the value it returns. */
type ResultShape = 'number' | 'reused-object' | 'fresh-object';

/**
 * `untracked` is a derived node of a lower index that the computation reads
 * through `readDerivedNode` before its tracked reads, when the node is
 * retained. The computation discards that value.
 */
type NodeSpec = {
  readonly selector: Read;
  readonly branches: ReadonlyArray<ReadonlyArray<Read>>;
  readonly untracked: number | undefined;
  readonly modulus: number;
  readonly shape: ResultShape;
  readonly evaluation: DerivedNodeEvaluation;
};

/** A derived node reads only sources and derived nodes of a lower index. */
type GraphSpec = {
  readonly sourceValues: ReadonlyArray<number>;
  readonly nodes: ReadonlyArray<NodeSpec>;
};

type Mutation =
  | { readonly kind: 'write'; readonly source: number; readonly value: number }
  | {
      readonly kind: 'setEvaluation';
      readonly node: number;
      readonly evaluation: DerivedNodeEvaluation;
    };

/** A read, retain, or release of a node the run does not hold is skipped. */
type ReadStep = { readonly kind: 'read'; readonly node: number };

type RetainStep = { readonly kind: 'retain'; readonly node: number };

type ReleaseStep = { readonly kind: 'release'; readonly node: number };

type Step = Mutation | ReadStep | RetainStep | ReleaseStep;

/** A callback reads each node of `reads` that the run holds. */
type CallbackSpec = {
  readonly id: number;
  readonly reads: ReadonlyArray<number>;
};

type Batch = {
  readonly kind: 'batch';
  readonly items: ReadonlyArray<BatchItem>;
};

type BatchItem =
  | Step
  | Batch
  | { readonly kind: 'single'; readonly mutation: Mutation };

type SetOnChange = {
  readonly kind: 'setOnChange';
  readonly node: number;
  readonly callback: CallbackSpec | undefined;
};

type Op = Step | Batch | SetOnChange;

type Case = { readonly graph: GraphSpec; readonly ops: ReadonlyArray<Op> };

/**
 * `single` makes every mutation a single call, and `one-op-batch` makes every
 * mutation a `startBatch` of one mutator call, so the two runs must log the
 * same events. `grouped` runs each `Batch` as one `startBatch` and its
 * `single` items as single calls inside the body.
 */
type Mode = 'single' | 'one-op-batch' | 'grouped';

type Value = number | { readonly n: number };

type Synchronization = 'in-sync' | 'out-of-sync';

/** A read of a node's latest computation, with the counters at that read. */
type ReadRecord =
  | {
      readonly kind: 'source';
      readonly source: number;
      readonly writes: number;
    }
  | {
      readonly kind: 'derived';
      readonly node: number;
      readonly changes: number;
    };

/**
 * `queue` holds the eager nodes the model queued in the batch. `expected` holds
 * the callbacks of the eager nodes that the flush recomputed to a changed
 * value, once per node, in the order of their first changed recompute, and
 * `invoked` the callbacks the flush called.
 */
type Frame = {
  readonly queue: Array<number>;
  readonly invoked: Array<number>;
  readonly expected: Array<number>;
  readonly expectedNodes: Array<number>;
};

type LinkedEdge = {
  readonly dependency: unknown;
  readonly dependent: unknown;
  readonly nextDependent: LinkedEdge | undefined;
};

type DependentsHolder = { readonly firstDependent: LinkedEdge | undefined };

const MAXIMUM_RUNS = 1_000_000;

function at<T>(array: ReadonlyArray<T>, index: number): T {
  const element = array[index];
  if (element === undefined) {
    throw new Error(`index ${index} is out of bounds`);
  }
  return element;
}

function valueNumber(value: Value): number {
  return typeof value === 'number' ? value : value.n;
}

function sameRead(left: Read, right: Read): boolean {
  switch (left.kind) {
    case 'source':
      return right.kind === 'source' && right.source === left.source;
    case 'derived':
      return right.kind === 'derived' && right.node === left.node;
  }
}

function evaluate(spec: NodeSpec, read: (read: Read) => number): number {
  const selected = read(spec.selector);
  const sum = at(spec.branches, selected % spec.branches.length).reduce(
    (total, branchRead) => total + read(branchRead),
    selected,
  );
  return sum % spec.modulus;
}

function shapeResult(
  shape: ResultShape,
  n: number,
  previous: Value | undefined,
): Value {
  switch (shape) {
    case 'number':
      return n;
    case 'reused-object':
      return typeof previous === 'object' && previous.n === n
        ? previous
        : { n };
    case 'fresh-object':
      return { n };
  }
}

/** The value of every derived node, computed from the source values alone. */
function scratchValues(
  graph: GraphSpec,
  sourceValues: ReadonlyArray<number>,
): ReadonlyArray<number> {
  return graph.nodes.reduce<ReadonlyArray<number>>(
    (values, spec) => [
      ...values,
      evaluate(spec, (read) =>
        read.kind === 'source'
          ? at(sourceValues, read.source)
          : at(values, read.node),
      ),
    ],
    [],
  );
}

/**
 * A naive model of the README's model and guarantees over arrays. Each node's
 * derived dependency list is an array of node indices that a run edits by
 * position, as the engine edits its edge list, and a node is needed while its
 * retain count is above 0 or another node's list holds it. A flush that
 * refreshes an eager node depends on the order of the eager queue, which the
 * guarantees leave open, so the model goes out of sync there and the run
 * adopts the engine's state at the end of the operation, after checking it
 * against the guarantees.
 */
class Model {
  readonly #graph: GraphSpec;
  #synchronization: Synchronization = 'in-sync';
  readonly #sourceValues: Array<number>;
  readonly #status: Array<Status>;
  readonly #evaluation: Array<DerivedNodeEvaluation>;
  readonly #value: Array<number | undefined>;
  readonly #reads: Array<ReadonlyArray<Read>>;
  readonly #derivedEdges: Array<Array<number>>;
  readonly #retainCount: Array<number>;
  /**
   * One list per `readDerivedNode` that has not returned, of the nodes its
   * runs left unneeded. The `readDerivedNode` disposes them when it returns.
   */
  readonly #pendingDisposals: Array<Array<number>> = [];
  readonly predictedRuns: Array<number> = [];

  constructor(graph: GraphSpec) {
    this.#graph = graph;
    this.#sourceValues = [...graph.sourceValues];
    this.#status = graph.nodes.map(() => UNCOMPUTED);
    this.#evaluation = graph.nodes.map((spec) => spec.evaluation);
    this.#value = graph.nodes.map(() => undefined);
    this.#reads = graph.nodes.map(() => []);
    this.#derivedEdges = graph.nodes.map(() => []);
    this.#retainCount = graph.nodes.map(() => 0);
  }

  get synchronization(): Synchronization {
    return this.#synchronization;
  }

  get sourceValues(): ReadonlyArray<number> {
    return this.#sourceValues;
  }

  status(node: number): Status {
    return at(this.#status, node);
  }

  value(node: number): number | undefined {
    return this.#value[node];
  }

  reads(node: number): ReadonlyArray<Read> {
    return at(this.#reads, node);
  }

  derivedEdges(node: number): ReadonlyArray<number> {
    return at(this.#derivedEdges, node);
  }

  retainCount(node: number): number {
    return at(this.#retainCount, node);
  }

  desynchronize(): void {
    this.#synchronization = 'out-of-sync';
  }

  resynchronize(
    status: ReadonlyArray<Status>,
    value: ReadonlyArray<number | undefined>,
    reads: ReadonlyArray<ReadonlyArray<Read>>,
    derivedEdges: ReadonlyArray<ReadonlyArray<number>>,
  ): void {
    status.forEach((nodeStatus, node) => {
      this.#status[node] = nodeStatus;
      this.#value[node] = value[node];
      this.#reads[node] = at(reads, node);
      this.#derivedEdges[node] = [...at(derivedEdges, node)];
    });
    this.#synchronization = 'in-sync';
  }

  /** Out of sync, it records only the source value and the evaluation. */
  mutate(mutation: Mutation, queue: Array<number>): void {
    switch (mutation.kind) {
      case 'write':
        this.#sourceValues[mutation.source] = mutation.value;
        if (this.#synchronization === 'in-sync') {
          this.#markDirty(
            this.#readers({ kind: 'source', source: mutation.source }),
            queue,
          );
        }
        break;
      case 'setEvaluation':
        this.#setEvaluation(mutation.node, mutation.evaluation, queue);
        break;
    }
  }

  retain(node: number): void {
    this.#retainCount[node] = at(this.#retainCount, node) + 1;
  }

  /** Out of sync, it records only the retain count. */
  release(node: number): void {
    this.#retainCount[node] = at(this.#retainCount, node) - 1;
    if (this.#synchronization === 'in-sync') {
      this.#dispose([node]);
    }
  }

  /** Whether the flush of a batch with `queue` refreshes an eager node. */
  refreshes(queue: ReadonlyArray<number>): boolean {
    return queue.some(
      (node) =>
        at(this.#evaluation, node) === EAGER &&
        (at(this.#status, node) === DIRTY || at(this.#status, node) === CHECK),
    );
  }

  /** A `readDerivedNode`, which disposes the nodes its runs left unneeded. */
  read(node: number): number {
    this.#pendingDisposals.push([]);
    const value = this.#pull(node);
    this.#dispose(
      at(this.#pendingDisposals, this.#pendingDisposals.length - 1),
    );
    this.#pendingDisposals.pop();
    return value;
  }

  #pull(node: number): number {
    switch (at(this.#status, node)) {
      case CLEAN:
        return this.#stored(node);
      case DIRTY:
        return at(this.#evaluation, node) === LAZY
          ? this.#recompute(node)
          : this.#stored(node);
      case CHECK:
        if (at(this.#evaluation, node) === EAGER) {
          return this.#stored(node);
        }
        return this.#verify(node) === DIRTY
          ? this.#recompute(node)
          : this.#stored(node);
      case UNCOMPUTED:
        return this.#computeFirst(node);
    }
  }

  #setEvaluation(
    node: number,
    evaluation: DerivedNodeEvaluation,
    queue: Array<number>,
  ): void {
    if (at(this.#evaluation, node) === evaluation) {
      return;
    }
    this.#evaluation[node] = evaluation;
    if (this.#synchronization === 'out-of-sync') {
      return;
    }
    switch (at(this.#status, node)) {
      case DIRTY:
      case CHECK:
        switch (evaluation) {
          case EAGER:
            queue.push(node);
            break;
          case LAZY:
            this.#markCheck([node], queue);
            break;
        }
        break;
      case CLEAN:
      case UNCOMPUTED:
        break;
    }
  }

  #readers(target: Read): ReadonlyArray<number> {
    switch (target.kind) {
      case 'source':
        return this.#reads.flatMap((reads, node) =>
          at(this.#status, node) !== UNCOMPUTED &&
          reads.some((read) => sameRead(read, target))
            ? [node]
            : [],
        );
      case 'derived':
        return this.#derivedEdges.flatMap((edges, node) =>
          edges.includes(target.node) ? [node] : [],
        );
    }
  }

  #hasDependents(node: number): boolean {
    return this.#derivedEdges.some((edges) => edges.includes(node));
  }

  #unneeded(node: number): boolean {
    return (
      at(this.#status, node) !== UNCOMPUTED &&
      at(this.#retainCount, node) === 0 &&
      !this.#hasDependents(node)
    );
  }

  /** Disposes each unneeded node of `start`, and then its dependencies. */
  #dispose(start: ReadonlyArray<number>): void {
    const stack = [...start];
    let node = stack.pop();
    while (node !== undefined) {
      if (this.#unneeded(node)) {
        stack.push(...at(this.#derivedEdges, node));
        this.#derivedEdges[node] = [];
        this.#reads[node] = [];
        this.#status[node] = UNCOMPUTED;
        this.#value[node] = undefined;
      }
      node = stack.pop();
    }
  }

  #markDirty(readers: ReadonlyArray<number>, queue: Array<number>): void {
    const takenOut: Array<number> = [];
    readers.forEach((node) => {
      switch (at(this.#status, node)) {
        case CLEAN:
          this.#status[node] = DIRTY;
          if (at(this.#evaluation, node) === EAGER) {
            queue.push(node);
          } else {
            takenOut.push(node);
          }
          break;
        case CHECK:
          this.#status[node] = DIRTY;
          break;
        case DIRTY:
        case UNCOMPUTED:
          break;
      }
    });
    this.#markCheck(takenOut, queue);
  }

  #markCheck(stack: Array<number>, queue: Array<number>): void {
    let marked = stack.pop();
    while (marked !== undefined) {
      this.#readers({ kind: 'derived', node: marked }).forEach((reader) => {
        if (at(this.#status, reader) !== CLEAN) {
          return;
        }
        this.#status[reader] = CHECK;
        if (at(this.#evaluation, reader) === EAGER) {
          queue.push(reader);
        } else {
          stack.push(reader);
        }
      });
      marked = stack.pop();
    }
  }

  #stored(node: number): number {
    const value = this.#value[node];
    if (value === undefined) {
      throw new Error(`the model holds no value for node ${node}`);
    }
    return value;
  }

  #verify(node: number): typeof DIRTY | typeof CLEAN {
    for (const dependency of [...at(this.#derivedEdges, node)]) {
      this.#pull(dependency);
      if (at(this.#status, node) === DIRTY) {
        return DIRTY;
      }
    }
    this.#status[node] = CLEAN;
    return CLEAN;
  }

  /**
   * Each `readDerived` reuses the entry at its position, repoints it, or
   * appends one unless it repeats the previous `readDerived` past the end of
   * the list. A node repointed away from, and the entries past the last read,
   * which the run drops, are checked when the outermost `readDerivedNode`
   * returns.
   */
  #run(node: number): number {
    this.predictedRuns.push(node);
    const spec = at(this.#graph.nodes, node);
    const edges = at(this.#derivedEdges, node);
    const candidates: Array<number> = [];
    const reads: Array<Read> = [];
    let cursor = 0;
    if (
      spec.untracked !== undefined &&
      at(this.#retainCount, spec.untracked) > 0
    ) {
      this.read(spec.untracked);
    }
    const value = evaluate(spec, (read) => {
      reads.push(read);
      if (read.kind === 'source') {
        return at(this.#sourceValues, read.source);
      }
      const readValue = this.#pull(read.node);
      const next = edges[cursor];
      if (next !== undefined) {
        if (next !== read.node) {
          edges[cursor] = read.node;
          if (at(this.#retainCount, next) === 0 && !this.#hasDependents(next)) {
            candidates.push(next);
          }
        }
        cursor += 1;
      } else if (cursor === 0 || edges[cursor - 1] !== read.node) {
        edges.push(read.node);
        cursor += 1;
      }
      return readValue;
    });
    const dropped = edges.splice(cursor);
    this.#reads[node] = reads;
    this.#status[node] = CLEAN;
    at(this.#pendingDisposals, this.#pendingDisposals.length - 1).push(
      ...dropped,
      ...candidates,
    );
    return value;
  }

  #computeFirst(node: number): number {
    const value = this.#run(node);
    this.#value[node] = value;
    return value;
  }

  #recompute(node: number): number {
    const previous = this.#stored(node);
    const value = this.#run(node);
    const changed =
      at(this.#graph.nodes, node).shape === 'fresh-object' ||
      value !== previous;
    if (!changed) {
      return previous;
    }
    this.#value[node] = value;
    this.#readers({ kind: 'derived', node }).forEach((reader) => {
      if (at(this.#status, reader) === CHECK) {
        this.#status[reader] = DIRTY;
      }
    });
    return value;
  }
}

/** One run of a case through pico-js in one mode, checked against the model. */
class Run {
  readonly #mode: Mode;
  readonly #graph: GraphSpec;
  readonly #model: Model;
  readonly #sources: ReadonlyArray<SourceNode<number>>;
  readonly #nodes: ReadonlyArray<DerivedNode<Value>>;
  readonly #indexOf = new Map<unknown, Read>();
  readonly log: Array<string> = [];
  readonly #frames: Array<Frame> = [];
  readonly #observedReads: Array<ReadonlyArray<Read>>;
  readonly #lastReads: Array<ReadonlyArray<ReadRecord> | undefined>;
  readonly #writeCount: Array<number>;
  readonly #changeCount: Array<number>;
  readonly #callbackOf: Array<number | undefined>;
  readonly #retained: Array<number>;
  readonly #actualRuns: Array<number> = [];
  #totalRuns = 0;
  #bodyDepth = 0;
  #where = 'setup';

  constructor(graph: GraphSpec, mode: Mode) {
    this.#mode = mode;
    this.#graph = graph;
    this.#model = new Model(graph);
    this.#sources = graph.sourceValues.map((value) => createSourceNode(value));
    this.#nodes = graph.nodes.map((spec, index) => {
      const computation = (
        callingNode: DerivedNode<unknown>,
        previous: Value | undefined,
      ): Value => this.#compute(index, callingNode, previous);
      return spec.evaluation === EAGER
        ? createEagerDerivedNode<Value>(computation)
        : createDerivedNode<Value>(computation);
    });
    this.#sources.forEach((source, index) =>
      this.#indexOf.set(source, { kind: 'source', source: index }),
    );
    this.#nodes.forEach((node, index) =>
      this.#indexOf.set(node, { kind: 'derived', node: index }),
    );
    this.#observedReads = graph.nodes.map(() => []);
    this.#lastReads = graph.nodes.map(() => undefined);
    this.#writeCount = graph.sourceValues.map(() => 0);
    this.#changeCount = graph.nodes.map(() => 0);
    this.#callbackOf = graph.nodes.map(() => undefined);
    this.#retained = graph.nodes.map(() => 0);
  }

  execute(op: Op, index: number): void {
    this.#where = `op ${index} (${JSON.stringify(op)})`;
    this.log.push(`op ${index}`);
    switch (op.kind) {
      case 'write':
      case 'setEvaluation':
        this.#topLevelMutation(op);
        break;
      case 'read':
        this.#read(op.node);
        break;
      case 'retain':
        this.#retain(op.node);
        break;
      case 'release':
        this.#release(op.node);
        break;
      case 'setOnChange':
        this.#setOnChange(op);
        break;
      case 'batch':
        switch (this.#mode) {
          case 'single':
          case 'one-op-batch':
            this.#flattened(op.items);
            break;
          case 'grouped':
            this.#batch(op.items);
            break;
        }
        break;
    }
    this.#endOp();
  }

  /**
   * When every retain is released, every derived node is disposed, and no
   * source has a dependent.
   */
  finish(): void {
    this.#where = 'after the last operation';
    if (this.#retained.some((count) => count !== 0)) {
      return;
    }
    this.#nodes.forEach((node, index) => {
      if (node.status !== UNCOMPUTED || node.firstDependent !== undefined) {
        this.#fail(
          `every retain is released, and node ${index} has status ${node.status}`,
        );
      }
    });
    this.#sources.forEach((source, index) => {
      if (source.firstDependent !== undefined) {
        this.#fail(
          `every retain is released, and source ${index} has a dependent`,
        );
      }
    });
  }

  #fail(message: string): never {
    throw new Error(`${this.#mode} mode, ${this.#where}: ${message}`);
  }

  #flattened(items: ReadonlyArray<BatchItem>): void {
    items.forEach((item) => {
      switch (item.kind) {
        case 'write':
        case 'setEvaluation':
          this.#topLevelMutation(item);
          break;
        case 'read':
          this.#read(item.node);
          break;
        case 'retain':
          this.#retain(item.node);
          break;
        case 'release':
          this.#release(item.node);
          break;
        case 'batch':
          this.#flattened(item.items);
          break;
        case 'single':
          this.#topLevelMutation(item.mutation);
          break;
      }
    });
  }

  #topLevelMutation(mutation: Mutation): void {
    switch (this.#mode) {
      case 'single':
      case 'grouped':
        this.#single(mutation);
        break;
      case 'one-op-batch':
        this.#batch([mutation]);
        break;
    }
  }

  #setOnChange(op: SetOnChange): void {
    const node = at(this.#nodes, op.node);
    const callback = op.callback;
    if (callback === undefined) {
      node.onChange = undefined;
      this.#callbackOf[op.node] = undefined;
      return;
    }
    node.onChange = () => this.#invoke(callback);
    this.#callbackOf[op.node] = callback.id;
  }

  #invoke(callback: CallbackSpec): void {
    const frame = this.#frames.at(-1);
    if (frame === undefined) {
      this.#fail(`callback ${callback.id} ran outside a flush`);
    }
    frame.invoked.push(callback.id);
    this.log.push(`callback ${callback.id}`);
    if (this.#bodyDepth === 0) {
      this.#checkEagerSettled(`in callback ${callback.id}`);
    }
    callback.reads.forEach((node) => this.#read(node));
  }

  #retain(node: number): void {
    retainDerivedNode(at(this.#nodes, node));
    this.#retained[node] = at(this.#retained, node) + 1;
    this.#model.retain(node);
    this.log.push(`retain ${node}`);
  }

  /** Releases only a retain the run holds, so each retain is released once. */
  #release(node: number): void {
    if (at(this.#retained, node) === 0) {
      this.log.push(`skip release ${node}`);
      return;
    }
    const runs = this.#actualRuns.length;
    const synchronization = this.#model.synchronization;
    releaseDerivedNode(at(this.#nodes, node));
    this.#retained[node] = at(this.#retained, node) - 1;
    this.#model.release(node);
    this.log.push(`release ${node}`);
    if (this.#actualRuns.length !== runs) {
      this.#fail(`releaseDerivedNode(${node}) ran a computation`);
    }
    if (synchronization === 'in-sync') {
      this.#compareModel(`after releaseDerivedNode(${node})`);
    }
  }

  #skips(mutation: Mutation): boolean {
    const skips =
      mutation.kind === 'write' &&
      at(this.#sources, mutation.source).value === mutation.value;
    if (skips) {
      this.log.push('skip');
    }
    return skips;
  }

  #pushFrame(): Frame {
    const frame: Frame = {
      queue: [],
      invoked: [],
      expected: [],
      expectedNodes: [],
    };
    this.#frames.push(frame);
    return frame;
  }

  /** Records the mutation in the model and in the run's counters. */
  #predictMutation(frame: Frame, mutation: Mutation): Synchronization {
    const synchronization = this.#model.synchronization;
    this.#model.mutate(mutation, frame.queue);
    switch (mutation.kind) {
      case 'write':
        this.#writeCount[mutation.source] =
          at(this.#writeCount, mutation.source) + 1;
        break;
      case 'setEvaluation':
        break;
    }
    return synchronization;
  }

  /** Called when the batch acted and before its flush starts. */
  #predictFlush(frame: Frame): void {
    if (
      this.#model.synchronization === 'in-sync' &&
      this.#model.refreshes(frame.queue)
    ) {
      this.#model.desynchronize();
    }
  }

  #single(mutation: Mutation): void {
    if (this.#skips(mutation)) {
      return;
    }
    const frame = this.#pushFrame();
    this.#predictMutation(frame, mutation);
    this.#predictFlush(frame);
    switch (mutation.kind) {
      case 'write':
        writeSource(at(this.#sources, mutation.source), mutation.value);
        break;
      case 'setEvaluation':
        setDerivedNodeEvaluation(
          at(this.#nodes, mutation.node),
          mutation.evaluation,
        );
        break;
    }
    this.#popFrame(frame);
  }

  #batch(items: ReadonlyArray<BatchItem>): void {
    const frame = this.#pushFrame();
    startBatch((mutators) => {
      this.#bodyDepth += 1;
      items.forEach((item) => this.#bodyItem(mutators, frame, item));
      this.#predictFlush(frame);
      this.#bodyDepth -= 1;
    });
    this.#popFrame(frame);
  }

  #bodyItem(mutators: Mutators, frame: Frame, item: BatchItem): void {
    switch (item.kind) {
      case 'write':
      case 'setEvaluation':
        this.#mutator(mutators, frame, item);
        break;
      case 'read':
        this.#read(item.node);
        break;
      case 'retain':
        this.#retain(item.node);
        break;
      case 'release':
        this.#release(item.node);
        break;
      case 'batch':
        this.#batch(item.items);
        break;
      case 'single':
        this.#single(item.mutation);
        break;
    }
  }

  #mutator(mutators: Mutators, frame: Frame, mutation: Mutation): void {
    if (this.#skips(mutation)) {
      return;
    }
    const runs = this.#actualRuns.length;
    const invoked = frame.invoked.length;
    const synchronization = this.#predictMutation(frame, mutation);
    switch (mutation.kind) {
      case 'write':
        mutators.writeSource(
          at(this.#sources, mutation.source),
          mutation.value,
        );
        break;
      case 'setEvaluation':
        mutators.setDerivedNodeEvaluation(
          at(this.#nodes, mutation.node),
          mutation.evaluation,
        );
        break;
    }
    if (this.#actualRuns.length !== runs || frame.invoked.length !== invoked) {
      this.#fail(`mutators.${mutation.kind} ran a computation or a callback`);
    }
    if (synchronization === 'in-sync') {
      this.#compareModel(`after mutators.${mutation.kind}`);
    }
  }

  /**
   * The flush calls the `onChange` of each eager node that it recomputed to a
   * changed value, once per node, in the order of their first changed
   * recompute, and no other callback.
   */
  #popFrame(frame: Frame): void {
    if (this.#frames.pop() !== frame) {
      this.#fail('the batch frames are unbalanced');
    }
    if (frame.invoked.join(',') !== frame.expected.join(',')) {
      this.#fail(
        `the flush recomputed eager nodes [${frame.expectedNodes.join(', ')}] to changed values, whose callbacks are [${frame.expected.join(', ')}], and called [${frame.invoked.join(', ')}]`,
      );
    }
  }

  /** Reads only a node the run holds, as the `readDerivedNode` contract says. */
  #read(node: number): void {
    if (at(this.#retained, node) === 0) {
      this.log.push(`skip read ${node}`);
      return;
    }
    const settled = this.#eagerSettled();
    const synchronization = this.#model.synchronization;
    const predictedStart = this.#model.predictedRuns.length;
    const actualStart = this.#actualRuns.length;
    const predicted =
      synchronization === 'in-sync' ? this.#model.read(node) : undefined;
    const value = valueNumber(readDerivedNode(at(this.#nodes, node)));
    this.log.push(`read ${node} ${value}`);
    const runs = this.#actualRuns.slice(actualStart);
    if (synchronization === 'in-sync') {
      const predictedRuns = this.#model.predictedRuns.slice(predictedStart);
      if (value !== predicted) {
        this.#fail(
          `readDerivedNode(${node}) returned ${value}, the model ${predicted}`,
        );
      }
      if (runs.join(',') !== predictedRuns.join(',')) {
        this.#fail(
          `readDerivedNode(${node}) ran [${runs.join(', ')}], the model [${predictedRuns.join(', ')}]`,
        );
      }
      this.#compareModel(`after readDerivedNode(${node})`);
    }
    if (settled) {
      const expected = at(
        scratchValues(this.#graph, this.#model.sourceValues),
        node,
      );
      if (value !== expected) {
        this.#fail(
          `readDerivedNode(${node}) returned ${value} with no eager node queued, and recomputing from the sources gives ${expected}`,
        );
      }
    }
    const runsBefore = this.#actualRuns.length;
    const again = valueNumber(readDerivedNode(at(this.#nodes, node)));
    if (again !== value || this.#actualRuns.length !== runsBefore) {
      this.#fail(
        `a second readDerivedNode(${node}) ran a computation or changed value`,
      );
    }
  }

  #compute(
    index: number,
    callingNode: DerivedNode<unknown>,
    previous: Value | undefined,
  ): Value {
    this.#totalRuns += 1;
    if (this.#totalRuns > MAXIMUM_RUNS) {
      this.#fail(`more than ${MAXIMUM_RUNS} computations ran`);
    }
    const status = callingNode.status;
    this.#actualRuns.push(index);
    this.log.push(`run ${index}`);
    if (status !== UNCOMPUTED) {
      this.#checkJustified(index);
    }
    const spec = at(this.#graph.nodes, index);
    if (
      spec.untracked !== undefined &&
      at(this.#retained, spec.untracked) > 0
    ) {
      const untracked = valueNumber(
        readDerivedNode(at(this.#nodes, spec.untracked)),
      );
      this.log.push(`untracked ${spec.untracked} ${untracked}`);
    }
    const reads: Array<Read> = [];
    const records: Array<ReadRecord> = [];
    const n = evaluate(spec, (read) => {
      reads.push(read);
      switch (read.kind) {
        case 'source':
          records.push({
            kind: 'source',
            source: read.source,
            writes: at(this.#writeCount, read.source),
          });
          return readSource(callingNode, at(this.#sources, read.source));
        case 'derived': {
          const value = valueNumber(
            readDerived(callingNode, at(this.#nodes, read.node)),
          );
          records.push({
            kind: 'derived',
            node: read.node,
            changes: at(this.#changeCount, read.node),
          });
          return value;
        }
      }
    });
    const result = shapeResult(spec.shape, n, previous);
    const changed = result !== previous;
    if (changed) {
      this.#changeCount[index] = at(this.#changeCount, index) + 1;
    }
    // Only the flush recomputes an eager node that is not Uncomputed, and it
    // refreshes every queued eager node before it calls the first callback.
    if (callingNode.evaluation === EAGER && status !== UNCOMPUTED) {
      const frame = this.#frames.at(-1);
      if (frame === undefined) {
        this.#fail(`eager node ${index} recomputed outside a flush`);
      }
      if (frame.invoked.length !== 0) {
        this.#fail(
          `eager node ${index} recomputed after the flush called a callback`,
        );
      }
      const callback = this.#callbackOf[index];
      if (
        changed &&
        callback !== undefined &&
        !frame.expectedNodes.includes(index)
      ) {
        frame.expectedNodes.push(index);
        frame.expected.push(callback);
      }
    }
    this.#observedReads[index] = reads;
    this.#lastReads[index] = records;
    return result;
  }

  /**
   * A recompute runs only when a source its latest computation read was
   * written, or a derived node it read changed value.
   */
  #checkJustified(index: number): void {
    const records = this.#lastReads[index];
    if (records === undefined) {
      this.#fail(`node ${index} recomputed with no earlier computation`);
    }
    const justified = records.some((record) =>
      record.kind === 'source'
        ? at(this.#writeCount, record.source) > record.writes
        : at(this.#changeCount, record.node) > record.changes,
    );
    if (!justified) {
      this.#fail(
        `node ${index} recomputed, and no node its latest computation read changed`,
      );
    }
  }

  #eagerSettled(): boolean {
    return this.#nodes.every(
      (node) =>
        node.evaluation === LAZY ||
        node.status === CLEAN ||
        node.status === UNCOMPUTED,
    );
  }

  #checkEagerSettled(where: string): void {
    this.#nodes.forEach((node, index) => {
      if (
        node.evaluation === EAGER &&
        node.status !== CLEAN &&
        node.status !== UNCOMPUTED
      ) {
        this.#fail(`${where}, eager node ${index} has status ${node.status}`);
      }
    });
  }

  #engineDerivedEdges(node: DerivedNode<Value>): ReadonlyArray<number> {
    const edges: Array<number> = [];
    let edge = node.firstDerivedDependency;
    while (edge !== undefined) {
      const dependency = this.#indexOf.get(edge.dependency);
      if (dependency === undefined || dependency.kind !== 'derived') {
        this.#fail('a derived dependency list holds an edge to no node');
      }
      edges.push(dependency.node);
      edge = edge.nextDependency;
    }
    return edges;
  }

  #compareModel(where: string): void {
    if (this.#model.synchronization === 'out-of-sync') {
      return;
    }
    this.#nodes.forEach((node, index) => {
      const status = this.#model.status(index);
      if (node.status !== status) {
        this.#fail(
          `${where}, node ${index} has status ${node.status}, the model ${status}`,
        );
      }
      const edges = this.#engineDerivedEdges(node);
      if (edges.join(',') !== this.#model.derivedEdges(index).join(',')) {
        this.#fail(
          `${where}, node ${index} has derived edges [${edges.join(', ')}], the model [${this.#model.derivedEdges(index).join(', ')}]`,
        );
      }
      if (status === UNCOMPUTED) {
        return;
      }
      const value = valueNumber(node.value);
      if (value !== this.#model.value(index)) {
        this.#fail(
          `${where}, node ${index} holds ${value}, the model ${this.#model.value(index)}`,
        );
      }
      if (
        JSON.stringify(this.#model.reads(index)) !==
        JSON.stringify(this.#observedReads[index])
      ) {
        this.#fail(
          `${where}, node ${index} read ${JSON.stringify(this.#observedReads[index])}, the model ${JSON.stringify(this.#model.reads(index))}`,
        );
      }
    });
  }

  /**
   * After a computation, its node is in the dependents list of each node it
   * read and of no other node. A source whose `dependentsStatus` is 1 has
   * only `Dirty` readers.
   */
  #checkDependents(): void {
    const listedIn: Array<Array<Read>> = this.#graph.nodes.map(() => []);
    const holders: ReadonlyArray<readonly [Read, DependentsHolder]> = [
      ...this.#sources.map(
        (source, index) => [{ kind: 'source', source: index }, source] as const,
      ),
      ...this.#nodes.map(
        (node, index) => [{ kind: 'derived', node: index }, node] as const,
      ),
    ];
    holders.forEach(([holderRead, holder]) => {
      let edge = holder.firstDependent;
      let count = 0;
      while (edge !== undefined) {
        count += 1;
        if (count > 100_000) {
          this.#fail(
            `the dependents list of ${JSON.stringify(holderRead)} has a cycle`,
          );
        }
        const dependent = this.#indexOf.get(edge.dependent);
        if (
          edge.dependency !== holder ||
          dependent === undefined ||
          dependent.kind !== 'derived'
        ) {
          this.#fail(
            `the dependents list of ${JSON.stringify(holderRead)} holds an edge of another node`,
          );
        }
        listedIn[dependent.node]?.push(holderRead);
        edge = edge.nextDependent;
      }
    });
    this.#nodes.forEach((node, index) => {
      const listed = at(listedIn, index);
      const read =
        node.status === UNCOMPUTED ? [] : at(this.#observedReads, index);
      const missing = read.filter(
        (target) => !listed.some((entry) => sameRead(entry, target)),
      );
      const extra = listed.filter(
        (entry) => !read.some((target) => sameRead(entry, target)),
      );
      if (missing.length !== 0 || extra.length !== 0) {
        this.#fail(
          `node ${index} (status ${node.status}) is missing from the dependents of ${JSON.stringify(missing)} and is extra in those of ${JSON.stringify(extra)}`,
        );
      }
    });
    this.#sources.forEach((source, sourceIndex) => {
      if (source.dependentsStatus !== 1) {
        return;
      }
      this.#nodes.forEach((node, index) => {
        if (
          node.status !== UNCOMPUTED &&
          node.status !== DIRTY &&
          at(this.#observedReads, index).some((target) =>
            sameRead(target, { kind: 'source', source: sourceIndex }),
          )
        ) {
          this.#fail(
            `source ${sourceIndex} has dependentsStatus 1, and its reader ${index} has status ${node.status}`,
          );
        }
      });
    });
  }

  /**
   * Each node's `retainCount` is the run's count of its unreleased retains. A
   * node that is not `Uncomputed` is retained or has a dependent, and an
   * `Uncomputed` node holds no value, no dependency, and no dependent.
   */
  #checkLifetimes(): void {
    this.#nodes.forEach((node, index) => {
      if (node.retainCount !== at(this.#retained, index)) {
        this.#fail(
          `node ${index} has retainCount ${node.retainCount}, and the run holds ${at(this.#retained, index)} retains`,
        );
      }
      if (node.retainCount !== this.#model.retainCount(index)) {
        this.#fail(
          `node ${index} has retainCount ${node.retainCount}, the model ${this.#model.retainCount(index)}`,
        );
      }
      if (node.disposalCandidates !== undefined) {
        this.#fail(`node ${index} holds disposal candidates after its run`);
      }
      if (node.status === UNCOMPUTED) {
        if (
          node.value !== undefined ||
          node.firstSourceDependency !== undefined ||
          node.firstDerivedDependency !== undefined ||
          node.firstDependent !== undefined
        ) {
          this.#fail(
            `Uncomputed node ${index} holds a value, a dependency, or a dependent`,
          );
        }
      } else if (node.retainCount === 0 && node.firstDependent === undefined) {
        this.#fail(
          `node ${index} has status ${node.status}, retainCount 0, and no dependent`,
        );
      }
    });
  }

  #endOp(): void {
    if (this.#bodyDepth !== 0 || this.#frames.length !== 0) {
      this.#fail('a batch is still open after the operation');
    }
    this.#checkEagerSettled('after the operation');
    const scratch = scratchValues(this.#graph, this.#model.sourceValues);
    this.#nodes.forEach((node, index) => {
      if (
        node.status === CLEAN &&
        valueNumber(node.value) !== at(scratch, index)
      ) {
        this.#fail(
          `Clean node ${index} holds ${valueNumber(node.value)}, and recomputing from the sources gives ${at(scratch, index)}`,
        );
      }
    });
    this.#checkDependents();
    this.#checkLifetimes();
    switch (this.#model.synchronization) {
      case 'in-sync':
        this.#compareModel('after the operation');
        if (
          this.#actualRuns.join(',') !== this.#model.predictedRuns.join(',')
        ) {
          this.#fail(
            `the operation ran [${this.#actualRuns.join(', ')}], the model [${this.#model.predictedRuns.join(', ')}]`,
          );
        }
        break;
      case 'out-of-sync':
        this.#model.resynchronize(
          this.#nodes.map((node) => node.status),
          this.#nodes.map((node) =>
            node.status === UNCOMPUTED ? undefined : valueNumber(node.value),
          ),
          this.#nodes.map((node, index) =>
            node.status === UNCOMPUTED ? [] : at(this.#observedReads, index),
          ),
          this.#nodes.map((node) => this.#engineDerivedEdges(node)),
        );
        break;
    }
    this.#actualRuns.length = 0;
    this.#model.predictedRuns.length = 0;
    this.log.push(
      this.#nodes
        .map(
          (node) =>
            `${node.status}:${node.status === UNCOMPUTED ? '-' : valueNumber(node.value)}:${node.retainCount}`,
        )
        .join(' '),
    );
  }
}

/** mulberry32. */
function prng(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * The generator holds the retains it has generated and not released, reads
 * only those nodes, and releases each of them once, the outstanding ones at
 * the end of the case.
 */
function generateCase(seed: number): Case {
  const random = prng(seed);
  const integer = (bound: number): number => Math.floor(random() * bound);
  const pick = <T>(choices: ReadonlyArray<T>): T =>
    at(choices, integer(choices.length));
  const sourceCount = 2 + integer(4);
  const nodeCount = 3 + integer(12);
  const readFor = (node: number): Read =>
    node === 0 || random() < 0.4
      ? { kind: 'source', source: integer(sourceCount) }
      : { kind: 'derived', node: integer(node) };
  const graph: GraphSpec = {
    sourceValues: Array.from({ length: sourceCount }, () => integer(4)),
    nodes: Array.from({ length: nodeCount }, (_, node) => ({
      selector: readFor(node),
      branches: Array.from({ length: 3 }, () =>
        Array.from({ length: integer(4) }, () => readFor(node)),
      ),
      untracked: node > 0 && random() < 0.2 ? integer(node) : undefined,
      modulus: pick([2, 3, 1000]),
      shape: pick<ResultShape>(['number', 'reused-object', 'fresh-object']),
      evaluation: random() < 0.3 ? EAGER : LAZY,
    })),
  };
  const retained: Array<number> = [];
  let nextCallbackId = 0;
  const retain = (): RetainStep => {
    const node = integer(nodeCount);
    retained.push(node);
    return { kind: 'retain', node };
  };
  const release = (): ReleaseStep | RetainStep => {
    if (retained.length === 0) {
      return retain();
    }
    const index = integer(retained.length);
    const node = at(retained, index);
    retained.splice(index, 1);
    return { kind: 'release', node };
  };
  const read = (): ReadStep | RetainStep =>
    retained.length === 0 ? retain() : { kind: 'read', node: pick(retained) };
  const mutation = (): Mutation =>
    random() < 0.8
      ? { kind: 'write', source: integer(sourceCount), value: integer(4) }
      : {
          kind: 'setEvaluation',
          node: integer(nodeCount),
          evaluation: random() < 0.5 ? EAGER : LAZY,
        };
  const batch = (depth: number): Batch => ({
    kind: 'batch',
    items: Array.from({ length: 1 + integer(5) }, (): BatchItem => {
      const roll = random();
      if (roll < 0.45) {
        return mutation();
      }
      if (roll < 0.6) {
        return read();
      }
      if (roll < 0.7) {
        return retain();
      }
      if (roll < 0.8) {
        return release();
      }
      if (roll < 0.9 && depth < 2) {
        return batch(depth + 1);
      }
      return { kind: 'single', mutation: mutation() };
    }),
  });
  const op = (): Op => {
    const roll = random();
    if (roll < 0.22) {
      return { kind: 'write', source: integer(sourceCount), value: integer(4) };
    }
    if (roll < 0.4) {
      return read();
    }
    if (roll < 0.52) {
      return retain();
    }
    if (roll < 0.62) {
      return release();
    }
    if (roll < 0.7) {
      return {
        kind: 'setEvaluation',
        node: integer(nodeCount),
        evaluation: random() < 0.5 ? EAGER : LAZY,
      };
    }
    if (roll < 0.82) {
      const node = integer(nodeCount);
      if (random() < 0.15) {
        return { kind: 'setOnChange', node, callback: undefined };
      }
      const id = nextCallbackId;
      nextCallbackId += 1;
      return {
        kind: 'setOnChange',
        node,
        callback: {
          id,
          reads: Array.from({ length: integer(3) }, () =>
            retained.length === 0 ? integer(nodeCount) : pick(retained),
          ),
        },
      };
    }
    return batch(0);
  };
  const ops = Array.from({ length: 40 + integer(80) }, op);
  const releases: Array<Op> = [];
  while (retained.length !== 0) {
    releases.push(release());
  }
  return { graph, ops: [...ops, ...releases] };
}

function runCase(testCase: Case, mode: Mode): ReadonlyArray<string> {
  const run = new Run(testCase.graph, mode);
  testCase.ops.forEach((op, index) => run.execute(op, index));
  run.finish();
  return run.log;
}

function compareLogs(
  single: ReadonlyArray<string>,
  batched: ReadonlyArray<string>,
): void {
  const length = Math.max(single.length, batched.length);
  for (let index = 0; index < length; index += 1) {
    if (single[index] !== batched[index]) {
      const context = single.slice(Math.max(0, index - 8), index).join(' | ');
      throw new Error(
        `single calls and one-op batches differ at log entry ${index}: ${single[index]} against ${batched[index]}, after ${context}`,
      );
    }
  }
}

function failureOf(testCase: Case): string | undefined {
  try {
    compareLogs(runCase(testCase, 'single'), runCase(testCase, 'one-op-batch'));
    runCase(testCase, 'grouped');
    return undefined;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

function* simplerItems(
  items: ReadonlyArray<BatchItem>,
): Generator<ReadonlyArray<BatchItem>> {
  for (let index = 0; index < items.length; index += 1) {
    yield [...items.slice(0, index), ...items.slice(index + 1)];
  }
  for (let index = 0; index < items.length; index += 1) {
    const item = at(items, index);
    if (item.kind === 'batch') {
      for (const simpler of simplerItems(item.items)) {
        yield [
          ...items.slice(0, index),
          { kind: 'batch', items: simpler },
          ...items.slice(index + 1),
        ];
      }
    }
  }
}

function* simplerOps(op: Op): Generator<Op> {
  switch (op.kind) {
    case 'batch':
      for (const items of simplerItems(op.items)) {
        yield { kind: 'batch', items };
      }
      break;
    case 'setOnChange': {
      const callback = op.callback;
      if (callback === undefined) {
        break;
      }
      for (let index = 0; index < callback.reads.length; index += 1) {
        yield {
          ...op,
          callback: {
            ...callback,
            reads: [
              ...callback.reads.slice(0, index),
              ...callback.reads.slice(index + 1),
            ],
          },
        };
      }
      break;
    }
    case 'write':
    case 'setEvaluation':
    case 'read':
    case 'retain':
    case 'release':
      break;
  }
}

function* candidates(testCase: Case): Generator<Case> {
  const ops = testCase.ops;
  for (let size = ops.length >> 1; size >= 1; size >>= 1) {
    for (let start = 0; start < ops.length; start += size) {
      yield {
        ...testCase,
        ops: [...ops.slice(0, start), ...ops.slice(start + size)],
      };
    }
  }
  for (let index = 0; index < ops.length; index += 1) {
    for (const simpler of simplerOps(at(ops, index))) {
      yield {
        ...testCase,
        ops: [...ops.slice(0, index), simpler, ...ops.slice(index + 1)],
      };
    }
  }
}

/** Removes operations, batch items, and callback reads while the case fails. */
function shrink(
  testCase: Case,
  message: string,
): { readonly testCase: Case; readonly message: string } {
  let current = testCase;
  let currentMessage = message;
  let progress = true;
  while (progress) {
    progress = false;
    for (const candidate of candidates(current)) {
      const failure = failureOf(candidate);
      if (failure !== undefined) {
        current = candidate;
        currentMessage = failure;
        progress = true;
        break;
      }
    }
  }
  return { testCase: current, message: currentMessage };
}

function integerFromEnvironment(name: string, fallback: number): number {
  const text = process.env[name];
  if (text === undefined || text === '') {
    return fallback;
  }
  const value = Number(text);
  if (!Number.isSafeInteger(value)) {
    throw new Error(`${name} is ${text}, which is not an integer`);
  }
  return value;
}

const seedCount = integerFromEnvironment('PICO_JS_RANDOM_SEEDS', 2000);
const baseSeed = integerFromEnvironment('PICO_JS_RANDOM_BASE_SEED', 1);

test(`random graphs and operations, seeds ${baseSeed} to ${baseSeed + seedCount - 1}`, () => {
  for (let seed = baseSeed; seed < baseSeed + seedCount; seed += 1) {
    const testCase = generateCase(seed);
    const failure = failureOf(testCase);
    if (failure !== undefined) {
      const minimal = shrink(testCase, failure);
      throw new Error(
        `seed ${seed} fails: ${failure}\n\nminimal failing case (${minimal.testCase.ops.length} of ${testCase.ops.length} operations): ${minimal.message}\n${JSON.stringify(minimal.testCase)}`,
      );
    }
  }
});
```
