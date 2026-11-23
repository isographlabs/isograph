import type { ItemCleanupPair } from '@isograph/disposable-types';
import {
  normalizeData,
  type EncounteredIds,
  type NetworkResponseObject,
} from './cache';
import type { FetchOptions } from './check';
import { check, DEFAULT_SHOULD_FETCH_VALUE } from './check';
import { getOrCreateCachedComponent } from './componentCache';
import type {
  IsographEntrypoint,
  NormalizationAst,
  NormalizationAstLoader,
  ReaderWithRefetchQueries,
  RefetchQueryNormalizationArtifact,
} from './entrypoint';
import type {
  ExtractParameters,
  FragmentReference,
  UnknownTReadFromStore,
} from './FragmentReference';
import type { RetainedQuery } from './garbageCollection';
import {
  garbageCollectEnvironment,
  retainQuery,
  unretainQuery,
} from './garbageCollection';
import type { IsographEnvironment, StoreLink } from './IsographEnvironment';
import { ROOT_ID } from './IsographEnvironment';
import { logMessage } from './logging';
import {
  addNetworkResponseStoreLayer,
  addOptimisticNetworkResponseStoreLayer,
  revertOptimisticStoreLayerAndMaybeReplace,
  type OptimisticStoreLayer,
  type StoreLayerWithData,
} from './optimisticProxy';
import type { AnyError, PromiseWrapper } from './PromiseWrapper';
import { wrapPromise, wrapResolvedValue } from './PromiseWrapper';
import { readButDoNotEvaluate } from './read';
import { getOrCreateCachedStartUpdate } from './startUpdate';
import { callSubscriptions } from './subscribe';

let networkRequestId = 0;

export function maybeMakeNetworkRequest<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        TNormalizationAst,
        TRawResponseType
      >,
  variables: ExtractParameters<TReadFromStore>,
  readerWithRefetchQueries: PromiseWrapper<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  > | null,
  fetchOptions: FetchOptions<TClientFieldValue, TRawResponseType> | null,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  switch (fetchOptions?.shouldFetch ?? DEFAULT_SHOULD_FETCH_VALUE) {
    case 'Yes': {
      return makeNetworkRequest(
        environment,
        artifact,
        variables,
        readerWithRefetchQueries,
        fetchOptions,
      );
    }
    case 'No': {
      return retainQueryWithoutMakingNetworkRequest(
        environment,
        artifact,
        variables,
      );
    }
    case 'IfNecessary': {
      if (
        artifact.networkRequestInfo.normalizationAst.kind ===
        'NormalizationAstLoader'
      ) {
        throw new Error(
          'Using lazy loaded normalizationAst with shouldFetch: "IfNecessary" is ' +
            'not supported as it will lead to a network waterfall.',
        );
      }
      const result = check(
        environment,
        artifact.networkRequestInfo.normalizationAst.selections,
        variables,
        {
          __link: ROOT_ID,
          __typename: artifact.concreteType,
        },
      );

      if (result.kind === 'EnoughData') {
        return retainQueryWithoutMakingNetworkRequest(
          environment,
          artifact,
          variables,
        );
      } else {
        return makeNetworkRequest(
          environment,
          artifact,
          variables,
          readerWithRefetchQueries,
          fetchOptions,
        );
      }
    }
  }
}

export function retainQueryWithoutMakingNetworkRequest<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TRawResponseType extends NetworkResponseObject,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        NormalizationAst | NormalizationAstLoader,
        TRawResponseType
      >,
  variables: ExtractParameters<TReadFromStore>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  let status:
    | NetworkRequestStatusUndisposedComplete
    | NetworkRequestStatusDisposed = {
    kind: 'UndisposedComplete',
    retainedQuery: fetchNormalizationAstAndRetainArtifact(
      environment,
      artifact,
      variables,
    ),
  };
  return [
    wrapResolvedValue(undefined),
    () => {
      if (status.kind !== 'Disposed') {
        status = unretainAndGarbageCollect(environment, status);
      }
    },
  ];
}

