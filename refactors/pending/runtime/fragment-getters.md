# Runtime: resolver fragment data through getters

Status: parked exploration. Not planned, not under discussion for implementation. In the current design (`refactors/pending/runtime/pico-runtime.md`) fragment data is materialized everywhere: the reader walk reads every selection of a resolver's reader AST, records each read as a dependency, builds a plain object, and then calls the resolver with it. This doc records an alternative we are not pursuing now, the reasons it is hard, and what adopting it would change.

Upstream references are the checkout at `/Users/rbalicki/code/isograph` (commit `9d72f6fc`; `libs/isograph-react/src/core/read.ts` and `startUpdate.ts` are unchanged from there to `9b81e0b6`). Paths without a directory are under `isograph/libs/isograph-react/src/core`.

## The idea

A resolver receives `data` as an object whose fields are getters. A getter reads the store (or pulls a nested resolver's derived node) when the resolver reads that field, and records the dependency at that moment. Only the fields the resolver actually reads during its synchronous call become dependencies of its derived node.

Upstream materializes. `readData` builds `target` field by field (`read.ts:145-277`; a scalar at `:180-187`, `target[field.alias ?? field.fieldName] = data.data`), returns `MissingData` before the resolver runs if any selection is absent (`:183-185`), and `readResolverFieldData` calls the resolver with the finished object (`read.ts:571-601`, the call at `:600`). i2's v1 walk does the same (`libs/isograph-runtime/src/read.ts:42-58`, `:101-121`), and pico-runtime.md's `resolverDefinition` calls `readData` and then `callResolver` (pico-runtime.md, change 2, The store's side).

Upstream already uses getters in one place: updatable data. `defineCachedProperty` (`startUpdate.ts:133-171`) defines each field with `Object.defineProperty` and a `get` that reads the store on first access, and `readUpdatableData` (`startUpdate.ts:173-345`) builds the object that `startUpdate` passes to the user's updater (`startUpdate.ts:67-76`). That object is valid only inside the updater call by contract; nothing closes it after the updater returns.

## Trace

```ts
// from libs/isograph-runtime/fixtures/store/src/Verdict.ts (hypothetical fixture)
export const Verdict = iso(`
  field Pet.Verdict {
    isAwesome
    praise
    jeer
  }
`)(function Verdict({ data }) {
  return data.isAwesome ? data.praise : data.jeer;
});
```

```text
state: Pet:1.isAwesome = true, praise = "good", jeer = "bad"

materialized
  compute Verdict        reads isAwesome, praise, jeer        deps [isAwesome, praise, jeer]   value "good"
  write jeer = "worse"   markChanged(jeer) -> Verdict Dirty
  pull Verdict           reads isAwesome, praise, jeer        deps [isAwesome, praise, jeer]   value "good"
                         "good" === "good" -> cutoff, no dependent marked

getters
  compute Verdict        reads isAwesome, praise              deps [isAwesome, praise]         value "good"
  write jeer = "worse"   markChanged(jeer) -> no dependent, nothing marked
```

The gain is one rerun of a branching resolver per write to a field on an untaken branch. Nothing downstream recomputes under either design, because the rerun returns a value `===` the previous one and the cutoff stops propagation (pico-runtime.md, R4). The gain is bounded by that cutoff.

Two further differences follow from the same mechanism. A nested resolver selected on an untaken branch is never pulled under getters, so its derived node is not computed and not kept by the parent's dependency. A field on an untaken branch that is absent from the store does not make the result `MissingData` under getters, where under materialization the walk returns `MissingData` and the resolver never runs. That second difference changes which results are `MissingData`, and so changes suspension.

## Reads outside the synchronous computation

A memoized value is correct only if every input that affects it is a recorded dependency. A getter that runs after the computation returned performs an unrecorded read. The places this happens are a returned closure, an event handler, a `setTimeout` callback, and a promise continuation, each of which closes over `data` or over a nested object read from it.

- The cached result and the closure disagree. The derived node holds the value computed from the store at computation time. The closure reads the store at call time. A write between the two changes what the closure sees and marks nothing, because the closure's read is not a dependency of any derived node.
- A closure can observe a mid-batch write. A handler that runs while `normalizeData` is partway through a response reads a store that is neither the old nor the new state.
- With ambient tracking (MobX's `globalState.trackingDerivation`, Vue's `activeEffect`, Solid's current owner), a late read is attributed to whatever computation is running at that moment, which records a dependency edge that the data does not justify. pico-runtime.md records reads through a `ComputeContext` parameter, not ambient state (pico-runtime.md, change 2, The engine). A getter built for one computation holds that computation's context, so a late read calls `readSource` on a context whose `compute` already returned and whose `dependencies` array `setDependencies` already consumed. The read is lost. Neither outcome is a correct dependency.

Under materialization a closure captures the plain object its result was computed from. The closure and the cached result agree, and the closure never reads the store.

## Enforcement

### 1. A lint rule

In a resolver body, the `data` parameter and every non-leaf value read from it must not escape the synchronous body. A value escapes when it is captured by a nested function, returned, stored (assigned to a variable outside the body, a property, or a collection), or passed to a call. A leaf (a scalar or a JSON value) is a plain value and may go anywhere. Destructuring and spread read every named field immediately, so they are allowed. Calls to an allowlist of synchronous array methods (`map`, `filter`, `some`, `every`, `find`, `reduce`, `forEach`) are allowed, including with a callback that reads the element, because the callback runs before the method returns.

```ts
// from libs/isograph-runtime/fixtures/store/src/Examples.ts (hypothetical fixture)
// allowed
return data.isAwesome ? data.praise : data.jeer;
const { name, nickname } = data;
return data.pets.map((pet) => pet.name);

// forbidden
return data;                                   // returned
return () => data.name;                        // captured by a nested function
setTimeout(() => console.log(data.pet.name));  // captured, and passed to a call
format(data.pet);                              // non-leaf passed to a call
```

A sound version needs type information. A non-leaf value can reach a forbidden position through a local alias, a helper's return value, or a generic, and a syntactic rule cannot follow those. The sound version brands every non-leaf fragment type as `Live<T>` and flags a `Live<T>` value at a forbidden position. That requires ESLint with typescript-eslint, or the TypeScript checker. A compiler diagnostic from a syntactic parse of the host file (the shape `refactors/pending/entrypoint-lint.md` holds for entrypoint literals, its open decision 1 (c)) cannot be sound here.

The cost is adoption. entrypoint-lint.md records the user.s requirement that its rule be easy for every Isograph user to adopt (entrypoint-lint.md, Problem), and the same requirement would apply here. entrypoint-lint.md held a compiler diagnostic because an ESLint rule needs a new package, a release, and a config step for every user, oxlint runs ESLint-API plugins only experimentally and not in its language server, and Biome users get nothing (entrypoint-lint.md, open decision 1 (a)). A type-aware rule has all of those costs, and also needs typescript-eslint's type-aware setup, which many projects do not run.

### 2. A runtime backstop

Each data object is open only during its own computation call. The runtime opens it when it calls the resolver and closes it when the call returns or throws. A getter on a closed object throws. A nested object read from a data object closes with its parent.

The window is the computation call, not the tick. Other derived nodes compute in the same synchronous tick (a pull computes inputs before dependents, pico-runtime.md, change 2, The engine), so a tick-based window would let a closure from one computation read during another's.

### 3. `MissingData` inside a resolver

Under getters the resolver runs before the runtime knows whether its data is present. A getter for an absent value throws a sentinel. The runtime records on the computation that it hit missing data, so a sentinel that user code catches in its own `try`/`catch` still yields `MissingData` for that computation. The resolver has already run up to the read, so any work before the read happened.

## Consequences

- Returning fragment data needs an opt-out. `PetName` returns `data` (`libs/isograph-runtime/fixtures/store/src/PetName.ts`), as do `PetSummary` and `StatsCard` in the same directory. Under getters a returned `data` is an escaping non-leaf value, and every later read of it throws on a closed object. That resolver must receive materialized data.
- Component fragments must be materialized. A component's fragment is read during React render, which is not a computation of the resolver graph, and its props reach event handlers and effects. No computation window covers those reads.
- pico-runtime.md's structural equality on walk-assembled data does not apply to getter objects. `mergeObjectsUsingReaderAst` (`areEqualWithDeepComparison.ts:55-121`) walks the reader AST over two data objects; over a getter object it reads every field, which records every dependency and removes the gain. Subscription delivery in Step A and pre-memoized selection sets (pico-runtime.md, Follow-ups 8) rely on that merge.
- Generated getters are a class per reader AST with getters on the prototype, constructed per computation with the record, the variables, and the computation's context. They are not a `Proxy`. AGENTS.md (TypeScript) forbids wrapping an object with private fields in a `Proxy`, and a class lets the open state be a private field.

```ts
// generated by crates/typescript_artifacts (hypothetical)
class VerdictData {
  readonly #reader: FragmentReader;
  constructor(reader: FragmentReader) {
    this.#reader = reader;
  }
  get isAwesome(): boolean {
    return this.#reader.scalar('isAwesome');
  }
  get praise(): string {
    return this.#reader.scalar('praise');
  }
  get jeer(): string {
    return this.#reader.scalar('jeer');
  }
}
```

`FragmentReader` holds the store, the record, the variables, the computation's `ComputeContext`, and the open state; `scalar` throws on a closed reader, records `valueSource` through the context, and throws the missing-data sentinel when the value is absent. Its exact shape is not designed.

## Default direction

If this is ever built, getters should be an opt-in per resolver, not the default. The name of the opt-in is not chosen. A wrong default under getters throws at runtime in common patterns: returning `data`, passing a nested object to a helper, closing over `data` in a returned callback. A wrong default under materialization costs one extra rerun of a branching resolver, cut off by `===`.

During discussion the user leaned toward getters by default, enforced by the lint rule, and then parked the whole idea. Both directions are open decision 1 below, with no held option.

## Prior art

- MobX tracks reads through getters on observable objects and records them on the ambient `globalState.trackingDerivation`. A read outside a reaction or action is untracked. `configure({ observableRequiresReaction: true })` warns in development on an observable read outside a reactive context, and `computedRequiresReaction` warns when an unobserved computed is read outside an action or reaction, which MobX does not cache. `enforceActions` governs writes, not reads.
- Vue 3's `reactive` wraps objects in a `Proxy` whose `get` trap calls `track` against the active effect. A read outside an effect is untracked, and `toRaw` returns the unwrapped object.
- Solid's `createStore` returns a `Proxy` that tracks property reads in the current tracking scope. A read outside a tracking scope (an event handler) is untracked, and `untrack(fn)` reads inside a tracking scope without recording.
- In all three a late read is legal and either untracked or attributed to the running computation. None closes the object after a computation, because their objects are long-lived state, not per-computation inputs.
- Upstream isograph materializes resolver data (`read.ts:145-277`, `:600`) and uses getters only for updatable data inside an updater (`startUpdate.ts:133-171`).

## What pico-runtime.md would need

- Dependency recording moves from the walk into the getters. pico-runtime.md's `contextHooks` reports each read as `readData` walks the reader AST (pico-runtime.md, change 2, The store's side). Under getters, `resolverDefinition`'s computation constructs the generated data class with its `ComputeContext` and calls the resolver; each getter calls `readSource` or `readDerivedNode` when read. The walk is still the reader for materialized resolvers, subscriptions, and components.
- The statement that the walk is the only reader of a source on behalf of user code (pico-runtime.md, change 2, The engine) becomes: the walk and the generated getters, the latter only while their computation is open.
- `callResolver` distinguishes a resolver's own throw (`ResolverThrew`) from the missing-data sentinel and from a read on a closed object, and checks the computation's missing-data record after the call.
- The reader artifact carries which mode the resolver uses and, for getters, the generated class.
- Open decision 7 (Result equality) and Follow-ups 8 state that `mergeObjectsUsingReaderAst` applies only to materialized data.

## Open decisions

None is held.

### 1. Default

- (a) Materialized by default, getters opt-in per resolver.
- (b) Getters by default, enforced by the lint rule, with an opt-out to materialized data for resolvers that return or pass on `data`.

### 2. Name and syntax of the opt-in or opt-out

A directive on the field, a reader artifact flag, or something else. Not discussed.

### 3. Where the lint rule runs

- (a) A typescript-eslint rule over a `Live<T>` brand. Sound, with the adoption costs above.
- (b) A syntactic rule (ESLint without types, or a compiler diagnostic over a host-file parse). Easy to adopt, and unsound for aliases, helpers, and generics.
- (c) No lint rule; the runtime backstop only.

### 4. Whether the runtime backstop ships in production builds

A closed-object check on every getter costs a branch per read. Development-only leaves a production late read unrecorded and silent.

### 5. `MissingData` semantics

Whether a missing field on an untaken branch yielding `Ok` is acceptable, given that it changes which results suspend.

### 6. Lists and the array-method allowlist

Whether a list of linked records is a plain array of getter objects or a getter object itself, and whether the allowlist includes methods beyond `map`, `filter`, `some`, `every`, `find`, `reduce`, and `forEach` (`flatMap`, `findIndex`, `slice`, iteration with `for...of`).

### 7. Nested objects

Whether a linked field's object closes with its parent (as above) or is materialized when read.
