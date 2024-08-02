import { FragmentReference, Variables } from './FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ROOT_ID } from './IsographEnvironment';
import { IsographEntrypoint } from './entrypoint';
import { getOrCreateCacheForArtifact } from './cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';

export function useLazyReference<
  TReadFromStore extends Object,
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: Variables,
): {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
} {
  const environment = useIsographEnvironment();
  const cache = getOrCreateCacheForArtifact(environment, entrypoint, variables);

  // TODO add comment explaining why we never use this value
  // @ts-ignore(6133)
  const _data = useLazyDisposableState(cache).state;

  return {
    fragmentReference: {
      kind: 'FragmentReference',
      readerArtifact: entrypoint.readerArtifact,
      root: ROOT_ID,
      variables,
      nestedRefetchQueries: entrypoint.nestedRefetchQueries,
    },
  };
}
