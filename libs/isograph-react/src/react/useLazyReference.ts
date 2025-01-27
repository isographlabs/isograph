import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { FetchOptions } from '../core/check';
import { IsographEntrypoint } from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { logMessage } from '../core/logging';
import type { StartUpdate } from '../core/reader';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export function useLazyReference<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
} {
  const environment = useIsographEnvironment();

  if (entrypoint?.kind !== 'Entrypoint') {
    // TODO have a separate error logger
    logMessage(environment, {
      kind: 'NonEntrypointReceived',
      entrypoint,
    });
  }

  const cache = getOrCreateCacheForArtifact(
    environment,
    entrypoint,
    variables,
    fetchOptions,
  );

  return {
    fragmentReference: useLazyDisposableState(cache).state,
  };
}
