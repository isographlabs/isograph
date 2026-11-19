import { createTemporarilyRetainedCacheItem } from '@isograph/disposable-types';
import { afterEach, describe, expect, test, vi } from 'vitest';
import petDetail from '../fixtures/store/src/__isograph/Query/PetDetail/entrypoint';
import {
  check,
  createIsographEnvironment,
  fetchEntrypoint,
  garbageCollectEnvironment,
  getOrCreateCacheForArtifact,
  getPromiseState,
  type IsographEnvironment,
  normalize,
  readAndSubscribe,
  type SubscriptionResult,
  subscriptionSnapshot,
} from '../src/index';
import { deferredNetwork, petDetailResponse, queryRoot } from './fixtures';

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

const variables = { id: '1', skip: null };

/** A `queueMicrotask` stub; `reported()` runs each queued function and returns what each threw. */
function stubQueueMicrotask() {
  const queueMicrotask = vi.fn<(task: () => void) => void>();
  vi.stubGlobal('queueMicrotask', queueMicrotask);
  return {
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

function environmentWith(network: ReturnType<typeof deferredNetwork>) {
  return createIsographEnvironment(new Map(), network.networkFunction);
}

function writePetDetail(environment: IsographEnvironment): void {
  normalize(
    environment.store,
    petDetail.normalizationAst,
    petDetailResponse(),
    variables,
  );
}

test('render, commit, write, release, collect', async () => {
  vi.useFakeTimers();
  const network = deferredNetwork();
  const environment = environmentWith(network);

  // Render: get or create the request, temporarily retained; subscribe, temporarily retained.
  const cache = getOrCreateCacheForArtifact(
    environment,
    petDetail,
    variables,
    null,
  );
  const [requestItem, networkRequest, releaseRequestRender] =
    cache.getOrPopulateAndTemporaryRetain();
  expect(network.calls).toHaveLength(1);
  const onResult = vi.fn<(result: SubscriptionResult) => void>();
  const [subscriptionItem, releaseSubscriptionRender] =
    createTemporarilyRetainedCacheItem(
      () =>
        readAndSubscribe(
          environment,
          petDetail.readerArtifact(),
          queryRoot,
          variables,
          onResult,
        ),
      () => {},
    );
  const subscription = subscriptionItem.getValue();
  expect(subscriptionSnapshot(subscription).kind).toBe('MissingData');

  // The response arrives between render and commit; the subscription delivers it.
  network.calls[0]?.resolve(petDetailResponse({ name: 'Rex' }));
  await networkRequest.promise;
  expect(onResult).toHaveBeenCalledTimes(1);
  expect(onResult.mock.calls[0]?.[0]).toMatchObject({
    kind: 'Ok',
    value: { pet: { name: 'Rex' } },
  });
  expect(subscriptionSnapshot(subscription)).toBe(onResult.mock.calls[0]?.[0]);

  // Commit: permanent retains. The item was created by this test's render and its timer has
  // not fired, so neither retain is null.
  const committedRequest =
    requestItem.permanentRetainIfNotDisposed(releaseRequestRender);
  const committedSubscription = subscriptionItem.permanentRetainIfNotDisposed(
    releaseSubscriptionRender,
  );
  expect(committedRequest).not.toBeNull();
  expect(committedSubscription).not.toBeNull();

  // Unmount: release; the store keeps the records until the caller collects.
  committedSubscription?.[1]();
  committedRequest?.[1]();
  expect(environment.subscriptions.size).toBe(0);
  expect(environment.retainedQueries.size).toBe(0);
  expect(getPromiseState(networkRequest).kind).toBe('Ok');
  garbageCollectEnvironment(environment);
  expect(environment.store).toEqual(new Map());
});

describe('fetchEntrypoint', () => {
  test('IfNecessary on an empty store sends the request and normalizes the response', async () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      { shouldFetch: 'IfNecessary' },
    );
    expect(network.calls).toHaveLength(1);
    expect(network.calls[0]?.operation).toEqual({ text: petDetail.queryText });
    expect(network.calls[0]?.variables).toBe(variables);
    expect(environment.retainedQueries.size).toBe(1);
    expect(getPromiseState(networkRequest).kind).toBe('Pending');

    network.calls[0]?.resolve(petDetailResponse());
    await networkRequest.promise;
    expect(getPromiseState(networkRequest)).toEqual({
      kind: 'Ok',
      value: undefined,
    });
    expect(
      check(environment.store, petDetail.normalizationAst, variables),
    ).toEqual({ kind: 'EnoughData' });
  });

  test('IfNecessary with the data present retains without sending', () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    writePetDetail(environment);
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      { shouldFetch: 'IfNecessary' },
    );
    expect(network.calls).toHaveLength(0);
    expect(getPromiseState(networkRequest).kind).toBe('Ok');
    expect(environment.retainedQueries.size).toBe(1);
  });

  test('Yes sends with the data present', () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    writePetDetail(environment);
    fetchEntrypoint(environment, petDetail, variables, { shouldFetch: 'Yes' });
    expect(network.calls).toHaveLength(1);
  });

  test('No retains without sending on an empty store', () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      {
        shouldFetch: 'No',
      },
    );
    expect(network.calls).toHaveLength(0);
    expect(getPromiseState(networkRequest).kind).toBe('Ok');
    expect(environment.retainedQueries.size).toBe(1);
    expect(environment.store).toEqual(new Map());
  });

  test('null options behave as IfNecessary', () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    fetchEntrypoint(environment, petDetail, variables, null);
    expect(network.calls).toHaveLength(1);
    writePetDetail(environment);
    fetchEntrypoint(environment, petDetail, variables, null);
    expect(network.calls).toHaveLength(1);
  });

  test('a cleanup before the response drops the response', async () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const onComplete = vi.fn();
    const [networkRequest, cleanup] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      { onComplete },
    );
    cleanup();
    expect(environment.retainedQueries.size).toBe(0);

    network.calls[0]?.resolve(petDetailResponse());
    await networkRequest.promise;
    expect(environment.store).toEqual(new Map());
    expect(getPromiseState(networkRequest).kind).toBe('Ok');
    expect(onComplete).not.toHaveBeenCalled();
  });

  test('a cleanup after the response unretains and does not collect', async () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const [networkRequest, cleanup] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      null,
    );
    network.calls[0]?.resolve(petDetailResponse());
    await networkRequest.promise;

    cleanup();
    expect(environment.retainedQueries.size).toBe(0);
    expect(environment.store.get('Pet')?.has('1')).toBe(true);
    garbageCollectEnvironment(environment);
    expect(environment.store).toEqual(new Map());

    retainSomethingElse(environment);
    cleanup();
    expect(environment.retainedQueries.size).toBe(1);
  });

  test('onComplete receives the resolver’s value', async () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const onComplete = vi.fn();
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      {
        onComplete,
      },
    );
    network.calls[0]?.resolve(petDetailResponse({ name: 'Rex' }));
    await networkRequest.promise;
    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(onComplete.mock.calls[0]?.[0]).toMatchObject({
      pet: { id: '1', name: 'Rex', bestFriend: { id: '2', name: 'Fido' } },
    });
  });

  test('an onComplete that throws leaves the wrapper Ok and is reported', async () => {
    const { reported } = stubQueueMicrotask();
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const error = new Error('onComplete');
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      {
        onComplete: () => {
          throw error;
        },
      },
    );
    network.calls[0]?.resolve(petDetailResponse());
    await networkRequest.promise;
    expect(getPromiseState(networkRequest).kind).toBe('Ok');
    expect(reported()).toEqual([error]);
  });

  test('a rejection makes the wrapper Err and calls onError', async () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const onError = vi.fn();
    const error = new Error('network');
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      {
        onError,
      },
    );
    network.calls[0]?.reject(error);
    await expect(networkRequest.promise).rejects.toBe(error);
    expect(getPromiseState(networkRequest)).toEqual({ kind: 'Err', error });
    expect(onError).toHaveBeenCalledTimes(1);
    expect(environment.store).toEqual(new Map());
  });

  test('a malformed response makes the wrapper Err, calls onError, and calls no subscription', async () => {
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const onError = vi.fn();
    const onResult = vi.fn();
    readAndSubscribe(
      environment,
      petDetail.readerArtifact(),
      queryRoot,
      variables,
      onResult,
    );
    const [networkRequest] = fetchEntrypoint(
      environment,
      petDetail,
      variables,
      {
        onError,
      },
    );
    network.calls[0]?.resolve({});
    const message =
      'Malformed response at []: expected the response key "pet____id___v_id".';
    await expect(networkRequest.promise).rejects.toThrow(message);
    expect(getPromiseState(networkRequest)).toMatchObject({
      kind: 'Err',
      error: new Error(message),
    });
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onResult).not.toHaveBeenCalled();
  });

  test('a network function that throws synchronously throws out of fetchEntrypoint and retains nothing', () => {
    const error = new Error('synchronous');
    const environment = createIsographEnvironment(new Map(), () => {
      throw error;
    });
    expect(() =>
      fetchEntrypoint(environment, petDetail, variables, null),
    ).toThrow(error);
    expect(environment.retainedQueries.size).toBe(0);
  });
});

