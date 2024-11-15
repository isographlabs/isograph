# Data Driven Dependencies

Congratulations! If you have gotten this far, you are about to learn about one of the coolest features of Isograph: the ability to load just the data and JavaScript that you need for a given page.

To pull this off, we'll combine [`@loadable` fields](/docs/loadable-fields/), [pagination](/docs/pagination/) and [`asConcreteType`](/docs/abstract-types/) fields, so feel free to brush up on those before reading on.

## Setting the scene

Imagine you have a news feed containing text, images, videos, polls, events, songs, and albums, and not to mention variations of those that are only shown to users in some experiment or other. It is tremendously wasteful to download all of the JavaScript required to render an object of each type — very few or no users will see all possible variations!

And not only does that waste your users' bandwidth, it also slows down how fast your page can start, as parsing all of that JavaScript isn't free.

What can be done about it? Well, we should avoid loading the JavaScript for any news feed item types that we don't encounter. We call this "data driven dependencies", or 3D.

And you guessed it — Isograph makes that easy!

## Isograph makes it easy

Let's see how that's done with Isograph. First, we use the `useConnectionSpecPagination` hook to fetch some news items:

```jsx
export const Newsfeed = iso(`
  field Query.Newsfeed @component {
    firstPage: NewsfeedConnection(first: 10)
    NewsfeedConnection @loadable
  }
`)(function NewsfeedComponent({ data }) {
  const pagination = useConnectionSpecPagination(
    data.NewsfeedConnection,
    data.firstPage.pageInfo,
  );
  const repositories = (data.firstPage.edges ?? []).concat(pagination.results);
  return repositories.map((data) => {
    if (data == null || data.node == null) {
      return null;
    }
    const { node } = data;
    return <node.NewsfeedItem key={node.id} />;
  });
});

export const NewsfeedPaginationComponent = iso(`
  field Viewer.NewsfeedPaginationComponent($skip: Int!, $limit: Int!) {
    newsfeed(skip: $skip, limit: $limit) {
      NewsfeedItem
    }
  }
`)(({ data }) => {
  return data.newsfeed;
});
```

Here, we're also pre-fetching the first 10 items (via `firstPage`).

### `NewsfeedRow`

In our schema, `newsfeed` has type `NewsfeedItem`, which is a union of `TextItem`, `PhotoItem` and others. Armed with that knowledge, let's go ahead and define `NewsfeedItem`!

```jsx
export const NewsfeedAdOrBlog = iso(`
  field NewsfeedItem.NewsfeedAdOrBlog @component {
    asTextItem {
      TextItemDisplay
    }
    asPhotoItem {
      PhotoItemDisplay
    }
    asRareItem {
      TopSecretDisplay
    }
  }
`)(({ data: newsfeedItem }) => {
  if (newsfeedItem.asTextItem != null) {
    return <newsfeedItem.asTextItem.TextItemDisplay />;
  } else if (newsfeedItem.asPhotoItem != null) {
    return <newsfeedItem.asBlogItem.PhotoItemDisplay />;
  } else if (newsfeedItem.asRareItem != null) {
    return <newsfeedItem.asRareItem.TopSecretDisplay />;
  }

  // oops, it's a type we can't handle! Return null to not impact the
  // the UI.
  return null;
});
```

That's great! Now, we render something different for each type of newsfeed item we encounter.

### But wait

But wait &mdash; wasn't the point of this to not load the JavaScript for the types of items we don't encounter? Yes! Let's do that, starting with `RareItem` and `TopSecretDisplay` (though you can, and should, do this for every variant.)

We start by replacing `TopSecretDisplay` with `TopSecretDisplayWrapper`. (This will not be necessary once [this issue](https://github.com/isographlabs/isograph/issues/273) is tackled!) `TopSecretDisplayWrapper` is defined as follows:

```jsx
export const TopSecretDisplayWrapper = iso(`
  field RareItem.TopSecretDisplayWrapper @component {
    TopSecretDisplay @loadable(lazyLoadArtifact: true)
  }
`)(({ data }) => {
  const { fragmentReference } = useClientSideDefer(data.TopSecretDisplay);
  return (
    <Suspense fallback={null}>
      <FragmentReader fragmentReference={fragmentReference} />
    </Suspense>
  );
});
```

Nice! Now, we only download the JavaScript for `TopSecretDisplay` if we actually encounter an item of the type `RareItem`. So, we've saved some bandwidth!

### All is not well

Unfortunately, we've introduced a regression. Now, not only is the JavaScript for `TopSecretDisplay` fetched only when needed, but the data for `TopSecretDisplay` is fetched only when needed. But GraphQL already gives us a way to fetch that data only if we encounter a newsfeed item with the `RareItem` type &mdash; inline fragments! And that's what our `asRareItem` selection compiles to:

```graphql
... on RareItem {
  # fields needed by TopSecretDisplay
}
```

:::note
Making this easy (i.e. by supporting `@loadable(lazyLoadArtifact: true, lazyLoadData: false)` or the like) is on the roadmap! This is a temporary workaround.
:::

So, is there an inevitable tradeoff? No. We can restructure our app slightly to avoid this problem. We can restructure our `TopSecretDisplay` to take its data separately. Then, we select another field `TopSecretDisplayData` that provides that data! See how this all fits together:

```jsx
export const NewsfeedAdOrBlog = iso(`
  field NewsfeedItem.NewsfeedAdOrBlog @component {
    asRareItem {
      TopSecretDisplayData
      TopSecretDisplayWrapper
    }
    # etc
  }
`)(({ data: newsfeedItem }) => {
  if (newsfeedItem.asRareItem != null) {
    return (
      <newsfeedItem.asRareItem.TopSecretDisplayWrapper
        data={newsfeedItem.asRareItem.TopSecretDisplayData}
      />
    );
  }
  // etc
});
```

`TopSecretDisplay` no longer selects any data of its own.

Nice! Now, we fetch the data for `TopSecretDisplay` as part of the parent query and only if we encounter a newsfeed item with type `RareItem`, and load the JavaScript for `TopSecretDisplay` only if we encounter an item of this type!

Neat!

## Conclusion

So, let's describe what we did. We fetched a list of newsfeed items. We used `asConcreteType` fields to allow us to refine a given concrete type, and then fetched the JavaScript for the given renderer only if we encountered an item of that type!

And we did it all in userland!

## PS

We can also use these techniques (minus the pagination) to conditionally fetch JavaScript for fields that are nullable, and not just for fields that have an interface/union type!
