import {
  createIsographEnvironment,
  fetchEntrypoint,
  read,
  readAndSubscribe,
  type ReadResult,
} from '@isograph/runtime';
import { HomeRouteEntrypoint as homeRoute } from './HomeRoute';
import { mockNetworkFunction } from './mockNetwork';
import { PetDetailRouteEntrypoint as petDetailRoute } from './PetDetailRoute';
import { SetTaglineEntrypoint as setTagline } from './SetTagline';

/**
 * Fetches and reads `Query.HomeRoute` and `Query.PetDetailRoute`, then changes a pet's
 * tagline with `Mutation.SetTagline`, after which the `Query.HomeRoute` subscription prints
 * the root again.
 */
export async function fetchAndRead(): Promise<void> {
  const environment = createIsographEnvironment(new Map(), mockNetworkFunction);
  const readHomeRoute = () =>
    valueOrThrow(
      homeRoute.name,
      read(
        environment.store,
        homeRoute.readerArtifact(),
        homeRoute.normalizationAst.root,
        {},
      ),
    );

  const [homeRouteRequest, releaseHomeRoute] = fetchEntrypoint(
    environment,
    homeRoute,
    {},
    null,
  );
  await homeRouteRequest.promise;
  console.log(homeRoute.name, readHomeRoute());

  const petDetailVariables = { id: '0' };
  const [petDetailRouteRequest, releasePetDetailRoute] = fetchEntrypoint(
    environment,
    petDetailRoute,
    petDetailVariables,
    null,
  );
  await petDetailRouteRequest.promise;
  console.log(
    petDetailRoute.name,
    petDetailVariables,
    valueOrThrow(
      petDetailRoute.name,
      read(
        environment.store,
        petDetailRoute.readerArtifact(),
        petDetailRoute.normalizationAst.root,
        petDetailVariables,
      ),
    ),
  );

  const [, unsubscribe] = readAndSubscribe(
    environment,
    homeRoute.readerArtifact(),
    homeRoute.normalizationAst.root,
    {},
    () =>
      console.log(
        `${homeRoute.name} after ${setTagline.name}`,
        readHomeRoute(),
      ),
  );

  const [setTaglineRequest, releaseSetTagline] = fetchEntrypoint(
    environment,
    setTagline,
    { id: '0', tagline: 'I AM FULL' },
    {
      shouldFetch: 'Yes',
      onComplete: (tagline) => console.log(setTagline.name, tagline),
    },
  );
  await setTaglineRequest.promise;

  unsubscribe();
  releaseSetTagline();
  releasePetDetailRoute();
  releaseHomeRoute();
}

/** The read's value. A read right after its response was normalized that is `MissingData`
 * means the fixture and the query disagree. */
function valueOrThrow<T>(name: string, result: ReadResult<T>): T {
  if (result.kind === 'MissingData') {
    throw new Error(
      `${name} is missing data after its response was normalized: ${result.reason}`,
    );
  }
  return result.value;
}
