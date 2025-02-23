import { beforeEach, describe, expect, test, vi } from 'vitest';
import {
  createIsographEnvironment,
  type IsographStore,
} from '../core/IsographEnvironment';
import { mergeOptimisticLayer } from '../core/optimisticProxy';

describe('optimisticProxy', () => {
  let environment: ReturnType<typeof createIsographEnvironment>;

  beforeEach(() => {
    const store: IsographStore = {
      Query: {
        __ROOT: {},
      },
      Economist: {
        0: {
          __typename: 'Economist',
          id: '0',
          name: 'Jeremy Bentham',
          successor: { __link: '1', __typename: 'Economist' },
        },
      },
    };
    const networkFunction = vi
      .fn()
      .mockRejectedValue(new Error('Fetch failed'));
    environment = createIsographEnvironment(store, networkFunction);
  });

  test('is equal to store', () => {
    expect(environment.optimisticStore.Economist?.[0]).toStrictEqual({
      __typename: 'Economist',
      id: '0',
      name: 'Jeremy Bentham',
      successor: { __link: '1', __typename: 'Economist' },
    });
  });

  test('writes update optimistic layer', () => {
    environment.optimisticStore.Economist![0]!.name = 'Updated Jeremy Bentham';

    expect(environment.optimisticLayer).toStrictEqual({
      Economist: {
        0: { name: 'Updated Jeremy Bentham' },
      },
    });
  });

  test('mergeOptimisticLayer', () => {
    environment.optimisticStore.Economist![0]!.name = 'Updated Jeremy Bentham';

    mergeOptimisticLayer(environment);
    expect(environment.optimisticLayer).toStrictEqual({});
    expect(environment.optimisticStore.Economist?.[0]).toStrictEqual({
      __typename: 'Economist',
      id: '0',
      name: 'Updated Jeremy Bentham',
      successor: { __link: '1', __typename: 'Economist' },
    });
  });
});
