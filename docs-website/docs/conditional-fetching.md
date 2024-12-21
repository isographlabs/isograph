# Conditionally fetching

`useLazyReference`, `useImperativeReference`, `useClientSideDefer` and `useImperativeLoadableField` all accept a `FetchOptions` parameter.

You can use this parameter to control whether to make a network request. For example:

```jsx
const { fragmentReference: mutationRef, loadFragmentReference: loadMutation } =
  useImperativeReference(iso(`entrypoint Mutation.SetTagline`));

const onClick = () => {
  loadMutation(
    // parameters
    {},
    // FetchOptions
    {
      shouldFetch: 'Yes',
    },
  );
};
```

`shouldFetch` can be `"Yes"`, `"No"` or `"IfNecessary"`.

`"Yes"` forces the network request to be made. `"IfNecessary"` will avoid making the network request if there is sufficient data in the Isograph store to fulfill the request, and `"No"` will not make the network request.

:::note
This is called a "fetch policy" in Relay.
:::

## Why would I want `shouldFetch`: `"No"`?

Two reasons:

- The hooks `useLazyReference` and `useClientSideDefer` cannot be called conditionally. Passing `"No"` is a way of avoiding the network request, despite calling the hooks unconditionally.
- Otherwise, this isn't a very useful feature currently, because reading a fragment with missing data will cause it to suspend. However, in the future, we will loosen this requirement (i.e. you will be able to read fragments with missing data.) In that world, avoiding making a network request and rendering whatever data happens to be availabe in the Isograph store will be possible, and may even be useful in certain circumstances.
