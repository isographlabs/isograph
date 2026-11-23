# pico-js: implementation

This doc is the complete source and package of `@isograph/pico-js` in `libs/pico-js`.

## Model

- A source node holds a value that the caller writes. A derived node holds a computation, its last value, a status, the source and derived nodes its computation read, and the derived nodes that read it. Each dependency is an edge object that sits in the reader's dependency list, in read order, and in the dependents list of the node it read.
- A write marks the source's direct readers `Dirty` and the nodes above them `Check`. It runs no computation.
- A derived node has one of four statuses: `Uncomputed`, `Dirty`, `Check`, and `Clean`. The `status` field holds a number: `0` is `Uncomputed`, `1` is `Dirty`, `2` is `Check`, and `3` is `Clean`. A new node is `Uncomputed`.
- `pull` brings a node up to date. An `Uncomputed` node computes for the first time. A `Clean` node returns its value. A `Check` node pulls its derived dependencies in the order they were read and recomputes after the first pull that leaves it `Dirty`. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node, and that other dependency changed. If no pull leaves the node `Dirty`, it becomes `Clean` without running. A `Dirty` node recomputes.
- A computation receives its node and the previous value. An `Uncomputed` node's computation receives `undefined`, as Solid's `createMemo` passes it. A recompute whose value is `===` the previous value marks nothing. A computation that finds its result structurally equal to `previous` returns `previous`.
- Under `===`, a `NaN` value always counts as changed. Under `===`, `0` and `-0` count as equal.
- Each read during a computation records the node it read in the calling node's dependency list and links the calling node into the dependents list of the node it read. A recompute reuses the edges of the previous computation in read order and drops the edges past its last read.
- Code outside a computation reads a source's current value as `source.value`. An untracked read inside a computation, `source.value` or `pull(node)`, records no dependency and is allowed. A later change to a node read that way does not mark the computing node.
- Marking a node `Dirty` keeps its edges. A `Dirty` node stays in the dependents lists of the nodes it read, and marking that reaches it again changes nothing. Its next computation keeps each edge whose dependency it reads at the same position.
- Disposing a node marks its readers `Dirty` and returns the node to `Uncomputed`, the state of a new node.

The engine does not check these contracts:

- The caller writes a source only when its value changed.
- A computation, and any function it calls, does not throw.
- A computation does not write a source and does not dispose a node.
- A computation is synchronous, and every `readSource` and `readDerived` call it makes receives, as `callingNode`, the node the computation received.
- `readSource` and `readDerived` are called only while the computation of the `callingNode` they receive is running.
- A computation's result depends only on what it reads through `readSource` and `readDerived` and on `previous`.
- A caller does not write a field of a source node or a derived node.
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

TypeScript rules (AGENTS.md, TypeScript): the engine has no `any`. Every type field is writable except `Edge.dependent`: `value`, `status`, and the edge-list fields change on every write, read, and recompute, and a reused edge changes its `dependency`. The engine has no class, so the ES private field rule does not apply. oxlint's `strict-boolean-expressions` (`.oxlintrc.json`) holds for every condition in the body. oxlint's `switch-exhaustiveness-check` holds for every `switch` in the body, and no `switch` has a `default` arm.

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

A write runs no computation. It marks the derived nodes that read the source `Dirty`, because one of their inputs is known to have changed, and it marks every node that depends on those through derived nodes `Check`, because one of their inputs might have changed. A write that reaches a node that is already `Dirty` changes nothing. Nothing is recomputed until a node is pulled.

A pull brings one node up to date and runs only the computations that requires. A `Dirty` node recomputes. A `Check` node pulls its derived dependencies in the order its computation read them. As soon as a pull leaves the node `Dirty`, the node recomputes, without pulling the rest. The dependency that pull targeted may be unchanged, because pulling it can recompute another dependency of the node, and that other dependency changed. If no pull leaves the node `Dirty`, the node becomes `Clean` and returns its value without running. Because the pulls follow read order, a computation such as `readDerived(callingNode, flag) ? readDerived(callingNode, b) : readDerived(callingNode, c)` recomputes as soon as `flag` comes back changed, before it pulls `b`, which the new run may not read.

## Example

