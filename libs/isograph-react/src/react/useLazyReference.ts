import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { logMessage } from '../core/logging';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export function useLazyReference<
  TReadFromStore extends { parameters: object; data: object },
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst
  >,
  variables: ExtractParameters<TReadFromStore>,
  ...[fetchOptions]: NormalizationAstLoader extends TNormalizationAst
    ? [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
    : [fetchOptions?: FetchOptions<TClientFieldValue>]
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

// @ts-ignore
function tsTests() {
  let withAst!: IsographEntrypoint<any, unknown, NormalizationAst>;
  let withAstLoader!: IsographEntrypoint<any, unknown, NormalizationAstLoader>;
  let withAstOrLoader!: IsographEntrypoint<
    any,
    unknown,
    NormalizationAst | NormalizationAstLoader
  >;

  useLazyReference(withAst, {});
  useLazyReference(withAst, {}, { shouldFetch: 'Yes' });
  useLazyReference(withAst, {}, { shouldFetch: 'IfNecessary' });

  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useLazyReference(withAstLoader, {});
  useLazyReference(withAstLoader, {}, { shouldFetch: 'Yes' });
  // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
  useLazyReference(withAstLoader, {}, { shouldFetch: 'IfNecessary' });

  // if the type is unknown there can be no ast so we should use the same rules
  // @ts-expect-error if the type is unknown, require `shouldFetch` to be specified
  useLazyReference(withAstOrLoader, {});
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'Yes' });
  // @ts-expect-error if the type is unknown, `shouldFetch` can't be `IfNecessary`
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'IfNecessary' });
}
