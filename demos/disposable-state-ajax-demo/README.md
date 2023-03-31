# Using `react-disposable-state` to roll your own lazy loading + preloading

## What are we doing?

This repository contains a NextJS application whose code demonstrates two things: "lazy loading" network requests and imperatively making network requests.

In this demo, we treat network requests as disposable items. That is, even in the face of Suspense and concurrent mode, we don't want to:

- make unnecessary network calls
- use a network request after it has been disposed. (In this demo, we don't actually do anything when disposing of network requests, so it would be okay to re-use network requests after they have been disposed.)
  - See the TODO in PreloadedPostsPage.

### What is a disposable item?

A disposable item is anything that is either explicitly created or must be explicitly cleaned up. That is, it is an item with a lifecycle.

A disposable item is safe to use as long as its destructor has not been called.

Code that manages disposable items (such as the `useDisposableState` hook) should also ensure that each destructor is eventually called, and should not provide access to the underlying item once the destructor has been called.

Disposable items are allowed to have side effects when created or when destroyed.

### What is lazy loading?

Lazy loading means loading a resource during a functional component's render phase. This will have better performance than loading the resource in an effect, but worse performance than loading that resource imperatively, in advance.

## Lazy loaded demo

```bash
npm run dev
open http://localhost:3000/lazy-loaded
```

In this demo, we lazily load a list of posts, the author of each post, and (when revealed) the comments for a given post.

See the comments in [this file](./src/components/LazyLoadPostsPage.tsx) for more detail.

## Preloaded demo

```bash
npm run dev
open http://localhost:3000/preloaded
```

In this demo, we imperatively load a list of posts in an effect, and preload (on hover) the comments for a given post.

This demonstrates good and bad behaviors.

The good is preloading comments on hover. This gives a great user experience: one the user signals that they are likely to need some data, we fetch it, meaning we can show data to the user as soon as possible.

The bad is that we imperatively make a network request for the list of posts in an effect. This is means that we start the network request **strictly later** than if we had lazily loaded the data. A better alternative would be to integrate the loading of disposable items with a router, which would allow us to start fetching data for a given route in advance.

See the comments in [this file](./src/components/PreloadedPostsPage.tsx) for more detail.

## Caching behavior

In the lazy loaded demo, more than ten posts get loaded, but then only ten requests for authors' details are made. This is because all simultaneous calls to `getOrCreateCacheForUrl` that are passed the same cache key (e.g. `https://jsonplaceholder.typicode.com/users/{userId}`) re-use the same cache, and therefore share the same network call.
