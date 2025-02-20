import { beforeEach, describe, expect, test, vi } from 'vitest';
import { getOrCreateCacheForArtifact } from '../core/cache';
import type { ExtractStartUpdate } from '../core/FragmentReference';
import {
  createIsographEnvironment,
  ROOT_ID,
  type IsographEnvironment,
  type IsographStore,
} from '../core/IsographEnvironment';
import { createStartUpdate } from '../core/startUpdate';
import { iso } from './__isograph/iso';
import type { Query__startUpdate__param } from './__isograph/Query/startUpdate/param_type';

export const omitted = iso(`
  field Economist.omitted {
    name
  }
`)(({ data }) => data.name);

export const startUpdate = iso(`
  field Query.startUpdate($id: ID!) {
    node(id: $id) {
      asEconomist @updatable {
        omitted
        id
        name @updatable
        successor {
          id  
          name
        }
      }
    }  
  }
`)(() => {});

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

describe('startUpdate', () => {
  describe('updatable linked field', () => {
    let startUpdate: ExtractStartUpdate<Query__startUpdate__param>;
    let environment: IsographEnvironment;

    beforeEach(() => {
      const store = getDefaultStore();
      const networkFunction = vi
        .fn()
        .mockRejectedValue(new Error('Fetch failed'));
      environment = createIsographEnvironment(store, networkFunction);
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(
          environment,
          iso(`entrypoint Query.startUpdate`),
          {
            id: '1',
          },
        ).getOrPopulateAndTemporaryRetain();
      startUpdate = createStartUpdate(environment, item);
    });

    test.skip('reads nested client fields', () => {
      let result: string | undefined;
      startUpdate((data) => {
        if (data.node?.asEconomist) {
          result = data.node.asEconomist.omitted;
        }
      });

      expect(result).toBe('Jeremy Bentham');
    });

    test.skip('omits nested client fields when updating', () => {
      startUpdate((data) => {
        if (data.node?.asEconomist) {
          data.node.asEconomist = {
            id: data.node.asEconomist.id,
            name: data.node.asEconomist.name,
            successor: {
              id: '2',
              name: 'Updated Henry Sidgwick',
            },
            // @ts-expect-error We can't update client fields
            omitted: 'Updated Jeremy Bentham',
          };
        }
      });
    });

    test.skip('normalizes nested object', () => {
      startUpdate((data) => {
        if (data.node && data.node.asEconomist) {
          data.node.asEconomist = {
            id: data.node.asEconomist.id,
            name: data.node.asEconomist.name,
            successor: {
              id: '2',
              name: 'Updated Henry Sidgwick',
            },
          };
        }
      });

      expect(environment.store).toBe({
        Query: {
          [ROOT_ID]: {
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
            successor: { __link: '2', __typename: 'Economist' },
          },
          2: {
            __typename: 'Economist',
            id: '2',
            name: 'Updated Henry Sidgwick',
            predecessor: { __link: '1', __typename: 'Economist' },
          },
        },
      });
    });

    test.skip('keeps side effect of normalizing nested object', () => {
      startUpdate((data) => {
        if (data.node?.asEconomist) {
          let { id, name } = data.node.asEconomist;

          data.node.asEconomist = {
            id,
            name,
            successor: {
              id: '2',
              name: 'Updated Henry Sidgwick',
            },
          };
          data.node.asEconomist = {
            id,
            name,
            successor: null,
          };
        }
      });

      expect(environment.store).toBe({
        Query: {
          [ROOT_ID]: {
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
            successor: null,
          },
          2: {
            __typename: 'Economist',
            id: '2',
            name: 'Updated Henry Sidgwick',
            predecessor: { __link: '1', __typename: 'Economist' },
          },
        },
      });
    });

    test.skip('updates updatable scalar nested in updatable object', () => {
      startUpdate((data) => {
        if (data.node && data.node.asEconomist) {
          data.node.asEconomist.name = 'Updated Jeremy Bentham';
        }
      });

      expect(environment.store).toBe({
        Query: {
          [ROOT_ID]: {
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
            name: 'Updated Jeremy Bentham',
            successor: null,
          },
        },
      });
    });

    test.skip('updates updatable scalar nested in updatable object after the object update ', () => {
      startUpdate((data) => {
        if (data.node?.asEconomist) {
          data.node.asEconomist = {
            id: '2',
            name: 'Henry Sidgwick',
            successor: null,
          };
        }
        if (data.node?.asEconomist) {
          data.node.asEconomist.name = 'Updated Henry Sidgwick';
        }
      });

      expect(environment.store).toBe({
        Query: {
          [ROOT_ID]: {
            node____id___0: {
              __link: '2',
              __typename: 'Economist',
            },
          },
        },
        Economist: {
          0: {
            __typename: 'Economist',
            id: '2',
            name: 'Updated Henry Sidgwick',
            successor: null,
          },
        },
      });
    });
  });
});
