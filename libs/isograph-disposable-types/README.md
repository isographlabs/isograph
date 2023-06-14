# @isograph/disposable-types

> A shared library that exports commonly-used types related to disposable items.

See the `@isograph/react-disposable-state` library for more information.

## What is a disposable item?

A disposable item is anything that is either explicitly created or must be explicitly cleaned up. That is, it is an item with a lifecycle.

A disposable item is safe to use as long as its destructor has not been called.

Code that manages disposable items (such as the `useDisposableState` hook) should also ensure that each destructor is eventually called, and should not provide access to the underlying item once the destructor has been called.

Disposable items are allowed to have side effects when created or when destroyed.

### Example

An example might be a claim to a resource in a shared store.

```js
const [claim, disposeClaim]: ItemCleanupPair<Claim> = store.getClaimToResource({
  id: 4,
});

// Because the claim has not been disposed, this is safe to do:
const data = store.lookup(claim);

disposeClaim();

// Now that we've disposed of the claim, the underlying resource might have been removed from the store, so the following is not safe:
const unsafeData = store.lookup(claim);
```
