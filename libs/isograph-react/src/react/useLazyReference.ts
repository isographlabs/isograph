import { FragmentReference, Variables } from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { IsographEntrypoint } from '../core/entrypoint';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';

export function useLazyReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: Variables,
): {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
} {
  // @ts-expect-error
  if (typeof window !== 'undefined' && window.__LOG) {
    if (entrypoint?.kind !== 'Entrypoint') {
      console.warn(
        'useLazyReference was passed an unexpected or invalid object as the first parameter. ' +
          'Is the babel plugin correctly configured? Received=',
        entrypoint,
      );
    }
  }

  const environment = useIsographEnvironment();
  const cache = getOrCreateCacheForArtifact(environment, entrypoint, variables);

  return {
    fragmentReference: useLazyDisposableState(cache).state,
  };
}
