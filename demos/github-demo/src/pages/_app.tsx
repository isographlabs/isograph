import type { AppProps } from 'next/app';
import { useMemo } from 'react';
import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
} from '@isograph/react';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  const promise = fetch('https://api.github.com/graphql', {
    method: 'POST',
    headers: {
      Authorization: 'Bearer ' + process.env.NEXT_PUBLIC_GITHUB_TOKEN,
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

export default function App({ Component, pageProps }: AppProps) {
  const environment = useMemo(() => {
    return createIsographEnvironment(createIsographStore(), makeNetworkRequest);
  }, []);
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Component {...pageProps} />
    </IsographEnvironmentProvider>
  );
}
