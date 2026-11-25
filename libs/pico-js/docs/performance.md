# Performance

This document describes the data structures and techniques behind the algorithm in the [design document](./design.md), and what each saves. It uses the graph of the README. Each example continues from the previous one unless it names its starting state, and the first one starts from the state after the last line.

```ts
const a = createSourceNode(1);
const b = createSourceNode(2);
const useA = createSourceNode(true);

const sum = createDerivedNode(
  (callingNode) => readSource(callingNode, a) + readSource(callingNode, b),
);
const doubled = createDerivedNode(
  (callingNode) => readDerived(callingNode, sum) * 2,
);
const chosen = createDerivedNode((callingNode) =>
  readSource(callingNode, useA)
    ? readSource(callingNode, a)
    : readSource(callingNode, b),
);

const doubledRetain = retainDerivedNode(doubled);
retainDerivedNode(chosen);
// sum is 3, doubled is 6, and chosen is 1
```

## Edge lists

Each read that a computation records is an edge object. An edge is in two intrusive lists, so adding or removing it takes constant time and allocates nothing beyond the edge. One list belongs to the dependent. A derived node keeps one list of the source nodes it read and one of the derived nodes it read, each in read order, and each singly linked, because a recompute only walks it from the start and a trim cuts its end. The other list belongs to the dependency, and holds its dependents. It is doubly linked, because it holds every dependent of the node, and an edge can be removed from anywhere in it. The source list of `sum` holds an edge to `a` and an edge to `b`, and the list of dependents of `a` holds the edges from `sum` and `chosen`. Marking walks the lists of dependents, and a read walks the derived list of what a node read.

## Positional edge reuse

A recompute reuses the edges of the previous run by position. The n-th source read takes the n-th source edge, the n-th derived read takes the n-th derived edge, and an edge moves only when its read is of a different node. Once the edges of the previous run are used up, a read adds an edge, unless it reads the same node as the previous read of its kind. Edges beyond the last read are removed. When `sum` recomputes and reads `a` and then `b` again, both edges stay and no edge is allocated. After `writeSource(useA, false)` and a read of `chosen`, its first source edge stays and its second moves from the dependents of `a` to the dependents of `b`.

A `DIRTY` node that is still needed keeps its edges until it recomputes, so its recompute reuses them instead of unlinking and relinking them. An update disposes an unneeded `DIRTY` node (see [disposing nodes](./disposing-nodes.md)).

## The source's record

After `writeSource(a, 10)` walks its dependents, `a` records that every node that read it is already `DIRTY`. A computation that reads `a` clears the record. Until then, `writeSource(a, 11)` only stores the value and walks nothing, which saves the walk over the dependents of a source that is written repeatedly without being read in between.

## Passing `callingNode`

A computation receives its `callingNode` and passes it to every read, so pico-js does not keep the running computation in a global. A computation pays no save and restore around its run. `readSource` and `readDerived` take a `CallingNode` as their first parameter, and the type checker rejects a source or a derived node there.

## Explicit stacks

Marking, disposal, and the descent of verification through lazy `MAYBE_DIRTY` nodes walk the graph with explicit stacks, so a long chain of such nodes does not overflow the call stack. A chain of nodes that compute for the first time, or that are all `DIRTY`, recurses through the computations, and uses the call stack. Verification through `MAYBE_DIRTY` nodes with callbacks uses the call stack too, because `verify` pulls a dependency with a callback, and that pull verifies the dependency in a call of its own. The README gives the chain lengths at which each of these overflowed. A stack is allocated only when it is needed. Marking allocates one when a walk through lazy nodes branches, and allocates the `beyond` array when it puts a node with a callback that has dependents on the flush queue, whether or not the walk past that node branches. Verification allocates one when it descends into a node with several dependents. Disposal allocates one when a disposed node leaves a dependency that is not `CLEAN` unneeded.

## Allocation

An update that puts no node with a callback on the flush queue allocates no flush queue, and a batch allocates the flush queue when it first puts a node with a callback there. The list of unneeded nodes is one module-level array that every update reuses. pico-js writes its entries by index and keeps a count of the entries in use. Clearing the list sets each entry in use to `undefined` and the count to 0, as the list of changed nodes does, so the array is never truncated, and a later update writes into the slots it already has.

## Object shape and equality

Each kind of node has one object shape, and all edges share one shape. A source node and a derived node have different shapes. A property access that sees only one kind of node stays monomorphic in V8. `linkEdge`, `unlinkEdge`, and `unlinkAll` in `src/edges.ts` read and write `firstDependent` and `lastDependent` on `edge.dependency`, which is a source node in some calls and a derived node in others, so each of those accesses sees two shapes and is polymorphic. Equality is `===` only, with no comparison function to call.

## Constants written as literals

Each use of a status or flag constant in `src/` is written `<literal> satisfies typeof <NAME>`, for example `case 3 satisfies typeof CLEAN:` and `flags & ~(4 satisfies typeof BEYOND_DONE)`. The type checker rejects a literal that differs from the constant, and switch narrowing and `typescript/switch-exhaustiveness-check` still see each case. tsdown emits the bare literal and drops the declarations, so no status compare or flag test loads a module-scope binding. A use written by name compiles to a load of a module-scope `const` in `dist`, because rolldown 1.0.0-rc.1, which tsdown 0.20.1 pins, does not inline a constant used in its own module, and it emits a `const enum` as an object with a property load at each use. The constants stay exported from `src/nodes.ts` for the types built from them and for the tests. `TIMEOUT_DISPOSAL_MS` in `src/cleanup.ts` is not a status or flag constant, and its use stays a load of its binding.

## No checks at run time

pico-js checks none of its contracts at run time, so no read, write, or computation pays for a check. The README lists the contracts.