export function makeNetworkRequest<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        TNormalizationAst,
        TRawResponseType
      >,
  variables: ExtractParameters<TReadFromStore>,
  readerWithRefetchQueries: PromiseWrapper<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  > | null,
  fetchOptions: FetchOptions<TClientFieldValue, TRawResponseType> | null,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  // TODO this should be a DataId and stored in the store
  const myNetworkRequestId = networkRequestId + '';
  networkRequestId++;
  let status: NetworkRequestStatus = {
    kind: 'UndisposedIncomplete',
    retainedQuery: fetchNormalizationAstAndRetainArtifact(
      environment,
      artifact,
      variables,
    ),
    optimistic:
      fetchOptions?.optimisticNetworkResponse != null
        ? makeOptimisticUpdate(
            environment,
            artifact,
            variables,
            fetchOptions?.optimisticNetworkResponse,
          )
        : null,
  };

  logMessage(environment, () => ({
    kind: 'MakeNetworkRequest',
    artifact,
    variables,
    networkRequestId: myNetworkRequestId,
  }));

  // This should be an observable, not a promise
  const promise = Promise.all([
    environment.networkFunction(
      artifact.networkRequestInfo.operation,
      variables,
    ),
    status.retainedQuery.normalizationAst.promise,
    readerWithRefetchQueries?.promise,
  ])
    .then(([networkResponse, normalizationAst, readerWithRefetchQueries]) => {
      logMessage(environment, () => ({
        kind: 'ReceivedNetworkResponse',
        networkResponse,
        networkRequestId: myNetworkRequestId,
      }));

      if (networkResponse.errors != null) {
        try {
          fetchOptions?.onError?.();
        } catch {}
        throw new Error('Network response had errors', {
          cause: networkResponse,
        });
      }

      const root = { __link: ROOT_ID, __typename: artifact.concreteType };

      if (status.kind === 'UndisposedIncomplete') {
        if (status.optimistic != null) {
          status =
            revertOptimisticStoreLayerAndMaybeReplaceIfUndisposedIncomplete(
              environment,
              status,
              (storeLayer) =>
                normalizeData(
                  environment,
                  storeLayer,
                  normalizationAst.selections,
                  networkResponse.data ?? {},
                  variables,
                  root,
                  new Map(),
                ),
            );
        } else {
          const encounteredIds: EncounteredIds = new Map();
          environment.store = addNetworkResponseStoreLayer(environment.store);
          normalizeData(
            environment,
            environment.store,
            normalizationAst.selections,
            networkResponse.data ?? {},
            variables,
            root,
            encounteredIds,
          );

          logMessage(environment, () => ({
            kind: 'AfterNormalization',
            store: environment.store,
            encounteredIds: encounteredIds,
          }));

          callSubscriptions(environment, encounteredIds);

          status = {
            kind: 'UndisposedComplete',
            retainedQuery: status.retainedQuery,
          };
        }
      }

      const onComplete = fetchOptions?.onComplete;
      if (onComplete != null) {
        let data = readDataForOnComplete(
          artifact,
          environment,
          root,
          variables,
          readerWithRefetchQueries,
        );

        try {
          // @ts-expect-error this problem will be fixed when we remove RefetchQueryNormalizationArtifact
          // (or we can fix this by having a single param of type { kind: 'Entrypoint', entrypoint,
          // fetchOptions: FetchOptions<TReadFromStore> } | { kind: 'RefetchQuery', refetchQuery,
          // fetchOptions: FetchOptions<void> }).
          onComplete(data);
        } catch {}
      }
    })
    .catch((e) => {
      logMessage(environment, () => ({
        kind: 'ReceivedNetworkError',
        networkRequestId: myNetworkRequestId,
        error: e,
      }));
      try {
        fetchOptions?.onError?.();
      } catch {}

      if (status.kind === 'UndisposedIncomplete') {
        status =
          revertOptimisticStoreLayerAndMaybeReplaceIfUndisposedIncomplete(
            environment,
            status,
            null,
          );
      }

      throw e;
    });

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<void, AnyError>> = [
    wrapper,
    () => {
      if (status.kind === 'UndisposedIncomplete') {
        status =
          revertOptimisticStoreLayerAndMaybeReplaceIfUndisposedIncomplete(
            environment,
            status,
            null,
          );
      }
      if (status.kind !== 'Disposed') {
        status = unretainAndGarbageCollect(environment, status);
      }
    },
  ];
  return response;
}

type NetworkRequestStatusUndisposedIncomplete = {
  readonly kind: 'UndisposedIncomplete';
  readonly retainedQuery: RetainedQuery;
  readonly optimistic: OptimisticStoreLayer | null;
};

type NetworkRequestStatusUndisposedComplete = {
  readonly kind: 'UndisposedComplete';
  readonly retainedQuery: RetainedQuery;
};

type NetworkRequestStatusDisposed = {
  readonly kind: 'Disposed';
};

type NetworkRequestStatus =
  | NetworkRequestStatusUndisposedIncomplete
  | NetworkRequestStatusUndisposedComplete
  | NetworkRequestStatusDisposed;

