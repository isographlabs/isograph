import { beforeEach, describe, expect, test, vi } from 'vitest';
import {
  createIsographEnvironment,
  createIsographStore,
} from '../core/IsographEnvironment';
import {
  addNetworkResponseNode,
  addOptimisticNode,
  addStartUpdateNode,
  readOptimisticRecord,
} from '../core/optimisticProxy';

describe('optimisticLayer', () => {
  let environment: ReturnType<typeof createIsographEnvironment>;

  beforeEach(() => {
    const networkFunction = vi
      .fn()
      .mockRejectedValue(new Error('Fetch failed'));
    environment = createIsographEnvironment(
      createIsographStore(),
      networkFunction,
    );
    addNetworkResponseNode(
      environment,
      update(() => 0),
    );
  });

  function update(value: (counter: number) => number) {
    return {
      Query: {
        __ROOT: {
          counter: value(
            Number(
              readOptimisticRecord(environment, {
                __link: '__ROOT',
                __typename: 'Query',
              }).counter,
            ),
          ),
        },
      },
    };
  }

  describe('addNetworkResponseNode', () => {
    test('has child BaseNode', () => {
      expect(environment.store).toMatchObject({
        kind: 'BaseNode',
      });
    });

    test('merge', () => {
      addOptimisticNode(environment, () => update((counter) => counter + 1));
      addNetworkResponseNode(
        environment,
        update(() => 3),
      );
      addNetworkResponseNode(
        environment,
        update(() => 4),
      );

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
    test('merge', () => {
      addOptimisticNode(environment, () => update(() => 1));
      addStartUpdateNode(environment, () => update((counter) => counter + 1));
      addStartUpdateNode(environment, () => update((counter) => counter + 1));

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
    test('add tree nodes', () => {
      addOptimisticNode(environment, () => update((counter) => counter + 1));
      addOptimisticNode(environment, () => update((counter) => counter + 1));
      addOptimisticNode(environment, () => update((counter) => counter + 1));

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(3);
    });
  });

  describe('replaceOptimisticNodeWithNetworkResponseNode', () => {
    test('has child BaseNode and parent node', () => {
      const revert = addOptimisticNode(environment, () =>
        update((counter) => counter + 1),
      );
      addOptimisticNode(environment, () => update((counter) => counter + 1));

      revert(update(() => 5));

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
      addOptimisticNode(environment, () => update((counter) => counter + 1));
      const revert = addOptimisticNode(environment, () =>
        update((counter) => counter + 1),
      );

      revert(update(() => 5));

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
      const revert = addOptimisticNode(environment, () =>
        update((counter) => counter + 1),
      );

      revert(update(() => 5));

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
      addOptimisticNode(environment, () => update((counter) => counter + 1));
      const revert = addOptimisticNode(environment, () =>
        update((counter) => counter + 1),
      );
      addStartUpdateNode(environment, () => update((counter) => counter + 1));

      revert(update(() => 5));

      expect(
        readOptimisticRecord(environment, {
          __link: '__ROOT',
          __typename: 'Query',
        }).counter,
      ).toBe(6);
    });

    test('has parent NetworkResponseNode', () => {
      addOptimisticNode(environment, () => update((counter) => counter + 1));
      const revert = addOptimisticNode(environment, () =>
        update((counter) => counter + 1),
      );
      addNetworkResponseNode(
        environment,
        update(() => 12),
      );
      revert(update(() => 5));

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
});
