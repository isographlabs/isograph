import {
  FragmentReference,
  ExtractParameters,
} from '../core/FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { logMessage } from '../core/logging';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';

export function useLazyReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    NormalizationAst
  >,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue>,
): {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
};
export function useLazyReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    NormalizationAstLoader
  >,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions: RequiredFetchOptions<TClientFieldValue>,
): {
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
};
export function useLazyReference<
  TReadFromStore extends { parameters: object; data: object },
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
