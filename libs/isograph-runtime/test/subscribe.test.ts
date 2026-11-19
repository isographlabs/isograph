import { createTemporarilyRetainedCacheItem } from '@isograph/disposable-types';
import { afterEach, describe, expect, test, vi } from 'vitest';
import constantReaderArtifact from '../fixtures/store/src/__isograph/Pet/Constant/resolver_reader';
import feedNormalizationAst from '../fixtures/store/src/__isograph/Query/Feed/normalization_ast';
import feedReaderArtifact from '../fixtures/store/src/__isograph/Query/Feed/resolver_reader';
import leavesNormalizationAst from '../fixtures/store/src/__isograph/Query/Leaves/normalization_ast';
import leavesReaderArtifact from '../fixtures/store/src/__isograph/Query/Leaves/resolver_reader';
import petDetailNormalizationAst from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import petDetailReaderArtifact from '../fixtures/store/src/__isograph/Query/PetDetail/resolver_reader';
import petListsNormalizationAst from '../fixtures/store/src/__isograph/Query/PetLists/normalization_ast';
import petSummaryNormalizationAst from '../fixtures/store/src/__isograph/Query/PetSummary/normalization_ast';
import petSummaryReaderArtifact from '../fixtures/store/src/__isograph/Query/PetSummary/resolver_reader';
import throwingPetNormalizationAst from '../fixtures/store/src/__isograph/Query/ThrowingPet/normalization_ast';
import throwingPetReaderArtifact from '../fixtures/store/src/__isograph/Query/ThrowingPet/resolver_reader';
import {
  callSubscriptions,
  createIsographEnvironment,
  type IsographEnvironment,
  type JsonObject,
  type JsonValue,
  listenToSubscription,
  type NormalizationAst,
  normalize,
  readAndSubscribe,
  type SubscriptionResult,
  subscriptionSnapshot,
  type Variables,
} from '../src/index';
import {
  deferredNetwork,
  feedResponse,
  petDetailResponse,
  petListsPet9Response,
  queryRoot,
} from './fixtures';

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const petDetailVariables = { id: '1', skip: null };

function environmentOverDeferredNetwork(): IsographEnvironment {
  return createIsographEnvironment(
    new Map(),
    deferredNetwork().networkFunction,
  );
}

/** `normalize` on the environment's store, then `callSubscriptions`. */
function write(
  environment: IsographEnvironment,
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): void {
  normalize(environment.store, ast, response, variables);
  callSubscriptions(environment);
}

function writePetDetail(
  environment: IsographEnvironment,
  overrides: Parameters<typeof petDetailResponse>[0] = {},
): void {
  write(
    environment,
    petDetailNormalizationAst,
    petDetailResponse(overrides),
    petDetailVariables,
  );
}

function writePet9(environment: IsographEnvironment): void {
  write(environment, petListsNormalizationAst, petListsPet9Response(), {});
}

function callback() {
  return vi.fn<(result: SubscriptionResult) => void>();
}

function subscribeToPetDetail(
  environment: IsographEnvironment,
  onResult: (result: SubscriptionResult) => void,
) {
  return readAndSubscribe(
    environment,
    petDetailReaderArtifact(),
    queryRoot,
    petDetailVariables,
    onResult,
  );
}

/** A `queueMicrotask` stub; `reported()` runs each queued function and returns what each threw. */
function stubQueueMicrotask() {
  const queueMicrotask = vi.fn<(task: () => void) => void>();
  vi.stubGlobal('queueMicrotask', queueMicrotask);
  return {
    queueMicrotask,
    reported: (): ReadonlyArray<unknown> =>
      queueMicrotask.mock.calls.map(([task]) => {
        try {
          task();
        } catch (error: unknown) {
          return error;
        }
        throw new Error('A queued task did not throw.');
      }),
  };
}

