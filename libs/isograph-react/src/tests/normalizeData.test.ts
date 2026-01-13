import { iso } from '@iso';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import {
  createIsographStore,
  ROOT_ID,
  type BaseStoreLayerData,
} from '../core/IsographEnvironment';
import { normalizeData } from '../core/cache';
import { createIsographEnvironment } from '../react/createIsographEnvironment';

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

describe('normalize undefined field', () => {
  test('should normalize scalar field to null', () => {
    normalizeData(
      environment,
      environment.store,
      normalizeUndefinedFieldEntrypoint.networkRequestInfo.normalizationAst
        .selections,
      {
        me: { __typename: 'Economist', id: '1' },
        id: 'query',
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
          id: 'query',
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
      environment.store,
      normalizeUndefinedFieldEntrypoint.networkRequestInfo.normalizationAst
        .selections,
      { id: 'query' },
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
          id: 'query',
          me: null,
        },
      },
    });
  });
});

export const subquery = iso(`
  field Query.subquery {
    __refetch
  }
`)(() => {});

const entrypoint = iso(`entrypoint Query.subquery`);

import entrypointRefetch from './__isograph/Query/subquery/__refetch__0';

describe('nested Query', () => {
  test('should be normalized', () => {
    normalizeData(
      environment,
      environment.store,
      entrypointRefetch.networkRequestInfo.normalizationAst.selections,
      {
        node____id___v_id: { __typename: 'Query', id: '1' },
      },
      { id: '1' },
      { __link: ROOT_ID, __typename: entrypoint.concreteType },
      new Map(),
    );

    expect(store).toStrictEqual({
      Query: {
        [ROOT_ID]: {
          id: '1',
          __typename: 'Query',
          node____id___1: { __typename: 'Query', __link: ROOT_ID },
        },
      },
    } satisfies BaseStoreLayerData);
  });
});
