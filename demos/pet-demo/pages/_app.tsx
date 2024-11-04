import {
  IsographEnvironmentProvider,
  StoreRecord,
  createIsographEnvironment,
  createIsographStore,
  type Link,
} from '@isograph/react';
import type { AppProps } from 'next/app';
import { useMemo } from 'react';

function makeNetworkRequest<T>(
  queryText: string,
  variables: unknown,
): Promise<T> {
  const promise = fetch('http://localhost:4000/graphql', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then(async (response) => {
    const json = await response.json();

    if (response.ok) {
      return json;
    } else {
      throw new Error('NetworkError', {
        cause: json,
      });
    }
  });
  return promise;
}
const missingFieldHandler = (
  storeRecord: StoreRecord,
  root: Link,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null,
): Link | undefined => {
  // This is the custom missing field handler
  //
  // N.B. this **not** correct. We need to pass the correct variables/args here.
  // But it works for this demo.
  if (
    fieldName === 'pet' &&
    variables?.id != null &&
    root.__link === '__ROOT'
  ) {
    return { __link: variables.id, __typename: 'Pet' };
  }
};

export default function App({ Component, pageProps }: AppProps) {
  const environment = useMemo(
    () =>
      createIsographEnvironment(
        createIsographStore(),
        makeNetworkRequest,
        missingFieldHandler,
        typeof window != 'undefined' ? console.log : null,
      ),
    [],
  );
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Component {...pageProps} />
    </IsographEnvironmentProvider>
  );
}
