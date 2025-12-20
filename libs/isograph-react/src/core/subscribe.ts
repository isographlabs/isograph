import { mergeObjectsUsingReaderAst } from './areEqualWithDeepComparison';
import type { EncounteredIds } from './cache';
import type {
  FragmentReference,
  UnknownTReadFromStore,
} from './FragmentReference';
import type {
  FragmentSubscription,
  IsographEnvironment,
} from './IsographEnvironment';
import { logMessage } from './logging';
import { type WithEncounteredRecords, readButDoNotEvaluate } from './read';
import type { ReaderAst } from './reader';

export function subscribe<TReadFromStore extends UnknownTReadFromStore>(
  environment: IsographEnvironment,
  encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  callback: (
    newEncounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  ) => void,
  readerAst: ReaderAst<TReadFromStore>,
): () => void {
  const fragmentSubscription: FragmentSubscription<TReadFromStore> = {
    kind: 'FragmentSubscription',
    callback,
    encounteredDataAndRecords,
    fragmentReference,
    readerAst,
  };

  // subscribe is called in an effect. (We should actually subscribe during the
  // initial render.) Because it's called in an effect, we might have missed some
  // changes since the initial render! So, at this point, we re-read and call the
  // subscription (i.e. re-render) if the fragment data has changed.
  callSubscriptionIfDataChanged(environment, fragmentSubscription);

  environment.subscriptions.add(fragmentSubscription);
  return () => environment.subscriptions.delete(fragmentSubscription);
}

// Calls to readButDoNotEvaluate can suspend (i.e. throw a promise).
// Maybe in the future, they will be able to throw errors.
//
// That's probably okay to ignore. We don't, however, want to prevent
// updating other subscriptions if one subscription had missing data.
function logAnyError(
  environment: IsographEnvironment,
  context: any,
  f: () => void,
) {
  try {
    f();
  } catch (e) {
    logMessage(environment, () => ({
      kind: 'ErrorEncounteredInWithErrorHandling',
      error: e,
      context,
    }));
  }
}

export function callSubscriptions(
  environment: IsographEnvironment,
  recordsEncounteredWhenNormalizing: EncounteredIds,
) {
  environment.subscriptions.forEach((subscription) =>
    logAnyError(environment, { situation: 'calling subscriptions' }, () => {
      switch (subscription.kind) {
        case 'FragmentSubscription': {
          // TODO if there are multiple components subscribed to the same
          // fragment, we will call readButNotEvaluate multiple times. We
          // should fix that.
          if (
            hasOverlappingIds(
              recordsEncounteredWhenNormalizing,
              subscription.encounteredDataAndRecords.encounteredRecords,
            )
          ) {
            callSubscriptionIfDataChanged(environment, subscription);
          }
          return;
        }
        case 'AnyRecords': {
          logAnyError(
            environment,
            { situation: 'calling AnyRecords callback' },
            () => subscription.callback(),
          );
          return;
        }
        case 'AnyChangesToRecord': {
          if (
            recordsEncounteredWhenNormalizing
              .get(subscription.recordLink.__typename)
              ?.has(subscription.recordLink.__link) != null
          ) {
            logAnyError(
              environment,
              { situation: 'calling AnyChangesToRecord callback' },
              () => subscription.callback(),
            );
          }
          return;
        }
      }
    }),
  );
}

function callSubscriptionIfDataChanged<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  subscription: FragmentSubscription<TReadFromStore>,
) {
  const newEncounteredDataAndRecords = readButDoNotEvaluate(
    environment,
    subscription.fragmentReference,
    // Is this wrong?
    // Reasons to think no:
    // - we are only updating the read-out value, and the network
    //   options only affect whether we throw.
    // - the component will re-render, and re-throw on its own, anyway.
    //
    // Reasons to think not:
    // - it seems more efficient to suspend here and not update state,
    //   if we expect that the component will just throw anyway
    // - consistency
    // - it's also weird, this is called from makeNetworkRequest, where
    //   we don't currently pass network request options
    {
      suspendIfInFlight: false,
      throwOnNetworkError: false,
    },
  );
  if (
    newEncounteredDataAndRecords.kind === 'Errors' ||
    subscription.encounteredDataAndRecords.kind === 'Errors'
  ) {
    logAnyError(
      environment,
      { situation: 'calling FragmentSubscription callback' },
      () => {
        subscription.callback(newEncounteredDataAndRecords);
      },
    );
    subscription.encounteredDataAndRecords = newEncounteredDataAndRecords;
    return;
  }

  const oldItem = subscription.encounteredDataAndRecords.item;

  const mergedItem = mergeObjectsUsingReaderAst(
    subscription.readerAst,
    oldItem,
    newEncounteredDataAndRecords.item,
  );

  logMessage(environment, () => ({
    kind: 'DeepEqualityCheck',
    fragmentReference: subscription.fragmentReference,
    old: oldItem,
    new: newEncounteredDataAndRecords.item,
    deeplyEqual: mergedItem === oldItem,
  }));

  if (mergedItem !== oldItem) {
    logAnyError(
      environment,
      { situation: 'calling FragmentSubscription callback' },
      () => {
        subscription.callback(newEncounteredDataAndRecords);
      },
    );
    subscription.encounteredDataAndRecords = newEncounteredDataAndRecords;
  }
}

function hasOverlappingIds(
  ids1: EncounteredIds,
  ids2: EncounteredIds,
): boolean {
  for (const [typeName, set1] of ids1.entries()) {
    const set2 = ids2.get(typeName);
    if (set2 === undefined) {
      continue;
    }

    if (isNotDisjointFrom(set1, set2)) {
      return true;
    }
  }
  return false;
}

// TODO use a polyfill library
function isNotDisjointFrom<T>(set1: Set<T>, set2: Set<T>): boolean {
  for (const id of set1) {
    if (set2.has(id)) {
      return true;
    }
  }
  return false;
}
