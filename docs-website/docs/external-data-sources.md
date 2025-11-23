# Working with external data sources

There is now a typesafe way to write data into the Isograph store. This can be used to integrate with other (non-GraphQL) data sources.

## Example: preloading

Entrypoints have raw response types, which are types that describe the shape of the network response one would receive from the GraphQL endpoint. (In this example, the raw response type might be located at `__isograph/Query/UserDetailPage/raw_response_type.ts`.)

In this example, we call `getRawResponseData()`, which somehow procures data in the correct shape, and preloads the data for the `UserDetailPage`.

```tsx
import { iso } from '@iso';
import { useIsographEnvironment, writeData } from '@isograph/react';

export const Button = iso(`
  field Query.PreloadUserDetailPageButton @component {}`)(({ data }) => {
  const environment = useIsographEnvironment();
  return (
    <button
      onClick={() => {
        writeData(
          environment,
          iso(`entrypoint Query.UserDetailPage`),
          getRawResponseData(),
          {
            id: '0',
          },
        );
      }}
    >
      Preload data for user detail page
    </button>
  );
});
```

It is up to you to implement `getRawResponseData`!

## Example: integrating with other data sources

Let's modify this example to integrate with another data source.

```tsx
import { iso } from '@iso';
import { useIsographEnvironment, writeData } from '@isograph/react';

type State = { kind: 'NotLoaded' } | { kind: 'Loading' } | { kind: 'Loaded' };

export const Button = iso(`
  field Query.PreloadUserDetailPageButton @component {}`)(({ data }) => {
  const [state, setState] = useState<State>({ kind: 'NotLoaded' });
  const environment = useIsographEnvironment();
  if (state.kind === 'NotLoaded') {
    return (
      <button
        onClick={() => {
          // Note: this will return a fragmentReference in the future and this documentation will be updated.
          setState({ kind: 'Loading' });
          makeRestNetworkRequest().then((restResponse) => {
            writeData(
              environment,
              iso(`entrypoint Query.UserDetailPage`),
              transformRestResponseToRawResponseShape(restResponse),
              {
                id: '0',
              },
            );
            setState({ kind: 'Loaded' });
          });
        }}
      >
        Preload data
      </button>
    );
  } else if (state.kind === 'Loading') {
    return <Loading />;
  } else {
    return <PreloadedChildComponent />;
  }
});

function PreloadedChildComponent() {
  // This will not make a network call, as the data is already in the store!
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.UserDetailPage`),
    { id: '0' },
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <FragmentRenderer
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}
```

In the future, `writeData` will return a fragment reference, so this example can be cleaned up some more.

## Example: subscriptions

One can also use `writeData` to work with subscriptions to external data sources. In this example, we set up a subscription to a chat service.

Now, whenever we receive data, any component that shows data from the chat will be automatically re-rendered!

```tsx
import { iso } from '@iso';
import { useIsographEnvironment, writeData } from '@isograph/react';

export const MySubscriptionComponent = iso(`
  field Query.MySubscriptionComponent @component {}`)(({ data }) => {
  const environment = useIsographEnvironment();
  const subscription = useSubscriptionToSomeExternalData(
    (subscriptionPayload) => {
      writeData(
        environment,
        iso(`entrypoint Query.Chat`),
        transformSubscriptionPayloadToRawResponseShape(
          subscriptionPayload.data,
        ),
        {
          chatId: subscriptionPayload.chatId,
        },
      );
    },
  );

  return null;
});
```
