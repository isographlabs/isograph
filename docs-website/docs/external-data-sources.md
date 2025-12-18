# Working with external data sources

There is now a typesafe way to write data into the Isograph store. This can be used to integrate with other (non-GraphQL) data sources.

## Example: preloading

Entrypoints have raw response types, which are types that describe the shape of the network response one would receive from the GraphQL endpoint. (In this example, the raw response type might be located at `__isograph/Query/UserDetailPage/raw_response_type.ts`.)

In this example, we call `getRawResponseData()`, which somehow procures data in the correct shape, and preloads the data for the `UserDetailPage`.

```tsx
import { iso } from '@iso';
import { useIsographEnvironment, writeData } from '@isograph/react';
import { useUpdatableDisposableState } from '@isograph/react-disposable-state';

export const Button = iso(`
  field Query.PreloadUserDetailPageButton @component {}`)(({ data }) => {
  const environment = useIsographEnvironment();
  const { state: fragmentReference, setState } = useUpdatableDisposableState();
  return (
    <button
      onClick={() => {
        setState(
          writeData(
            environment,
            iso(`entrypoint Query.UserDetailPage`),
            getRawResponseData(),
            {
              id: '0',
            },
          ),
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
import {
  useUpdatableDisposableState,
  UNASSIGNED_STATE,
} from '@isograph/react-disposable-state';

type State = { kind: 'NotLoaded' } | { kind: 'Loading' };

export const Button = iso(`
  field Query.PreloadUserDetailPageButton @component {}`)(({ data }) => {
  const [state, setState] = useState<State>({ kind: 'NotLoaded' });
  const environment = useIsographEnvironment();
  const { state: fragmentReference, setState: setFragmentReference } =
    useUpdatableDisposableState();

  if (fragmentReference !== UNASSIGNED_STATE) {
    return (
      <Suspense>
        <FragmentRenderer fragmentReference={fragmentReference} />
      </Suspense>
    );
  }

  if (state.kind === 'NotLoaded') {
    return (
      <button
        onClick={() => {
          setState({ kind: 'Loading' });
          makeRestNetworkRequest().then((restResponse) => {
            setFragmentReference(
              writeData(
                environment,
                iso(`entrypoint Query.UserDetailPage`),
                transformRestResponseToRawResponseShape(restResponse),
                {
                  id: '0',
                },
              ),
            );
          });
        }}
      >
        Preload data
      </button>
    );
  } else {
    return <Loading />;
  }
});
```

## Retaining data

The `writeData` function returns a fragment reference wrapped in an item cleanup pair.
Data will be retained by `writeData` until the cleanup function is called. After the cleanup function is called there's no guarantee the data will remain in the store.

The `@isograph/react-disposable-state` package can be used for common cleanup scenarios.

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
      )[1](); // immediately call cleanup to prevent data from never being garbage collected
    },
  );

  return null;
});
```
