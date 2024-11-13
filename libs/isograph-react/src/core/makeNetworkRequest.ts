import { ItemCleanupPair, type CleanupFn } from '@isograph/disposable-types';
import {
  IsographEntrypoint,
  RefetchQueryNormalizationArtifact,
  type NormalizationAst,
  type NormalizationAstLoader,
} from './entrypoint';
import { Variables } from './FragmentReference';
import {
  garbageCollectEnvironment,
  RetainedQuery,
  retainQuery,
  unretainQuery,
} from './garbageCollection';
import { IsographEnvironment, ROOT_ID } from './IsographEnvironment';
import {
  AnyError,
  PromiseWrapper,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import { normalizeData } from './cache';
import { logMessage } from './logging';
import { check, ShouldFetch } from './check';

let networkRequestId = 0;

export function maybeMakeNetworkRequest(
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact | IsographEntrypoint<any, any>,
  variables: Variables,
  shouldFetch: ShouldFetch,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  switch (shouldFetch) {
    case 'Yes': {
      return makeNetworkRequest(environment, artifact, variables, () =>
        loadNormalizationAst(artifact.networkRequestInfo.normalizationAst),
      );
    }
    case 'No': {
      return [wrapResolvedValue(undefined), () => {}];
    }
    case 'IfNecessary': {
      return makeNetworkRequestIfNecessary(
        environment,
        artifact,
        variables,
        () =>
          loadNormalizationAst(artifact.networkRequestInfo.normalizationAst),
      );
    }
  }
}

function loadNormalizationAst(
  normalizationAst: NormalizationAstLoader | NormalizationAst,
) {
  switch (normalizationAst.kind) {
    case 'NormalizationAst': {
      return Promise.resolve(normalizationAst);
    }
    case 'NormalizationAstLoader': {
      return normalizationAst.loader();
    }
  }
}

export function makeNetworkRequest(
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact | IsographEntrypoint<any, any>,
  variables: Variables,
  normalizationAstLoader: () => Promise<NormalizationAst>,
): ItemCleanupPair<PromiseWrapper<void, AnyError>> {
  // TODO this should be a DataId and stored in the store
  const myNetworkRequestId = networkRequestId + '';
  networkRequestId++;

  logMessage(environment, {
    kind: 'MakeNetworkRequest',
    artifact,
    variables,
    networkRequestId: myNetworkRequestId,
  });

  let status: NetworkRequestStatus = {
    kind: 'UndisposedIncomplete',
  };
  // This should be an observable, not a promise
  const promise = Promise.all([
    environment.networkFunction(
      artifact.networkRequestInfo.queryText,
      variables,
    ),
    normalizationAstLoader(),
  ]).then(([networkResponse, normalizationAst]) => {
    logMessage(environment, {
      kind: 'ReceivedNetworkResponse',
      networkResponse,
      networkRequestId: myNetworkRequestId,
    });

    if (networkResponse.errors != null) {
      // @ts-expect-error Why are we getting the wrong constructor here?
      throw new Error('GraphQL network response had errors', {
        cause: networkResponse,
      });
    }

    if (status.kind === 'UndisposedIncomplete') {
      const root = { __link: ROOT_ID, __typename: artifact.concreteType };
      normalizeData(
        environment,
        normalizationAst,
        networkResponse.data ?? {},
        variables,
        artifact.kind === 'Entrypoint'
          ? artifact.readerWithRefetchQueries.nestedRefetchQueries
          : [],
        root,
      );
      const retainedQuery = {
        normalizationAst: normalizationAst,
        variables,
        root,
      };
      status = {
        kind: 'UndisposedComplete',
        retainedQuery,
      };
      retainQuery(environment, retainedQuery);
    }
  });

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<void, AnyError>> = [
    wrapper,
    () => {
      if (status.kind === 'UndisposedComplete') {
        const didUnretainSomeQuery = unretainQuery(
          environment,
          status.retainedQuery,
        );
        if (didUnretainSomeQuery) {
          garbageCollectEnvironment(environment);
        }
      }
      status = {
        kind: 'Disposed',
      };
    },
  ];
  return response;
}

type NetworkRequestStatus =
  | {
      readonly kind: 'UndisposedIncomplete';
    }
  | {
      readonly kind: 'Disposed';
    }
  | {
      readonly kind: 'UndisposedComplete';
      readonly retainedQuery: RetainedQuery;
    };

function makeNetworkRequestIfNecessary(
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact | IsographEntrypoint<any, any>,
  variables: Variables,
  normalizationAstLoader: () => Promise<NormalizationAst>,
) {
  return flatMapNetworkRequest(
    [wrapPromise(normalizationAstLoader()), () => {}],
    (normalizationAst) => {
      const result = check(environment, normalizationAst, variables, {
        __link: ROOT_ID,
        __typename: artifact.concreteType,
      });

      if (result.kind === 'EnoughData') {
        return [wrapResolvedValue(undefined), () => {}];
      } else {
        return makeNetworkRequest(
          environment,
          artifact,
          variables,
          normalizationAstLoader,
        );
      }
    },
  );
}

function flatMapNetworkRequest<T, R, E>(
  networkRequest: ItemCleanupPair<PromiseWrapper<T, E>>,
  fn: (value: T) => ItemCleanupPair<PromiseWrapper<R, E>>,
): ItemCleanupPair<PromiseWrapper<R, E>> {
  let networkRequestState:
    | {
        kind: 'Pending';
      }
    | {
        kind: 'NetworkRequestStarted';
        disposeNetworkRequest: CleanupFn;
      }
    | { kind: 'Disposed' } = { kind: 'Pending' };

  const promiseWrapper = wrapPromise(
    networkRequest[0].promise.then((value) => {
      if (networkRequestState.kind === 'Pending') {
        const [networkRequest, disposeNetworkRequest] = fn(value);
        networkRequestState = {
          kind: 'NetworkRequestStarted',
          disposeNetworkRequest,
        };
        return networkRequest.promise;
      }
      throw new Error('Expected network request to be pending');
    }),
  );

  return [
    promiseWrapper,
    () => {
      if (networkRequestState.kind === 'NetworkRequestStarted') {
        networkRequestState.disposeNetworkRequest();
      }
      networkRequestState = { kind: 'Disposed' };
    },
  ];
}