```ts
import {
  createDerivedNode,
  createSourceNode,
  pull,
  readDerived,
  readSource,
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

writeSource(person, { first: 'Ada', last: 'King', age: 37 });
pull(greeting); // 'Hello, Ada King', greetingRuns === 2
```

After the first `writeSource`, `name` recomputes and returns `previous`, so `greeting` becomes `Clean` without running.

## API

- `createSourceNode(value)` returns a source node that holds `value`.
- `writeSource(source, value)` stores `value` in `source` and marks the derived nodes that read it. It runs no computation.
- `createDerivedNode(computation)` returns an `Uncomputed` derived node whose value `computation(callingNode, previous)` computes.
- `readSource(callingNode, source)` returns the value of `source` and records `source` as a dependency of `callingNode`.
- `readDerived(callingNode, node)` pulls `node`, returns its value, and records `node` as a dependency of `callingNode`.
- `pull(node)` brings `node` up to date and returns its value.
- `disposeDerivedNode(node)` marks the readers of `node` `Dirty` and returns `node` to `Uncomputed`.
- `SourceNode<TValue>` is the type of a source node whose value has type `TValue`.
- `DerivedNode<TValue>` is the type of a derived node whose value has type `TValue`.
- `Status` is the type of a derived node's `status` field: `0` is `Uncomputed`, `1` is `Dirty`, `2` is `Check`, and `3` is `Clean`. The package exports no constants for these numbers, so a caller compares `node.status` with the number, for example `node.status === 3` for `Clean`.

Each `readSource` and `readDerived` call during a computation records a dependency. A node's dependencies are the nodes its latest computation read. Code outside a computation reads a source's current value as `source.value`. An untracked read inside a computation, `source.value` or `pull(node)`, records no dependency and is allowed. A later change to a node read that way does not mark the computing node.

## Statuses

- An `Uncomputed` node (`status` `0`) has no value. Its next pull runs its computation with `previous` `undefined`. A computation whose value type includes `undefined` cannot tell its first run from a run after a stored `undefined`, because `previous` is `undefined` in both.
- A `Dirty` node (`status` `1`) read a source that was written, or a derived node whose value changed or that was disposed. Its next pull runs its computation with its last value as `previous`.
- A `Check` node (`status` `2`) depends through derived nodes on a source that was written or on a derived node that was disposed. Its next pull pulls its derived dependencies in the order they were read and recomputes the node after the first pull that leaves it `Dirty`. If no pull leaves the node `Dirty`, the node becomes `Clean` without running.
- A `Clean` node (`status` `3`) holds an up-to-date value, which a pull returns without running anything.

## Cutoff

When a recompute returns a value `===` to its previous value, the node marks none of its readers `Dirty`, and a `Check` reader becomes `Clean` without running. The engine compares only by `===`. Structural sharing is the computation's responsibility: a computation that wants a structurally equal result to count as unchanged compares its new result with `previous` and returns `previous` when they are equal.

## Disposal

`disposeDerivedNode(node)` marks the readers of `node` `Dirty`, removes `node` from the dependents of every node it read, and discards its value. The node is `Uncomputed`, and its next pull computes it with `previous` `undefined`. Disposing an `Uncomputed` node changes nothing.

## Contracts

The engine does not check these contracts:

- The caller writes a source only when its value changed.
- A computation, and any function it calls, does not throw.
- A computation does not write a source and does not dispose a node.
- A computation is synchronous, and every `readSource` and `readDerived` call it makes receives, as `callingNode`, the node the computation received.
- `readSource` and `readDerived` are called only while the computation of the `callingNode` they receive is running.
- A computation's result depends only on what it reads through `readSource` and `readDerived` and on `previous`.
- A caller does not write a field of a source node or a derived node.
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

/** 0 Uncomputed, 1 Dirty, 2 Check, 3 Clean. */
export type Status = 0 | 1 | 2 | 3;

