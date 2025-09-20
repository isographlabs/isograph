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
import type { Query__errors__param } from './__isograph/Query/errors/param_type';
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

export const errors = iso(`
  field Query.errors($id: ID!) {
    node(id: $id) {
      asEconomist {
        id
        name
      } 
    }
  }
`)(() => {});
const errorsEntrypoint = iso(`entrypoint Query.errors`);

export const nicknameErrors = iso(`
  field Query.nicknameErrors($id: ID!) {
    node(id: $id) {
      asEconomist {
        id
        nickname
      } 
    }
  }
`)(() => {});
const nicknameErrorsEntrypoint = iso(`entrypoint Query.nicknameErrors`);

export const errorsSecond = iso(`
  field Query.errorsSecond($id: ID!) {
    node(id: $id) {
      id
    }
  }
`)(() => {});
const errorsSecondEntrypoint = iso(`entrypoint Query.errorsSecond`);

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
      undefined,
      [],
    );

    expect(store).toStrictEqual<IsographStore>({
      Economist: {
        '1': {
          errors: {},
          record: {
            __typename: 'Economist',
            id: '1',
          },
        },
      },
      Query: {
        [ROOT_ID]: {
          errors: {},
          record: {
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
      },
    });
  });
  describe('errors', () => {
    test('normalize errors', () => {
      const store = createIsographStore();
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        { node____id___v_id: null },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        [
          {
            message: 'Not found',
            path: ['node____id___v_id'],
          },
        ],
        [],
      );
      expect(store).toStrictEqual<IsographStore>({
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Not found',
                  path: ['node____id___v_id'],
                },
              ],
            },
            record: { node____id___1: null },
          },
        },
      });
    });

    test('nickname error', () => {
      const store = createIsographStore();
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        nicknameErrorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          node____id___v_id: {
            __typename: 'Economist',
            id: '1',
            nickname: null,
          },
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: nicknameErrorsEntrypoint.concreteType },
        [
          {
            message: 'Missing nickname',
            path: ['node____id___v_id', 'nickname'],
          },
        ],
        [],
      );
      expect(store).toStrictEqual<IsographStore>({
        Economist: {
          '1': {
            errors: {
              nickname: [
                {
                  message: 'Missing nickname',
                  path: ['node____id___v_id', 'nickname'],
                },
              ],
            },
            record: { __typename: 'Economist', id: '1', nickname: null },
          },
        },
        Query: {
          [ROOT_ID]: {
            errors: {},
            record: {
              node____id___1: {
                __typename: 'Economist',
                __link: '1',
              },
            },
          },
        },
      });
    });

    test('normalize nested errors', () => {
      const store = createIsographStore();
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        { node____id___v_id: null },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        [
          {
            message: 'Missing name',
            path: ['node____id___v_id', 'name'],
          },
        ],
        [],
      );
      expect(store).toStrictEqual<IsographStore>({
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: { node____id___1: null },
          },
        },
      });
    });

    test('deletes previous errors when node is null', () => {
      const store: IsographStore = {
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: { node____id___1: null },
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          node____id___v_id: null,
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        undefined,
        [],
      );

      expect(store).toStrictEqual<IsographStore>({
        Query: {
          [ROOT_ID]: {
            errors: {},
            record: {
              node____id___1: null,
            },
          },
        },
      });
    });

    test('deletes previous nickname errors when nickname is null', () => {
      const store: IsographStore = {
        Economist: {
          '1': {
            errors: {
              nickname: [
                {
                  message: 'Missing nickname',
                  path: ['node____id___v_id', 'nickname'],
                },
              ],
            },
            record: {
              __typename: 'Economist',
              id: '1',
              nickname: null,
            },
          },
        },
        Query: {
          [ROOT_ID]: {
            errors: {},
            record: {
              node____id___1: {
                __typename: 'Economist',
                __link: '1',
              },
            },
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        nicknameErrorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          node____id___v_id: {
            __typename: 'Economist',
            id: '1',
            nickname: null,
          },
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: nicknameErrorsEntrypoint.concreteType },
        undefined,
        [],
      );

      expect(store).toMatchObject<IsographStore>({
        Economist: {
          '1': {
            errors: {},
            record: {
              __typename: 'Economist',
              id: '1',
              nickname: null,
            },
          },
        },
        Query: {
          [ROOT_ID]: {
            errors: {},
            record: {
              node____id___1: {
                __typename: 'Economist',
                __link: '1',
              },
            },
          },
        },
      });
    });

    test('keeps nested errors', () => {
      const store: IsographStore = {
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: { node____id___1: null },
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          node____id___v_id: {
            __typename: 'Economist',
            id: '1',
            name: 'Bob',
          },
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        undefined,
        [],
      );

      expect(store).toStrictEqual<IsographStore>({
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: {
              node____id___1: {
                __typename: 'Economist',
                __link: '1',
              },
            },
          },
        },
        Economist: {
          '1': {
            errors: {},
            record: { __typename: 'Economist', id: '1', name: 'Bob' },
          },
        },
      });
    });

    test('keeps unrelated errors', () => {
      const store: IsographStore = {
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: {},
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        errorsSecondEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          node____id___v_id: {
            __typename: 'Economist',
            id: '1',
          },
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsSecondEntrypoint.concreteType },
        undefined,
        [],
      );

      expect(store).toStrictEqual<IsographStore>({
        Economist: {
          '1': {
            errors: {},
            record: { __typename: 'Economist', id: '1' },
          },
        },
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: {
              node____id___1: {
                __typename: 'Economist',
                __link: '1',
              },
            },
          },
        },
      });
    });
  });
});

