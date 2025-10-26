import { beforeEach, describe, expect, test, vi } from 'vitest';
import { callSubscriptions } from '../core/cache';
import {
  createIsographEnvironment,
  createIsographStore,
  type StoreLayerData as DataLayer,
  type IsographEnvironment,
} from '../core/IsographEnvironment';
import {
  addNetworkResponseStoreLayer as addNetworkResponseStoreLayerInner,
  addOptimisticStoreLayer as addOptimisticStoreLayerInner,
  addStartUpdateStoreLayer as addStartUpdateStoreLayerInner,
  readOptimisticRecord,
  type WithEncounteredIds,
} from '../core/optimisticProxy';

vi.mock(import('../core/cache'), { spy: true });

const CHANGES = new Map([['Query', new Set(['__ROOT'])]]);
const NO_CHANGES = new Map();

describe('optimisticLayer', () => {
  let environment: ReturnType<typeof createIsographEnvironment>;

  beforeEach(() => {
    vi.clearAllMocks();
    const networkFunction = vi
      .fn()
      .mockRejectedValue(new Error('Fetch failed'));
    environment = createIsographEnvironment(
      createIsographStore(),
      networkFunction,
    );
    addNetworkResponseStoreLayer(environment, 0);
  });

  describe('addNetworkResponseStoreLayer', () => {
    test('has parent BaseStoreLayer', () => {
      expect(environment.store).toMatchObject({
        kind: 'BaseStoreLayer',
      });
    });

    test('calls subscriptions', () => {
      expect(callSubscriptions).toHaveBeenCalledTimes(1);
      expect(callSubscriptions).toHaveBeenCalledWith(
        expect.anything(),
        CHANGES,
      );
    });

    test("doesn't call subscriptions if no changes", () => {
      addNetworkResponseStoreLayer(environment, 0);

      expect(callSubscriptions).toHaveBeenCalledTimes(2);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        2,
        expect.anything(),
        NO_CHANGES,
      );
    });

    test('merge', () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      addNetworkResponseStoreLayer(environment, 3);
      addNetworkResponseStoreLayer(environment, 4);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: {
          kind: 'OptimisticStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        },
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(4);
    });
  });

  describe('addStartUpdateStoreLayer', () => {
    test('calls subscriptions', () => {
      addStartUpdateStoreLayer(environment, () => 1);

      expect(callSubscriptions).toHaveBeenCalledTimes(2);
      expect(callSubscriptions).nthCalledWith(2, expect.anything(), CHANGES);
    });

    test('merge', () => {
      addOptimisticStoreLayer(environment, () => 1);
      addStartUpdateStoreLayer(environment, (counter) => counter + 1);
      addStartUpdateStoreLayer(environment, (counter) => counter + 1);

      expect(environment.store).toMatchObject({
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: {
          kind: 'OptimisticStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        },
      });
      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(3);
    });
  });

  describe('addOptimisticStoreLayer', () => {
    test('calls subscriptions', () => {
      addOptimisticStoreLayer(environment, () => 1);

      expect(callSubscriptions).toHaveBeenCalledTimes(2);
      expect(callSubscriptions).nthCalledWith(2, expect.anything(), CHANGES);
    });

    test('add tree nodes', () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      addOptimisticStoreLayer(environment, (counter) => counter + 1);

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(3);
    });
  });

  describe('replaceOptimisticStoreLayerWithNetworkResponseStoreLayer', () => {
    test('calls subscriptions if changes', () => {
      const revert = addOptimisticStoreLayer(environment, () => 1);

      revert(2);

      expect(callSubscriptions).toHaveBeenCalledTimes(3);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        3,
        expect.anything(),
        CHANGES,
      );
    });

    test("doesn't call subscriptions if no changes", () => {
      const revert = addOptimisticStoreLayer(environment, () => 1);

      revert(1);

      expect(callSubscriptions).toHaveBeenCalledTimes(3);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        3,
        expect.anything(),
        NO_CHANGES,
      );
    });

    test('calls subscriptions if nodes update different fields', () => {
      const revert = addOptimisticStoreLayerInner(environment, () => ({
        data: { Query: { __ROOT: { surname: 'foo' } } },
        encounteredIds: CHANGES,
      }));
      addNetworkResponseStoreLayerInner(
        environment,
        { Query: { __ROOT: { name: 'foo' } } },
        CHANGES,
      );

      revert({ Query: { __ROOT: { surname: 'bar' } } });

      expect(callSubscriptions).toHaveBeenCalledTimes(4);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        4,
        expect.anything(),
        CHANGES,
      );
    });

    test('has parent BaseStoreLayer and child node', () => {
      const revert = addOptimisticStoreLayer(
        environment,
        (counter) => counter + 1,
      );
      addOptimisticStoreLayer(environment, (counter) => counter + 1);

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'OptimisticStoreLayer',
        parentStoreLayer: {
          kind: 'BaseStoreLayer',
        },
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(6);
    });

    test('has parent node and no child node', () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      const revert = addOptimisticStoreLayer(
        environment,
        (counter) => counter + 1,
      );

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: {
          kind: 'OptimisticStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        },
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(5);
    });

    test('merges if has parent BaseStoreLayer', () => {
      const revert = addOptimisticStoreLayer(
        environment,
        (counter) => counter + 1,
      );

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'BaseStoreLayer',
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(5);
    });

    test("doesn't merge child nodes if has parent nodes", () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      const revert = addOptimisticStoreLayer(
        environment,
        (counter) => counter + 1,
      );
      addStartUpdateStoreLayer(environment, (counter) => counter + 1);

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: {
          kind: 'NetworkResponseStoreLayer',
          parentStoreLayer: {
            kind: 'OptimisticStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          },
        },
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(6);
    });

    test('merges child NetworkResponseStoreLayer', () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      const revert = addOptimisticStoreLayer(
        environment,
        (counter) => counter + 1,
      );
      addNetworkResponseStoreLayer(environment, 12);
      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: {
          kind: 'OptimisticStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        },
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(12);
    });

    test('merges child nodes if has parent BaseStoreLayer', () => {
      const revert = addOptimisticStoreLayer(
        environment,
        (counter) => counter + 1,
      );
      addStartUpdateStoreLayer(environment, (counter) => counter + 1);
      addNetworkResponseStoreLayer(environment, 12);
      addOptimisticStoreLayer(environment, (counter) => counter + 1);

      revert(4);

      expect(environment.store).toMatchObject({
        kind: 'OptimisticStoreLayer',
        parentStoreLayer: {
          kind: 'BaseStoreLayer',
        },
      });
      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(13);
    });
  });

  // utils
  function addNetworkResponseStoreLayer(
    environment: IsographEnvironment,
    counter: number,
  ) {
    const { data, encounteredIds } = update(() => counter);
    return addNetworkResponseStoreLayerInner(environment, data, encounteredIds);
  }

  function addOptimisticStoreLayer(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    const revert = addOptimisticStoreLayerInner(environment, () => {
      return update(updater);
    });
    return (counter: number) => {
      revert(update(() => counter).data);
    };
  }

  function addStartUpdateStoreLayer(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    addStartUpdateStoreLayerInner(environment, () => {
      return update(updater);
    });
  }

  function update(
    value: (counter: number) => number,
  ): WithEncounteredIds<DataLayer> {
    const { counter } = readOptimisticRecord(environment, {
      __link: '__ROOT',
      __typename: 'Query',
    });
    const nextCounter = value(Number(counter));
    return {
      encounteredIds: counter != nextCounter ? CHANGES : NO_CHANGES,
      data: {
        Query: {
          __ROOT: {
            counter: nextCounter,
          },
        },
      },
    };
  }
});
