# `@isograph/react-disposable-state`

> Primitives for managing disposable items in React state.

This library's purpose is to enable safely storing disposable items in React state. These hooks seek to guarantee that **each disposable item is eventually destroyed when it is no longer used** and that **no disposable item is returned from a library hook after it has been disposed**.

This library's goals **do not include** being ergonomic. A library built on top of `react-disposable-state` should expose easier-to-use hooks for common cases. Application developers can use the hooks exposed in `react-disposable-state` when more complicated cases arise.

This is unstable, alpha software. The API is likely to change.

## Conceptual overview

### What is a disposable item?

A disposable item is anything that is either explicitly created or must be explicitly cleaned up. That is, it is an item with a lifecycle.

A disposable item is safe to use as long as its destructor has not been called.

Code that manages disposable items (such as the `useDisposableState` hook) should also ensure that each destructor is eventually called, and should not provide access to the underlying item once the destructor has been called.

Disposable items are allowed to have side effects when created or when destroyed.

### What is disposable state?

Disposable state is React state that contains a disposable item.

### Examples of disposable items

- A subscription that periodically updates a displayed stock price. When the component where the stock price is displayed is unmounted, the subscription should be disposed, so as to avoid doing unproductive work.
- References to items that are stored externally. For example, consider a centralized store of profile photos. Photos are stored centrally to ensure consistency, meaning that every component displaying a given profile photo displays the same photo. In order to avoid the situation where no profile photo is ever garbage collected, individual components' "claims" to profile photos must be explicitly created and disposed.
- Items which you want to create **exactly once** when a functional React component is first rendered, such as a network request.
  - Due to how React behaves, this state must be stored externally. Hence, this can be thought of as an example of the previous bullet point.
  - Other frameworks make different choices. For example, a SolidJS component function is called exactly once. In these cases, the network request can easily be executed once, without being stored in external state.

### How does disposable state differ from regular React state?

Disposable state stands in contrast to "regular" React state (e.g. if `{isVisible: boolean, currentlySelectedItem: Item}` was stored in state), where

- creating the JavaScript object is the only work done when creating the regular state, and therefore it is okay to create the state multiple times; and
- the only necessary cleanup work is [garbage collection](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Memory_Management) of the underlying memory.

In particular, it is unobservable to the outside world if a piece of "regular" state is created multiple times.

### Can React primitives handle disposable state?

The primitives provided by React are a poor fit for storing disposable items in state. An upcoming blog post will explore this in more detail.

## This library

### Guarantees

This library guarantees that:

- First, each disposable item that is created is eventually disposed.

  > React and suspense prevent this library from ensuring that each disposable item is disposed immediately when the hook unmounts. Instead, the best we can do if a component suspends is often dispose after a configurable timeout.

- Second, no disposable item is returned from a library hook after it has been disposed.
- Third, if a component has committed, no disposable item returned from a library hook will be disposed while it is accessible from a mounted component.

  > Colloquially, this means that disposable items returned from library hooks are safe to use in event callbacks.

  > This guarantee is not upheld if an item returned from a library hook is re-stored in another state hook. So, don't do that!

### Supported behaviors

The hooks in this library enable the following behavior:

- Lazily creating a disposable item. In this context, "lazily" means creating the item during the render phase of a component, before that component has committed. The item is then available in the functional component.

  > Note that this is how [Relay](relay.dev) uses the term lazy. Libraries like [react-query](...) use the word lazy differently.

- Creating a disposable item outside of the render phase and after a hook's initial commit and storing the item in React state, making it available during the next render of that functional component.

## API Overview

### `useLazyDisposableState`

A hook that:

- Takes a mutable parent cache and a loader function, and returns a `{ state: T }`.
- The returned `T` is guaranteed to not be disposed during the tick of the render.
- If this hook commits, the returned `T` will not be disposed until the component unmounts.

```typescript
const { state }: { state: T } = useLazyDisposableState<T>(
  parentCache: ParentCache<T>,
  factory: Loader<T>,
  options: ?Options,
);
```

### `useUpdatableDisposableState`

A hook that:

- Returns a `{ state, setState }` object.
- `setState` throws if called before the initial commit.
- The `state` (a disposable item) is guaranteed to be undisposed during the tick in which it is returned from the hook. It will not be disposed until after it can no longer be returned from this hook, even in the presence of concurrent rendering.
- Every time the hook commits, a given disposable item is currently exposed in the state. All items previously passed to `setState` are guaranteed to never be returned from the hook, so they are disposed at that time.
- When the hook unmounts, all disposable items passed to `setState` are disposed.

```typescript
const {
  state,
  setState,
}: {
  state: T | null,
  setState: (ItemCleanupPair<T>) => void,
} = useUpdatableDisposableState<T>(
  options: ?Options,
);
```

### `useDisposableState`

> This could properly be called `useLazyUpdatableDisposableState`, but that's quite long!

A hook that combines the behavior of the previous two hooks:

```typescript
const {
  state,
  setState,
}: {
  state: T,
  setState: (ItemCleanupPair<T>) => void,
} = useDisposableState<T>(
  parentCache: ParentCache<T>,
  factory: Loader<T>,
  options: ?Options,
);
```

## Miscellaneous notes

### Runtime overhead

- The hooks in this library are generic, and the type of the disposable items `T` is mostly as unconstrained as possible.
  - The only constraint we impose on `T` is to disallow `T` from including the value `UNASSIGNED_STATE`. This is for primarily for ergonomic purposes. However, it does prevent some runtime overhead.
- This incurs some runtime overhead. In particular, it means we need to keep track of an index (and create a new short-lived object) to distinguish items that can overthise be `===` to each other. Consider, a component that uses `useDisposableState` or `useUpdatableDiposableState`. If we execute `setState([1, cleanup1])` followed by `setState([1, cleanup2])`, we would expect `cleanup1` to be called when the hook commits. This index is required to distinguish those two, otherwise indistinguishable items.
  - This problem also occurs if disposable items are re-used, but their cleanup functions are distinct. That can occur if items are shared references held in a reference counted wrapper!
- However, client libraries may not require this flexbility! For example, if every disposable item is a newly-created object, then all disposable items are `!==` to each other!
- A future version of this library should expose alternative hooks that disallow `null` and do away with the above check. They may be
