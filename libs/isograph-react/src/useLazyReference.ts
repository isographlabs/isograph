import { FragmentReference, Variable } from './FragmentReference';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { ROOT_ID } from './IsographEnvironment';
import {
  ExtractReadFromStore,
  ExtractResolverResult,
  assertIsEntrypoint,
} from './entrypoint';
import { getOrCreateCacheForArtifact } from './cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { type PromiseWrapper } from './PromiseWrapper';

// Note: we cannot write TEntrypoint extends IsographEntrypoint<any, any, any>, or else
// if we do not explicitly pass a type, the read out type will be any.
// We cannot write TEntrypoint extends IsographEntrypoint<never, never, never>, or else
// any actual Entrypoint we pass will not be valid.
export function useLazyReference<TEntrypoint>(
  entrypoint:
    | TEntrypoint
    // Temporarily, we need to allow useLazyReference to take the result of calling
    // iso(`...`). At runtime, we confirm that the passed-in `iso` literal is actually
    // an entrypoint.
    | ((_: any) => any),
  variables: { [key: string]: Variable },
): {
  queryReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >;
} {
  const environment = useIsographEnvironment();
  assertIsEntrypoint<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >(entrypoint);
  const cache = getOrCreateCacheForArtifact<ExtractResolverResult<TEntrypoint>>(
    environment,
    entrypoint,
    variables,
  );

  // TODO add comment explaining why we never use this value
  // @ts-ignore
  const data =
    useLazyDisposableState<PromiseWrapper<ExtractResolverResult<TEntrypoint>>>(
      cache,
    ).state;

  return {
    queryReference: {
      kind: 'FragmentReference',
      readerArtifact: entrypoint.readerArtifact,
      root: ROOT_ID,
      variables,
      nestedRefetchQueries: entrypoint.nestedRefetchQueries,
    },
  };
}
