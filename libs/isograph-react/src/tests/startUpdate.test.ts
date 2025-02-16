import { beforeEach, describe, expect, test, vi } from 'vitest';
import { getOrCreateCacheForArtifact } from '../core/cache';
import {
  createIsographEnvironment,
  ROOT_ID,
  type IsographStore,
} from '../core/IsographEnvironment';
import { createStartUpdate } from '../core/startUpdate';
import { iso } from './__isograph/iso';

const getDefaultStore = (): IsographStore => ({
  Query: {
    [ROOT_ID]: {
      me: { __link: '0', __typename: 'Economist' },
      you: { __link: '1', __typename: 'Economist' },
      node____id___0: {
        __link: '0',
        __typename: 'Economist',
      },
    },
  },
  Economist: {
    0: {
      __typename: 'Economist',
      id: '0',
      name: 'Jeremy Bentham',
      successor: { __link: '1', __typename: 'Economist' },
    },
    1: {
      __typename: 'Economist',
      id: '1',
      name: 'John Stuart Mill',
      predecessor: { __link: '0', __typename: 'Economist' },
      successor: { __link: '2', __typename: 'Economist' },
    },
    2: {
      __typename: 'Economist',
      id: '2',
      name: 'Henry Sidgwick',
      predecessor: { __link: '1', __typename: 'Economist' },
    },
  },
});

export const startUpdate = iso(`
  field Query.startUpdate($id: ID!) {
    node(id: $id) {
      asEconomist {
        name @updatable
      }
    }
  }  
`)(() => {});

describe('startUpdate', () => {
  let environment: ReturnType<typeof createIsographEnvironment>;

  beforeEach(() => {
    const store = getDefaultStore();
    const networkFunction = vi
      .fn()
      .mockRejectedValue(new Error('Fetch failed'));
    environment = createIsographEnvironment(store, networkFunction);
  });

  test('startUpdate', () => {
    const [_cacheItem, item, _disposeOfTemporaryRetain] =
      getOrCreateCacheForArtifact(
        environment,
        iso(`entrypoint Query.startUpdate`),
        {
          id: '0',
        },
      ).getOrPopulateAndTemporaryRetain();
    let startUpdate = createStartUpdate(environment, item, {
      suspendIfInFlight: true,
      throwOnNetworkError: false,
    });

    startUpdate((data) => {
      if (data.node?.asEconomist) {
        data.node.asEconomist.name = 'Foo';
      }
    });

    expect(environment.store.Economist!['0']!.name).toBe('Foo');
  });
});
