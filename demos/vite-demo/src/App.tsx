import { Suspense, useMemo } from 'react';
import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
} from '@isograph/react';
import HomePageRoute from './components/HomePageRoute';

function makeNetworkRequest<T>(
  queryText: string,
  variables: unknown,
): Promise<T> {
  const promise = fetch('https://graphqlpokemon.favware.tech/v8', {
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
      throw new Error('NetworkError');
    }
  });
  return promise;
}

export default function App() {
  const environment = useMemo(
    () => createIsographEnvironment(createIsographStore(), makeNetworkRequest),
    [],
  );
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Suspense fallback={<div>Loading...</div>}>
        <HomePageRoute />
      </Suspense>
    </IsographEnvironmentProvider>
  );
}
