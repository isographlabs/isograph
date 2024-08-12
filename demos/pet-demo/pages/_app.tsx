import {
  DataId,
  Link,
  StoreRecord,
  defaultMissingFieldHandler,
  IsographEnvironmentProvider,
  createIsographEnvironment,
  createIsographStore,
} from '@isograph/react';
import { useMemo } from 'react';
import type { AppProps } from 'next/app';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
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
  root: DataId,
  fieldName: string,
  arguments_: { [index: string]: any } | null,
  variables: { [index: string]: any } | null,
): Link | undefined => {
  // @ts-expect-error
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('Missing field handler called', {
      storeRecord,
      root,
      fieldName,
      arguments_,
      variables,
    });
  }
  const val = defaultMissingFieldHandler(
    storeRecord,
    root,
    fieldName,
    arguments_,
    variables,
  );
  if (val == undefined) {
    // This is the custom missing field handler
    //
    // N.B. this **not** correct. We need to pass the correct variables/args here.
    // But it works for this demo.
    if (fieldName === 'pet' && variables?.id != null && root === '__ROOT') {
      return { __link: variables.id };
    }
  } else {
    return val;
  }
};

export default function App({ Component, pageProps }: AppProps) {
  const environment = useMemo(
    () =>
      createIsographEnvironment(
        createIsographStore(),
        makeNetworkRequest,
        missingFieldHandler,
      ),
    [],
  );
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Component {...pageProps} />
    </IsographEnvironmentProvider>
  );
}

// If window.__LOG is true, Isograph will log a bunch of diagnostics.
if (typeof window !== 'undefined') {
  // @ts-expect-error
  window.__LOG = true;
}
