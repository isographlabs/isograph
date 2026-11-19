import { type ItemCleanupPair, ParentCache } from '@isograph/disposable-types';
import type {
  EntityName,
  ExtractParameters,
  IsographEntrypoint,
  NormalizationAst,
  ResolverParam,
} from './artifacts';
import { check } from './check';
import {
  type IsographEnvironment,
  retainQuery,
  unretainQuery,
} from './environment';
import type { RetainedQuery } from './garbageCollection';
import { normalize } from './normalize';
import {
  type PromiseWrapper,
  wrapPromise,
  wrapResolvedValue,
} from './PromiseWrapper';
import { read } from './read';
import type { Variables } from './store';
import { callSubscriptions, reportCallbackError } from './subscribe';
import { stableCopy } from './util';

export type ShouldFetch = 'Yes' | 'No' | 'IfNecessary';

export const DEFAULT_SHOULD_FETCH_VALUE: ShouldFetch = 'IfNecessary';

export type FetchOptions<TReadOutData> = {
  readonly onComplete?: (data: TReadOutData) => void;
  readonly onError?: () => void;
  readonly shouldFetch?: ShouldFetch;
};

/** The entrypoint's query text, then `JSON.stringify(stableCopy(variables))`. */
export type NetworkRequestKey = string;

/**
 * The environment's cache for `entrypoint`'s query text and `variables`, created on first
 * use. Its factory is `fetchEntrypoint` with `fetchOptions`.
 */
export function getOrCreateCacheForArtifact<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ParentCache<PromiseWrapper<void, unknown>> {
  const key: NetworkRequestKey =
    entrypoint.queryText + JSON.stringify(stableCopy(variables));
  const existing = environment.networkRequestCache.get(key);
  if (existing !== undefined) {
    return existing;
  }
  const created = new ParentCache(() =>
    fetchEntrypoint(environment, entrypoint, variables, fetchOptions),
  );
  environment.networkRequestCache.set(key, created);
  return created;
}

/**
 * Sends `entrypoint`'s query with `variables`, or retains it without sending, by
 * `fetchOptions.shouldFetch`. The cleanup unretains the query.
 */
export function fetchEntrypoint<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  switch (fetchOptions?.shouldFetch ?? DEFAULT_SHOULD_FETCH_VALUE) {
    case 'Yes':
      return makeNetworkRequest(
        environment,
        entrypoint,
        variables,
        fetchOptions,
      );
    case 'No':
      return retainQueryWithoutMakingNetworkRequest(
        environment,
        entrypoint.normalizationAst,
        variables,
      );
    case 'IfNecessary':
      return check(environment.store, entrypoint.normalizationAst, variables)
        .kind === 'EnoughData'
        ? retainQueryWithoutMakingNetworkRequest(
            environment,
            entrypoint.normalizationAst,
            variables,
          )
        : makeNetworkRequest(environment, entrypoint, variables, fetchOptions);
  }
}

function retainQueryWithoutMakingNetworkRequest(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  variables: Variables,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  let status: NetworkRequestStatus = {
    kind: 'Undisposed',
    retainedQuery: retainArtifact(environment, ast, variables),
  };
  return [
    wrapResolvedValue(undefined),
    () => {
      if (status.kind === 'Undisposed') {
        unretainQuery(environment, status.retainedQuery);
        status = { kind: 'Disposed' };
      }
    },
  ];
}

function makeNetworkRequest<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
  fetchOptions: FetchOptions<TOutput> | null,
): ItemCleanupPair<PromiseWrapper<void, unknown>> {
  const promise = environment
    .networkFunction({ text: entrypoint.queryText }, variables)
    .then((response) => {
      if (status.kind === 'Disposed') {
        return;
      }
      normalize(
        environment.store,
        entrypoint.normalizationAst,
        response,
        variables,
      );
      callSubscriptions(environment);
      const onComplete = fetchOptions?.onComplete;
      if (onComplete !== undefined) {
        const data = readDataForOnComplete(environment, entrypoint, variables);
        reportCallbackError(() => onComplete(data));
      }
    })
    .catch((error: unknown) => {
      const onError = fetchOptions?.onError;
      if (onError !== undefined) {
        reportCallbackError(onError);
      }
      throw error;
    });
  let status: NetworkRequestStatus = {
    kind: 'Undisposed',
    retainedQuery: retainArtifact(
      environment,
      entrypoint.normalizationAst,
      variables,
    ),
  };
  return [
    wrapPromise(promise),
    () => {
      if (status.kind === 'Undisposed') {
        unretainQuery(environment, status.retainedQuery);
        status = { kind: 'Disposed' };
      }
    },
  ];
}

type NetworkRequestStatus =
  | NetworkRequestStatusUndisposed
  | NetworkRequestStatusDisposed;

type NetworkRequestStatusUndisposed = {
  readonly kind: 'Undisposed';
  readonly retainedQuery: RetainedQuery;
};

type NetworkRequestStatusDisposed = {
  readonly kind: 'Disposed';
};

/** The resolver's value for the response just normalized. */
function readDataForOnComplete<
  TParam extends ResolverParam<TEntityName>,
  TOutput,
  TEntityName extends EntityName,
>(
  environment: IsographEnvironment,
  entrypoint: IsographEntrypoint<TParam, TOutput, TEntityName>,
  variables: ExtractParameters<TParam>,
): TOutput {
  const result = read(
    environment.store,
    entrypoint.readerArtifact(),
    entrypoint.normalizationAst.root,
    variables,
  );
  if (result.kind === 'MissingData') {
    throw new Error(
      `Reading ${entrypoint.name} right after normalizing its response found missing data: ` +
        `${result.reason}. This is indicative of a bug in Isograph.`,
    );
  }
  return result.value;
}

function retainArtifact(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  variables: Variables,
): RetainedQuery {
  const retainedQuery: RetainedQuery = { ast, variables };
  retainQuery(environment, retainedQuery);
  return retainedQuery;
}
