import {
  FragmentReference,
  ExtractParameters,
} from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { IsographEntrypoint } from '../core/entrypoint';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { logMessage } from '../core/logging';

export function useLazyReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<TReadFromStore, TClientFieldValue>,
  variables: ExtractParameters<TReadFromStore>,
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

  const cache = getOrCreateCacheForArtifact(environment, entrypoint, variables);

  return {
    fragmentReference: useLazyDisposableState(cache).state,
  };
}
