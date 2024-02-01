import type { AppProps } from 'next/app';
import { useMemo } from 'react';
import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
} from '@isograph/react';

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch('https://api.github.com/graphql', {
    method: 'POST',
    headers: {
      Authorization: 'Bearer ' + process.env.NEXT_PUBLIC_GITHUB_TOKEN,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
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
