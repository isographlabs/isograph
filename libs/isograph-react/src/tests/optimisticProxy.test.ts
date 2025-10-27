import { beforeEach, describe, expect, test, vi } from 'vitest';
import { callSubscriptions, type EncounteredIds } from '../core/cache';
import {
  createIsographEnvironment,
  createIsographStore,
  type IsographEnvironment,
  type StoreLayerData,
} from '../core/IsographEnvironment';
import {
  addNetworkResponseStoreLayer as addNetworkResponseStoreLayerInner,
  addOptimisticStoreLayer as addOptimisticStoreLayerInner,
  addStartUpdateStoreLayer as addStartUpdateStoreLayerInner,
  readOptimisticRecord,
  type StoreLayer,
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
        readOptimisticRecord(environment.store, {
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
        readOptimisticRecord(environment.store, {
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
        readOptimisticRecord(environment.store, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(3);
    });
  });

  describe('replaceOptimisticStoreLayerWithNetworkResponseStoreLayer', () => {
    describe('subscriptions', () => {
      test('calls if network response differs', () => {
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
        const revert = addOptimisticStoreLayerInner(
          environment,
          (storeLayer) => {
            ignoreReadonly(storeLayer).data = {
              Query: {
                __ROOT: {
                  surname: 'foo',
                },
              },
            };
            return CHANGES;
          },
        );
        addNetworkResponseStoreLayerInner(environment, (storeLayer) => {
          ignoreReadonly(storeLayer).data = {
            Query: {
              __ROOT: {
                name: 'foo',
              },
            },
          };
          return CHANGES;
        });

        revert((storeLayer) => {
          ignoreReadonly(storeLayer).data = {
            Query: { __ROOT: { surname: 'bar' } },
          };
          return CHANGES;
        });

        expect(callSubscriptions).toHaveBeenCalledTimes(4);
        expect(callSubscriptions).toHaveBeenNthCalledWith(
          4,
          expect.anything(),
          CHANGES,
        );
      });

      test('calls subscriptions if reverted unrelated field', () => {
        const revert = addOptimisticStoreLayerInner(
          environment,
          (storeLayer) => {
            ignoreReadonly(storeLayer).data = {
              Pet: { 1: { surname: 'foo' } },
            };
            return new Map([['Pet', new Set(['1'])]]);
          },
        );

        addNetworkResponseStoreLayerInner(environment, (storeLayer) => {
          ignoreReadonly(storeLayer).data = {
            Query: { __ROOT: { name: 'foo' } },
          };
          return CHANGES;
        });

        revert((storeLayer) => {
          ignoreReadonly(storeLayer).data = {};
          return new Map();
        });

        expect(callSubscriptions).toHaveBeenCalledTimes(4);
        expect(callSubscriptions).toHaveBeenNthCalledWith(
          4,
          expect.anything(),
          new Map([['Pet', new Set(['1'])]]),
        );
      });
    });

    describe('with parent BaseStoreLayer', () => {
      test('merges ', () => {
        const revert = addOptimisticStoreLayer(
          environment,
          (counter) => counter + 1,
        );

        revert(5);

        expect(environment.store).toMatchObject({
          kind: 'BaseStoreLayer',
        });

        expect(
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(5);
      });

      test('merges children', () => {
        const revert = addOptimisticStoreLayer(
          environment,
          (counter) => counter + 1,
        );
        addNetworkResponseStoreLayer(environment, 12);

        revert(5);

        expect(environment.store).toMatchObject({
          kind: 'BaseStoreLayer',
        });

        expect(
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(12);
      });

      test('merges children and stops at optimistic child node', () => {
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
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(13);
      });

      test('stops at optimistic child node', () => {
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
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(6);
      });
    });

    describe('adjacent with NetworkResponseStoreLayer', () => {
      test('merges with parent NetworkResponseStoreLayer', () => {
        addOptimisticStoreLayer(environment, (counter) => counter + 1);
        addNetworkResponseStoreLayer(environment, 10);
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
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(12);
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
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(12);
      });

      test('merges between two NetworkResponseStoreLayers', () => {
        addOptimisticStoreLayer(environment, (counter) => counter + 1);
        addNetworkResponseStoreLayer(environment, 10);
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
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(12);
      });
    });

    describe('has parent OptimisticStoreLayer', () => {
      test('replaces self with network response', () => {
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
          readOptimisticRecord(environment.store, {
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
          readOptimisticRecord(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          }).counter,
        ).toBe(6);
      });
    });
  });

  // utils
  function addNetworkResponseStoreLayer(
    environment: IsographEnvironment,
    counter: number,
  ) {
    return addNetworkResponseStoreLayerInner(environment, (storeLayer) => {
      return update(storeLayer, () => counter);
    });
  }

  function addOptimisticStoreLayer(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    const revert = addOptimisticStoreLayerInner(environment, (storeLayer) => {
      return update(storeLayer, updater);
    });
    return (counter: number) => {
      revert((storeLayer) => update(storeLayer, () => counter));
    };
  }

  function addStartUpdateStoreLayer(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    addStartUpdateStoreLayerInner(environment, (storeLayer) => {
      return update(storeLayer, updater);
    });
  }

  const update = (
    storeLayer: StoreLayer,
    value: (counter: number) => number,
  ): EncounteredIds => {
    const { counter } = readOptimisticRecord(storeLayer, {
      __link: '__ROOT',
      __typename: 'Query',
    });
    const nextCounter = value(Number(counter));

    ignoreReadonly(storeLayer).data = {
      Query: {
        __ROOT: {
          counter: nextCounter,
        },
      },
    };

    return counter != nextCounter ? CHANGES : NO_CHANGES;
  };

  function ignoreReadonly(value: StoreLayer): { data: StoreLayerData } {
    return value;
  }
});