describe('readAndSubscribe', () => {
  test('on an empty store reads MissingData, and the response delivers Ok', () => {
    const environment = environmentOverDeferredNetwork();
    const onResult = callback();
    const [subscription] = subscribeToPetDetail(environment, onResult);
    expect(subscriptionSnapshot(subscription).kind).toBe('MissingData');
    expect(environment.subscriptions.size).toBe(1);
    expect(onResult).not.toHaveBeenCalled();

    writePetDetail(environment);
    expect(onResult).toHaveBeenCalledTimes(1);
    expect(onResult.mock.calls[0]?.[0]).toMatchObject({
      kind: 'Ok',
      value: { pet: { name: 'Rex' } },
    });
    expect(subscriptionSnapshot(subscription)).toBe(
      onResult.mock.calls[0]?.[0],
    );
  });

  test('the snapshot is the same object until a write changes the read', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const onResult = callback();
    const [subscription] = subscribeToPetDetail(environment, onResult);
    const first = subscriptionSnapshot(subscription);

    writePet9(environment);
    expect(subscriptionSnapshot(subscription)).toBe(first);

    writePetDetail(environment, { name: 'Rexford' });
    expect(subscriptionSnapshot(subscription)).not.toBe(first);
    expect(subscriptionSnapshot(subscription)).toBe(
      onResult.mock.calls[0]?.[0],
    );
  });

  test('a related write calls the callback; an unrelated or equal write does not', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const onResult = callback();
    const [subscription] = subscribeToPetDetail(environment, onResult);

    writePetDetail(environment, { name: 'Rexford' });
    expect(onResult).toHaveBeenCalledTimes(1);
    expect(onResult.mock.calls[0]?.[0]).toMatchObject({
      kind: 'Ok',
      value: { pet: { name: 'Rexford' } },
    });
    expect(subscription.delivered).toBe(onResult.mock.calls[0]?.[0]);

    writePet9(environment);
    expect(onResult).toHaveBeenCalledTimes(1);

    writePetDetail(environment, { name: 'Rexford' });
    expect(onResult).toHaveBeenCalledTimes(1);
  });

  test('nested resolvers that return new objects make every re-read unequal', () => {
    const environment = environmentOverDeferredNetwork();
    write(
      environment,
      petSummaryNormalizationAst,
      {
        pet____id___v_id: {
          __typename: 'Pet',
          id: '1',
          checkins____limit___l_2: [],
          name: 'Rex',
          nickname: null,
        },
      },
      { id: '1' },
    );
    const onResult = callback();
    const [subscription] = readAndSubscribe(
      environment,
      petSummaryReaderArtifact(),
      queryRoot,
      { id: '1' },
      onResult,
    );
    const first = subscriptionSnapshot(subscription);

    writePet9(environment);
    expect(onResult).toHaveBeenCalledTimes(1);
    expect(onResult.mock.calls[0]?.[0]).toEqual(first);
  });
});

