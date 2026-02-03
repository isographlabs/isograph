import { beforeEach, describe, expect, test, vi } from 'vitest';

import { type EncounteredIds } from '../core/cache';
import {
  createIsographStore,
  type IsographEnvironment,
  type StoreLayerData,
} from '../core/IsographEnvironment';
import {
  addNetworkResponseStoreLayer as addNetworkResponseStoreLayerInner,
  addOptimisticUpdaterStoreLayer as addOptimisticStoreLayerInner,
  addStartUpdateStoreLayer as addStartUpdateStoreLayerInner,
  getStoreRecordProxy,
  revertOptimisticStoreLayerAndMaybeReplace,
  type OptimisticStoreLayer,
  type StoreLayer,
} from '../core/optimisticProxy';
import { callSubscriptions } from '../core/subscribe';
import { createIsographEnvironment } from '../react/createIsographEnvironment';

vi.mock(import('../core/subscribe'), { spy: true });

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

    test('merge', () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      addNetworkResponseStoreLayer(environment, 3);
      addNetworkResponseStoreLayer(environment, 4);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseStoreLayer',
        parentStoreLayer: {
          kind: 'OptimisticUpdaterStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        },
      });

      expect(
        getStoreRecordProxy(environment.store, {
          __link: '__ROOT',
          __typename: 'Query',
        })?.['counter'],
      ).toBe(4);
    });
  });

  describe('addStartUpdateStoreLayer', () => {
    test('merge', () => {
      addOptimisticStoreLayer(environment, () => 1);
      addStartUpdateStoreLayer(environment, (counter) => counter + 1);
      addStartUpdateStoreLayer(environment, (counter) => counter + 1);

      expect(environment.store).toMatchObject({
        kind: 'StartUpdateStoreLayer',
        parentStoreLayer: {
          kind: 'OptimisticUpdaterStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        },
      });
      expect(
        getStoreRecordProxy(environment.store, {
          __link: '__ROOT',
          __typename: 'Query',
        })?.['counter'],
      ).toBe(3);
    });
  });

  describe('addOptimisticStoreLayer', () => {
    test('add tree nodes', () => {
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      addOptimisticStoreLayer(environment, (counter) => counter + 1);
      addOptimisticStoreLayer(environment, (counter) => counter + 1);

      expect(
        getStoreRecordProxy(environment.store, {
          __link: '__ROOT',
          __typename: 'Query',
        })?.['counter'],
      ).toBe(3);
    });
  });

  describe('replaceOptimisticStoreLayerWithNetworkResponseStoreLayer', () => {
    describe('subscriptions', () => {
      test('calls if network response differs', () => {
        const revert = addOptimisticStoreLayer(environment, () => 1);

        revert(2);

        expect(callSubscriptions).toHaveBeenCalledTimes(1);
        expect(callSubscriptions).toHaveBeenNthCalledWith(
          1,
          expect.anything(),
          CHANGES,
        );
      });

      test("doesn't call subscriptions if no changes", () => {
        const revert = addOptimisticStoreLayer(environment, () => 1);

        revert(1);

        expect(callSubscriptions).toHaveBeenCalledTimes(1);
        expect(callSubscriptions).toHaveBeenNthCalledWith(
          1,
          expect.anything(),
          NO_CHANGES,
        );
      });

      test('calls subscriptions if nodes update different fields', () => {
        const node = addOptimisticStoreLayerInner(
          environment.store,
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
        environment.store = node;
        environment.store = addNetworkResponseStoreLayerInner(
          environment.store,
        );
        ignoreReadonly(environment.store).data = {
          Query: {
            __ROOT: {
              name: 'foo',
            },
          },
        };

        revertOptimisticStoreLayerAndMaybeReplace(
          environment,
          node,
          (storeLayer) => {
            ignoreReadonly(storeLayer).data = {
              Query: { __ROOT: { surname: 'bar' } },
            };
            return CHANGES;
          },
        );

        expect(callSubscriptions).toHaveBeenCalledTimes(1);
        expect(callSubscriptions).toHaveBeenNthCalledWith(
          1,
          expect.anything(),
          CHANGES,
        );
      });

      test('calls subscriptions if reverted unrelated field', () => {
        const node = addOptimisticStoreLayerInner(
          environment.store,
          (storeLayer) => {
            ignoreReadonly(storeLayer).data = {
              Pet: { 1: { surname: 'foo' } },
            };
            return new Map([['Pet', new Set(['1'])]]);
          },
        );
        environment.store = node;

        environment.store = addNetworkResponseStoreLayerInner(
          environment.store,
        );
        ignoreReadonly(environment.store).data = {
          Query: { __ROOT: { name: 'foo' } },
        };

        revertOptimisticStoreLayerAndMaybeReplace(
          environment,
          node,
          (storeLayer) => {
            ignoreReadonly(storeLayer).data = {};
            return new Map();
          },
        );

        expect(callSubscriptions).toHaveBeenCalledTimes(1);
        expect(callSubscriptions).toHaveBeenNthCalledWith(
          1,
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
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
          kind: 'OptimisticUpdaterStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        });
        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
        ).toBe(13);
      });

      test('reexecutes updates while merging children', () => {
        const revert = addOptimisticStoreLayer(
          environment,
          (counter) => counter + 1,
        );
        addStartUpdateStoreLayer(environment, (counter) => counter * 2);
        addStartUpdateStoreLayer(environment, (counter) => counter + 7);
        addOptimisticStoreLayer(environment, (counter) => counter + 1);

        revert(4);

        expect(environment.store).toMatchObject({
          kind: 'OptimisticUpdaterStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        });
        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
        ).toBe(16);
      });

      test('stops at optimistic child node', () => {
        const revert = addOptimisticStoreLayer(
          environment,
          (counter) => counter + 1,
        );
        addOptimisticStoreLayer(environment, (counter) => counter + 1);

        revert(5);

        expect(environment.store).toMatchObject({
          kind: 'OptimisticUpdaterStoreLayer',
          parentStoreLayer: {
            kind: 'BaseStoreLayer',
          },
        });

        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          },
        });

        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          },
        });

        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          },
        });

        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          },
        });

        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
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
              kind: 'OptimisticUpdaterStoreLayer',
              parentStoreLayer: {
                kind: 'BaseStoreLayer',
              },
            },
          },
        });

        expect(
          getStoreRecordProxy(environment.store, {
            __link: '__ROOT',
            __typename: 'Query',
          })?.['counter'],
        ).toBe(6);
      });
    });

    describe('network error', () => {
      describe('subscriptions', () => {
        test('calls if network response differs', () => {
          const revert = addOptimisticStoreLayer(environment, () => 1);

          revert(null);

          expect(callSubscriptions).toHaveBeenCalledTimes(1);
          expect(callSubscriptions).toHaveBeenNthCalledWith(
            1,
            expect.anything(),
            CHANGES,
          );
        });

        test('calls subscriptions if reverted unrelated field', () => {
          const node = addOptimisticStoreLayerInner(
            environment.store,
            (storeLayer) => {
              ignoreReadonly(storeLayer).data = {
                Pet: { 1: { surname: 'foo' } },
              };
              return new Map([['Pet', new Set(['1'])]]);
            },
          );
          environment.store = node;

          environment.store = addNetworkResponseStoreLayerInner(
            environment.store,
          );
          ignoreReadonly(environment.store).data = {
            Query: { __ROOT: { name: 'foo' } },
          };

          revert(environment, node, null);

          expect(callSubscriptions).toHaveBeenCalledTimes(1);
          expect(callSubscriptions).toHaveBeenNthCalledWith(
            1,
            expect.anything(),
            new Map([['Pet', new Set(['1'])]]),
          );
        });
      });

      describe('with parent BaseStoreLayer', () => {
        test('removes self ', () => {
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'BaseStoreLayer',
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(0);
        });

        test('merges children', () => {
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );
          addNetworkResponseStoreLayer(environment, 12);

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'BaseStoreLayer',
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
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

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          });
          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(13);
        });

        test('reexecutes updates while merging children', () => {
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );
          addStartUpdateStoreLayer(environment, (counter) => counter + 2);
          addStartUpdateStoreLayer(environment, (counter) => counter * 7);
          addOptimisticStoreLayer(environment, (counter) => counter + 1);

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          });
          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(15);
        });

        test('stops at optimistic child node', () => {
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );
          addOptimisticStoreLayer(environment, (counter) => counter + 1);

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(1);
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
          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'NetworkResponseStoreLayer',
            parentStoreLayer: {
              kind: 'OptimisticUpdaterStoreLayer',
              parentStoreLayer: {
                kind: 'BaseStoreLayer',
              },
            },
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(12);
        });

        test('merges child NetworkResponseStoreLayer', () => {
          addOptimisticStoreLayer(environment, (counter) => counter + 1);
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );
          addNetworkResponseStoreLayer(environment, 12);
          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'NetworkResponseStoreLayer',
            parentStoreLayer: {
              kind: 'OptimisticUpdaterStoreLayer',
              parentStoreLayer: {
                kind: 'BaseStoreLayer',
              },
            },
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(12);
        });

        test('removes self and merges two adjacent NetworkResponseStoreLayers', () => {
          addOptimisticStoreLayer(environment, (counter) => counter + 1);
          addNetworkResponseStoreLayer(environment, 10);
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );
          addNetworkResponseStoreLayer(environment, 12);
          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'NetworkResponseStoreLayer',
            parentStoreLayer: {
              kind: 'OptimisticUpdaterStoreLayer',
              parentStoreLayer: {
                kind: 'BaseStoreLayer',
              },
            },
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(12);
        });
      });

      describe('has parent OptimisticStoreLayer', () => {
        test('removes self', () => {
          addOptimisticStoreLayer(environment, (counter) => counter + 1);
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'OptimisticUpdaterStoreLayer',
            parentStoreLayer: {
              kind: 'BaseStoreLayer',
            },
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(1);
        });

        test("doesn't merge child nodes if has parent nodes", () => {
          addOptimisticStoreLayer(environment, (counter) => counter + 1);
          const revert = addOptimisticStoreLayer(
            environment,
            (counter) => counter + 1,
          );
          addStartUpdateStoreLayer(environment, (counter) => counter + 1);

          revert(null);

          expect(environment.store).toMatchObject({
            kind: 'StartUpdateStoreLayer',
            parentStoreLayer: {
              kind: 'OptimisticUpdaterStoreLayer',
              parentStoreLayer: {
                kind: 'BaseStoreLayer',
              },
            },
          });

          expect(
            getStoreRecordProxy(environment.store, {
              __link: '__ROOT',
              __typename: 'Query',
            })?.['counter'],
          ).toBe(2);
        });
      });
    });
  });

  // utils
  function addNetworkResponseStoreLayer(
    environment: IsographEnvironment,
    counter: number,
  ) {
    environment.store = addNetworkResponseStoreLayerInner(environment.store);
    update(environment.store, () => counter);
  }

  function addOptimisticStoreLayer(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    const node = addOptimisticStoreLayerInner(
      environment.store,
      (storeLayer) => {
        return update(storeLayer, updater);
      },
    );
    environment.store = node;
    return (counter: null | number) => {
      revert(environment, node, counter);
    };
  }

  function revert(
    environment: IsographEnvironment,
    node: OptimisticStoreLayer,
    counter: null | number,
  ) {
    return revertOptimisticStoreLayerAndMaybeReplace(
      environment,
      node,
      counter == null
        ? counter
        : (storeLayer) => update(storeLayer, () => counter),
    );
  }

  function addStartUpdateStoreLayer(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    const node = addStartUpdateStoreLayerInner(
      environment.store,
      (storeLayer) => {
        return update(storeLayer, updater);
      },
    );
    environment.store = node;
  }

  const update = (
    storeLayer: StoreLayer,
    value: (counter: number) => number,
  ): EncounteredIds => {
    const { counter } =
      getStoreRecordProxy(storeLayer, {
        __link: '__ROOT',
        __typename: 'Query',
      }) ?? {};
    const nextCounter = value(Number(counter));

    ignoreReadonly(storeLayer).data = {
      Query: {
        __ROOT: {
          counter: nextCounter,
        },
      },
    };

    return counter !== nextCounter ? CHANGES : NO_CHANGES;
  };

  function ignoreReadonly(value: StoreLayer): { data: StoreLayerData } {
    return value;
  }
});
