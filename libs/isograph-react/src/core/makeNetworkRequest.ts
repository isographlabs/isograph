import { ItemCleanupPair } from '@isograph/disposable-types';
import { callSubscriptions, normalizeData, type EncounteredIds } from './cache';
import { check, DEFAULT_SHOULD_FETCH_VALUE, FetchOptions } from './check';
import { getOrCreateCachedComponent } from './componentCache';
import {
  IsographEntrypoint,
  ReaderWithRefetchQueries,
  RefetchQueryNormalizationArtifact,
  type NormalizationAst,
  type NormalizationAstLoader,
} from './entrypoint';
import {
  ExtractParameters,
  type FragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import {
  garbageCollectEnvironment,
  RetainedQuery,
  retainQuery,
  unretainQuery,
} from './garbageCollection';
import { IsographEnvironment, ROOT_ID, StoreLink } from './IsographEnvironment';
import { logMessage } from './logging';
import { addNetworkResponseStoreLayer } from './optimisticProxy';
import {
  AnyError,
  PromiseWrapper,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import { readButDoNotEvaluate } from './read';
import { getOrCreateCachedStartUpdate } from './startUpdate';

let networkRequestId = 0;

export function maybeMakeNetworkRequest<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TArtifact extends
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue, TNormalizationAst>,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  environment: IsographEnvironment,
  artifact: TArtifact,
  variables: ExtractParameters<TReadFromStore>,
  readerWithRefetchQueries: PromiseWrapper<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  > | null,
  fetchOptions: FetchOptions<TClientFieldValue> | null,
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
          'Using lazy loaded normalizationAst with shouldFetch: "IfNecessary" is not supported as it will lead to slower initial load time.',
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

function retainQueryWithoutMakingNetworkRequest<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        NormalizationAst | NormalizationAstLoader
      >,
  variables: ExtractParameters<TReadFromStore>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  let status: NetworkRequestStatus = {
    kind: 'Undisposed',
    retainedQuery: fetchNormalizationAstAndRetainArtifact(
      environment,
      artifact,
      variables,
    ),
  };
  return [
    wrapResolvedValue(undefined),
    () => {
      if (status.kind === 'Undisposed') {
        unretainAndGarbageCollect(environment, status);
      }
      status = {
        kind: 'Disposed',
      };
    },
  ];
}

export function makeNetworkRequest<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TArtifact extends
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue, TNormalizationAst>,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  environment: IsographEnvironment,
  artifact: TArtifact,
  variables: ExtractParameters<TReadFromStore>,
  readerWithRefetchQueries: PromiseWrapper<
    ReaderWithRefetchQueries<TReadFromStore, TClientFieldValue>
  > | null,
  fetchOptions: FetchOptions<TClientFieldValue> | null,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  // TODO this should be a DataId and stored in the store
  const myNetworkRequestId = networkRequestId + '';
  networkRequestId++;
  let status: NetworkRequestStatus = {
    kind: 'Undisposed',
    retainedQuery: fetchNormalizationAstAndRetainArtifact(
      environment,
      artifact,
      variables,
    ),
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
        throw new Error('GraphQL network response had errors', {
          cause: networkResponse,
        });
      }

      const root = { __link: ROOT_ID, __typename: artifact.concreteType };
      if (status.kind === 'Undisposed') {
        const encounteredIds: EncounteredIds = new Map();
        environment.store = addNetworkResponseStoreLayer(
          environment.store,
          (storeLayer) => {
            normalizeData(
              environment,
              storeLayer,
              normalizationAst.selections,
              networkResponse.data ?? {},
              variables,
              root,
              encounteredIds,
            );
          },
        );

        logMessage(environment, () => ({
          kind: 'AfterNormalization',
          store: environment.store,
          encounteredIds: encounteredIds,
        }));

        callSubscriptions(environment, encounteredIds);

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
      throw e;
    });

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<void, AnyError>> = [
    wrapper,
    () => {
      if (status.kind === 'Undisposed') {
        unretainAndGarbageCollect(environment, status);
      }
      status = {
        kind: 'Disposed',
      };
    },
  ];
  return response;
}

type NetworkRequestStatusUndisposed = {
  readonly kind: 'Undisposed';
  readonly retainedQuery: RetainedQuery;
};

type NetworkRequestStatus =
  | NetworkRequestStatusUndisposed
  | {
      readonly kind: 'Disposed';
    };

function readDataForOnComplete<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TArtifact extends
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<TReadFromStore, TClientFieldValue, TNormalizationAst>,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  artifact: TArtifact,
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
    const resolvedReaderWithRefetchQueries =
      readerWithRefetchQueries as ReaderWithRefetchQueries<
        TReadFromStore,
        TClientFieldValue
      >;

    const fragment: FragmentReference<TReadFromStore, TClientFieldValue> = {
      kind: 'FragmentReference',
      // TODO this smells.
      readerWithRefetchQueries: wrapResolvedValue(
        resolvedReaderWithRefetchQueries,
      ),
      root,
      variables,
      networkRequest: fakeNetworkRequest,
    };
    const fragmentResult = readButDoNotEvaluate(
      environment,
      fragment,
      fakeNetworkRequestOptions,
    ).item;
    const readerArtifact = resolvedReaderWithRefetchQueries.readerArtifact;
    switch (readerArtifact.kind) {
      case 'ComponentReaderArtifact': {
        // @ts-expect-error We should find a way to encode this in the type system:
        // if we have a ComponentReaderArtifact, we will necessarily have a
        // TClientFieldValue which is a React.FC<...>
        return getOrCreateCachedComponent(
          environment,
          readerArtifact.fieldName,
          {
            kind: 'FragmentReference',
            readerWithRefetchQueries: wrapResolvedValue({
              kind: 'ReaderWithRefetchQueries',
              readerArtifact: readerArtifact,
              nestedRefetchQueries:
                resolvedReaderWithRefetchQueries.nestedRefetchQueries,
            }),
            root,
            variables,
            networkRequest: fakeNetworkRequest,
          } as const,
          fakeNetworkRequestOptions,
        );
      }
      case 'EagerReaderArtifact': {
        return readerArtifact.resolver({
          data: fragmentResult,
          parameters: variables,
          ...(readerArtifact.hasUpdatable
            ? {
                startUpdate: getOrCreateCachedStartUpdate(
                  environment,
                  fragment,
                  resolvedReaderWithRefetchQueries.readerArtifact.fieldName,
                  fakeNetworkRequestOptions,
                ),
              }
            : undefined),
        });
      }
      default: {
        const _: never = readerArtifact;
        _;
        throw new Error('Expected case');
      }
    }
  }
  return null;
}

function fetchNormalizationAstAndRetainArtifact<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
>(
  environment: IsographEnvironment,
  artifact:
    | RefetchQueryNormalizationArtifact
    | IsographEntrypoint<
        TReadFromStore,
        TClientFieldValue,
        NormalizationAst | NormalizationAstLoader
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

function unretainAndGarbageCollect(
  environment: IsographEnvironment,
  status: NetworkRequestStatusUndisposed,
) {
  const didUnretainSomeQuery = unretainQuery(environment, status.retainedQuery);
  if (didUnretainSomeQuery) {
    garbageCollectEnvironment(environment);
  }
}