describe('equality by node', () => {
  function leavesResponse(tags: JsonValue, settings: JsonValue): JsonObject {
    return {
      greeting: 'hi',
      greeting____loud___l_true____name___s_Ada_Lovelace____times___l_3:
        'HI ADA',
      greeting____name___v_name: 'hi Bo',
      greeting____name___l_null: null,
      settings,
      tags,
    };
  }

  function subscribeToLeaves(
    environment: IsographEnvironment,
    onResult: (result: SubscriptionResult) => void,
  ) {
    return readAndSubscribe(
      environment,
      leavesReaderArtifact(),
      queryRoot,
      { name: 'Bo' },
      onResult,
    );
  }

  test('an equal list of scalars, written again as a new list, is equal', () => {
    const environment = environmentOverDeferredNetwork();
    const variables = { name: 'Bo' };
    write(
      environment,
      leavesNormalizationAst,
      leavesResponse(['a', null], 's'),
      variables,
    );
    const onResult = callback();
    subscribeToLeaves(environment, onResult);

    write(
      environment,
      leavesNormalizationAst,
      leavesResponse(['a', null], 's'),
      variables,
    );
    expect(onResult).not.toHaveBeenCalled();
  });

  test('an equal JSON object, written again as a new object, is unequal', () => {
    const environment = environmentOverDeferredNetwork();
    const variables = { name: 'Bo' };
    write(
      environment,
      leavesNormalizationAst,
      leavesResponse([], { theme: 'dark' }),
      variables,
    );
    const onResult = callback();
    subscribeToLeaves(environment, onResult);

    write(
      environment,
      leavesNormalizationAst,
      leavesResponse([], { theme: 'dark' }),
      variables,
    );
    expect(onResult).toHaveBeenCalledTimes(1);
  });

  test('a refinement that stays null is equal', () => {
    const environment = environmentOverDeferredNetwork();
    const variables = { limit: 2 };
    write(environment, feedNormalizationAst, feedResponse(), variables);
    const onResult = callback();
    const [subscription] = readAndSubscribe(
      environment,
      feedReaderArtifact(),
      queryRoot,
      variables,
      onResult,
    );
    expect(subscriptionSnapshot(subscription)).toMatchObject({
      kind: 'Ok',
      value: { feed: [{ asAdItem: null }, { asBlogItem: null }] },
    });

    write(environment, feedNormalizationAst, feedResponse(), variables);
    expect(onResult).not.toHaveBeenCalled();
  });
});

test('data, then MissingData, then data again', () => {
  const environment = environmentOverDeferredNetwork();
  writePetDetail(environment);
  const onResult = callback();
  subscribeToPetDetail(environment, onResult);

  environment.store.get('Pet')?.get('1')?.delete('nickname');
  callSubscriptions(environment);
  expect(onResult).toHaveBeenCalledTimes(1);
  expect(onResult.mock.calls[0]?.[0]).toMatchObject({
    kind: 'MissingData',
    recordLink: { entityName: 'Pet', id: '1' },
  });

  callSubscriptions(environment);
  expect(onResult).toHaveBeenCalledTimes(1);

  writePetDetail(environment);
  expect(onResult).toHaveBeenCalledTimes(2);
  expect(onResult.mock.calls[1]?.[0].kind).toBe('Ok');
});

test('after the cleanup, a changing write calls nothing', () => {
  const environment = environmentOverDeferredNetwork();
  writePetDetail(environment);
  const onResult = callback();
  const [, unsubscribe] = subscribeToPetDetail(environment, onResult);

  unsubscribe();
  expect(environment.subscriptions.size).toBe(0);
  writePetDetail(environment, { name: 'Rexford' });
  expect(onResult).not.toHaveBeenCalled();
});

describe('listenToSubscription', () => {
  test('a listener is called after the callback on each change, until its cleanup', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const order: string[] = [];
    const onResult = vi.fn((_result: SubscriptionResult) => {
      order.push('callback');
    });
    const listener = vi.fn(() => {
      order.push('listener');
    });
    const [subscription] = subscribeToPetDetail(environment, onResult);
    const stopListening = listenToSubscription(subscription, listener);
    const first = subscriptionSnapshot(subscription);

    writePet9(environment);
    expect(onResult).not.toHaveBeenCalled();
    expect(listener).not.toHaveBeenCalled();
    expect(subscriptionSnapshot(subscription)).toBe(first);

    writePetDetail(environment, { name: 'Rexford' });
    expect(order).toEqual(['callback', 'listener']);
    expect(listener).toHaveBeenCalledWith();
    expect(subscriptionSnapshot(subscription)).toBe(
      onResult.mock.calls[0]?.[0],
    );

    stopListening();
    writePetDetail(environment, { name: 'Rexanne' });
    expect(onResult).toHaveBeenCalledTimes(2);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(() => stopListening()).not.toThrow();
  });

  test('listeners are called in the order they were added, and a throwing listener does not stop the next', () => {
    const { reported } = stubQueueMicrotask();
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const [subscription] = subscribeToPetDetail(environment, () => {});
    const order: string[] = [];
    const error = new Error('listener');
    listenToSubscription(subscription, () => {
      order.push('first');
      throw error;
    });
    listenToSubscription(subscription, () => {
      order.push('second');
    });

    writePetDetail(environment, { name: 'Rexford' });
    expect(order).toEqual(['first', 'second']);
    expect(reported()).toEqual([error]);
  });
});