/**
 * An edge from `dependent` to `dependency`. It sits in two lists: the dependent's dependency list (`nextDependency`), in read order, and the dependency's dependents list (`previousDependent`, `nextDependent`). The read that creates it links it into the dependents list, and it stays there until a recompute of `dependent` reads another node at its position or reads fewer nodes, or `dependent` is disposed.
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

export type SourceNode<TValue> = {
  value: TValue;
  firstDependent: SourceEdge | undefined;
  lastDependent: SourceEdge | undefined;
};

/**
 * A derived node whose computation is not running satisfies these invariants for each status.
 * - When `status` is `Uncomputed`, `value` is `NOT_SET`, both dependency lists are empty, and every node in its dependents list is `Dirty`.
 * - When `status` is `Dirty`, `value` is the value the last computation returned, each edge of its dependency lists is in its dependency's dependents list, and every node in its dependents list is `Check` or `Dirty`.
 * - When `status` is `Check`, `value` is the value the last computation returned, each edge of its dependency lists is in its dependency's dependents list, and every node in its dependents list is `Check` or `Dirty`.
 * - When `status` is `Clean`, `value` is the up-to-date value, and each edge of its dependency lists is in its dependency's dependents list.
 * - No dependents list holds an `Uncomputed` node.
 *
 * These invariants form a discriminated union on `status`, but `DerivedNode` is a single object type. A node keeps its identity for its whole life, because callers and edges hold it, so a status change assigns fields of the same object. TypeScript does not narrow an object to a different variant of a union when one of its fields is assigned, so a union type would need a cast at every status change. The other way to type it holds the current variant in a field of the node and replaces that variant object on each status change. That allocates an object on every status change and adds a pointer dereference to every read of `status` or `value`. So `computedValue` and the throws in marking check these invariants at run time.
 */
export type DerivedNode<TValue> = {
  value: Option<TValue>;
  status: Status;
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
  return { value, firstDependent: undefined, lastDependent: undefined };
}

export function createDerivedNode<TValue>(
  computation: (
    callingNode: DerivedNode<unknown>,
    previous: TValue | undefined,
  ) => TValue,
): DerivedNode<TValue> {
  return {
    value: NOT_SET,
    status: 0,
    firstSourceDependency: undefined,
    lastSourceDependency: undefined,
    firstDerivedDependency: undefined,
    lastDerivedDependency: undefined,
    firstDependent: undefined,
    lastDependent: undefined,
    computation,
  };
}

