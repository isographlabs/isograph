import { describe, expect, test, vi } from 'vitest';
import { getOrCreateCacheForArtifact, normalizeData } from '../core/cache';
import {
  createIsographEnvironment,
  createIsographStore,
  ROOT_ID,
  type IsographStore,
} from '../core/IsographEnvironment';
import {
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import { iso } from './__isograph/iso';
import type { Query__subquery__param } from './__isograph/Query/subquery/param_type';

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

describe('normalizeData', () => {
  test('nested Query should be normalized', () => {
    const store = createIsographStore();
    const networkFunction = vi
      .fn()
      .mockRejectedValue(new Error('Fetch failed'));
    const environment = createIsographEnvironment(store, networkFunction);

    normalizeData(
      environment,
      entrypoint.networkRequestInfo.normalizationAst.selections,
      {
        query: { node____id___v_id: { __typename: 'Economist', id: '1' } },
      },
      { id: '1' },
      { __link: ROOT_ID, __typename: entrypoint.concreteType },
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
    } satisfies IsographStore);
  });
});

describe('readData', () => {
  test('nested Query should be read', () => {
    const store: IsographStore = {
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
