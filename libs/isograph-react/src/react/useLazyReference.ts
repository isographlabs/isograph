import { FragmentReference, Variables } from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { IsographEntrypoint } from '../core/entrypoint';
import { getOrCreateCacheForArtifact } from '../core/cache';
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

  return {
    fragmentReference: useLazyDisposableState(cache).state,
  };
}