describe('readData', () => {
  test('nested Query should be read', () => {
    const store: IsographStore = {
      Economist: {
        '1': {
          errors: {},
          record: {
            __typename: 'Economist',
            id: '1',
          },
        },
      },
      Query: {
        [ROOT_ID]: {
          errors: {},
          record: {
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

    expect(data).toStrictEqual<WithEncounteredRecords<Query__subquery__param>>({
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
      errors: null,
    });
  });

  describe('errors', () => {
    test('reads errors', () => {
      const store: IsographStore = {
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: {
              node____id___1: null,
            },
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(environment, errorsEntrypoint, {
          id: '1',
        }).getOrPopulateAndTemporaryRetain();

      const data = readButDoNotEvaluate(environment, item, {
        suspendIfInFlight: true,
        throwOnNetworkError: false,
      });

      expect(data).toStrictEqual<WithEncounteredRecords<Query__errors__param>>({
        encounteredRecords: new Map([['Query', new Set([ROOT_ID])]]),
        item: {
          node: null,
        },
        errors: [
          {
            message: 'Missing name',
            path: ['node____id___v_id', 'name'],
          },
        ],
      });
    });
    test('reads no errors', () => {
      const store: IsographStore = {
        Query: {
          [ROOT_ID]: {
            errors: {
              node____id___1: [
                {
                  message: 'Missing name',
                  path: ['node____id___v_id', 'name'],
                },
              ],
            },
            record: {
              node____id___1: {
                __typename: 'Economist',
                __link: '1',
              },
            },
          },
        },
        Economist: {
          '1': {
            errors: {},
            record: { __typename: 'Economist', id: '1', name: 'Bob' },
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(environment, errorsEntrypoint, {
          id: '1',
        }).getOrPopulateAndTemporaryRetain();

      const data = readButDoNotEvaluate(environment, item, {
        suspendIfInFlight: true,
        throwOnNetworkError: false,
      });

      expect(data).toStrictEqual<WithEncounteredRecords<Query__errors__param>>({
        encounteredRecords: new Map([
          ['Economist', new Set(['1'])],
          ['Query', new Set([ROOT_ID])],
        ]),
        item: {
          node: {
            asEconomist: { id: '1', name: 'Bob' },
          },
        },
        errors: null,
      });
    });
  });
});
