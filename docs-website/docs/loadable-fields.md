# Loadable fields

## Overview

Client fields can be selected loadably. If a client field is selected loadably, the resolver will not receive the client field result directly, but will instead receive a `LoadableField`.

The `LoadableField` is a wrapper around a function that, when called, will make a network request for the data needed by the client field. Making the network request creates a `FragmentReference` that must be disposed, in order to prevent memory leaks.

As a result, usually, you should not call the `LoadableField` directly. Instead, pass it to a hook that knows what to do with it!

## Basic walk-through

In this example, we'll design a `BlogPostDisplay` component client field that renders a `BlogPostHeader` immediately, and defers the JavaScript and data for a `BlogPostBody`.

Let's start by defining a `BlogPostDisplay` component without loadable fields:

```tsx
import { iso } from '@iso';

export const BlogPostDisplay = iso(`
  field BlogPost.BlogPostDisplay {
    BlogHeader
    BlogBody
  }
`)((blogPost) => {
  return (
    <>
      <blogPost.BlogHeader />
      <blogPost.BlogBody />
    </>
  );
});
```

Now, we can add `@loadable` to `BlogBody` as follows:

```tsx
import { iso } from '@iso';

export const BlogPostDisplay = iso(`
  field BlogPost.BlogPostDisplay {
    BlogHeader
    BlogBody @loadable
  }
`)((blogPost) => {
  return (
    <>
      <blogPost.BlogHeader />
      {/* uh oh, fails to type check */}
      <blogPost.BlogBody />
    </>
  );
});
```

Uh oh! `<blogPost.BlogBody />` has started to fail to type check. That's because we've received a loadable field instead of the component directly.

The first thing we should do is to pass `blogPost.BlogBody` to `useClientSideDefer`, which returns a fragment reference.

We can pass this fragment reference to `useResult` to read its result. But, that would cause the component to suspend. Since we want to render the `BlogHeader` immediately, we shouldn't do that.

So, instead let's pass it to `FragmentReader`, and wrap that in a `Suspense` boundary:

```tsx
import { iso } from '@iso';
import { FragmentReader, useClientSideDefer } from '@isograph/react';

export const BlogPostDisplay = iso(`
  field BlogPost.BlogPostDisplay {
    BlogHeader
    BlogBody @loadable
  }
`)((blogPost) => {
  const fragmentReference = useClientSideDefer(
    blogPost.BlogBody,
    // any parameters that were not passed to BlogBody above
    // can we passed here. There are none, so we just pass an empty
    // object.
    {},
  );
  return (
    <>
      <blogPost.BlogHeader />
      <React.Suspense fallback={'Loading...'}>
        <FragmentReader fragmentReference={fragmentReference} />
      </React.Suspense>
    </>
  );
});
```

Great! Now when this component is initially rendered, we'll make a network request for the data required by the `BlogPostBody`. While that request is in flight, we render a suspense fallback. When that network request completes, the `BlogPostBody` is rendered.

The final step is to change `@loadable` to `@loadable(lazyLoadArtifact: true)`.

```tsx
import { iso } from '@iso';
import { FragmentReader, useClientSideDefer } from '@isograph/react';

export const BlogPostDisplay = iso(`
  field BlogPost.BlogPostDisplay {
    BlogHeader
    BlogBody @loadable(lazyLoadArtifact: true)
  }
`)((blogPost) => {
  const fragmentReference = useClientSideDefer(
    blogPost.BlogBody,
    // any parameters that were not passed to BlogBody above
    // can we passed here. There are none, so we just pass an empty
    // object.
    {},
  );
  return (
    <>
      <blogPost.BlogHeader />
      <React.Suspense fallback={'Loading...'}>
        <FragmentReader fragmentReference={fragmentReference} />
      </React.Suspense>
    </>
  );
});
```

Awesome! Now, in addition to making a request for the `BlogPostBody` data, we will also make a request for the component JavaScript (if it has not been fetched before.)

## Imperatively fetching

In addition to fetching during render, you can also pass the loadable field to `useImperativeLoadableField` to fetch it in response to an event (such as a click):

```tsx
import { iso } from '@iso';
import { FragmentReader, useImperativeLoadableField } from '@isograph/react';
import { UNASSIGNED_STATE } from '@isograph/react-disposable-state';

export const BlogPostDisplay = iso(`
  field BlogPost.BlogPostDisplay {
    BlogHeader
    BlogBody @loadable(lazyLoadArtifact: true)
  }
`)((blogPost) => {
  const { fragmentReference, loadField } = useImperativeLoadableField(
    blogPost.BlogBody,
  );
  return (
    <>
      <blogPost.BlogHeader />
      {fragmentReference != UNASSIGNED_STATE ? (
        <button
          onClick={() =>
            loadField(
              // any parameters that were not passed to BlogBody above
              // can we passed here. There are none, so we just pass an empty
              // object.
              {},
            )
          }
        >
          Load blog body
        </button>
      ) : (
        <React.Suspense fallback={'Loading...'}>
          <FragmentReader fragmentReference={fragmentReference} />
        </React.Suspense>
      )}
    </>
  );
});
```

## Pagination

See [the pagination docs](../pagination).
