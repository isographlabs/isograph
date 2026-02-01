import type { ItemCleanupPair } from '@isograph/isograph-disposable-types';
import type { ParentCache } from '@isograph/isograph-react-disposable-state';
import {
  type NetworkResponseObject,
  getOrCreateItemInSuspenseCache,
} from './cache';
import type { FetchOptions } from './check';
import type {
  IsographEntrypoint,
  NormalizationAst,
  NormalizationAstLoader,
} from './entrypoint';
import type {
  ExtractParameters,
  FragmentReference,
  UnknownTReadFromStore,
} from './FragmentReference';
import {
  type IsographEnvironment,
  getOrLoadReaderWithRefetchQueries,
  ROOT_ID,
} from './IsographEnvironment';
import { maybeMakeNetworkRequest } from './makeNetworkRequest';
import { stableCopy } from './util';

export function getOrCreateCacheForArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst,
    TRawResponseType
  >,
  variables: ExtractParameters<TReadFromStore>,
  fetchOptions?: FetchOptions<TClientFieldValue, TRawResponseType>,
): ParentCache<FragmentReference<TReadFromStore, TClientFieldValue>> {
  let cacheKey = '';
  switch (entrypoint.networkRequestInfo.operation.kind) {
    case 'Operation':
      cacheKey =
        entrypoint.networkRequestInfo.operation.text +
        JSON.stringify(stableCopy(variables));
      break;
    case 'PersistedOperation':
      cacheKey =
        entrypoint.networkRequestInfo.operation.operationId +
        JSON.stringify(stableCopy(variables));
      break;
  }
  const factory = () => {
    const { fieldName, readerArtifactKind, readerWithRefetchQueries } =
      getOrLoadReaderWithRefetchQueries(
        environment,
        entrypoint.readerWithRefetchQueries,
      );
    const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
      environment,
      entrypoint,
      variables,
      readerWithRefetchQueries,
      fetchOptions ?? null,
    );

    const itemCleanupPair: ItemCleanupPair<
      FragmentReference<TReadFromStore, TClientFieldValue>
    > = [
      {
        kind: 'FragmentReference',
        readerWithRefetchQueries,
        fieldName,
        readerArtifactKind,
        root: { __link: ROOT_ID, __typename: entrypoint.concreteType },
        variables,
        networkRequest: networkRequest,
      },
      disposeNetworkRequest,
    ];
    return itemCleanupPair;
  };
  return getOrCreateItemInSuspenseCache(environment, cacheKey, factory);
}