describe('the two-phase lifetime', () => {
  test('a write before the commit calls the callback; releasing the permanent retain unsubscribes', () => {
    vi.useFakeTimers();
    const environment = environmentOverDeferredNetwork();
    const onResult = callback();
    const [cacheItem, releaseTemporaryRetain] =
      createTemporarilyRetainedCacheItem(
        () => subscribeToPetDetail(environment, onResult),
        () => {},
      );
    expect(subscriptionSnapshot(cacheItem.getValue()).kind).toBe('MissingData');

    writePetDetail(environment);
    expect(onResult).toHaveBeenCalledTimes(1);

    const committed = cacheItem.permanentRetainIfNotDisposed(
      releaseTemporaryRetain,
    );
    expect(committed?.[0]).toBe(cacheItem.getValue());
    committed?.[1]();
    expect(environment.subscriptions.size).toBe(0);
  });

  test('without a commit, the temporary retain expires and unsubscribes', () => {
    vi.useFakeTimers();
    const environment = environmentOverDeferredNetwork();
    const onResult = callback();
    createTemporarilyRetainedCacheItem(
      () => subscribeToPetDetail(environment, onResult),
      () => {},
    );
    expect(environment.subscriptions.size).toBe(1);

    vi.advanceTimersToNextTimer();
    expect(environment.subscriptions.size).toBe(0);
    writePetDetail(environment);
    expect(onResult).not.toHaveBeenCalled();
  });
});

describe('read errors', () => {
  function throwingPetResponse(name: string): JsonObject {
    return { pet____id___v_id: { __typename: 'Pet', id: '1', name } };
  }

  function subscribeToThrowingPet(
    environment: IsographEnvironment,
    onResult: (result: SubscriptionResult) => void,
  ) {
    return readAndSubscribe(
      environment,
      throwingPetReaderArtifact(),
      queryRoot,
      { id: '1' },
      onResult,
    );
  }

  test('a nested resolver that throws at subscribe gives an Err snapshot', () => {
    const environment = environmentOverDeferredNetwork();
    normalize(
      environment.store,
      throwingPetNormalizationAst,
      throwingPetResponse('Throw'),
      { id: '1' },
    );
    const [subscription] = subscribeToThrowingPet(environment, callback());
    expect(subscriptionSnapshot(subscription)).toMatchObject({
      kind: 'Err',
      error: new Error('ThrowingName read the name Throw'),
    });
  });

  test('a nested resolver that throws at a re-read delivers Err, and other subscriptions are still called', () => {
    const environment = environmentOverDeferredNetwork();
    normalize(
      environment.store,
      throwingPetNormalizationAst,
      throwingPetResponse('Rex'),
      { id: '1' },
    );
    writePetDetail(environment);
    const onThrowingResult = callback();
    const onPetDetailResult = callback();
    subscribeToThrowingPet(environment, onThrowingResult);
    subscribeToPetDetail(environment, onPetDetailResult);

    writePetDetail(environment, { name: 'Throw' });
    expect(onThrowingResult).toHaveBeenCalledTimes(1);
    expect(onThrowingResult.mock.calls[0]?.[0]).toMatchObject({
      kind: 'Err',
      error: new Error('ThrowingName read the name Throw'),
    });
    expect(onPetDetailResult).toHaveBeenCalledTimes(1);

    callSubscriptions(environment);
    expect(onThrowingResult).toHaveBeenCalledTimes(2);
    expect(onPetDetailResult).toHaveBeenCalledTimes(1);
  });

  test('a stored non-link under a composite store key delivers Err with the store message', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const onResult = callback();
    subscribeToPetDetail(environment, onResult);

    environment.store.get('Pet')?.get('1')?.set('stats', 'x');
    callSubscriptions(environment);
    expect(onResult).toHaveBeenCalledTimes(1);
    expect(onResult.mock.calls[0]?.[0]).toMatchObject({
      kind: 'Err',
      error: new Error(
        'Store record Pet:1 holds a non-link under the composite store key "stats"; the store was written with an AST that disagrees with this one.',
      ),
    });
  });
});

