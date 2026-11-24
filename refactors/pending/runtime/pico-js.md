# pico-js: implementation

This doc is the complete source and package of `@isograph/pico-js` in `libs/pico-js`.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation, its last value, a status, the source and derived nodes its computation read, and the derived nodes that read it. Each dependency is an edge object that sits in the reader's dependency list, in read order, and in the dependents list of the node it read.
- A write marks the source's direct readers `Dirty` and the nodes above them `Check`. It runs no computation. Marking continues through a lazy node, created by `createDerivedNode`, and stops at an eager node, created by `createEagerDerivedNode`. A derived node's `evaluation` field holds `LAZY` or `EAGER`, and `setDerivedNodeEvaluation` and `mutators.setDerivedNodeEvaluation` change it. A source records that a write left every node in its dependents list `Dirty`, and until a computation reads the source again, a write to it visits none of those nodes.
- A derived node has one of four statuses: `Uncomputed`, `Dirty`, `Check`, and `Clean`. The `status` field holds one of the numeric constants `UNCOMPUTED`, `DIRTY`, `CHECK`, and `CLEAN`. A new node is `Uncomputed`.
- `pull` brings a node up to date. An `Uncomputed` node computes for the first time. A `Clean` node returns its value. A `Check` node pulls its derived dependencies in the order they were read and recomputes after the first pull that leaves it `Dirty`. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node, and that other dependency changed. If no pull leaves the node `Dirty`, it becomes `Clean` without running. A `Dirty` node recomputes.
- A computation receives its node and the previous value. An `Uncomputed` node's computation receives `undefined`, as Solid's `createMemo` passes it. A recompute whose value is `===` the previous value marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- Under `===`, a `NaN` value always counts as changed. Under `===`, `0` and `-0` count as equal.
- Each read during a computation records the node it read in the calling node's dependency list and links the calling node into the dependents list of the node it read. A recompute reuses the edges of the previous computation in read order and drops the edges past its last read.
- Code outside a computation reads a source's current value as `source.value`. An untracked read inside a computation, `source.value` or `pull(node)`, records no dependency and is allowed. A later change to a node read that way does not mark the computing node.
- Marking a node `Dirty` keeps its edges. A `Dirty` node stays in the dependents lists of the nodes it read, and marking that reaches it again changes nothing. Its next computation keeps each edge whose dependency it reads at the same position.
- Disposing a node marks its readers `Dirty` and returns the node to `Uncomputed`, the state of a new node.
- `startBatch(body)` creates a batch, calls `body` with `mutators`, an instance of the unexported class `Mutators`, and flushes the batch when `body` returns. The methods `writeSource(source, value)`, `disposeDerivedNode(node)`, and `setDerivedNodeEvaluation(node, evaluation)` of `mutators` act in its batch.
- The top-level `writeSource(source, value)`, `disposeDerivedNode(node)`, and `setDerivedNodeEvaluation(node, evaluation)` are a batch of one: each acts in a batch of its own, then flushes it before it returns, so each behaves as a `startBatch` whose `body` makes the one call. An update that writes several sources belongs in one `startBatch`, so that no callback observes an intermediate state. Each single call flushes before it returns, so a callback that its flush runs observes the state between two writes of the update.
- A batch holds two arrays, `eagerNodes` and `callbacks`, each in the order they were added. `startBatch` creates them, and `mutators` holds them in the private fields `#eagerNodes` and `#callbacks`. A single call creates them, in a `Queue`, on its first push, so a single call that queues nothing allocates no queue. Marking adds a node when it takes it out of `Clean`, which happens at most once until the node is pulled or refreshed.
- A `startBatch` or a single call inside the `body` of another `startBatch` or inside a flush flushes its own batch before it returns. Its callbacks can run while eager nodes of the outer batch are queued, and they read those nodes' previous values. Joining the outer batch would need module state that records the running batch, and the engine holds no module state.
- A `DerivedNode` has a writable field `onDirty: (() => void) | undefined`. Marking adds a lazy node's `onDirty` to `callbacks` when the node goes from `Clean` to `Dirty` or `Check`. Two nodes that share one callback function add it twice.
- Marking that takes an eager node out of `Clean`, to `Dirty` or, through a derived dependency, to `Check`, appends the node to `eagerNodes` and does not mark the node's dependents.
- `setDerivedNodeEvaluation(node, evaluation)` and `mutators.setDerivedNodeEvaluation(node, evaluation)` set `node.evaluation`. Setting the evaluation a node has, or the evaluation of an `Uncomputed` or `Clean` node, changes only that field. A `Dirty` or `Check` node set eager is appended to `eagerNodes`, so the flush refreshes it. Its dependents are `Check` or `Dirty`, because it was lazy, and they stay as they are. A `Dirty` or `Check` node set lazy can have `Clean` dependents, because it was eager, so the call marks those dependents `Check` and the nodes above them as marking does above a `Dirty` node, collects their callbacks, queues the eager nodes among them, and adds the node's own `onDirty` to `callbacks`, as marking does when a lazy node leaves `Clean`. The lazy node stays in `eagerNodes`, and the flush skips a queued node that is lazy when the flush reaches it. A node set lazy and then eager again before the flush is in `eagerNodes` twice, and the second refresh finds it `Clean` or lazy. A lazy node whose `onDirty` marking collected and that is set eager while `Dirty` or `Check` has its `onDirty` collected again when the refresh changes its value.
- The flush refreshes the next queued eager node while one is queued, and otherwise calls the next callback, until both arrays are drained. Refreshing verifies a `Check` eager node and recomputes it if a dependency changed, and recomputes a `Dirty` one. A recompute that returns a value `===` the previous one marks nothing. A recompute whose value changed adds the eager node's own `onDirty` to `callbacks`, marks its dependents `Dirty` and the nodes above them `Check` through `markDirty`, and so collects their callbacks and queues the eager nodes among them into the batch being flushed. So every callback runs while no eager node of the batch is queued, and callbacks run in the order they were collected.
- A callback may pull, write, dispose, and set a node's evaluation, either through a `startBatch` or a single call of its own, which flushes its own batch before it returns, or through the methods of the batch being flushed, which append to its arrays, and the same flush drains them, eager nodes first. A collected callback is called even when an earlier callback disposed its node.
- A queued eager node that was disposed before the flush reaches it stays `Uncomputed`, and the flush does not compute it. A queued eager node that is `Clean` when the flush reaches it is left as it is, and so is a queued node that is lazy when the flush reaches it.
- A pull of a `Dirty` or `Check` eager node returns its previous value and runs nothing, whether the pull is outside a computation or a `readDerived` inside one. Only the flush of its batch refreshes it. A refresh inside a running computation could take out of `Clean` a node that the computation already read, while the running node is `Dirty` and marking skips it, and the running node would end `Clean` over a `Dirty` dependency that nothing marks again.

### Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior. The types enforce the field contract, except for `computation`.

