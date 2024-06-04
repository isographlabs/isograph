# How does `useLazyReference` work, at an extremely detailed level?

## High level explanation

[`useLazyReference`](https://github.com/isographlabs/isograph/blob/main/libs/isograph-react/src/useLazyReference.ts) is the developer-facing API. From the user's perspective, they call this to make a network request during the initial render, and then receive a query reference that they can use to read out the data from that network request.

Our goal is to only create a single network request, even if `useLazyReference` is called multiple times. We cannot always achieve this (it is impossible in React's programming model), but we can get pretty close.

## Definitions

First, some definitions. Don't worry about these, but refer back to them if necessary:

- A `ParentCache` is initially empty, but can store a single `CacheItem`. The `ParentCache` must be stable across renders.
- A `CacheItem` is an object storing some value and is in one of three states: `InParentCacheAndNotDisposed`, `NotInParentCacheAndNotDisposed`, and `NotInParentCacheAndDisposed`.
  - An invariant is that these APIs will never return a disposed `CacheItem` to the user.
- Creating the `CacheItem` is allowed to have side effects (e.g. making network requests in this example). We want to create as few `CacheItem`'s as possible.
- A `CacheItem` can be disposed, at which point it can perform some cleanup. Another guarantee is that every `CacheItem` we create will be disposed (i.e. no memory leaks).
- A `CacheItem` can also have (undisposed) temporary retains and permanent retains. If it has neither, it gets disposed.
  - When a `CacheItem` has no more undisposed temporary retains, it removes itself from the parent cache.
  - When a `CacheItem` has no more undisposed temporary retains **and** no more undisposed permanent retains, it disposes itself.
- A **temporary retain** is a retain that last 5 seconds or until you explicitly dispose it.
- A **permanent retain** is a retain that lasts until you explicitly dispose it.
- If a `CacheItem` is in a `ParentCache`, it must have at least one temporary retain. (Permanent retains are irrelevant here!)

## How it all works

Okay, so now the actual description of how this all works:

- When `useLazyReference` is called, we get-or-create a `ParentCache`.
  - When the cache is filled with a `CacheItem`, we make a network request and return a query reference.
- `useLazyReference` calls `useLazyDisposableState`, which calls `useCachedPrecommitValue` with that `ParentCache` and a callback (1).
- When `useCachedPrecommitValue` is called and has not committed, it will:
  - If that `ParentCache` is empty, fill the cache with a `CacheItem` and create a "temporary retain" (2), and return that query reference to to `useLazyDisposableState`. (3a)
  - If that `ParentCache` is not empty, we create a "temporary retain" (2) on that `CacheItem`, and return the query reference already in the cache to `useLazyDisposableState`. (3b)
- When `useCachedPrecommitValue` commits, it will:
  - Check whether the `CacheItem` that we received during the render is disposed. If not disposed, we clear the temporary retain (2), permanently retain the item, and pass that value to the callback (1).
  - If the item we received during the render is disposed, check whether the `ParentCache` is filled. (Another render might have filled the cache!) If so, permanently retain that `CacheItem` and pass the value to that callback (1).
  - If that `ParentCache` is not filled, create a `CacheItem` **but not put it in the `ParentCache`**, permanently retain it, and pass it to the callback. (1)
- When `useCachedPrecommitValue` is called after it has committed, it will return `null`.
- So, at this point, we either have returned a value to `useLazyDisposableState` (3a and 3b), or we have committed and some callback has executed.
- The callback (1) that `useLazyDisposableState` passed to `useCachedPrecommitValue` stores the permanently retained `CacheItem` in a ref.
- `useLazyDisposableState` then returns either the value returned from `useCachedPrecommitValue` or the one in the ref.
  - If we have some logic error and neither of these contain values, we throw an error.
- When the `useLazyDisposableState` unmounts (i.e. which only occurs if it has committed), we dispose of the permanent retain.

## What about these temporary and permanent retains?

Let's go through some scenarios, in order to show how these temporary and permanent retains prevent us from making redundant network requests, and how they allow us to avoid memory leaks.

### Happiest path with no suspense

In the happiest path, `useLazyReference` is called once and commits in less than 5 seconds.

- `useLazyReference` is called. The `ParentCache` becomes filled, triggering a network request. The `CacheItem` has one temporary retain.
- `useLazyReference` commits. The temporary retain is cleared, causing the `ParentCache` to become empty. The `CacheItem` is permanently retained.
- `useLazyReference` unmounts. The `CacheItem` is disposed.

Subsequent calls to `useLazyReference` from unmounted components will see an empty cache and make new network requests.

### Happy path with suspense

In the happy path, we call `useLazyReference`, suspend, then call it again and commit.

- `useLazyReference` is called (render 1). The `ParentCache` becomes filled, triggering a network request. The `CacheItem` has one temporary retain.
- `useLazyReference` is called again (render 2). The `ParentCache` is already filled, so we simply create another temporary retain.
- `useLazyReference` commits (from render 2). The temporary retain is cleared, but there is still a temporary retain outstanding, so we do not empty the `ParentCache`. The `CacheItem` is permanently retained.
- After five seconds, the temporary retain clears itself, causing the `ParentCache` to become empty.
- `useLazyReference` unmounts. The `CacheItem` is disposed.

A key thing to note here is that, in the presence of suspense, we are never informed that render 1 will never commit! Hence, the temporary retain is important.

### Multiple components making the same network request

Another thing to note is that we cannot distinguish between the same component rendering multiple times (due to suspense) from two identical components rendering. That is, the user must help us distinguish these two possibilities; library code cannot do it. So, let's consider two components rendering and mounting.

- `useLazyReference` is called (render 1). The `ParentCache` becomes filled, triggering a network request. The `CacheItem` has one temporary retain.
- `useLazyReference` is called again (render 2). The `ParentCache` is already filled, so we simply create another temporary retain.
- `useLazyReference` commits (from render 1 or 2). The temporary retain is cleared, but there is still a temporary retain outstanding, so we do not empty the `ParentCache`. The `CacheItem` is permanently retained.
- `useLazyReference` commits (from the other render). The last temporary retain is cleared, so we empty the `ParentCache`. The `CacheItem` is permanently retained.
- When both of these components unmount, the `CacheItem` is finally disposed.

### A component that renders multiple times, but never mounts

Another scenario we must handle is a component rendering multiple times without mounting. For example, a component might render, something suspends, then a parent component unmounts. So, our component never commits. In scenarios like this, we do not want memory leaks!

- `useLazyReference` is called initially. The `ParentCache` becomes filled, triggering a network request. The `CacheItem` has one temporary retain.
- `useLazyReference` is potentially called again (potentially multiple times). The `ParentCache` is already filled, so we simply create another temporary retain for each render.
- Some parent component unmounts, so none of these renders will ever commit.
- 5 seconds after the last render, we clean up the last temporary retain and remove the item from the `ParentCache`.

### A component that takes too long to commit

A scenario we must face is a component that takes too long to commit (e.g. >5 seconds.) I don't know whether 5 seconds is the optimal amount of time to wait, maybe 30 seconds is more reasonable!

- `useLazyReference` is called initially. The `ParentCache` becomes filled, triggering a network request. The `CacheItem` has one temporary retain.
- For whatever reason, React is slow. After 5 seconds, the temporary retain is cleared, and the `ParentCache` is cleared.
- The render commits. We find that the item we created is disposed **and** the `ParentCache` is empty. So, we create a new `CacheItem` and permanently retain it.

## Conclusion: why?

The reason we go through this much effort is because we must deal with several facts:

- We have no guarantee that a render will be followed by a commit.
- If a render is not followed by a commit, we will never be informed.
- We cannot distinguish between a render that will never commit from a render that will commit in the future.

In light of these, the best we can do is to create temporary retains during render, and permanent retains only when a component commits.

### Why clean stuff up?

There are two reasons that we dispose of `CacheItem`s.

First is that we do not want the memory usage of the app to grow without bound as the user uses our app.

Second, consider a `CacheItem` that makes a network request during initial render of a page. If the user loaded the data for that page, then navigated away, then navigated back after a long time, then we would want to make a new network request for that data. If we do not clean up after ourselves, we would be locked into forever re-using the existing network response.

Note that avoiding that network request (which is reasonable if e.g. the user navigates back after 10 seconds) can be done by, when the `CacheItem` is created, choosing to re-use cached data and not make a network request.
