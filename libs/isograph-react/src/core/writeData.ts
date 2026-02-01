import type { ItemCleanupPair } from '@isograph/isograph-disposable-types';
import { callSubscriptions } from './subscribe';
import {
  type NetworkResponseObject,
  type EncounteredIds,
  normalizeData,
} from './cache';
import type { IsographEntrypoint, NormalizationAst } from './entrypoint';
import type {
  UnknownTReadFromStore,
  ExtractParameters,
  FragmentReference,
} from './FragmentReference';
import {
  type IsographEnvironment,
  ROOT_ID,
  getOrLoadReaderWithRefetchQueries,
} from './IsographEnvironment';
import { logMessage } from './logging';
import { retainQueryWithoutMakingNetworkRequest } from './makeNetworkRequest';
import { addNetworkResponseStoreLayer } from './optimisticProxy';

export function writeData<
  TReadFromStore extends UnknownTReadFromStore,
  TRawResponseType extends NetworkResponseObject,
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    NormalizationAst,
    TRawResponseType
  >,
  data: TRawResponseType,
  variables: ExtractParameters<TReadFromStore>,
): ItemCleanupPair<FragmentReference<TReadFromStore, TClientFieldValue>> {
  const encounteredIds: EncounteredIds = new Map();
  environment.store = addNetworkResponseStoreLayer(environment.store);
  normalizeData(
    environment,
    environment.store,
    entrypoint.networkRequestInfo.normalizationAst.selections,
    data,
    variables,
    { __link: ROOT_ID, __typename: entrypoint.concreteType },
    encounteredIds,
  );
  logMessage(environment, () => ({
    kind: 'AfterNormalization',
    store: environment.store,
    encounteredIds,
  }));

  callSubscriptions(environment, encounteredIds);

  const { fieldName, readerArtifactKind, readerWithRefetchQueries } =
    getOrLoadReaderWithRefetchQueries(
      environment,
      entrypoint.readerWithRefetchQueries,
    );
  const [networkRequest, disposeNetworkRequest] =
    retainQueryWithoutMakingNetworkRequest(environment, entrypoint, variables);

  return [
    {
      kind: 'FragmentReference',
      readerWithRefetchQueries,
      fieldName,
      readerArtifactKind,
      root: { __link: ROOT_ID, __typename: entrypoint.concreteType },
      variables,
      networkRequest,
    },
    () => {
      disposeNetworkRequest();
    },
  ];
}