- The caller writes a source only when its value changed.
- A computation, and any function it calls, does not throw. A callback, and the `body` of `startBatch`, do not throw.
- A computation does not write a source, dispose a node, set a node's evaluation, call `startBatch`, or call a method of `mutators`.
- A computation is synchronous. `readSource` and `readDerived` are called only inside a computation, with the node that computation received as `callingNode`.
- A computation's result depends only on `previous` and on the values its `readSource` and `readDerived` calls return. An untracked read, `source.value` or `pull(node)` inside a computation, is outside this contract: it records no dependency, so a later change to the node it read does not mark the computing node.
- The derived nodes form a directed acyclic graph, in which an untracked `pull` inside a computation counts as a read.
- A callback that writes eventually stops writing.
- The chains of derived nodes that a first pull or a pull of `Dirty` nodes descends stay well below the call stack limit. Each node of such a chain adds frames to the call stack. In Node 24.12 with its default stack and one-line computations, a first pull of a chain overflows at about 1,900 derived nodes, and a pull through a chain of `Dirty` derived nodes at about 1,850.
- A caller does not mutate a value that `pull`, `readSource`, or `readDerived` returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode`, `createDerivedNode`, and `createEagerDerivedNode`, and writes no field of a node except `onDirty`. Every field of `SourceNode` and `DerivedNode` except `onDirty` is `readonly`, so the type enforces this for every field except `computation`, a method, which a type cannot mark `readonly`.
- A caller calls each method of `mutators` through `mutators`, as in `mutators.writeSource(source, value)`. A method detached from `mutators`, or called through a `Proxy` of it, throws a `TypeError`, because it cannot reach the private fields of `mutators`.
- A caller calls the methods of `mutators` only until its `startBatch` returns.
- A caller disposes each derived node it stops using. Disposal is the only release. A derived node dropped without `disposeDerivedNode` stays in the dependents list of every node it read, so its edges, its value, and its computation closure stay reachable, every later write that reaches it visits it, and its `onDirty` is still called. In Node 24, 100,000 derived nodes that each read 34 sources, pulled once and then dropped without disposal, kept 237.6 MB reachable after two forced collections, against 0.04 MB when each was disposed first.
- A caller that needs current values reads after the batch flushes. Between a write and the flush of its batch, a marked eager node and the nodes that depend on it return their previous values. A `startBatch` or a single call inside `body` or inside a callback flushes its own batch while the outer batch's eager nodes are still queued, so its callbacks can read those nodes' previous values.

### Guarantees

pico-js upholds these guarantees while every contract holds.

- A `pull` made while no batch has an eager node queued, which includes every pull outside a `startBatch` body and outside a flush, returns the node's stored value. The stored value is the value the node's latest computation returned, or the previous value when the latest computation returned a value `===` to it. Each `readSource` and `readDerived` call of that latest computation returned the value that the node it read holds at the time of the pull.
- A computation runs only inside a `pull` or `readDerived` of its node or of a node that depends on it, or inside a flush that refreshes its node or an eager node that depends on it. It runs only when its node is `Uncomputed`, because it was never computed or was disposed, or when, since the node's latest computation, a source that computation read was written, or a derived node it read was disposed or recomputed to a value that is not `===` its previous value.
- A pull of a `Check` node pulls its derived dependencies in the order its latest computation read them, and recomputes the node as soon as one of those pulls leaves it `Dirty`, without pulling the rest. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node to a changed value. If no pull leaves the node `Dirty`, the node becomes `Clean` without running. A pull of a `Dirty` lazy node recomputes it without pulling its dependencies first.
- A recompute whose value is `===` to its node's previous value keeps the previous value, marks no reader, and collects no callback. `NaN` is not `===` to itself, so a recompute that returns `NaN` counts as changed. `-0 === 0`, so a recompute that returns `-0` where the stored value is `0` keeps `0`.
- Marking collects a lazy node's `onDirty` each time it takes the node out of `Clean`. The flush collects an eager node's `onDirty` each time it recomputes the node to a value that is not `===` its previous value. A call that sets a `Dirty` or `Check` eager node lazy collects that node's `onDirty`. Disposal does not collect the disposed node's own `onDirty`, and a pull collects no callback. Two nodes that share one callback function collect it twice, and a lazy node whose `onDirty` marking collected and that is set eager while `Dirty` or `Check` has its `onDirty` collected again when the flush changes its value.
- A batch calls each collected callback once, in the order the callbacks were collected, after its `body` returns or its single call acts. Before each callback, the flush refreshes every eager node queued in the batch. A collected callback is called even if its node is disposed or its `onDirty` is changed before the flush reaches it. A `startBatch` or a single call inside `body` or inside a callback flushes its own batch before it returns.
- When a flush returns, every eager node is `Clean` or `Uncomputed`, except an eager node queued in a batch whose flush has not returned. Only a pull computes an eager node for the first time. A pull of a `Dirty` or `Check` eager node, including a `readDerived` inside a computation, returns its previous value and runs nothing. An eager node can recompute more than once in one flush, when it reads another eager node that the flush refreshes after it.
- `mutators.writeSource`, `mutators.disposeDerivedNode`, and `mutators.setDerivedNodeEvaluation` run no computation and call no callback. A write walks the dependents list of the source, and of each lazy derived node it takes out of `Clean`, at most once each, and does not walk the dependents list of a `Dirty`, `Check`, or eager node it reaches. A write returns after storing the value, without visiting any derived node, when no `readSource` of the source has run since the source was created or last written.
- After a computation, its node is in the dependents list of each node that computation read through `readSource` and `readDerived`, and of no other node, until its next computation or its disposal. Marking unlinks no edge, so a `Dirty` or `Check` node keeps its edges.
- The reads of a recompute allocate no edge and link or unlink none when they match, in order, the edges of the previous run. A computation that makes the same reads on every run allocates no edge from run `r + 2` on, where `r` is the number of its reads that repeat the run's previous read of the same kind: a `readSource` of the source the previous `readSource` read, or a `readDerived` of the node the previous `readDerived` read.
- Marking and the verification of `Check` nodes keep their stacks in arrays, so a write, and a pull through a chain of `Check` nodes, add no call-stack frame per node. In Node 24.12, a pull through a chain of 200,000 `Check` nodes does not overflow.
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

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`. Every type field is writable except `Edge.dependent`: `value`, `status`, `dependentsStatus`, and the edge-list fields change on every write, read, and recompute, a reused edge changes its `dependency`, a caller sets `onDirty`, and `setDerivedNodeEvaluation` and `mutators.setDerivedNodeEvaluation` set `evaluation`. `Mutators` is the engine's one class. Its two arrays are ES private fields, `readonly` because a batch appends to them in place and never replaces them, and `index.ts` does not export `Mutators`, so a caller gets an instance only from `startBatch`. A `Queue`'s two arrays are `readonly` for the same reason. oxlint's `strict-boolean-expressions` (`.oxlintrc.json`) holds for every condition in the body. oxlint's `switch-exhaustiveness-check` holds for every `switch` in the body, and no `switch` has a `default` arm.

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

The graph has two kinds of node. A source node holds a value that code outside the graph writes. A derived node holds a computation and the value that computation last returned. A computation reads other nodes through `readSource` and `readDerived`, and each read records the node it read as a dependency. A node depends on exactly the nodes its latest computation read, so a computation that reads different nodes on different runs has different dependencies after each run. A node keeps its dependencies while it is `Dirty`, and its next computation reuses them in read order. Each read records one edge, of 64 bytes in Node 24, unless no edge of the previous run is left at its position and it is a `readSource` of the source that the run's previous `readSource` read, or a `readDerived` of the node that the run's previous `readDerived` read. A computation that reads `a, b, a` holds two edges to `a`. A computation that reads `a, a, b` holds two edges after its first run. Its second run reuses the edge to `b` for the second read of `a` and appends a third edge, to `b`, and later runs reuse those three.

A write runs no computation. It marks the derived nodes that read the source `Dirty`, because one of their inputs is known to have changed, and it marks every node that depends on those through derived nodes `Check`, because one of their inputs might have changed. A write that reaches a node that is already `Dirty` changes nothing. A source records that a write left every node that read it `Dirty`. No lazy node is recomputed until it is pulled.

`startBatch(body)` calls `body` with a batch and flushes the batch when `body` returns. The batch's `writeSource`, `disposeDerivedNode`, and `setDerivedNodeEvaluation` methods collect the `onDirty` callbacks of the observed nodes they take out of `Clean`, and the flush calls them after the last write, so a callback sees every write of its batch. The top-level `writeSource`, `disposeDerivedNode`, and `setDerivedNodeEvaluation` are a batch of one: each acts, then flushes before it returns. An update that writes several sources belongs in one `startBatch`, so that no callback observes an intermediate state. Each single call flushes before it returns, so a callback that its flush runs observes the state between two writes of the update. A `startBatch` or a single call inside `body` or inside a callback flushes its own batch before it returns.

An eager derived node, created by `createEagerDerivedNode` or set eager by `setDerivedNodeEvaluation(node, EAGER)`, stops marking. A write that marks it `Dirty` or `Check` queues it in the batch and marks none of its readers. The flush first recomputes every queued eager node, in the order they were queued. A recompute that returns the previous value marks nothing, so its readers stay `Clean` and no callback runs. A recompute whose value changed marks its readers as a write does. A selector that many readers read and that rarely changes value, such as `isSelected` for each row of a list, is eager, so a write marks only the readers whose selector changed. Until the flush, a marked eager node and the nodes that read it return their previous values.

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

