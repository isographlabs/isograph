# Using `react-disposable-state` and `reference-counted-pointer` to manage complicated state.

When managing more complicated state, such as an array of disposable items, we often do not want to dispose all of the previous state when creating new state.

For example, if our state goes from `[item1]` to `[item1, item2]`, it would be inefficient to dispose `item1` and re-create it. If the disposable item represents something like a network request, this may not even be possible!

In situations like this, we can wrap each item in a reference counted pointer. Then, when the state changes to `[item1, item2]` (actually from `[activeReferenceToItem1]` to `[activeReferenceToItem1, activeReferenceToItem2]`), then because there is always at least one undisposed active reference to `item1`, the underlying item is not disposed, and we do not need to recreate the item from scratch.

## Example

Consider using `useUpdatableDisposableState` state to manage an array of disposable items.

The behavior of `useUpdatableDisposableState` is to entirely dispose of all previously-held items after a new item is set in state. So, if we had

```js
const { state, setState } = useUpdatableDisposableState();
// assume state === [item1], and the cleanup function will dispose item1

const addItem2ToState = () => {
  setState([item1, item2], () => {
    disposeItem1();
    disposeItem2();
  });
};

return makePrettyJSX(state[0]);
```

In the above, `item1` would be disposed after the state was updated to be `[item1, item2]`. Meaning, the hook returns an item that has already been disposed. Clearly, this will not do.

## Alternatives

- We can re-create `item1` in `addItem2ToState`. This is costly, and not possible in all cases.
- We can store a mutable array in state, and dispose all items when the component unmounts. This is inefficient or unergonomic. Either:
  - we never remove items from the array, meaning we inefficiently only clean up items when the component unmounts, or
  - we dispose the items ourselves when we remove them. This is not ergonomic, and also means that the hook will likely misbehave in concurrent mode.
- Or, we can wrap each item in a reference counted pointer.

## Reference counted pointers

A reference counted pointer is an object that wraps a disposable item, and keeps track of all active references to that item. When all active references have been removed, the item itself is disposed.

Given an undisposed active reference `r1`, one can get a new active reference `r2` and a cleanup function `cleanupR2` by cloning `r1`. If `r1`'s cleanup function is called, it will appear disposed, but the underlying item will not be disposed until all remaining cleanup functions are called. In particular, this means that `cleanupR2` must be called before the underlying item is disposed.

> `CacheItem<T>` is form of a reference counted pointer! Though, it's behavior is slightly different than what is described here.

> Note that in practice, you will never actually access the "original reference counted pointer". Instead, a properly designed API will simply return an active reference.

## Using reference counted pointers in the example

Let's use reference counted pointers in the example.

```js
const { state, setState } = useUpdatableDisposableState();
// assume state === [item1activeReference], and the cleanup function will dispose that active reference

const addItem2ToState = () => {
  const [item2, disposeItem2] = createDisposeItem2();
  const [item2ActiveReference, disposeItem2ActiveReference] =
    createReferenceCountedPointer([item2, disposeItem2]);

  // get a new active reference to the existing item1
  const [item1ActiveReference, disposeItem1ActiveReference] = nullthrows(
    state[0].cloneIfNotDisposed(),
  );
  setState([item1ActiveReference, item2ActiveReference], () => {
    disposeItem1ActiveReference();
    disposeItem2ActiveReference();
  });
};

return makePrettyJSX(nullthrows(state[0].getItemIfNotDisposed()));
```

Not the most ergonomic, but it does the job.

### When `item2` is added to state:

Let's discuss what happens with `item1` when we call `addItem2ToState`:

- First, a new active reference is created from the previous active reference to `item1`.
- Next, this item is set in state.
- When that state commits, the previous state's dispose function is called. This cleans up the first active reference to `item1`. However, because there is an undisposed active reference to `item1` (created during `addItem2ToState`), the item itself is not disposed.
- On subsequent renders, `item1` is still safe to access.
- When the component unmounts, the item in state is disposed. At this point, all remaining active references to `item1` will be disposed, and the item itself will be disposed.

### When `item2` is removed from state:

Let's discuss what happens to `item2` if we removed `item2` from state with the following:

```js
const removeItem2FromState = () => {
  // assume state === [activeReferenceToItem1, activeReferenceToItem2]

  // get a new active reference to the existing item1
  const [item1ActiveReference, disposeItem1ActiveReference] = nullthrows(
    state[0].cloneIfNotDisposed(),
  );
  setState([item1ActiveReference], () => {
    disposeItem1ActiveReference();
  });
};
```

- First, the new item is set in state.
- When the hook commits, it runs the previous dispose function. This disposes the active reference to `item2` created in `addItem2ToState`. Since there are no undisposed active references to `item2`, the underlying item is disposed.

Awesome!

## Does this work with concurrent mode?

Yes! `useDisposableState` and `useUpdatableDisposableState` are compatible with concurrent mode.

## Ergonomics

Unfortunately, reference counted pointers have worse DevEx than hooks.

Libraries built off of `react-disposable-state` and `reference-counted-pointers` may choose to expose more ergonomic hooks for application developers to use.

(In particular, it would probably be a hook's responsibility to call `nullthrows`, improving DevEx slighly. A properly written hook will not expose disposed items.)

Though, developers that want to precisely and correctly model application state may need to reach for the lower-level hooks.

## When should you use reference counted pointers to manage disposable state?

Reference counted pointers are useful when you want structural sharing. Strutural sharing means that when your component transitions from `state1` to `state2`, the part of the states that is in common should not be disposed.

For example:

- Adding and removing items from an array or an object
- Re-using the same disposable item, but modifying other parts of state. Consider the state:

  ```ts
  type MyState =
    | {
        kind: 'ConnectedToDatabase';
        connectionToDatabase: ConnectionToDatabase;
        currentUserId: number;
      }
    | {
        kind: 'DisconnectedFromDatabase';
      };
  ```

  When we change the `currentUserId`, we should not dispose and re-create the database connection! So, the database connection should be managed by a reference counted pointer.

  However, we also should not model this state as two pieces of state: an optional `ConnectionToDatabase` and an optional `currentUserId`, as that would allow us to have a user id but no connection, or a connection and no user id. (We are following the canonical rule: _Make impossible states unrepresentable_.)

- An external cache. Consider a cache that might cache the results of network requests, and a user who navigates to an item detail page, navigates away, and navigates back. If the item is still in the cache when the user navigates back, we can avoid an expensive network request.
  - For this to work, the external cache might, instead of disposing the results of the network request, create a reference-counted pointer that is disposed after five minutes. If the user navigates back before the five minutes, they will be able to re-use the results.
  - This is similar to how `CacheItem` works!
