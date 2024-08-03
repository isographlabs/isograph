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
import { IsographEnvironment } from './IsographEnvironment';
import { PromiseWrapper, wrapPromise } from './PromiseWrapper';
import { normalizeData } from './cache';

export function makeNetworkRequest<T>(
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact | IsographEntrypoint<any, any>,
  variables: Variables,
): ItemCleanupPair<PromiseWrapper<T>> {
  if (typeof window !== 'undefined' && window.__LOG) {
    console.log('make network request', artifact, variables);
  }
  let status: NetworkRequestStatus = {
    kind: 'UndisposedIncomplete',
  };
  // This should be an observable, not a promise
  const promise = environment
    .networkFunction(artifact.queryText, variables)
    .then((networkResponse) => {
      if (typeof window !== 'undefined' && window.__LOG) {
        console.log('network response', artifact, networkResponse);
      }

      if (status.kind === 'UndisposedIncomplete') {
        normalizeData(
          environment,
          artifact.normalizationAst,
          networkResponse.data ?? {},
          variables,
          artifact.kind === 'Entrypoint' ? artifact.nestedRefetchQueries : [],
        );
        const retainedQuery = {
          normalizationAst: artifact.normalizationAst,
          variables,
        };
        status = {
          kind: 'UndisposedComplete',
          retainedQuery,
        };
        retainQuery(environment, retainedQuery);
      }
      // TODO return null
      return networkResponse;
    });

  const wrapper = wrapPromise(promise);

  const response: ItemCleanupPair<PromiseWrapper<T>> = [
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