startBatch((mutators) => {
  mutators.writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
  mutators.writeSource(person, { first: 'Ada', last: 'King', age: 38 });
});
pull(greeting); // 'Hello, Ada King', greetingRuns === 2
```

After the first write, `name` recomputes and returns `previous`, so `greeting` becomes `Clean` without running.

## API

- `createSourceNode(value)` returns a source node that holds `value`.
- `startBatch(body)` creates a batch, calls `body(mutators)`, and then flushes the batch. The flush recomputes the queued eager nodes, then calls the collected callbacks in the order they were collected. Before each callback, it recomputes the eager nodes that the earlier callbacks' calls queued.
- `mutators.writeSource(source, value)` stores `value` in `source`, marks the derived nodes that read it, and adds to the batch each eager node that marking reaches and the `onDirty` of each other observed node that leaves `Clean`. It runs no computation.
- `writeSource(source, value)` is `mutators.writeSource(source, value)` in a batch of its own, flushed before it returns. A write that queues an eager node or a callback allocates its queue, 128 bytes in Node 24 for a queue of one. A write that queues neither allocates nothing, unless marking takes out of `Clean` two or more lazy readers of the source that have readers of their own, or a lazy node that has readers of its own and is not the last entry of the dependents list that marking reached it through. Either case allocates one array for the marking stack. A write to a source with 100 lazy readers that each have one reader allocates about 2,360 bytes.
- `createDerivedNode(computation)` returns an `Uncomputed` lazy derived node whose value `computation(callingNode, previous)` computes.
- `createEagerDerivedNode(computation)` returns an `Uncomputed` eager derived node.
- `readSource(callingNode, source)` returns the value of `source` and records `source` as a dependency of `callingNode`.
- `readDerived(callingNode, node)` pulls `node`, returns its value, and records `node` as a dependency of `callingNode`.
- `pull(node)` brings `node` up to date and returns its value.
- `mutators.disposeDerivedNode(node)` marks the readers of `node` `Dirty`, adds their eager nodes and `onDirty` callbacks to the batch, and returns `node` to `Uncomputed`. `disposeDerivedNode(node)` does the same in a batch of its own.
- `mutators.setDerivedNodeEvaluation(node, evaluation)` makes `node` lazy (`LAZY`) or eager (`EAGER`). A `Dirty` or `Check` node set eager is queued in the batch, so the flush recomputes it. A `Dirty` or `Check` node set lazy marks its readers `Check`, as a write marks the nodes above a written source, and adds its own `onDirty` and theirs to the batch. A node of any other status only changes evaluation. `setDerivedNodeEvaluation(node, evaluation)` does the same in a batch of its own.
- `node.onDirty` is a callback that a write or disposal collects when the lazy node `node` leaves `Clean`, and that the flush collects when it recomputes the eager node `node` to a changed value. A collected callback is called even if its node is disposed or its `onDirty` is changed before the flush reaches it.
- `startBatch` calls `body` with `mutators`, an instance of `Mutators`, a class the package does not export. It holds the batch's eager nodes and callbacks in private fields.
- `SourceNode<TValue>` is the type of a source node whose value has type `TValue`.
- `DerivedNode<TValue>` is the type of a derived node whose value has type `TValue`.
- `Status` is the type of a derived node's `status` field, one of the exported constants `UNCOMPUTED`, `DIRTY`, `CHECK`, and `CLEAN`. A caller compares `node.status` with a constant, for example `node.status === CLEAN`.
- `DerivedNodeEvaluation` is the type of a derived node's `evaluation` field and of the `evaluation` parameter of `setDerivedNodeEvaluation` and `mutators.setDerivedNodeEvaluation`, one of the exported constants `LAZY` and `EAGER`.
- `DependentsStatus` is the type of a source node's `dependentsStatus` field: `1` means every derived node that read the source is `Dirty`, so a write visits none of them. A new source, which no derived node has read, starts at `1`. `0` means one of them may be `Check` or `Clean`, so a write marks them.

Each `readSource` and `readDerived` call during a computation records a dependency. A node's dependencies are the nodes its latest computation read. Code outside a computation reads a source's current value as `source.value`. A computation may also read untracked, through `source.value` or `pull(node)`, which records no dependency.

## Statuses

- An `Uncomputed` node (`status` `UNCOMPUTED`) has no value. Its next pull runs its computation with `previous` `undefined`. A computation whose value type includes `undefined` cannot tell its first run from a run after a stored `undefined`, because `previous` is `undefined` in both.
- A `Dirty` node (`status` `DIRTY`) read a source that was written, or a derived node whose value changed or that was disposed. Its next pull runs its computation with its last value as `previous`.
- A `Check` node (`status` `CHECK`) depends through derived nodes on a source that was written or on a derived node that was disposed. Its next pull pulls its derived dependencies in the order they were read and recomputes the node after the first pull that leaves it `Dirty`. If no pull leaves the node `Dirty`, the node becomes `Clean` without running.
- A `Clean` node (`status` `CLEAN`) holds an up-to-date value, which a pull returns without running anything, unless it depends on an eager node whose batch has not been flushed.
- A `Dirty` or `Check` eager node is queued in a batch. A pull returns its previous value without running anything, and the flush of that batch brings it up to date. A queued node that is set lazy before the flush is left for a pull to recompute.

## Cutoff

The engine compares a recompute's value with the previous value only by `===`. Structural sharing is the computation's responsibility: a computation that wants a structurally equal result to count as unchanged compares its new result with `previous` and returns `previous` when they are equal.

## Disposal

`disposeDerivedNode(node)` marks the readers of `node` `Dirty`, removes `node` from the dependents of every node it read, and discards its value. The node is `Uncomputed`, and its next pull computes it with `previous` `undefined`. Disposing an `Uncomputed` node changes nothing. A queued eager node that is disposed before the flush stays `Uncomputed`, and the flush does not compute it.

## Contracts

These are the caller's obligations. pico-js checks none of them at run time, and a program that breaks one has unspecified behavior. The types enforce the field contract, except for `computation`.

- The caller writes a source only when its value changed.
- A computation, and any function it calls, does not throw. A callback, and the `body` of `startBatch`, do not throw.
- A computation does not write a source, dispose a node, set a node's evaluation, call `startBatch`, or call a method of `mutators`.
- A computation is synchronous. `readSource` and `readDerived` are called only inside a computation, with the node that computation received as `callingNode`.
- A computation's result depends only on `previous` and on the values its `readSource` and `readDerived` calls return. An untracked read, `source.value` or `pull(node)` inside a computation, is outside this contract: it records no dependency, so a later change to the node it read does not mark the computing node.
- The derived nodes form a directed acyclic graph, in which an untracked `pull` inside a computation counts as a read.
- A callback that writes eventually stops writing.
- The chains of derived nodes that a first pull or a pull of `Dirty` nodes descends stay well below the call stack limit. Each node of such a chain adds frames to the call stack. In Node 24.12 with its default stack and one-line computations, a first pull of a chain overflows at about 1,900 derived nodes, and a pull through a chain of `Dirty` derived nodes at about 1,850.
- A caller does not mutate a value that `pull`, `readSource`, or `readDerived` returns, or a value it passed to `createSourceNode` or `writeSource`.
- A caller creates nodes only through `createSourceNode`, `createDerivedNode`, and `createEagerDerivedNode`, and writes no field of a node except `onDirty`. Every field of `SourceNode` and `DerivedNode` except `onDirty` is `readonly`, so the type enforces this for every field except `computation`, a method, which a type cannot mark `readonly`.
- A caller calls each method of `mutators` through `mutators`, as in `mutators.writeSource(source, value)`. A method detached from `mutators`, or called through a `Proxy` of it, throws a `TypeError`, because it cannot reach the private fields of `mutators`.
- A caller calls the methods of `mutators` only until its `startBatch` returns.
- A caller disposes each derived node it stops using. Disposal is the only release. A derived node dropped without `disposeDerivedNode` stays in the dependents list of every node it read, so its edges, its value, and its computation closure stay reachable, every later write that reaches it visits it, and its `onDirty` is still called. In Node 24, 100,000 derived nodes that each read 34 sources, pulled once and then dropped without disposal, kept 237.6 MB reachable after two forced collections, against 0.04 MB when each was disposed first.
- A caller that needs current values reads after the batch flushes. Between a write and the flush of its batch, a marked eager node and the nodes that depend on it return their previous values. A `startBatch` or a single call inside `body` or inside a callback flushes its own batch while the outer batch's eager nodes are still queued, so its callbacks can read those nodes' previous values.

## Guarantees

pico-js upholds these guarantees while every contract holds.

- A `pull` made while no batch has an eager node queued, which includes every pull outside a `startBatch` body and outside a flush, returns the node's stored value. The stored value is the value the node's latest computation returned, or the previous value when the latest computation returned a value `===` to it. Each `readSource` and `readDerived` call of that latest computation returned the value that the node it read holds at the time of the pull.
- A computation runs only inside a `pull` or `readDerived` of its node or of a node that depends on it, or inside a flush that refreshes its node or an eager node that depends on it. It runs only when its node is `Uncomputed`, because it was never computed or was disposed, or when, since the node's latest computation, a source that computation read was written, or a derived node it read was disposed or recomputed to a value that is not `===` its previous value.
- A pull of a `Check` node pulls its derived dependencies in the order its latest computation read them, and recomputes the node as soon as one of those pulls leaves it `Dirty`, without pulling the rest. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node to a changed value. If no pull leaves the node `Dirty`, the node becomes `Clean` without running. A pull of a `Dirty` lazy node recomputes it without pulling its dependencies first.
- A recompute whose value is `===` to its node's previous value keeps the previous value, marks no reader, and collects no callback. `NaN` is not `===` to itself, so a recompute that returns `NaN` counts as changed. `-0 === 0`, so a recompute that returns `-0` where the stored value is `0` keeps `0`.
- Marking collects a lazy node's `onDirty` each time it takes the node out of `Clean`. The flush collects an eager node's `onDirty` each time it recomputes the node to a value that is not `===` its previous value. A call that sets a `Dirty` or `Check` eager node lazy collects that node's `onDirty`. Disposal does not collect the disposed node's own `onDirty`, and a pull collects no callback. Two nodes that share one callback function collect it twice, and a lazy node whose `onDirty` marking collected and that is set eager while `Dirty` or `Check` has its `onDirty` collected again when the flush changes its value.
- A batch calls each collected callback once, in the order the callbacks were collected, after its `body` returns or its single call acts. Before each callback, the flush refreshes every eager node queued in the batch. A collected callback is called even if its node is disposed or its `onDirty` is changed before the flush reaches it. A `startBatch` or a single call inside `body` or inside a callback flushes its own batch before it returns.
- When a flush returns, every eager node is `Clean` or `Uncomputed`, except an eager node queued in a batch whose flush has not returned. Only a pull computes an eager node for the first time. A pull of a `Dirty` or `Check` eager node, including a `readDerived` inside a computation, returns its previous value and runs nothing. An eager node can recompute more than once in one flush, when it reads another eager node that the flush refreshes after it.
- `mutators.writeSource`, `mutators.disposeDerivedNode`, and `mutators.setDerivedNodeEvaluation` run no computation and call no callback. A write walks the dependents list of the source, and of each lazy derived node it takes out of `Clean`, at most once each, and does not walk the dependents list of a `Dirty`, `Check`, or eager node it reaches. A write returns after storing the value, without visiting any derived node, when no `readSource` of the source has run since the source was created or last written.
- After a computation, its node is in the dependents list of each node that computation read through `readSource` and `readDerived`, and of no other node, until its next computation or its disposal. Marking unlinks no edge, so a `Dirty` or `Check` node keeps its edges.
- The reads of a recompute allocate no edge and link or unlink none when they match, in order, the edges of the previous run. A computation that makes the same reads on every run allocates no edge from run `r + 2` on, where `r` is the number of its reads that repeat the run's previous read of the same kind: a `readSource` of the source the previous `readSource` read, or a `readDerived` of the node the previous `readDerived` read.
- Marking and the verification of `Check` nodes keep their stacks in arrays, so a write, and a pull through a chain of `Check` nodes, add no call-stack frame per node. In Node 24.12, a pull through a chain of 200,000 `Check` nodes does not overflow.
- pico-js holds no module state, so two graphs that share no node do not affect each other.
````

The Contracts and Guarantees lists copy the Model section's lists word for word, and the API, statuses, cutoff, and disposal sections describe `src/graph.ts` above.

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
type MutableDerivedNode<TValue> = Mutable<DerivedNode<TValue>>;

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

/**
 * Internal invariants of a derived node whose computation is not running:
 * - `Uncomputed`: `value` is `undefined`, both dependency lists are empty, and
 *   every dependent is `Dirty`. No dependents list holds the node.
 * - `Dirty`: `value` is the last computed value. A lazy node's dependents are
 *   `Check` or `Dirty`. An eager node is queued in an unflushed batch, and its
 *   dependents may be `Clean`.
 * - `Check`: as `Dirty`.
 * - `Clean`: `value` is up to date, unless the node depends through `Clean`
 *   nodes on a `Dirty` or `Check` eager node.
 *
 * In every status, each edge in the node's dependency lists is in its
 * dependency's dependents list.
 *
 * Contract: a caller writes no field except `onDirty`. The type enforces it.
 */
export type DerivedNode<TValue> = {
  readonly value: TValue;
  // Contract: the callback does not throw. A callback that writes eventually
  // stops writing.
  onDirty: (() => void) | undefined;
  readonly status: Status;
  readonly evaluation: DerivedNodeEvaluation;
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
 * nodes only through `readSource` and `readDerived` with the `callingNode` it
 * receives, and it does not write, dispose, set an evaluation, or start a
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
  return {
    value: undefined as unknown as TValue,
    onDirty: undefined,
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
}

/**
 * The object `startBatch` passes to its body. Its methods defer eager refreshes
 * and `onDirty` callbacks to the flush.
 *
 * Contract: a caller calls these methods through `mutators`, and only until
 * its `startBatch` returns.
 */
class Mutators {
  readonly #eagerNodes: Array<MutableDerivedNode<unknown>>;
  readonly #callbacks: Array<() => void>;

  constructor(
    eagerNodes: Array<MutableDerivedNode<unknown>>,
    callbacks: Array<() => void>,
  ) {
    this.#eagerNodes = eagerNodes;
    this.#callbacks = callbacks;
  }

  writeSource<TValue>(source: SourceNode<TValue>, value: TValue): void {
    const first = storeValue(source as MutableSourceNode<TValue>, value);
    if (first !== undefined) {
      markDirty(this.#eagerNodes, this.#callbacks, first);
    }
  }

  disposeDerivedNode(node: DerivedNode<unknown>): void {
    const first = node.firstDependent;
    resetToUncomputed(node as MutableDerivedNode<unknown>);
    if (first !== undefined) {
      markDirty(this.#eagerNodes, this.#callbacks, first);
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
            // As an eager node it left Clean without queuing its callback, so
            // the callback is queued now.
            if (mutableNode.onDirty !== undefined) {
              this.#callbacks.push(mutableNode.onDirty);
            }
            markCheck(
              this.#eagerNodes,
              this.#callbacks,
              mutableNode,
              undefined,
            );
            break;
        }
        break;
      // Internal invariant: a Clean node is up to date under either
      // evaluation, and an Uncomputed node has only Dirty dependents.
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
  const callbacks: Array<() => void> = [];
  body(new Mutators(eagerNodes, callbacks));
  flush(eagerNodes, callbacks);
}

/**
 * The eager nodes and callbacks of a batch of one, created on its first push,
 * so a single call that queues nothing allocates no queue.
 */
type Queue = {
  readonly eagerNodes: Array<MutableDerivedNode<unknown>>;
  readonly callbacks: Array<() => void>;
};

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
    flush(queue.eagerNodes, queue.callbacks);
  }
}

/** A batch of one disposal. */
export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const first = node.firstDependent;
  resetToUncomputed(node as MutableDerivedNode<unknown>);
  if (first === undefined) {
    return;
  }
  const queue = markDirtyQueue(first);
  if (queue !== undefined) {
    flush(queue.eagerNodes, queue.callbacks);
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
          flush([mutableNode], []);
          break;
        case LAZY: {
          // The callback is queued first, as in the batch's method.
          const queue = markCheckQueue(
            mutableNode.onDirty === undefined
              ? undefined
              : { eagerNodes: [], callbacks: [mutableNode.onDirty] },
            mutableNode,
            undefined,
          );
          if (queue !== undefined) {
            flush(queue.eagerNodes, queue.callbacks);
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

/** Leaves the dependents list for marking. */
function resetToUncomputed(node: MutableDerivedNode<unknown>): void {
  unlinkAll(node.firstSourceDependency);
  unlinkAll(node.firstDerivedDependency);
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = UNCOMPUTED;
  node.value = undefined;
}

/**
 * Guarantee: each callback runs while no eager node of the batch is queued.
 *
 * Internal invariant: a node queued twice, by setting it lazy and then eager,
 * is `Clean` or lazy the second time, so the eager list needs no
 * deduplication.
 */
function flush(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  callbacks: Array<() => void>,
): void {
  // Indices walk both arrays, because a refresh appends to them during the
  // walk.
  let eagerIndex = 0;
  let callbackIndex = 0;
  while (true) {
    // Every queued eager node is refreshed before the next callback runs. A
    // refresh whose value changed can queue more of both.
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
  const value = pull(node);
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
      unlinkEdge(next);
      next.dependency = node as DerivedNode<unknown>;
      linkEdge(next);
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
 * Guarantee: a `Dirty` or `Check` eager node returns its previous value and
 * runs nothing.
 */
export function pull<TValue>(node: DerivedNode<TValue>): TValue {
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

/**
 * A node set lazy after it was queued is skipped, because setting it lazy
 * marked its dependents.
 */
function refreshEager(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  callbacks: Array<() => void>,
  node: MutableDerivedNode<unknown>,
): void {
  if (node.evaluation === LAZY) {
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
    // Internal invariant: a Clean node was disposed and then pulled, or is in
    // the list twice. An Uncomputed node was disposed, and computing it would
    // relink its sources.
    case CLEAN:
    case UNCOMPUTED:
      break;
  }
}

function markDirty<TDependency>(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  callbacks: Array<() => void>,
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
      // Internal invariant: a Check node marked its dependents, and queued
      // its callback or itself, when it left Clean.
      case CHECK:
        node.status = DIRTY;
        break;
      // Internal invariant: a Dirty node was marked earlier. No dependents
      // list holds an Uncomputed node.
      case DIRTY:
      case UNCOMPUTED:
        break;
    }
  }
  // The dependents of the nodes marked Dirty might keep their values, so they
  // become Check.
  if (held !== undefined) {
    markCheck(eagerNodes, callbacks, held, stack);
  }
}

/**
 * Marks `Clean` dependents of `held` and of each node on the stack `Check`,
 * and continues through each lazy node it marks.
 */
function markCheck(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  callbacks: Array<() => void>,
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
          if (dependent.onDirty !== undefined) {
            callbacks.push(dependent.onDirty);
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
        if (node.onDirty !== undefined) {
          queue = queueCallback(queue, node.onDirty);
        }
        if (node.firstDependent !== undefined) {
          if (held !== undefined) {
            (stack ??= []).push(held);
          }
          held = node;
        }
        break;
      // Internal invariant: a Check node marked its dependents, and queued
      // its callback or itself, when it left Clean.
      case CHECK:
        node.status = DIRTY;
        break;
      // Internal invariant: a Dirty node was marked earlier. No dependents
      // list holds an Uncomputed node.
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
          if (dependent.onDirty !== undefined) {
            queue = queueCallback(queue, dependent.onDirty);
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
    return { eagerNodes: [node], callbacks: [] };
  }
  queue.eagerNodes.push(node);
  return queue;
}

/** `queueEagerNode` for a callback. */
function queueCallback(queue: Queue | undefined, callback: () => void): Queue {
  if (queue === undefined) {
    return { eagerNodes: [], callbacks: [callback] };
  }
  queue.callbacks.push(callback);
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
 *
 * Guarantee: an eager node's `onDirty` is collected only when its value
 * changed.
 */
function recomputeEager(
  eagerNodes: Array<MutableDerivedNode<unknown>>,
  callbacks: Array<() => void>,
  node: MutableDerivedNode<unknown>,
): void {
  const previous = node.value;
  const value = rerun(node, previous);
  // On cutoff the dependents stay Clean and no callback runs.
  if (value === previous) {
    return;
  }
  node.value = value;
  // Marking the node queued no callback, so its callback is queued now.
  if (node.onDirty !== undefined) {
    callbacks.push(node.onDirty);
  }
  // Eager dependents that this marks join the list the flush is walking.
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(eagerNodes, callbacks, first);
  }
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

/** Unlinks and drops the edges after the last one the computation read. */
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
    unlinkAll(node.firstDerivedDependency);
    node.firstDerivedDependency = undefined;
  } else if (lastDerived.nextDependency !== undefined) {
    unlinkAll(lastDerived.nextDependency);
    lastDerived.nextDependency = undefined;
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
- An eager node that is `Dirty` or `Check` may have `Clean` dependents, and only until the flush of the batch whose `eagerNodes` holds it. Marking queues it when it leaves `Clean`, and `setDerivedNodeEvaluation` and `mutators.setDerivedNodeEvaluation` queue a `Dirty` or `Check` node it makes eager, so every `Dirty` or `Check` eager node is queued. The flush refreshes it at the top level, outside any computation, through `refreshEager`. `recomputeEager` marks a changed value through `markDirty`, which takes the arrays of the batch being flushed, takes `Clean` dependents out of `Clean`, collects their callbacks, and queues the eager nodes among them. It collects the eager node's own `onDirty` there, when its value changed, and not when marking reached the node, so an observer of an eager node is called only when the node's value changed.
- `mutators.setDerivedNodeEvaluation` and `setDerivedNodeEvaluation` restore the lazy invariant when they make a `Dirty` or `Check` eager node lazy: they mark the node's `Clean` dependents `Check` through `markCheck` or `markCheckQueue`, the second phase of `markDirty`, which starts from the node as if marking had just taken it out of `Clean`. Leaving the node queued and its dependents `Clean` until the flush would break the invariant until then: a pull of the lazy `Dirty` node would recompute it, and `markCheckDependentsDirty` would leave its `Clean` dependents `Clean` and stale, and marking that reached the node would stop there and leave its `Clean` dependents stale. `refreshEager` skips a lazy node, because its dependents are already `Check` or `Dirty` and its `onDirty` is already collected, and a refresh would collect it again.
- `startBatch` allocates the two arrays and one `Mutators` per call, and the caller's `body` is a closure. The methods of `Mutators` are prototype methods over its private fields. A single call allocates no queue until marking pushes, and then one `Queue` and its two arrays. `markDirtyQueue` and `markCheckQueue` are copies of `markDirty` and `markCheck` that create the queue on the first push, through `queueEagerNode` and `queueCallback`, with the array that receives the push as a literal that holds the pushed element. A literal of one element has room for one, and a push onto an empty array allocates room for 17, so a `writeSource` that queues one callback allocates 128.7 bytes against 257.0 for a queue of two empty arrays, and one that queues one eager node 128.8 against 257.2 (Node 24.12, `heapUsed` over 20,000 writes). Per write, median of six processes of `bench/single` with two more cases, a `writeSource` to a source whose one reader has an `onDirty`, with a pull after each write, takes 12.6 ns against 19.7 ns for the queue of two empty arrays, and one to a source whose one reader is eager 11.9 ns against 16.8 ns. The two helpers are functions of their own because the same branches written inside `markDirtyQueue` and `markCheckQueue` slowed a write to a source that one lazy derived node reads, which queues nothing, from 5.1 ns to 7.6 ns. With the helpers it takes 5.2 ns. One `markDirty` that took an optional queue and returned it slowed `startBatch`: on Node 24.12, median of five processes, a batch of one write with an `onDirty` took 24.9 ns against 22.8 ns, and a batch of two writes 29.2 ns against 23.7 ns. Creating the queue in `startBatch` and passing it to that `markDirty` left 23.8 ns and 25.0 ns. With the copies, every batched case is within 0.2 ns of the engine without single calls. Per write, median of five processes of nine rounds of 2,000,000 writes (`bench/single`): 1.1 ns for `writeSource` against 4.3 ns for a `startBatch` of one `mutators.writeSource` to a source that no derived node has read, and 5.1 ns against 14.6 ns to a source that one derived node reads, with a pull after each write.
- `flush` reads both arrays by index and checks `eagerNodes` before each callback, so the entries that a refresh or a callback appends are drained in the same flush, and no callback runs while an eager node of the batch is queued. It does not empty the arrays, because a batch is flushed once, when its `startBatch` body or its single call returns.
- While a computation runs, its node is `Uncomputed` or `Dirty` and is linked to what it has read so far. Every node it read is `Clean`, or is a `Dirty` or `Check` eager node whose previous value it read. A pull inside the computation recomputes only lazy nodes, which take no node out of `Clean`, so nothing takes a node the computation read out of `Clean` before the computation returns, and marking never reaches the running node while it is `Uncomputed`.
- `verify` walks the derived dependency lists in read order with an explicit stack of edges. It descends into a lazy `Check` dependency before its reader continues and pulls any other dependency, so a `Check` eager dependency returns its previous value. After a pull that leaves the current node `Dirty`, it recomputes that node and returns to the node's reader. It does not recompute `root`. It returns the status it leaves `root` in, `DIRTY` or `CLEAN`, and its caller recomputes a `Dirty` root: `pull` through `recompute`, and `refreshEager` through `recomputeEager`. The return value carries the status because TypeScript narrows `node.status` in the caller's `switch` and does not see `verify` change it. The order of pulls and recomputes is that of a recursive `verify`. It pushes an edge only when the dependency it descends into has more than one dependent, and it allocates the stack on the first push. A node with one dependent returns to its reader through its only dependents-list edge, which is the edge `verify` descended through. A node `verify` descended into has at least one dependent, and one with more than one has an edge on the stack. `verify` does not check either contract. A first computation still recurses through `readDerived` and `pull`.
- Because a write marks the source's direct readers `Dirty`, a `Check` node has no changed source, and `verify` pulls only its derived dependencies. A dependency's `recompute` marks its readers `Dirty` exactly when its new value is not `===` its previous value, so after a pull in `verify` the node is `Dirty` exactly when a lazy dependency it read changed value. A `Dirty` or `Check` eager dependency returns its previous value and leaves the node as it is, and its flush marks the node if its value changed.
- `status === UNCOMPUTED` alone records that a node has no value. An `Uncomputed` node's `value` is `undefined`, cast to `TValue` once in `newDerivedNode` and assigned in `disposeDerivedNode`, where `value` is `unknown` and needs no cast. In the other three statuses, `value` holds a value the computation returned, which can be `undefined`, so a `Dirty` node always has a value for its next computation to receive as `previous`, and a pull of a `Clean` or `Check` node reads `node.value` directly.
- The `DerivedNode` doc comment lists internal invariants: the engine's state between computations, which the engine keeps and the README does not list. pico-js checks none of them at run time, so a bug that breaks one returns a wrong value or throws a `TypeError` instead of an error that names the invariant. The README's Contracts section lists the caller's obligations, and its Guarantees section lists what pico-js upholds while they hold. A comment in `graph.ts` that states a caller's obligation starts with `Contract:`, and one that states what the engine ensures starts with `Guarantee:` or `Internal invariant:`.
- The exported `SourceNode` and `DerivedNode` mark every field `readonly` except `onDirty`, so a caller's write to any other field is a type error, and the type test checks it. `computation` is a method, which a type cannot mark `readonly`. It stays a method so that `DerivedNode<TValue>` is assignable to `DerivedNode<unknown>`. Inside `graph.ts`, `Mutable<T>` removes `readonly`, and the engine writes through `MutableSourceNode` and `MutableDerivedNode`. Each exported function and each method of `Mutators` casts its node arguments to the mutable view once. `Edge` holds the public node types, because the public edge-list fields expose edges, and an edge that held the mutable view would let a caller write a node's fields through `node.firstDependent.dependent`. So `markDirty`, `markCheck`, their queue copies, `markCheckDependentsDirty`, and `verify` cast each node they reach through an edge. `linkEdge`, `unlinkEdge`, and `unlinkAll` write a node's dependents-list fields through the constraint `NodeWithDependents`, whose fields are mutable, and TypeScript lets a `readonly` field satisfy it. `Edge.dependent` is `readonly`. `Edge.dependency` is repointed when a recompute reads a different node at that position, and appends, `trim`, `linkEdge`, and `unlinkEdge` rewrite `nextDependency`, `previousDependent`, and `nextDependent`.

### Checks for a future debug build

A `__DEV__` build can restore these checks. Each throws `new Error(message)`.

- `markDirty`, `markCheck`, and `markCheckDependentsDirty`: a dependent's `status === UNCOMPUTED`; "An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js."
- `markCheckDependentsDirty`: a dependent's `status === CLEAN`; "A Clean node depends on a Dirty node. This is indicative of a bug in pico-js."
- `verify`: `node.firstDependent === undefined` for a node it descended into; "A node that verify descended into has no dependents. This is indicative of a bug in pico-js."
- `verify`: `verifyStack?.pop() === undefined` for a node it descended into with more than one dependent; "A node that verify descended into has more than one dependent and no edge on the verification stack. This is indicative of a bug in pico-js."
- `pull`, `recompute`, and `recomputeEager`: a node whose `status !== UNCOMPUTED` holds no computed value, which needs a private `NOT_SET` sentinel in `value` while `Uncomputed`; "A node that is not Uncomputed has no value. This is indicative of a bug in pico-js."
- A first computation has no cutoff and marks nothing, because every node in an `Uncomputed` node's dependents list is already `Dirty`: a new node has no readers, and disposal marked every reader `Dirty`.
- On cutoff the node keeps its stored value, so a recompute that returns `-0` where the stored value is `0` leaves `0` stored and returned, and every reader of the node reads the same value.
- Disposal marks every reader `Dirty`, unlinks the node's own edges, and discards its value, so its next computation receives `previous` `undefined`. The readers stay in the node's dependents list, and each reader's next recompute either reads the node again, which computes it first, or drops the edge. An `Uncomputed` node has empty dependency lists and only `Dirty` dependents, so its disposal changes nothing. A `Dirty` node's dependents are `Check` or `Dirty`, and its disposal marks them `Dirty`. A disposed eager node that is still in `eagerNodes` is `Uncomputed` when the flush reaches it, and `refreshEager` leaves it `Uncomputed`, because computing it would link it to its sources again. A `Clean` eager node in `eagerNodes` is one that was disposed and then pulled, or one that is in the list twice, and `refreshEager` leaves it as it is.

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
  disposeDerivedNode,
  EAGER,
  LAZY,
  pull,
  readDerived,
  readSource,
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

function dispose(node: DerivedNode<unknown>): void {
  startBatch((mutators) => mutators.disposeDerivedNode(node));
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
    write(source, 2);
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
    write(a, 2);
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
    pull(top);
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
    pull(label);
    write(a, 0);
    write(b, 3);
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
    write(a, 3);
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
    write(a, 2);
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
    write(a, 10);
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
    write(source, 2);
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
    write(gateSource, 2);
    write(otherSource, 2);
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
    write(bInput, 2);
    write(a, false);
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
    write(useA, false);
    expect(pull(pick)).toBe(2);
    write(a, 10);
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
    write(a, 3);
    expect(pull(sum)).toBe(5);
    write(b, 4);
    expect(sum.status).toBe(DIRTY);
    expect(pull(sum)).toBe(7);
    write(a, 5);
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
    write(a, 3);
    expect(hasDependent(b, sum)).toBe(true);
    write(b, 4);
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
    write(a, 2);
    expect(pull(node)).toBe(14);
    write(b, 20);
    expect(pull(node)).toBe(24);
    write(a, 3);
    write(b, 30);
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
    write(count, 1);
    expect(pull(node)).toBe(0);
    write(third, 100);
    expect(node.status).toBe(CLEAN);
    write(count, 3);
    expect(pull(node)).toBe(101);
    write(third, 200);
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
    write(a, 2);
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
    dispose(inner);
    expect(inner.status).toBe(UNCOMPUTED);
    expect(outer.status).toBe(DIRTY);
    expect(pull(outer)).toBe(2);
    expect(runs).toEqual({ inner: 2, outer: 2 });
    write(source, 2);
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
    dispose(inner);
    expect(inner.status).toBe(UNCOMPUTED);
    expect(hasDependent(source, inner)).toBe(false);
    write(source, 2);
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
    write(source, 2);
    expect(inner.status).toBe(DIRTY);
    expect(outer.status).toBe(CHECK);
    dispose(inner);
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
    dispose(node);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect(calls).toEqual([]);
    });
    expect(calls).toEqual(['double']);
  });

  test('two writes in one batch call each onDirty once, after both writes', () => {
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.writeSource(b, 2);
      expect(calls).toEqual([]);
    });
    expect(calls).toEqual(['readsA', 'readsB']);
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
    startBatch((mutators) => {
      mutators.writeSource(b, 3);
      mutators.writeSource(a, 4);
      mutators.writeSource(b, 5);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 10);
      mutators.writeSource(b, 20);
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
    write(a, 2);
    write(a, 3);
    expect(calls).toBe(1);
    pull(double);
    write(a, 4);
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
    write(a, 2);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
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
      dispose(second);
    };
    second.onDirty = () => calls.push('second');
    write(a, 2);
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
    startBatch((mutators) => {
      readsA.onDirty = () => {
        calls.push('readsA');
        mutators.writeSource(b, 2);
      };
      readsB.onDirty = () => calls.push('readsB');
      mutators.writeSource(a, 2);
    });
    expect(calls).toEqual(['readsA', 'readsB']);
  });

  test('a callback that pulls its node and is marked again in the same flush is called again', () => {
    const calls: Array<number> = [];
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    pull(node);
    startBatch((mutators) => {
      node.onDirty = () => {
        const value = pull(node);
        calls.push(value);
        if (value < 3) {
          mutators.writeSource(a, value + 1);
        }
      };
      mutators.writeSource(a, 2);
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
    write(a, 2);
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
    dispose(inner);
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
    write(a, 3);
    write(a, 4);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.writeSource(b, 2);
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
    write(a, 2);
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
    startBatch((mutators) => {
      readsA.onDirty = () => {
        calls.push('readsA');
        mutators.writeSource(b, 2);
      };
      readsANext.onDirty = () =>
        calls.push(`readsANext, eagerB ${eagerB.status}`);
      readsEager.onDirty = () => calls.push('readsEager');
      mutators.writeSource(a, 2);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect(eager.status).toBe(DIRTY);
      mutators.disposeDerivedNode(eager);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 5);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 10);
      mutators.writeSource(b, 20);
      mutators.writeSource(a, 30);
    });
    expect(runs).toBe(2);
    expect(pull(eager)).toBe(50);
  });
});

describe('nested batches', () => {
  test('a startBatch inside a callback flushes its own batch before the callback continues', () => {
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
      write(b, 2);
      calls.push(`readsA after write, eagerB ${eagerB.status}`);
    };
    readsB.onDirty = () => calls.push('readsB');
    readsEager.onDirty = () => calls.push(`readsEager ${pull(readsEager)}`);
    readsANext.onDirty = () => calls.push('readsANext');
    write(a, 2);
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
    pull(reader);
    inner.onDirty = () => calls.push(`inner ${eager.status}`);
    eager.onDirty = () => calls.push('eager');
    reader.onDirty = () => calls.push('reader');
    return { a, calls, inner, eager, reader };
  }

  function statuses(graph: Graph): ReadonlyArray<number> {
    return [graph.inner.status, graph.eager.status, graph.reader.status];
  }

  test('writeSource calls the same callbacks and leaves the same statuses and values as a batch of one write', () => {
    const single = build();
    const batched = build();
    writeSource(single.a, 2);
    startBatch((mutators) => mutators.writeSource(batched.a, 2));
    expect(single.calls).toEqual([`inner ${CLEAN}`, 'eager', 'reader']);
    expect(batched.calls).toEqual(single.calls);
    expect(statuses(batched)).toEqual(statuses(single));
    expect(
      [single.inner, single.eager, single.reader].map((node) => pull(node)),
    ).toEqual([2, 4, 5]);
    expect(
      [batched.inner, batched.eager, batched.reader].map((node) => pull(node)),
    ).toEqual([2, 4, 5]);
  });

  test('disposeDerivedNode calls the same callbacks and leaves the same statuses and values as a batch of one disposal', () => {
    const single = build();
    const batched = build();
    disposeDerivedNode(single.eager);
    startBatch((mutators) => mutators.disposeDerivedNode(batched.eager));
    expect(single.calls).toEqual(['reader']);
    expect(batched.calls).toEqual(single.calls);
    expect(statuses(single)).toEqual([CLEAN, UNCOMPUTED, DIRTY]);
    expect(statuses(batched)).toEqual(statuses(single));
    expect(pull(single.reader)).toBe(3);
    expect(pull(batched.reader)).toBe(3);
  });

  test('setDerivedNodeEvaluation calls the same callbacks and leaves the same statuses and values as a batch of one evaluation change', () => {
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
    expect(single.calls).toEqual([`inner ${CHECK}`, 'eager', 'eager']);
    expect(batched.calls).toEqual(single.calls);
    expect(statuses(single)).toEqual([CLEAN, CLEAN, DIRTY]);
    expect(statuses(batched)).toEqual(statuses(single));
    expect(pull(single.reader)).toBe(7);
    expect(pull(batched.reader)).toBe(7);
  });

  test('a writeSource inside a callback flushes before the callback continues', () => {
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
    readsA.onDirty = () => {
      calls.push('readsA');
      writeSource(b, 2);
      calls.push('readsA after write');
    };
    readsB.onDirty = () => calls.push('readsB');
    writeSource(a, 2);
    expect(calls).toEqual(['readsA', 'readsB', 'readsA after write']);
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
    node.onDirty = () => {
      calls += 1;
    };
    setEvaluation(node, EAGER);
    expect(node.evaluation).toBe(EAGER);
    expect(node.status).toBe(UNCOMPUTED);
    pull(node);
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
    expect(pull(top)).toBe(11);
    setEvaluation(n, LAZY);
    write(s, 2);
    expect(n.status).toBe(DIRTY);
    expect(top.status).toBe(CHECK);
    setEvaluation(n, EAGER);
    expect(n.status).toBe(CLEAN);
    expect(pull(n)).toBe(20);
    expect(pull(top)).toBe(21);
    write(s, 3);
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
    write(a, -1);
    node.onDirty = () => calls.push('node');
    reader.onDirty = () => calls.push('reader');
    startBatch((mutators) => {
      mutators.setDerivedNodeEvaluation(node, EAGER);
      expect(node.status).toBe(DIRTY);
      expect(runs).toBe(1);
    });
    expect(runs).toBe(2);
    expect(node.status).toBe(CLEAN);
    expect(reader.status).toBe(DIRTY);
    expect(calls).toEqual(['node']);
    expect(pull(reader)).toBe('not positive');
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
    pull(node);
    write(a, 3);
    expect(node.status).toBe(CHECK);
    setEvaluation(node, EAGER);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.setDerivedNodeEvaluation(eager, LAZY);
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.setDerivedNodeEvaluation(eager, LAZY);
      mutators.setDerivedNodeEvaluation(eager, EAGER);
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

    startBatch((mutators) => {
      mutators.setDerivedNodeEvaluation(node, LAZY);
      mutators.writeSource(a, 2);
    });
    expect([node.evaluation, node.status, reader.status]).toEqual([
      LAZY,
      DIRTY,
      CHECK,
    ]);
    expect(calls).toEqual([`reader ${CHECK}`]);

    startBatch((mutators) => mutators.setDerivedNodeEvaluation(node, EAGER));
    expect([node.status, reader.status, runs]).toEqual([CLEAN, DIRTY, 2]);
    expect(pull(reader)).toBe(102);

    startBatch((mutators) => mutators.writeSource(a, 5));
    expect([node.status, reader.status, runs]).toEqual([CLEAN, CLEAN, 3]);
    expect(calls).toEqual([`reader ${CHECK}`]);

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
    expect(calls).toEqual([`reader ${CHECK}`, `reader ${CHECK}`]);
    expect(pull(reader)).toBe(100);

    startBatch((mutators) => {
      mutators.setDerivedNodeEvaluation(node, EAGER);
      mutators.writeSource(a, 7);
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
    expect(pull(top)).toBe(3);
    writeSource(source, 2);
    expect([bottom.status, middle.status, top.status]).toEqual([
      DIRTY,
      CHECK,
      CHECK,
    ]);
    expect(pull(top)).toBe(4);
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
    expect(pull(top)).toBe(99_999);
    writeSource(source, 1);
    expect(top.status).toBe(CHECK);
    expect(pull(top)).toBe(100_000);
    expect(top.status).toBe(CLEAN);
  });

  test('a pull of the top of a chain of 100,000 Check nodes above a node that cuts off runs none of them and leaves them Clean', () => {
    let runs = 0;
    const source = createSourceNode(0);
    const bottom = createDerivedNode((callingNode) => {
      readSource(callingNode, source);
      return 0;
    });
    pull(bottom);
    const nodes: Array<DerivedNode<number>> = [];
    const top = Array.from({ length: 99_999 }).reduce<DerivedNode<number>>(
      (below) => {
        const node = createDerivedNode((callingNode) => {
          runs += 1;
          return readDerived(callingNode, below) + 1;
        });
        pull(node);
        nodes.push(node);
        return node;
      },
      bottom,
    );
    expect(runs).toBe(99_999);
    writeSource(source, 1);
    expect(top.status).toBe(CHECK);
    expect(pull(top)).toBe(99_999);
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
    expect(pull(node)).toBe(true);
    writeSource(a, 2);
    writeSource(b, 2);
    expect(node.status).toBe(CHECK);
    expect(pull(node)).toBe(true);
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
    expect(pull(top)).toBe(10);
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
    expect(pull(graph.top)).toBe(15);
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
    expect(pull(graph.top)).toBe(10);
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
    expect(pull(graph.right)).toBe(9);
    expect(graph.runs).toEqual({
      bottom: 2,
      shared: 2,
      left: 1,
      right: 2,
      top: 1,
    });
    expect([graph.left.status, graph.top.status]).toEqual([DIRTY, DIRTY]);
    expect(pull(graph.top)).toBe(15);
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
    expect(pull(graph.right)).toBe(6);
    expect([
      graph.shared.status,
      graph.left.status,
      graph.right.status,
      graph.top.status,
    ]).toEqual([CLEAN, CHECK, CLEAN, CHECK]);
    expect(pull(graph.top)).toBe(10);
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
      expect(pull(top)).toBe('1:1');
      writeSource(source, 3);
      expect([base.status, parity.status, top.status]).toEqual([
        DIRTY,
        CHECK,
        CHECK,
      ]);
      expect(pull(top)).toBe('1:3');
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
      top.forEach((node) => pull(node));
      const calls: Array<string> = [];
      direct.forEach((node, index) => {
        node.onDirty = () => calls.push(`direct ${index}`);
      });
      middle.forEach((node, index) => {
        node.onDirty = () => calls.push(`middle ${index}`);
      });
      top.forEach((node, index) => {
        node.onDirty = () => calls.push(`top ${index}`);
      });
      writeWith(a, 2);
      expect(direct.map((node) => node.status)).toEqual([DIRTY, DIRTY, DIRTY]);
      expect(middle.map((node) => node.status)).toEqual([CHECK, CHECK, CHECK]);
      expect(top.map((node) => node.status)).toEqual([CHECK, CHECK, CHECK]);
      expect([...calls].sort()).toEqual([
        'direct 0',
        'direct 1',
        'direct 2',
        'middle 0',
        'middle 1',
        'middle 2',
        'top 0',
        'top 1',
        'top 2',
      ]);
      expect(top.map((node) => pull(node))).toEqual([2, 3, 4]);
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
      third.forEach((node) => pull(node));
      const calls: Array<string> = [];
      [...first, ...second, ...third].forEach((node, index) => {
        node.onDirty = () => calls.push(`${index}`);
      });
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
      expect([...calls].sort()).toEqual([
        '0',
        '1',
        '2',
        '3',
        '4',
        '5',
        '6',
        '7',
        '8',
      ]);
      expect(third.map((node) => pull(node))).toEqual([2, 3, 4]);
    });
  }

  test('writeSource marks a Check direct reader Dirty, and calls its onDirty only when it left Clean', () => {
    let calls = 0;
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
    pull(node);
    node.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 2);
    expect(node.status).toBe(CHECK);
    writeSource(b, 20);
    expect(node.status).toBe(DIRTY);
    expect(calls).toBe(1);
    expect(pull(node)).toBe(20);
    expect(runs).toBe(2);
  });

  test('writeSource does not call the onDirty of a Check node a second time when a second write reaches it', () => {
    let calls = 0;
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
    pull(above);
    top.onDirty = () => {
      calls += 1;
    };
    above.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 2);
    expect([top.status, above.status]).toEqual([CHECK, CHECK]);
    expect(calls).toBe(2);
    writeSource(b, 2);
    expect([top.status, above.status]).toEqual([CHECK, CHECK]);
    expect(calls).toBe(2);
    expect(pull(above)).toBe(4);
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
    pull(aboveDirect);
    pull(aboveIndirect);
    let calls = 0;
    aboveDirect.onDirty = () => {
      calls += 1;
    };
    aboveIndirect.onDirty = () => {
      calls += 1;
    };
    direct.onDirty = () => {
      calls += 1;
    };
    indirect.onDirty = () => {
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
    [...direct, ...aboveLazy].forEach((node) => pull(node));
    refreshed.length = 0;
    writeSource(a, 2);
    expect(refreshed).toEqual(['direct 0', 'direct 1']);
    writeSource(b, 3);
    expect(refreshed).toEqual(['direct 0', 'direct 1', 'above 0', 'above 1']);
    expect([...direct, ...aboveLazy].map((node) => node.value)).toEqual([
      2, 2, 3, 3,
    ]);
  });

  test('writeSource calls the onDirty of a chain of four observed nodes in the order they left Clean', () => {
    const calls: Array<string> = [];
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
    pull(fourth);
    first.onDirty = () => calls.push('first');
    second.onDirty = () => calls.push('second');
    third.onDirty = () => calls.push('third');
    fourth.onDirty = () => calls.push('fourth');
    writeSource(a, 2);
    expect(calls).toEqual(['first', 'second', 'third', 'fourth']);
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
    pull(outer);
    writeSource(a, 1);
    expect([reader.status, outer.status]).toEqual([DIRTY, CHECK]);
    expect(pull(outer)).toBe(2);
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
    expect(pull(first)).toBe(2);
    expect(a.dependentsStatus).toBe(0);
    writeSource(a, 3);
    expect(a.dependentsStatus).toBe(1);
    expect(first.status).toBe(DIRTY);
    const second = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    expect(pull(second)).toBe(3);
    expect(a.dependentsStatus).toBe(0);
    startBatch((mutators) => mutators.writeSource(a, 4));
    expect(a.dependentsStatus).toBe(1);
    expect([first.status, second.status]).toEqual([DIRTY, DIRTY]);
    expect([pull(first), pull(second)]).toEqual([4, 4]);
  });
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
    expect(Object.is(pull(reader), 0)).toBe(true);
    writeSource(a, -1);
    expect(Object.is(pull(node), 0)).toBe(true);
    expect(Object.is(node.value, 0)).toBe(true);
    expect(Object.is(pull(reader), 0)).toBe(true);
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
    expect(pull(node)).toBe(200);
    expectSame(sourceDependenciesOf(node), [a]);
    expectSame(derivedDependenciesOf(node), [inner]);
    expectSame(dependentsOf(a), [node, inner]);
    expectSame(dependentsOf(inner), [node]);
    writeSource(a, 2);
    expect(pull(node)).toBe(400);
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
    pull(node);
    const firstSourceEdge = node.firstSourceDependency;
    const lastSourceEdge = node.lastSourceDependency;
    const firstDerivedEdge = node.firstDerivedDependency;
    const lastDerivedEdge = node.lastDerivedDependency;
    writeSource(a, 3);
    expect(pull(node)).toBe(10);
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
    expect(pull(pick)).toBe(1);
    const edge = pick.firstDerivedDependency;
    writeSource(useLeft, false);
    expect(pull(pick)).toBe(10);
    expect(pick.firstDerivedDependency === edge).toBe(true);
    expectSame(derivedDependenciesOf(pick), [right]);
    expectSame(dependentsOf(left), []);
    expectSame(dependentsOf(right), [pick]);
    writeSource(leftInput, 2);
    expect(pick.status).toBe(CLEAN);
    writeSource(rightInput, 20);
    expect(pick.status).toBe(CHECK);
    expect(pull(pick)).toBe(20);
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
    expect(pull(node)).toBe(7);
    expectSame(derivedDependenciesOf(node), derived);
    writeSource(count, 1);
    expect(pull(node)).toBe(1);
    expectSame(derivedDependenciesOf(node), [derived[0]]);
    expectSame(dependentsOf(derived[1]!), []);
    expectSame(dependentsOf(derived[2]!), []);
    writeSource(inputs[2]!, 8);
    expect(node.status).toBe(CLEAN);
    writeSource(count, 0);
    expect(pull(node)).toBe(0);
    expectSame(derivedDependenciesOf(node), []);
    expect(node.lastDerivedDependency === undefined).toBe(true);
    expectSame(dependentsOf(derived[0]!), []);
    writeSource(inputs[0]!, 16);
    expect(node.status).toBe(CLEAN);
    writeSource(count, 3);
    expect(pull(node)).toBe(26);
    expectSame(derivedDependenciesOf(node), derived);
    writeSource(inputs[2]!, 32);
    expect(node.status).toBe(CHECK);
    expect(pull(node)).toBe(50);
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
    expect(pull(node)).toBe(7);
    expectSame(sourceDependenciesOf(node), sources);
    writeSource(countInput, 1);
    expect(pull(node)).toBe(1);
    expectSame(sourceDependenciesOf(node), [sources[0]]);
    expectSame(dependentsOf(sources[1]!), []);
    expectSame(dependentsOf(sources[2]!), []);
    writeSource(countInput, 0);
    expect(pull(node)).toBe(0);
    expectSame(sourceDependenciesOf(node), []);
    expect(node.lastSourceDependency === undefined).toBe(true);
    expectSame(dependentsOf(sources[0]!), []);
    writeSource(sources[0]!, 16);
    expect(node.status).toBe(CLEAN);
    writeSource(countInput, 3);
    expect(pull(node)).toBe(22);
    expectSame(sourceDependenciesOf(node), sources);
    writeSource(sources[1]!, 32);
    expect(node.status).toBe(DIRTY);
    expect(pull(node)).toBe(52);
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
    expect(pull(node)).toBe(-9);
    writeSource(forward, false);
    expect(pull(node)).toBe(9);
    expectSame(sourceDependenciesOf(node), [forward, b, a]);
    expectSame(dependentsOf(a), [node]);
    expectSame(dependentsOf(b), [node]);
    writeSource(a, 2);
    expect(node.status).toBe(DIRTY);
    expect(pull(node)).toBe(8);
    writeSource(b, 20);
    expect(node.status).toBe(DIRTY);
    expect(pull(node)).toBe(18);
  });

  test('removing the first, the middle, and the last reader of a source leaves the others in its dependents list in read order', () => {
    const a = createSourceNode(1);
    const readers = [0, 1, 2, 3].map((offset) =>
      createDerivedNode((callingNode) => readSource(callingNode, a) + offset),
    );
    readers.forEach((node) => pull(node));
    expectSame(dependentsOf(a), readers);
    disposeDerivedNode(readers[0]!);
    expectSame(dependentsOf(a), [readers[1], readers[2], readers[3]]);
    disposeDerivedNode(readers[2]!);
    expectSame(dependentsOf(a), [readers[1], readers[3]]);
    disposeDerivedNode(readers[3]!);
    expectSame(dependentsOf(a), [readers[1]]);
    expect(a.lastDependent?.dependent === readers[1]).toBe(true);
    pull(readers[0]!);
    expectSame(dependentsOf(a), [readers[1], readers[0]]);
    writeSource(a, 2);
    expect(readers.map((node) => node.status)).toEqual([
      DIRTY,
      DIRTY,
      UNCOMPUTED,
      UNCOMPUTED,
    ]);
    disposeDerivedNode(readers[1]!);
    disposeDerivedNode(readers[0]!);
    expectSame(dependentsOf(a), []);
    expect(a.firstDependent === undefined).toBe(true);
    expect(a.lastDependent === undefined).toBe(true);
  });
});

describe('disposal of nodes in each status', () => {
  test('a disposed node holds no value and no dependency, and leaves the dependents list of the derived node it read', () => {
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, a),
    );
    const outer = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, a) + readDerived(callingNode, inner),
    );
    expect(pull(outer)).toBe(2);
    expectSame(dependentsOf(inner), [outer]);
    disposeDerivedNode(outer);
    expect(outer.status).toBe(UNCOMPUTED);
    expect(outer.value).toBeUndefined();
    expectSame(dependentsOf(inner), []);
    expectSame(dependentsOf(a), [inner]);
    expect(
      [
        outer.firstSourceDependency,
        outer.lastSourceDependency,
        outer.firstDerivedDependency,
        outer.lastDerivedDependency,
      ].map((edge) => edge === undefined),
    ).toEqual([true, true, true, true]);
  });

  test('a node that was disposed and pulled again is marked by the source and the derived node it read', () => {
    const a = createSourceNode(1);
    const b = createSourceNode(10);
    const inner = createDerivedNode((callingNode) =>
      readSource(callingNode, b),
    );
    const node = createDerivedNode(
      (callingNode) =>
        readSource(callingNode, a) + readDerived(callingNode, inner),
    );
    expect(pull(node)).toBe(11);
    disposeDerivedNode(node);
    expect(pull(node)).toBe(11);
    expectSame(sourceDependenciesOf(node), [a]);
    expectSame(derivedDependenciesOf(node), [inner]);
    writeSource(a, 2);
    expect(node.status).toBe(DIRTY);
    expect(pull(node)).toBe(12);
    writeSource(b, 20);
    expect(node.status).toBe(CHECK);
    expect(pull(node)).toBe(22);
  });

  test('disposing a Check node marks its Check reader Dirty, and the reader recomputes with the new value', () => {
    const runs = { bottom: 0, middle: 0, top: 0 };
    const a = createSourceNode(1);
    const bottom = createDerivedNode((callingNode) => {
      runs.bottom += 1;
      return readSource(callingNode, a);
    });
    const middle = createDerivedNode((callingNode) => {
      runs.middle += 1;
      return readDerived(callingNode, bottom) + 1;
    });
    const top = createDerivedNode((callingNode) => {
      runs.top += 1;
      return readDerived(callingNode, middle) + 1;
    });
    pull(top);
    writeSource(a, 2);
    expect([middle.status, top.status]).toEqual([CHECK, CHECK]);
    disposeDerivedNode(middle);
    expect([bottom.status, middle.status, top.status]).toEqual([
      DIRTY,
      UNCOMPUTED,
      DIRTY,
    ]);
    expectSame(dependentsOf(bottom), []);
    expect(pull(top)).toBe(4);
    expect(runs).toEqual({ bottom: 2, middle: 2, top: 2 });
  });

  test('disposing a node two levels below a reader leaves that reader Check, and it becomes Clean without running when the node between them recomputes to the same value', () => {
    const runs = { inner: 0, middle: 0, outer: 0 };
    const a = createSourceNode(1);
    const inner = createDerivedNode((callingNode) => {
      runs.inner += 1;
      return readSource(callingNode, a);
    });
    const middle = createDerivedNode((callingNode) => {
      runs.middle += 1;
      return readDerived(callingNode, inner) + 1;
    });
    const outer = createDerivedNode((callingNode) => {
      runs.outer += 1;
      return readDerived(callingNode, middle) + 1;
    });
    expect(pull(outer)).toBe(3);
    disposeDerivedNode(inner);
    expect([inner.status, middle.status, outer.status]).toEqual([
      UNCOMPUTED,
      DIRTY,
      CHECK,
    ]);
    expect(pull(outer)).toBe(3);
    expect(runs).toEqual({ inner: 2, middle: 2, outer: 1 });
    expect(outer.status).toBe(CLEAN);
  });

  test('disposing a node twice changes nothing the second time', () => {
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
    disposeDerivedNode(inner);
    startBatch((mutators) => mutators.disposeDerivedNode(inner));
    expect(calls).toBe(1);
    expect(inner.status).toBe(UNCOMPUTED);
    expect(pull(outer)).toBe(2);
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
    expect(pull(reader)).toBe(3);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect(eager.status).toBe(CHECK);
      expect(pull(eager)).toBe(2);
      expect(pull(reader)).toBe(3);
      expect(eager.status).toBe(CHECK);
      expect(runs).toEqual({ inner: 1, eager: 1 });
    });
    expect(runs).toEqual({ inner: 2, eager: 2 });
    expect([eager.status, reader.status]).toEqual([CLEAN, DIRTY]);
    expect(pull(reader)).toBe(5);
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
    expect(pull(top)).toBe(2);
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      mutators.writeSource(b, 2);
      expect([eager.status, top.status]).toEqual([CHECK, CHECK]);
      expect(pull(top)).toBe(2);
      expect([eager.status, top.status]).toEqual([CHECK, CLEAN]);
      expect(eagerRuns).toBe(1);
    });
    expect(eagerRuns).toBe(2);
    expect(top.status).toBe(DIRTY);
    expect(pull(top)).toBe(4);
  });
});

describe('setDerivedNodeEvaluation as a single call', () => {
  test('a queued eager node set lazy by the single call marks its readers Check and calls its onDirty and theirs before the call returns', () => {
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
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CLEAN,
        CLEAN,
      ]);
      setDerivedNodeEvaluation(eager, LAZY);
      expect(calls).toEqual(['eager', 'reader', 'outer']);
      expect([eager.status, reader.status, outer.status]).toEqual([
        DIRTY,
        CHECK,
        CHECK,
      ]);
    });
    expect(calls).toEqual(['eager', 'reader', 'outer']);
    expect(eager.evaluation).toBe(LAZY);
    expect(runs).toBe(1);
    expect(pull(outer)).toBe(6);
    expect(runs).toBe(2);
  });

  test('a queued eager node without onDirty set lazy by the single call marks its readers Check, calls their onDirty, and refreshes an eager reader of a reader', () => {
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
    pull(above);
    pull(sibling);
    reader.onDirty = () => calls.push('reader');
    sibling.onDirty = () => calls.push('sibling');
    above.onDirty = () => calls.push('above');
    startBatch((mutators) => {
      mutators.writeSource(a, 2);
      setDerivedNodeEvaluation(eager, LAZY);
      expect(runs).toEqual({ eager: 2, above: 2 });
      expect(calls).toEqual(['reader', 'sibling', 'above']);
      expect([
        eager.status,
        reader.status,
        above.status,
        sibling.status,
      ]).toEqual([CLEAN, CLEAN, CLEAN, DIRTY]);
    });
    expect(pull(above)).toBe(6);
    expect(pull(sibling)).toBe(104);
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
    pull(check);
    pull(dirty);
    writeSource(a, 3);
    expect([check.status, dirty.status]).toEqual([CHECK, DIRTY]);
    setDerivedNodeEvaluation(check, EAGER);
    setDerivedNodeEvaluation(dirty, EAGER);
    expect([check.evaluation, dirty.evaluation]).toEqual([EAGER, EAGER]);
    expect([check.status, dirty.status]).toEqual([CLEAN, CLEAN]);
    expect(runs).toEqual({ inner: 2, check: 1, dirty: 2 });
    expect([pull(check), pull(dirty)]).toEqual([10, 30]);
  });
});

describe('setDerivedNodeEvaluation with the evaluation the node has', () => {
  test('setting a Dirty lazy node lazy, by the single call and by the batch method, calls nothing and marks nothing', () => {
    let calls = 0;
    const a = createSourceNode(1);
    const node = createDerivedNode((callingNode) => readSource(callingNode, a));
    pull(node);
    node.onDirty = () => {
      calls += 1;
    };
    writeSource(a, 2);
    expect(calls).toBe(1);
    setDerivedNodeEvaluation(node, LAZY);
    startBatch((mutators) => mutators.setDerivedNodeEvaluation(node, LAZY));
    expect(calls).toBe(1);
    expect(node.status).toBe(DIRTY);
  });
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
    expect(pull(greeting)).toBe('Hello, Ada Lovelace');
    expect(greetingRuns).toBe(1);
    writeSource(person, { first: 'Ada', last: 'Lovelace', age: 37 });
    expect(pull(greeting)).toBe('Hello, Ada Lovelace');
    expect(greetingRuns).toBe(1);
    startBatch((mutators) => {
      mutators.writeSource(person, { first: 'Ada', last: 'Lovelace', age: 38 });
      mutators.writeSource(person, { first: 'Ada', last: 'King', age: 38 });
    });
    expect(pull(greeting)).toBe('Hello, Ada King');
    expect(greetingRuns).toBe(2);
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
  pull,
  readDerived,
  readSource,
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

test('the constructors, reads, and pull carry the value type, and the exported types name the node fields', () => {
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
  expectTypeOf(pull(eager)).toEqualTypeOf<string>();
  expectTypeOf(lazy.status).toEqualTypeOf<Status>();
  expectTypeOf(lazy.evaluation).toEqualTypeOf<DerivedNodeEvaluation>();
  expectTypeOf<Status>().toEqualTypeOf<
    typeof UNCOMPUTED | typeof DIRTY | typeof CHECK | typeof CLEAN
  >();
  expectTypeOf<DerivedNodeEvaluation>().toEqualTypeOf<
    typeof LAZY | typeof EAGER
  >();
  expectTypeOf<DependentsStatus>().toEqualTypeOf<0 | 1>();
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

test('a caller cannot write a node field except onDirty', () => {
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
  node.onDirty = () => {};
});
```

All 59 tests and the type test pass under vitest 2.1.9, and `tsc -b` reports no errors.
