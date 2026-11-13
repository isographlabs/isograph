import {
  createIsographEnvironment,
  createIsographStore,
  IsographEnvironmentProvider,
  type IsographOperation,
} from '@isograph/react';
import { Suspense, useMemo } from 'react';
import HomePageRoute from './components/HomePageRoute';

function makeNetworkRequest<T>(
  operation: IsographOperation,
  variables: unknown,
): Promise<T> {
  const promise = fetch('https://graphqlpokemon.favware.tech/v8', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ query: operation.text, variables }),
  }).then(async (response) => {
    const json = await response.json();

    if (response.ok) {
      /**
       * Enforce that the network response follows the specification:: {@link https://spec.graphql.org/draft/#sec-Errors}.
       */
      if (Object.hasOwn(json, 'errors')) {
        if (!Array.isArray(json.errors) || json.errors.length === 0) {
          throw new Error('GraphQLSpecificationViolationError', {
            cause: json,
          });
        }
        throw new Error('GraphQLError', {
          cause: json.errors,
        });
      }
      return json;
    }
    throw new Error('NetworkError', {
      cause: json,
    });
  });
  return promise;
}

export default function App() {
  const environment = useMemo(
    () =>
      createIsographEnvironment(
        createIsographStore(),
        // @ts-expect-error network function and environment should be generated
        makeNetworkRequest,
        null,
        typeof window !== 'undefined' ? console.log : null,
      ),
    [],
  );
  return (
    <IsographEnvironmentProvider environment={environment}>
      <Suspense fallback={<div>Loading Pokemon...</div>}>
        <HomePageRoute />
      </Suspense>
    </IsographEnvironmentProvider>
  );
}
