import { iso } from '@iso';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type { ExtractUpdatableData } from '../core/FragmentReference';
import { getOrCreateCacheForArtifact } from '../core/getOrCreateCacheForArtifact';
import {
  ROOT_ID,
  type BaseStoreLayerData,
  type StoreLayerData,
  type WithErrorsData,
} from '../core/IsographEnvironment';
import { createUpdatableProxy } from '../core/startUpdate';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import type { Query__linkedUpdate__param } from './__isograph/Query/linkedUpdate/param_type';
import type { Query__startUpdate__param } from './__isograph/Query/startUpdate/param_type';

function ok<T>(value: T): WithErrorsData<T> {
  return {
    kind: 'Data',
    value,
  };
}

const getDefaultStore = (): BaseStoreLayerData => ({
  Query: {
    [ROOT_ID]: {
      node____id___0: ok({
        __link: '0',
        __typename: 'Economist',
      }),
      node____id___1: ok({
        __link: '1',
        __typename: 'Economist',
      }),
    },
  },
  Economist: {
    0: {
      __typename: 'Economist',
      id: '0',
      name: 'Jeremy Bentham',
    },
    1: {
      __typename: 'Economist',
      id: '1',
      name: 'John Stuart Mill',
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

export const linkedUpdate = iso(`
  field Query.linkedUpdate {
    node(id: 0) @updatable {
      asEconomist {
        name @updatable
      }
    }
    john_stuart_mill: node(id: 1) {
      __link
      asEconomist {
        name
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

  describe('linked field', () => {
    let data: ExtractUpdatableData<Query__linkedUpdate__param>;

    beforeEach(() => {
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(
          environment,
          iso(`entrypoint Query.linkedUpdate`),
          {},
        ).getOrPopulateAndTemporaryRetain();

      data = createUpdatableProxy(
        environment,
        environment.store,
        item,
        {
          suspendIfInFlight: true,
          throwOnNetworkError: true,
        },
        new Map(),
      );
    });

    test('updates updatable scalar nested in updatable object', () => {
      data.node!.asEconomist!.name = 'Updated Jeremy Bentham';

      expect(environment.store.data).toMatchObject<StoreLayerData>({
        Economist: {
          '0': {
            name: 'Updated Jeremy Bentham',
          },
        },
      });
    });

    test('updates linked field in data', () => {
      data.node = data.john_stuart_mill;

      expect(data).toMatchObject({
        node: {
          asEconomist: {
            name: 'John Stuart Mill',
          },
        },
      });
    });

    test('updates scalar in old object after setting linked field', () => {
      let jeremy = data.node;
      data.node = data.john_stuart_mill;
      jeremy!.asEconomist!.name = 'Updated Jeremy Bentham';

      expect(environment.store.data).toMatchObject<StoreLayerData>({
        Economist: {
          '0': {
            name: 'Updated Jeremy Bentham',
          },
        },
        Query: {
          __ROOT: {
            node____id___0: ok({
              __link: '1',
              __typename: 'Economist',
            }),
          },
        },
      });
    });
  });

  describe('scalar field', () => {
    let data: ExtractUpdatableData<Query__startUpdate__param>;

    beforeEach(() => {
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(
          environment,
          iso(`entrypoint Query.startUpdate`),
          {
            id: '0',
          },
        ).getOrPopulateAndTemporaryRetain();

      data = createUpdatableProxy(
        environment,
        environment.store,
        item,
        {
          suspendIfInFlight: true,
          throwOnNetworkError: true,
        },
        new Map(),
      );
    });

    test('reads data', () => {
      expect(data).toStrictEqual({
        node: {
          asEconomist: {
            name: 'Jeremy Bentham',
          },
        },
      });
    });

    test('updates scalar in cache', () => {
      data.node!.asEconomist!.name = 'Foo';

      expect(environment.store.data).toMatchObject<StoreLayerData>({
        Economist: {
          0: {
            name: 'Foo',
          },
        },
      });
    });

    test('updates scalar in data', () => {
      data.node!.asEconomist!.name = 'Foo';

      expect(data).toStrictEqual({
        node: {
          asEconomist: {
            name: 'Foo',
          },
        },
      });
    });
  });
});