/** Retains a query no test releases, so that a size of 1 shows a cleanup removed nothing. */
function retainSomethingElse(environment: IsographEnvironment): void {
  fetchEntrypoint(environment, petDetail, variables, { shouldFetch: 'No' });
}

describe('the request cache', () => {
  test('equal variables share a cache; other variables do not', () => {
    const environment = environmentWith(deferredNetwork());
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      { id: '1', skip: null },
      null,
    );
    expect(
      getOrCreateCacheForArtifact(
        environment,
        petDetail,
        { skip: null, id: '1' },
        null,
      ),
    ).toBe(cache);
    expect(
      getOrCreateCacheForArtifact(
        environment,
        petDetail,
        { id: '2', skip: null },
        null,
      ),
    ).not.toBe(cache);
  });

  test('two renders before a commit share one request', () => {
    vi.useFakeTimers();
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      variables,
      null,
    );
    const [, first] = cache.getOrPopulateAndTemporaryRetain();
    const [, second] = cache.getOrPopulateAndTemporaryRetain();
    expect(network.calls).toHaveLength(1);
    expect(environment.retainedQueries.size).toBe(1);
    expect(second).toBe(first);
  });

  test('a commit returns the render’s request, and the item leaves the cache', () => {
    vi.useFakeTimers();
    const environment = environmentWith(deferredNetwork());
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      variables,
      null,
    );
    const [item, networkRequest, releaseRender] =
      cache.getOrPopulateAndTemporaryRetain();
    const committed = item.permanentRetainIfNotDisposed(releaseRender);
    expect(committed?.[0]).toBe(networkRequest);
    expect(cache.isEmpty()).toBe(true);
  });

  test('a render with no commit is released when its temporary retain expires', async () => {
    vi.useFakeTimers();
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      variables,
      null,
    );
    const [, networkRequest] = cache.getOrPopulateAndTemporaryRetain();

    vi.advanceTimersToNextTimer();
    expect(environment.retainedQueries.size).toBe(0);
    network.calls[0]?.resolve(petDetailResponse());
    await networkRequest.promise;
    expect(environment.store).toEqual(new Map());
  });

  test('a commit after the temporary retain expired creates the request again', () => {
    vi.useFakeTimers();
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      variables,
      null,
    );
    const [item, , releaseRender] = cache.getOrPopulateAndTemporaryRetain();

    vi.advanceTimersToNextTimer();
    expect(item.permanentRetainIfNotDisposed(releaseRender)).toBeNull();
    expect(cache.getAndPermanentRetainIfPresent()).toBeNull();
    cache.factory();
    expect(network.calls).toHaveLength(2);
  });

  test('a render after every holder committed checks again', async () => {
    vi.useFakeTimers();
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      variables,
      null,
    );
    const [firstItem, firstRequest, releaseFirstRender] =
      cache.getOrPopulateAndTemporaryRetain();
    firstItem.permanentRetainIfNotDisposed(releaseFirstRender);

    const [secondItem, , releaseSecondRender] =
      cache.getOrPopulateAndTemporaryRetain();
    expect(network.calls).toHaveLength(2);
    secondItem.permanentRetainIfNotDisposed(releaseSecondRender);

    network.calls[0]?.resolve(petDetailResponse());
    await firstRequest.promise;
    cache.getOrPopulateAndTemporaryRetain();
    expect(network.calls).toHaveLength(2);
  });

  test('releasing the committed pair unretains and keeps the records', async () => {
    vi.useFakeTimers();
    const network = deferredNetwork();
    const environment = environmentWith(network);
    const cache = getOrCreateCacheForArtifact(
      environment,
      petDetail,
      variables,
      null,
    );
    const [item, networkRequest, releaseRender] =
      cache.getOrPopulateAndTemporaryRetain();
    network.calls[0]?.resolve(petDetailResponse());
    await networkRequest.promise;
    const committed = item.permanentRetainIfNotDisposed(releaseRender);

    committed?.[1]();
    expect(environment.retainedQueries.size).toBe(0);
    expect(environment.store.get('Pet')?.has('1')).toBe(true);
  });
});