export function writeSource<TValue>(
  source: SourceNode<TValue>,
  value: TValue,
): void {
  source.value = value;
  const first = source.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
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

export function pull<TValue>(node: DerivedNode<TValue>): TValue {
  switch (node.status) {
    case 3:
      return computedValue(node);
    case 1:
      return recompute(node);
    case 2:
      return verify(node);
    case 0:
      return computeFirst(node);
  }
}

export function disposeDerivedNode(node: DerivedNode<unknown>): void {
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
  unlinkAll(node.firstSourceDependency);
  unlinkAll(node.firstDerivedDependency);
  node.firstSourceDependency = undefined;
  node.lastSourceDependency = undefined;
  node.firstDerivedDependency = undefined;
  node.lastDerivedDependency = undefined;
  node.status = 0;
  node.value = NOT_SET;
}

function markDirty<TDependency>(first: Edge<TDependency>): void {
  const stack: Array<DerivedNode<unknown>> = [];
  let edge: Edge<TDependency> | undefined = first;
  while (edge !== undefined) {
    const node = edge.dependent;
    edge = edge.nextDependent;
    switch (node.status) {
      case 3:
        node.status = 1;
        if (node.firstDependent !== undefined) {
          stack.push(node);
        }
        break;
      case 2:
        node.status = 1;
        break;
      case 1:
        break;
      case 0:
        throw new Error(
          'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
        );
    }
  }
  let marked: DerivedNode<unknown> | undefined;
  while ((marked = stack.pop()) !== undefined) {
    let dependentEdge = marked.firstDependent;
    while (dependentEdge !== undefined) {
      const dependent = dependentEdge.dependent;
      dependentEdge = dependentEdge.nextDependent;
      switch (dependent.status) {
        case 3:
          dependent.status = 2;
          if (dependent.firstDependent !== undefined) {
            stack.push(dependent);
          }
          break;
        case 2:
        case 1:
          break;
        case 0:
          throw new Error(
            'An Uncomputed node is in a dependents list. This is indicative of a bug in pico-js.',
          );
      }
    }
  }
}

/**
 * Pulls the derived dependencies of `root` in read order, and recomputes a node after the first pull that leaves it `Dirty`. A `Check` dependency is verified the same way before its reader continues, with an explicit stack in place of recursion.
 */
function verify<TValue>(root: DerivedNode<TValue>): TValue {
  const verifyStack: Array<DerivedEdge> = [];
  let node: DerivedNode<unknown> = root;
  let edge = node.firstDerivedDependency;
  for (;;) {
    if (edge !== undefined) {
      const dependency = edge.dependency;
      if (dependency.status === 2) {
        verifyStack.push(edge);
        node = dependency;
        edge = dependency.firstDerivedDependency;
        continue;
      }
      pull(dependency);
      if (node.status !== 1) {
        edge = edge.nextDependency;
        continue;
      }
      recompute(node);
    } else {
      node.status = 3;
    }
    for (;;) {
      const popped = verifyStack.pop();
      if (popped === undefined) {
        return computedValue(root);
      }
      edge = popped;
      node = edge.dependent;
      if (node.status !== 1) {
        break;
      }
      recompute(node);
    }
    edge = edge.nextDependency;
  }
}

function computeFirst<TValue>(node: DerivedNode<TValue>): TValue {
  const value = node.computation(node, undefined);
  trim(node);
  node.status = 3;
  node.value = value;
  return value;
}

function recompute<TValue>(node: DerivedNode<TValue>): TValue {
  const previous = computedValue(node);
  node.lastSourceDependency = undefined;
  node.lastDerivedDependency = undefined;
  const value = node.computation(node, previous);
  trim(node);
  node.status = 3;
  if (value === previous) {
    return previous;
  }
  node.value = value;
  const first = node.firstDependent;
  if (first !== undefined) {
    markDirty(first);
  }
  return value;
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
- `Status` is numeric because a `switch` and a comparison on small integers cost less than on strings: numeric statuses cut the upstream benchmark total by 4.6% (pico-js-performance.md, change 5). `graph.ts` exports no constants for the numbers. A caller compares `status` with the number, and the README states the mapping.
- A read links its edge into the dependency's dependents list at the time of the read, and `readDerived` pulls the dependency before it links the edge, so a node is linked only to a source or to a `Clean` derived node.
- `recompute` resets `lastSourceDependency` and `lastDerivedDependency`, and each read takes the next edge of the list. An edge whose dependency is the node read stays linked. An edge whose dependency differs is unlinked from the old dependency and linked to the node read. A read past the end of the list allocates an edge, unless the previous edge already points at the node read. `trim` unlinks and drops the edges past the last read. So a recompute that reads the same nodes in the same order allocates, links, and unlinks nothing, and after a computation the dependency lists hold exactly its reads, with adjacent reads of one node sharing one edge.
- A node that reads a dependency at two non-adjacent positions (`a, b, a`) has two edges in `a`'s dependents list. Marking visits it twice, and the second visit finds it `Dirty` and changes nothing.
- Marking unlinks no edge. A `Dirty` node stays in the dependents lists of its dependencies, and marking skips it, because its dependents are already `Check` or `Dirty`. A `Check` node that marking sets `Dirty` is not pushed, for the same reason. So repeated writes to the sources of a node that is not pulled visit that node on each write and stop there.
- Marking pushes a node onto its stack only when the node leaves `Clean` and its dependents list is not empty, so a write visits each node's dependents at most once. The marking stack and the verification stack are arrays local to the call, so the depth of a marked or verified chain does not reach the call stack, and the engine holds no module state.
- A node becomes `Clean` only after its computation returns or after `verify` has pulled every derived dependency it is linked to. Each dependency a computation links is `Clean` when it is read, so every derived dependency of a `Clean` node is `Clean`, and every dependent of a node that is not `Clean` is `Check` or `Dirty`. A recompute therefore marks only `Check` and `Dirty` nodes and takes no node out of `Clean`.
- While a computation runs, its node is `Uncomputed` or `Dirty` and is linked to what it has read so far. Every node it read is `Clean` and has only `Clean` derived dependencies, so nothing marks those nodes before the computation returns, and marking never reaches the running node while it is `Uncomputed`.
- `verify` walks the derived dependency lists in read order with an explicit stack of edges. It descends into a `Check` dependency before its reader continues and pulls any other dependency. After a pull that leaves the current node `Dirty`, it recomputes that node and returns to the node's reader. The order of pulls and recomputes is that of a recursive `verify`. A first computation still recurses through `readDerived` and `pull`.
- Because a write marks the source's direct readers `Dirty`, a `Check` node has no changed source, and `verify` pulls only its derived dependencies. A dependency's `recompute` marks its readers `Dirty` exactly when its new value is not `===` its previous value, so after a pull in `verify` the node is `Dirty` exactly when a dependency it read changed value.
- A node's `status` is `Uncomputed` exactly when its `value` is `NOT_SET`. In the other three statuses, `value` holds a value the computation returned, which can be `undefined`, so a `Dirty` node always has a value for its next computation to receive as `previous`. `NOT_SET` and `Option` are private to `graph.ts`, so no `TValue` contains `NOT_SET`. `computedValue` throws if a node that is not `Uncomputed` holds `NOT_SET`, because a broken invariant throws in this library. The check costs one comparison per pull of a `Clean` node.
- A first computation has no cutoff and marks nothing, because every node in an `Uncomputed` node's dependents list is already `Dirty`: a new node has no readers, and disposal marked every reader `Dirty`.
- On cutoff the node keeps its stored value, so a recompute that returns `-0` where the stored value is `0` leaves `0` stored and returned, and every reader of the node reads the same value.
- Disposal marks every reader `Dirty`, unlinks the node's own edges, and discards its value, so its next computation receives `previous` `undefined`. The readers stay in the node's dependents list, and each reader's next recompute either reads the node again, which computes it first, or drops the edge. An `Uncomputed` node has empty dependency lists and only `Dirty` dependents, so its disposal changes nothing. A `Dirty` node's dependents are `Check` or `Dirty`, and its disposal marks them `Dirty`.

## `src/index.ts`

```ts
// from libs/pico-js/src/index.ts
export {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
  writeSource,
} from './graph';
export type { DerivedNode, SourceNode, Status } from './graph';
```

## Tests

```ts
// from libs/pico-js/test/graph.test.ts
import { describe, expect, test } from 'vitest';
import {
  createDerivedNode,
  createSourceNode,
  disposeDerivedNode,
  pull,
  readDerived,
  readSource,
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
    expect(double.status).toBe(0);
    expect(pull(double)).toBe(2);
    expect(double.status).toBe(3);
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
    expect(outer.status).toBe(2);
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
    expect(sum.status).toBe(1);
    expect(label.status).toBe(2);
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
      1, 2, 2, 2,
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
    expect(bottom.status).toBe(1);
    expect(top.status).toBe(2);
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
    expect(label.status).toBe(3);
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
    expect(reader.status).toBe(3);
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
    expect(node.status).toBe(3);
    expect(pull(node)).toBeUndefined();
    expect(runs).toEqual({ node: 1, reader: 1 });
    writeSource(a, 2);
    expect(node.status).toBe(1);
    expect(pull(reader)).toBe(true);
    expect(reader.status).toBe(3);
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
    expect(foo.status).toBe(1);
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
    expect(pick.status).toBe(3);
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
    expect(sum.status).toBe(1);
    expect(pull(sum)).toBe(7);
    writeSource(a, 5);
    expect(sum.status).toBe(1);
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
    expect(sum.status).toBe(1);
    expect(label.status).toBe(2);
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
    expect(node.status).toBe(3);
    writeSource(count, 3);
    expect(pull(node)).toBe(101);
    writeSource(third, 200);
    expect(node.status).toBe(1);
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
    expect(inner.status).toBe(0);
    expect(outer.status).toBe(1);
    expect(pull(outer)).toBe(2);
    expect(runs).toEqual({ inner: 2, outer: 2 });
    writeSource(source, 2);
    expect(outer.status).toBe(2);
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
    expect(inner.status).toBe(0);
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
    expect(inner.status).toBe(1);
    expect(outer.status).toBe(2);
    disposeDerivedNode(inner);
    expect(inner.status).toBe(0);
    expect(outer.status).toBe(1);
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
    expect(node.status).toBe(0);
    disposeDerivedNode(node);
    expect(node.status).toBe(0);
    expect(pull(node)).toBe(1);
    expect(previousValues).toEqual([undefined]);
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
