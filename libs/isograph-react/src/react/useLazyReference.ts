import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { type NetworkResponseObject } from '../core/cache';
import type { FetchOptions } from '../core/check';
import { type RequiredFetchOptions } from '../core/check';
import type { IsographEntrypoint } from '../core/entrypoint';
import {
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import type {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { type UnknownTReadFromStore } from '../core/FragmentReference';
import { logMessage } from '../core/logging';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { getOrCreateCacheForArtifact } from '../core/getOrCreateCacheForArtifact';

export function useLazyReference<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst,
    TRawResponseType
  >,
  variables: ExtractParameters<TReadFromStore>,
  ...[fetchOptions]: TNormalizationAst extends NormalizationAstLoader
    ? [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
    : [fetchOptions?: FetchOptions<TClientFieldValue, TRawResponseType>]
): NormalizationAst | NormalizationAstLoader extends TNormalizationAst
  ? unknown
  : {
      fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
    } {
  const environment = useIsographEnvironment();

  if (entrypoint?.kind !== 'Entrypoint') {
    // TODO have a separate error logger
    logMessage(environment, () => ({
      kind: 'NonEntrypointReceived',
      entrypoint,
    }));
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
  let withAst!: IsographEntrypoint<any, unknown, NormalizationAst, any>;
  let withAstLoader!: IsographEntrypoint<
    any,
    unknown,
    NormalizationAstLoader,
    {}
  >;
  let withAstOrLoader!: IsographEntrypoint<
    any,
    unknown,
    NormalizationAst | NormalizationAstLoader,
    {}
  >;

  useLazyReference(withAst, {}) satisfies {};
  useLazyReference(withAst, {}, { shouldFetch: 'Yes' }) satisfies {};
  useLazyReference(withAst, {}, { shouldFetch: 'IfNecessary' }) satisfies {};

  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useLazyReference(withAstLoader, {});
  useLazyReference(withAstLoader, {}, { shouldFetch: 'Yes' }) satisfies {};
  // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
  useLazyReference(withAstLoader, {}, { shouldFetch: 'IfNecessary' });

  // if the type is unknown there can be no ast so we should use the same rules
  // but because of TS bugs with inference we just return unknown
  // @ts-expect-error this returns unknown which doesn't satisfy the constraint
  useLazyReference(withAstOrLoader, {}) satisfies {};
  // @ts-expect-error this returns unknown which doesn't satisfy the constraint
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'Yes' }) satisfies {};
  useLazyReference(
    withAstOrLoader,
    {},
    { shouldFetch: 'IfNecessary' },
    // @ts-expect-error this returns unknown which doesn't satisfy the constraint
  ) satisfies {};
}