test('a callback that throws does not stop the other subscriptions, and its error is reported', () => {
  const { reported } = stubQueueMicrotask();
  const environment = environmentOverDeferredNetwork();
  writePetDetail(environment);
  const error = new Error('callback');
  const onFirstResult = vi.fn((_result: SubscriptionResult) => {
    throw error;
  });
  const onSecondResult = callback();
  subscribeToPetDetail(environment, onFirstResult);
  subscribeToPetDetail(environment, onSecondResult);

  expect(() => writePetDetail(environment, { name: 'Rexford' })).not.toThrow();
  expect(onFirstResult).toHaveBeenCalledTimes(1);
  expect(onSecondResult).toHaveBeenCalledTimes(1);
  expect(reported()).toEqual([error]);
});

describe('ordering', () => {
  test('subscriptions are called in subscription order', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const order: number[] = [];
    [1, 2, 3].forEach((index) =>
      subscribeToPetDetail(environment, () => {
        order.push(index);
      }),
    );

    writePetDetail(environment, { name: 'Rexford' });
    expect(order).toEqual([1, 2, 3]);
  });

  test('a callback that unsubscribes a later subscription removes it from the pass', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const onThirdResult = callback();
    subscribeToPetDetail(environment, () => {
      unsubscribeThird();
    });
    const onSecondResult = callback();
    subscribeToPetDetail(environment, onSecondResult);
    const [, unsubscribeThird] = subscribeToPetDetail(
      environment,
      onThirdResult,
    );

    writePetDetail(environment, { name: 'Rexford' });
    expect(onSecondResult).toHaveBeenCalledTimes(1);
    expect(onThirdResult).not.toHaveBeenCalled();
  });

  test('a callback that unsubscribes itself does not stop the others', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const onFirstResult = callback();
    subscribeToPetDetail(environment, onFirstResult);
    const [, unsubscribeSecond] = subscribeToPetDetail(environment, () => {
      unsubscribeSecond();
    });
    const onThirdResult = callback();
    subscribeToPetDetail(environment, onThirdResult);

    writePetDetail(environment, { name: 'Rexford' });
    expect(onFirstResult).toHaveBeenCalledTimes(1);
    expect(onThirdResult).toHaveBeenCalledTimes(1);
    expect(environment.subscriptions.size).toBe(2);
  });
});

describe('degenerate', () => {
  test('callSubscriptions with no subscriptions does nothing', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const before = structuredClone(environment.store);
    callSubscriptions(environment);
    expect(environment.store).toEqual(before);
    expect(environment.subscriptions.size).toBe(0);
  });

  test('an empty reader AST reads Ok with no data and is never called after a write', () => {
    const environment = environmentOverDeferredNetwork();
    writePetDetail(environment);
    const onResult = callback();
    const [subscription] = readAndSubscribe(
      environment,
      constantReaderArtifact(),
      { entityName: 'Pet', id: '1' },
      {},
      onResult,
    );
    expect(subscriptionSnapshot(subscription)).toEqual({
      kind: 'Ok',
      value: {},
    });

    writePetDetail(environment, { name: 'Rexford' });
    writePet9(environment);
    expect(onResult).not.toHaveBeenCalled();
  });
});
