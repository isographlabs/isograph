import { beforeEach, describe, expect, test, vi } from 'vitest';
import { callSubscriptions } from '../core/cache';
import {
  createIsographEnvironment,
  createIsographStore,
  type DataLayer,
  type IsographEnvironment,
} from '../core/IsographEnvironment';
import {
  addNetworkResponseNode as addNetworkResponseNodeInner,
  addOptimisticNode as addOptimisticNodeInner,
  addStartUpdateNode as addStartUpdateNodeInner,
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
    addNetworkResponseNode(environment, 0);
  });

  describe('addNetworkResponseNode', () => {
    test('has child BaseNode', () => {
      expect(environment.store).toMatchObject({
        kind: 'BaseNode',
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
      addNetworkResponseNode(environment, 0);

      expect(callSubscriptions).toHaveBeenCalledTimes(2);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        2,
        expect.anything(),
        NO_CHANGES,
      );
    });

    test('merge', () => {
      addOptimisticNode(environment, (counter) => counter + 1);
      addNetworkResponseNode(environment, 3);
      addNetworkResponseNode(environment, 4);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseNode',
        childNode: {
          kind: 'OptimisticNode',
          childNode: {
            kind: 'BaseNode',
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

  describe('addStartUpdateNode', () => {
    test('calls subscriptions', () => {
      addStartUpdateNode(environment, () => 1);

      expect(callSubscriptions).toHaveBeenCalledTimes(2);
      expect(callSubscriptions).nthCalledWith(2, expect.anything(), CHANGES);
    });

    test('merge', () => {
      addOptimisticNode(environment, () => 1);
      addStartUpdateNode(environment, (counter) => counter + 1);
      addStartUpdateNode(environment, (counter) => counter + 1);

      expect(environment.store).toMatchObject({
        kind: 'StartUpdateNode',
        childNode: {
          kind: 'OptimisticNode',
          childNode: {
            kind: 'BaseNode',
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

  describe('addOptimisticNode', () => {
    test('calls subscriptions', () => {
      addOptimisticNode(environment, () => 1);

      expect(callSubscriptions).toHaveBeenCalledTimes(2);
      expect(callSubscriptions).nthCalledWith(2, expect.anything(), CHANGES);
    });

    test('add tree nodes', () => {
      addOptimisticNode(environment, (counter) => counter + 1);
      addOptimisticNode(environment, (counter) => counter + 1);
      addOptimisticNode(environment, (counter) => counter + 1);

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(3);
    });
  });

  describe('replaceOptimisticNodeWithNetworkResponseNode', () => {
    test('calls subscriptions if changes', () => {
      const revert = addOptimisticNode(environment, () => 1);

      revert(2);

      expect(callSubscriptions).toHaveBeenCalledTimes(3);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        3,
        expect.anything(),
        CHANGES,
      );
    });

    test("doesn't call subscriptions if no changes", () => {
      const revert = addOptimisticNode(environment, () => 1);

      revert(1);

      expect(callSubscriptions).toHaveBeenCalledTimes(3);
      expect(callSubscriptions).toHaveBeenNthCalledWith(
        3,
        expect.anything(),
        NO_CHANGES,
      );
    });

    test('has child BaseNode and parent node', () => {
      const revert = addOptimisticNode(environment, (counter) => counter + 1);
      addOptimisticNode(environment, (counter) => counter + 1);

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'OptimisticNode',
        childNode: {
          kind: 'BaseNode',
        },
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(6);
    });

    test('has child node and no parent node', () => {
      addOptimisticNode(environment, (counter) => counter + 1);
      const revert = addOptimisticNode(environment, (counter) => counter + 1);

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseNode',
        childNode: {
          kind: 'OptimisticNode',
          childNode: {
            kind: 'BaseNode',
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

    test('has child BaseNode and no parent node', () => {
      const revert = addOptimisticNode(environment, (counter) => counter + 1);

      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'BaseNode',
      });

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(5);
    });

    test('has child node and parent node', () => {
      addOptimisticNode(environment, (counter) => counter + 1);
      const revert = addOptimisticNode(environment, (counter) => counter + 1);
      addStartUpdateNode(environment, (counter) => counter + 1);

      revert(5);

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(6);
    });

    test('has parent NetworkResponseNode', () => {
      addOptimisticNode(environment, (counter) => counter + 1);
      const revert = addOptimisticNode(environment, (counter) => counter + 1);
      addNetworkResponseNode(environment, 12);
      revert(5);

      expect(environment.store).toMatchObject({
        kind: 'NetworkResponseNode',
        childNode: {
          kind: 'OptimisticNode',
          childNode: {
            kind: 'BaseNode',
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
  });

  // utils
  function addNetworkResponseNode(
    environment: IsographEnvironment,
    counter: number,
  ) {
    const { data, encounteredIds } = update(() => counter);
    return addNetworkResponseNodeInner(environment, data, encounteredIds);
  }

  function addOptimisticNode(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    const revert = addOptimisticNodeInner(environment, () => {
      return update(updater);
    });
    return (counter: number) => {
      revert(update(() => counter).data);
    };
  }

  function addStartUpdateNode(
    environment: IsographEnvironment,
    updater: (prev: number) => number,
  ) {
    addStartUpdateNodeInner(environment, () => {
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
