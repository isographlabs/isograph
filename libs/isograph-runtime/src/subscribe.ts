import type { ItemCleanupPair } from '@isograph/disposable-types';
import { selectionSetDataAreEqual } from './areEqualWithDeepComparison';
import type {
  EntityName,
  ReaderArtifact,
  ResolverParam,
  SelectionSetData,
} from './artifacts';
import type { IsographEnvironment } from './environment';
import {
  readButDoNotEvaluate,
  type ReadMissingData,
  type ReadOk,
} from './read';
import type { Link, StoreLink, Variables } from './store';

/** What a subscription delivers: the reader's data, `MissingData`, or what the read threw. */
export type SubscriptionResult =
  | ReadOk<SelectionSetData>
  | ReadMissingData
  | ReadErr;

/** A read that threw: a nested resolver threw, or a store invariant broke (store.md, Errors). */
export type ReadErr = {
  readonly kind: 'Err';
  readonly error: unknown;
};

export type Subscription = {
  readonly artifact: ReaderArtifact<ResolverParam, unknown, EntityName>;
  readonly root: StoreLink;
  readonly variables: Variables;
  /** The result last read: at subscribe, or the last one the callback received. */
  delivered: SubscriptionResult;
  readonly callback: (result: SubscriptionResult) => void;
  /** Called after each change to `delivered`, after `callback`. */
  readonly listeners: Set<() => void>;
};

/**
 * Reads `readerArtifact` at `link` now and subscribes: after every write, re-reads and calls
 * `callback` when the result changed. Returns the subscription, whose `subscriptionSnapshot`
 * is the read, and the cleanup that unsubscribes.
 */
export function readAndSubscribe<TEntityName extends EntityName>(
  environment: IsographEnvironment,
  readerArtifact: ReaderArtifact<
    ResolverParam<TEntityName>,
    unknown,
    TEntityName
  >,
  link: NoInfer<Link<TEntityName>>,
  variables: Variables,
  callback: (result: SubscriptionResult) => void,
): ItemCleanupPair<Subscription> {
  const subscription: Subscription = {
    artifact: readerArtifact,
    root: link,
    variables,
    delivered: readSubscription(environment, readerArtifact, link, variables),
    callback,
    listeners: new Set(),
  };
  environment.subscriptions.add(subscription);
  return [
    subscription,
    () => {
      environment.subscriptions.delete(subscription);
    },
  ];
}

/** The subscription's last result: the same object until a write changes the read. */
export function subscriptionSnapshot(
  subscription: Subscription,
): SubscriptionResult {
  return subscription.delivered;
}

/** Calls `listener` after each change to `subscriptionSnapshot(subscription)`, until the
 * returned function is called. */
export function listenToSubscription(
  subscription: Subscription,
  listener: () => void,
): () => void {
  subscription.listeners.add(listener);
  return () => {
    subscription.listeners.delete(listener);
  };
}

/**
 * Re-reads every subscription in `environment.subscriptions`, in insertion order, and calls
 * each one's callback when its result changed. Never throws.
 */
export function callSubscriptions(environment: IsographEnvironment): void {
  environment.subscriptions.forEach((subscription) => {
    const result = readSubscription(
      environment,
      subscription.artifact,
      subscription.root,
      subscription.variables,
    );
    if (
      !subscriptionResultsAreEqual(
        subscription.artifact,
        subscription.delivered,
        result,
      )
    ) {
      subscription.delivered = result;
      reportCallbackError(() => subscription.callback(result));
      subscription.listeners.forEach((listener) =>
        reportCallbackError(listener),
      );
    }
  });
}

function readSubscription(
  environment: IsographEnvironment,
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  root: StoreLink,
  variables: Variables,
): SubscriptionResult {
  try {
    return readButDoNotEvaluate(environment.store, artifact, root, variables);
  } catch (error: unknown) {
    return { kind: 'Err', error };
  }
}

/** `Ok` results by the reader AST's rule; two `MissingData` are equal; two `Err` when their
 * errors are `===`; different kinds never. */
function subscriptionResultsAreEqual(
  artifact: ReaderArtifact<ResolverParam, unknown, EntityName>,
  delivered: SubscriptionResult,
  result: SubscriptionResult,
): boolean {
  switch (delivered.kind) {
    case 'Ok':
      return (
        result.kind === 'Ok' &&
        selectionSetDataAreEqual(
          artifact.readerAst,
          delivered.value,
          result.value,
        )
      );
    case 'MissingData':
      return result.kind === 'MissingData';
    case 'Err':
      return result.kind === 'Err' && delivered.error === result.error;
  }
}

/** Calls `f`; a throw is rethrown in its own microtask, so it reaches the host's uncaught-error
 * handler and does not stop the caller. */
export function reportCallbackError(f: () => void): void {
  try {
    f();
  } catch (error: unknown) {
    queueMicrotask(() => {
      throw error;
    });
  }
}
