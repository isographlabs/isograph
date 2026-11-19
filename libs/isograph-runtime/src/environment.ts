import type { ParentCache } from '@isograph/disposable-types';
import { garbageCollect, type RetainedQuery } from './garbageCollection';
import type { NetworkRequestKey } from './network';
import type { PromiseWrapper } from './PromiseWrapper';
import type { JsonObject, Store, Variables } from './store';
import type { Subscription } from './subscribe';

/** The store, the network function, and what is retained, cached, and subscribed. */
export type IsographEnvironment = {
  readonly store: Store;
  readonly networkFunction: IsographNetworkFunction;
  /** Every query a request holds; `garbageCollectEnvironment` keeps what these reach. */
  readonly retainedQueries: Set<RetainedQuery>;
  /** One cache per query text and variables, holding the request while a caller retains it. */
  readonly networkRequestCache: Map<
    NetworkRequestKey,
    ParentCache<PromiseWrapper<void, unknown>>
  >;
  /** Re-read after every write the environment makes. */
  readonly subscriptions: Set<Subscription>;
};

/** Sends `operation` with `variables`. Resolves to the operation's data object, or rejects. */
export type IsographNetworkFunction = (
  operation: IsographOperation,
  variables: Variables,
) => Promise<JsonObject>;

export type IsographOperation = {
  readonly text: string;
};

/** The app's one environment, created once. */
export function createIsographEnvironment(
  store: Store,
  networkFunction: IsographNetworkFunction,
): IsographEnvironment {
  return {
    store,
    networkFunction,
    retainedQueries: new Set(),
    networkRequestCache: new Map(),
    subscriptions: new Set(),
  };
}

export function retainQuery(
  environment: IsographEnvironment,
  queryToRetain: RetainedQuery,
): void {
  environment.retainedQueries.add(queryToRetain);
}

export function unretainQuery(
  environment: IsographEnvironment,
  retainedQuery: RetainedQuery,
): void {
  environment.retainedQueries.delete(retainedQuery);
}

/** `garbageCollect` of the store, keeping what the retained queries reach. */
export function garbageCollectEnvironment(
  environment: IsographEnvironment,
): void {
  garbageCollect(environment.store, Array.from(environment.retainedQueries));
}
