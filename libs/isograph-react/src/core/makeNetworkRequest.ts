import { ItemCleanupPair } from '@isograph/disposable-types';
import {
  IsographEntrypoint,
  RefetchQueryNormalizationArtifact,
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
      return makeNetworkRequest(environment, artifact, variables);
    }
    case 'No': {
      return [wrapResolvedValue(undefined), () => {}];
    }
    case 'IfNecessary': {
      const result = check(
        environment,
        artifact.networkRequestInfo.normalizationAst,
        variables,
        {
          __link: ROOT_ID,
          __typename: artifact.concreteType,
        },
      );
      if (result.kind === 'EnoughData') {
        return [wrapResolvedValue(undefined), () => {}];
      } else {
        return makeNetworkRequest(environment, artifact, variables);
      }
    }
  }
}

export function makeNetworkRequest(
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact | IsographEntrypoint<any, any>,
  variables: Variables,
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
  const promise = environment
    .networkFunction(artifact.networkRequestInfo.queryText, variables)
    .then((networkResponse) => {
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
          artifact.networkRequestInfo.normalizationAst,
          networkResponse.data ?? {},
          variables,
          artifact.kind === 'Entrypoint'
            ? artifact.readerWithRefetchQueries.nestedRefetchQueries
            : [],
          root,
        );
        const retainedQuery = {
          normalizationAst: artifact.networkRequestInfo.normalizationAst,
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
