import { iso } from '@iso';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import {
  createIsographStore,
  ROOT_ID,
  type BaseStoreLayerData,
} from '../core/IsographEnvironment';
import { normalizeData } from '../core/cache';
import { getOrCreateCacheForArtifact } from '../core/getOrCreateCacheForArtifact';
import type { StoreLayer } from '../core/optimisticProxy';
import {
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import type { Query__subquery__param } from './__isograph/Query/subquery/param_type';

let store: ReturnType<typeof createIsographStore>;
let environment: ReturnType<typeof createIsographEnvironment>;

beforeEach(() => {
  store = createIsographStore();
  const networkFunction = vi.fn().mockRejectedValue(new Error('Fetch failed'));
  environment = createIsographEnvironment(store, networkFunction);
});

export const normalizeUndefinedField = iso(`
  field Query.normalizeUndefinedField {
    me {
      name
    }
  }
`)(() => {});

const normalizeUndefinedFieldEntrypoint = iso(
  `entrypoint Query.normalizeUndefinedField`,
);

function getBaseStoreLayer(node: StoreLayer) {
  if (node.kind === 'BaseStoreLayer') {
    return node;
  }
  return getBaseStoreLayer(node.parentStoreLayer);
}

describe('normalize undefined field', () => {
  test('should normalize scalar field to null', () => {
    normalizeData(
      environment,
      getBaseStoreLayer(environment.store),
      normalizeUndefinedFieldEntrypoint.networkRequestInfo.normalizationAst
        .selections,
      {
        me: { __typename: 'Economist', id: '1' },
      },
      {},
      {
        __link: ROOT_ID,
        __typename: normalizeUndefinedFieldEntrypoint.concreteType,
      },
      new Map(),
    );
    expect(store).toStrictEqual({
      Economist: {
        '1': {
          id: '1',
          name: null,
        },
      },
      Query: {
        [ROOT_ID]: {
          me: {
            __typename: 'Economist',
            __link: '1',
          },
        },
      },
    });
  });

  test('should normalize linked field to null', () => {
    normalizeData(
      environment,
      getBaseStoreLayer(environment.store),
      normalizeUndefinedFieldEntrypoint.networkRequestInfo.normalizationAst
        .selections,
      {},
      {},
      {
        __link: ROOT_ID,
        __typename: normalizeUndefinedFieldEntrypoint.concreteType,
      },
      new Map(),
    );
    expect(store).toStrictEqual({
      Query: {
        [ROOT_ID]: {
          me: null,
        },
      },
    });
  });
});

export const subquery = iso(`
  field Query.subquery($id: ID!) {
    query {
      node(id: $id) {
        id
      }
    }
  }
`)(() => {});

const entrypoint = iso(`entrypoint Query.subquery`);

describe('nested Query', () => {
  test('should be normalized', () => {
    normalizeData(
      environment,
      getBaseStoreLayer(environment.store),
      entrypoint.networkRequestInfo.normalizationAst.selections,
      {
        query: { node____id___v_id: { __typename: 'Economist', id: '1' } },
      },
      { id: '1' },
      { __link: ROOT_ID, __typename: entrypoint.concreteType },
      new Map(),
    );

    expect(store).toStrictEqual({
      Economist: {
        '1': {
          __typename: 'Economist',
          id: '1',
        },
      },
      Query: {
        [ROOT_ID]: {
          node____id___1: {
            __typename: 'Economist',
            __link: '1',
          },
          query: {
            __link: ROOT_ID,
            __typename: 'Query',
          },
        },
      },
    } satisfies BaseStoreLayerData);
  });

  test('should be read', () => {
    const store: BaseStoreLayerData = {
      Economist: {
        '1': {
          __typename: 'Economist',
          id: '1',
        },
      },
      Query: {
        [ROOT_ID]: {
          node____id___1: {
            __typename: 'Economist',
            __link: '1',
          },
          query: {
            __link: ROOT_ID,
            __typename: 'Query',
          },
        },
      },
    };
    const networkFunction = vi
      .fn()
      .mockRejectedValue(new Error('Fetch failed'));
    const environment = createIsographEnvironment(store, networkFunction);
    const [_cacheItem, item, _disposeOfTemporaryRetain] =
      getOrCreateCacheForArtifact(environment, entrypoint, {
        id: '1',
      }).getOrPopulateAndTemporaryRetain();

    const data = readButDoNotEvaluate(environment, item, {
      suspendIfInFlight: true,
      throwOnNetworkError: false,
    });

    expect(data).toStrictEqual({
      encounteredRecords: new Map([
        ['Economist', new Set(['1'])],
        ['Query', new Set([ROOT_ID])],
      ]),
      item: {
        query: {
          node: {
            id: '1',
          },
        },
      },
    } satisfies WithEncounteredRecords<Query__subquery__param>);
  });
});