function readDataForOnComplete<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        TNormalizationAst,
        TRawResponseType
      >,
  environment: IsographEnvironment,
  root: StoreLink,
  variables: ExtractParameters<TReadFromStore>,
  readerWithRefetchQueries:
    | ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
    | undefined,
): TClientFieldValue | null {
  // An entrypoint, but not a RefetchQueryNormalizationArtifact, has a reader ASTs.
  // So, we can only pass data to onComplete if makeNetworkRequest was passed an entrypoint.
  // This is awkward, since we don't express that in the types of the parameters
  // (i.e. FetchOptions could be passed, along with a RefetchQueryNormalizationArtifact).
  //
  // However, this isn't a big deal: RefetchQueryNormalizationArtifact is going away.
  if (artifact.kind === 'Entrypoint') {
    // TODO this is a smell!
    const fakeNetworkRequest = wrapResolvedValue(undefined);
    // TODO this is a smell — we know the network response is not in flight,
    // so we don't really care!
    const fakeNetworkRequestOptions = {
      suspendIfInFlight: false,
      throwOnNetworkError: false,
    };
    if (readerWithRefetchQueries == null) {
      throw new Error(
        'Expected readerWithRefetchQueries to be not null. This is indicative of a bug in Isograph.',
      );
    }

    const fragment: FragmentReference<TReadFromStore, TClientFieldValue> = {
      kind: 'FragmentReference',
      // TODO this smells.
      readerWithRefetchQueries: wrapResolvedValue(readerWithRefetchQueries),
      fieldName: readerWithRefetchQueries.readerArtifact.fieldName,
      readerArtifactKind: readerWithRefetchQueries.readerArtifact.kind,
      root,
      variables,
      networkRequest: fakeNetworkRequest,
    };
    const fragmentResult = readButDoNotEvaluate(
      environment,
      fragment,
      fakeNetworkRequestOptions,
    ).item;
    const readerArtifact = readerWithRefetchQueries.readerArtifact;
    switch (readerArtifact.kind) {
      case 'ComponentReaderArtifact': {
        // @ts-expect-error We should find a way to encode this in the type system:
        // if we have a ComponentReaderArtifact, we will necessarily have a
        // TClientFieldValue which is a React.FC<...>
        return getOrCreateCachedComponent(
          environment,
          {
            kind: 'FragmentReference',
            readerWithRefetchQueries: wrapResolvedValue({
              kind: 'ReaderWithRefetchQueries',
              readerArtifact: readerArtifact,
              nestedRefetchQueries:
                readerWithRefetchQueries.nestedRefetchQueries,
            }),
            fieldName: readerArtifact.fieldName,
            readerArtifactKind: readerArtifact.kind,
            root,
            variables,
            networkRequest: fakeNetworkRequest,
          } as const,
          fakeNetworkRequestOptions,
        );
      }
      case 'EagerReaderArtifact': {
        return readerArtifact.resolver({
          firstParameter: {
            data: fragmentResult,
            parameters: variables,
            ...(readerArtifact.hasUpdatable
              ? {
                  startUpdate: getOrCreateCachedStartUpdate(
                    environment,
                    fragment,
                    fakeNetworkRequestOptions,
                  ),
                }
              : undefined),
          },
        });
      }
    }
  }
  return null;
}

function fetchNormalizationAstAndRetainArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TRawResponseType extends NetworkResponseObject,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        NormalizationAst | NormalizationAstLoader,
        TRawResponseType
      >,
  variables: ExtractParameters<TReadFromStore>,
): RetainedQuery {
  const normalizationAst =
    artifact.networkRequestInfo.normalizationAst.kind === 'NormalizationAst'
      ? wrapResolvedValue(artifact.networkRequestInfo.normalizationAst)
      : wrapPromise(artifact.networkRequestInfo.normalizationAst.loader());

  const root = { __link: ROOT_ID, __typename: artifact.concreteType };
  const retainedQuery: RetainedQuery = {
    normalizationAst: normalizationAst,
    variables,
    root,
  };
  retainQuery(environment, retainedQuery);
  return retainedQuery;
}

function makeOptimisticUpdate<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        TNormalizationAst,
        TRawResponseType
      >,
  variables: ExtractParameters<TReadFromStore>,
  optimisticNetworkResponse: TRawResponseType,
): OptimisticStoreLayer {
  const root = { __link: ROOT_ID, __typename: artifact.concreteType };

  if (
    artifact.networkRequestInfo.normalizationAst.kind ===
    'NormalizationAstLoader'
  ) {
    throw new Error(
      'Using lazy loaded normalizationAst with optimisticNetworkResponse is not supported.',
    );
  }
  const encounteredIds: EncounteredIds = new Map();
  const optimistic = (environment.store =
    addOptimisticNetworkResponseStoreLayer(environment.store));
  normalizeData(
    environment,
    environment.store,
    artifact.networkRequestInfo.normalizationAst.selections,
    optimisticNetworkResponse,
    variables,
    root,
    encounteredIds,
  );

  logMessage(environment, () => ({
    kind: 'AfterNormalization',
    store: environment.store,
    encounteredIds: encounteredIds,
  }));

  callSubscriptions(environment, encounteredIds);
  return optimistic;
}

function revertOptimisticStoreLayerAndMaybeReplaceIfUndisposedIncomplete(
  environment: IsographEnvironment,
  status: NetworkRequestStatusUndisposedIncomplete,
  normalizeData: null | ((storeLayer: StoreLayerWithData) => void),
): NetworkRequestStatusUndisposedComplete {
  if (status.optimistic != null) {
    revertOptimisticStoreLayerAndMaybeReplace(
      environment,
      status.optimistic,
      normalizeData,
    );
  }

  return {
    kind: 'UndisposedComplete',
    retainedQuery: status.retainedQuery,
  };
}

function unretainAndGarbageCollect(
  environment: IsographEnvironment,
  status: NetworkRequestStatusUndisposedComplete,
): NetworkRequestStatusDisposed {
  const didUnretainSomeQuery = unretainQuery(environment, status.retainedQuery);
  if (didUnretainSomeQuery) {
    garbageCollectEnvironment(environment);
  }

  return {
    kind: 'Disposed',
  };
}
