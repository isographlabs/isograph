import { iso } from '@iso';
import { beforeEach, describe, expect, test, vi, vitest } from 'vitest';

import { normalizeData } from '../core/cache';
import { ReadFieldAggregateError } from '../core/errors';
import { getOrCreateCacheForArtifact } from '../core/getOrCreateCacheForArtifact';
import {
  createIsographStore,
  ROOT_ID,
  type BaseStoreLayerData,
  type StoreError,
  type WithErrors,
  type WithErrorsData,
} from '../core/IsographEnvironment';
import type { NonEmptyArray } from '../core/NonEmptyArray';
import type { BaseStoreLayer } from '../core/optimisticProxy';
import {
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import type { Query__errors__param } from './__isograph/Query/errors/param_type';
import type { Query__errorsClientField__param } from './__isograph/Query/errorsClientField/param_type';
import type { Query__errorsClientFieldComponent__param } from './__isograph/Query/errorsClientFieldComponent/param_type';
import type { Query__errorsClientPointer__param } from './__isograph/Query/errorsClientPointer/param_type';
import type { Query__subquery__param } from './__isograph/Query/subquery/param_type';

function ok<T>(value: T): WithErrorsData<T> {
  return {
    kind: 'Data',
    value,
  };
}

function err(errors: NonEmptyArray<StoreError>): WithErrors<never, StoreError> {
  return {
    kind: 'Errors',
    errors,
  };
}

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
      environment.store as BaseStoreLayer,
      normalizeUndefinedFieldEntrypoint.networkRequestInfo.normalizationAst
        .selections,
      {
        data: { me: { __typename: 'Economist', id: '1' } },
        errors: undefined,
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
          id: ok('1'),
          name: ok(null),
        },
      },
      Query: {
        [ROOT_ID]: {
          me: ok({
            __typename: 'Economist',
            __link: '1',
          }),
        },
      },
    });
  });

  test('should normalize linked field to null', () => {
    normalizeData(
      environment,
      environment.store as BaseStoreLayer,
      normalizeUndefinedFieldEntrypoint.networkRequestInfo.normalizationAst
        .selections,
      { data: undefined, errors: undefined },
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
          me: ok(null),
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
      environment.store as BaseStoreLayer,
      entrypoint.networkRequestInfo.normalizationAst.selections,
      {
        data: {
          query: { node____id___v_id: { __typename: 'Economist', id: '1' } },
        },
        errors: undefined,
      },
      { id: '1' },
      { __link: ROOT_ID, __typename: entrypoint.concreteType },
      new Map(),
    );

    expect(store).toStrictEqual({
      Economist: {
        '1': {
          __typename: ok('Economist'),
          id: ok('1'),
        },
      },
      Query: {
        [ROOT_ID]: {
          node____id___1: ok({
            __typename: 'Economist',
            __link: '1',
          }),
          query: ok({
            __link: ROOT_ID,
            __typename: 'Query',
          }),
        },
      },
    } satisfies BaseStoreLayerData);
  });

  test('should be read', () => {
    const store: BaseStoreLayerData = {
      Economist: {
        '1': {
          __typename: ok('Economist'),
          id: ok('1'),
        },
      },
      Query: {
        [ROOT_ID]: {
          node____id___1: ok({
            __typename: 'Economist',
            __link: '1',
          }),
          query: ok({
            __link: ROOT_ID,
            __typename: 'Query',
          }),
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
      kind: 'Success',
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

let errorsClientFieldFieldMock = vitest.fn();

export const errorsClientFieldField = iso(`
  field Economist.errorsClientFieldField {
    id
    nickname
  }
`)(errorsClientFieldFieldMock);

export const errorsClientField = iso(`
  field Query.errorsClientField($id: ID!) {
    node(id: $id) {
      asEconomist {
        errorsClientFieldField
      } 
    }
  }
`)(() => {});
const errorsClientFieldEntrypoint = iso(`entrypoint Query.errorsClientField`);

let errorsClientFieldComponentFieldMock = vitest.fn();

export const errorsClientFieldComponentField = iso(`
  field Economist.errorsClientFieldComponentField @component {
    id
    nickname
  }
`)(errorsClientFieldComponentFieldMock);

export const errorsClientFieldComponent = iso(`
  field Query.errorsClientFieldComponent($id: ID!) {
    node(id: $id) {
      asEconomist {
        errorsClientFieldComponentField
      } 
    }
  }
`)(() => {});

const errorsClientFieldComponentEntrypoint = iso(
  `entrypoint Query.errorsClientFieldComponent`,
);

let errorsClientPointerFieldMock = vitest.fn();

export const errorsClientPointerField = iso(`
  pointer Economist.errorsClientPointerField to Economist {
    id
    nickname
  }
`)(errorsClientPointerFieldMock);

export const errorsClientPointer = iso(`
  field Query.errorsClientPointer($id: ID!) {
    node(id: $id) {
      asEconomist {
        errorsClientPointerField {
          id
        }
      } 
    }
  }
`)(() => {});

const errorsClientPointerEntrypoint = iso(
  `entrypoint Query.errorsClientPointer`,
);

describe('errors', () => {
  describe('normalizeData', () => {
    test('normalize errors', () => {
      const store = createIsographStore();
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        environment.store as BaseStoreLayer,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          data: { node____id___v_id: null },
          errors: [
            {
              message: 'Not found',
              path: ['node____id___v_id'],
            },
          ],
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        new Map(),
      );
      expect(store).toStrictEqual<BaseStoreLayerData>({
        Query: {
          [ROOT_ID]: {
            node____id___1: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
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
        environment.store as BaseStoreLayer,
        nicknameErrorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          data: {
            node____id___v_id: {
              __typename: 'Economist',
              id: '1',
              nickname: null,
            },
          },
          errors: [
            {
              message: 'Missing nickname',
              path: ['node____id___v_id', 'nickname'],
            },
          ],
        },
        { id: '1' },
        {
          __link: ROOT_ID,
          __typename: nicknameErrorsEntrypoint.concreteType,
        },
        new Map(),
      );
      expect(store).toStrictEqual<BaseStoreLayerData>({
        Economist: {
          '1': {
            __typename: ok('Economist'),
            id: ok('1'),
            nickname: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __typename: 'Economist',
              __link: '1',
            }),
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
        environment.store as BaseStoreLayer,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          data: { node____id___v_id: null },
          errors: [
            {
              message: 'Missing name',
              path: ['node____id___v_id', 'name'],
            },
          ],
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        new Map(),
      );
      expect(store).toStrictEqual<BaseStoreLayerData>({
        Query: {
          [ROOT_ID]: {
            node____id___1: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
      });
    });

    test('deletes previous errors when node is null', () => {
      const store: BaseStoreLayerData = {
        Query: {
          [ROOT_ID]: {
            node____id___1: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        environment.store as BaseStoreLayer,
        errorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          data: { node____id___v_id: null },
          errors: undefined,
        },
        { id: '1' },
        { __link: ROOT_ID, __typename: errorsEntrypoint.concreteType },
        new Map(),
      );

      expect(store).toStrictEqual<BaseStoreLayerData>({
        Query: {
          [ROOT_ID]: {
            node____id___1: ok(null),
          },
        },
      });
    });

    test('deletes previous nickname errors when nickname is null', () => {
      const store: BaseStoreLayerData = {
        Economist: {
          '1': {
            __typename: ok('Economist'),
            id: ok('1'),
            nickname: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __typename: 'Economist',
              __link: '1',
            }),
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );

      normalizeData(
        environment,
        environment.store as BaseStoreLayer,
        nicknameErrorsEntrypoint.networkRequestInfo.normalizationAst.selections,
        {
          data: {
            node____id___v_id: {
              __typename: 'Economist',
              id: '1',
              nickname: null,
            },
          },
          errors: undefined,
        },
        { id: '1' },
        {
          __link: ROOT_ID,
          __typename: nicknameErrorsEntrypoint.concreteType,
        },
        new Map(),
      );

      expect(store).toMatchObject<BaseStoreLayerData>({
        Economist: {
          '1': {
            __typename: ok('Economist'),
            id: ok('1'),
            nickname: ok(null),
          },
        },
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __typename: 'Economist',
              __link: '1',
            }),
          },
        },
      });
    });
  });

  describe('readData', () => {
    test('reads errors', () => {
      const store: BaseStoreLayerData = {
        Query: {
          [ROOT_ID]: {
            node____id___1: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
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

      expect(data).toEqual<WithEncounteredRecords<Query__errors__param>>({
        kind: 'Errors',
        encounteredRecords: new Map([['Query', new Set([ROOT_ID])]]),
        errors: new ReadFieldAggregateError([
          {
            errors: [{ extensions: undefined, locations: undefined }],
            path: ['node'],
          },
        ]),
      });
    });

    test('reads null for client field with error', () => {
      const store: BaseStoreLayerData = {
        Economist: {
          1: {
            __typename: ok('Economist'),
            id: ok('1'),
            nickname: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __link: '1',
              __typename: 'Economist',
            }),
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(environment, errorsClientFieldEntrypoint, {
          id: '1',
        }).getOrPopulateAndTemporaryRetain();

      const data = readButDoNotEvaluate(environment, item, {
        suspendIfInFlight: true,
        throwOnNetworkError: false,
      });

      expect(errorsClientFieldFieldMock).not.toBeCalled();
      expect(data).toStrictEqual<
        WithEncounteredRecords<Query__errorsClientField__param>
      >({
        kind: 'Errors',
        encounteredRecords: new Map([
          ['Query', new Set([ROOT_ID])],
          ['Economist', new Set(['1'])],
        ]),
        errors: new ReadFieldAggregateError([
          {
            errors: [{ extensions: undefined, locations: undefined }],
            path: ['node', 'asEconomist', 'errorsClientFieldField', 'nickname'],
          },
        ]),
      });
    });

    test('reads client field component with error', () => {
      const store: BaseStoreLayerData = {
        Economist: {
          1: {
            __typename: ok('Economist'),
            id: ok('1'),
            nickname: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __link: '1',
              __typename: 'Economist',
            }),
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(
          environment,
          errorsClientFieldComponentEntrypoint,
          {
            id: '1',
          },
        ).getOrPopulateAndTemporaryRetain();

      const data = readButDoNotEvaluate(environment, item, {
        suspendIfInFlight: true,
        throwOnNetworkError: false,
      });

      expect(errorsClientFieldComponentFieldMock).not.toBeCalled();
      expect(data).toStrictEqual<
        WithEncounteredRecords<Query__errorsClientFieldComponent__param>
      >({
        kind: 'Success',
        encounteredRecords: new Map([
          ['Query', new Set([ROOT_ID])],
          ['Economist', new Set(['1'])],
        ]),
        item: {
          node: {
            asEconomist: {
              errorsClientFieldComponentField: expect.any(Function),
            },
          },
        },
      });
    });

    test('reads null for client pointer with error', () => {
      const store: BaseStoreLayerData = {
        Economist: {
          1: {
            __typename: ok('Economist'),
            id: ok('1'),
            nickname: err([
              {
                extensions: undefined,
                locations: undefined,
              },
            ]),
          },
        },
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __link: '1',
              __typename: 'Economist',
            }),
          },
        },
      };
      const environment = createIsographEnvironment(
        store,
        vi.fn().mockRejectedValue(new Error('Fetch failed')),
      );
      const [_cacheItem, item, _disposeOfTemporaryRetain] =
        getOrCreateCacheForArtifact(
          environment,
          errorsClientPointerEntrypoint,
          {
            id: '1',
          },
        ).getOrPopulateAndTemporaryRetain();

      const data = readButDoNotEvaluate(environment, item, {
        suspendIfInFlight: true,
        throwOnNetworkError: false,
      });

      expect(errorsClientPointerFieldMock).not.toBeCalled();
      expect(data).toStrictEqual<
        WithEncounteredRecords<Query__errorsClientPointer__param>
      >({
        kind: 'Errors',
        encounteredRecords: new Map([
          ['Query', new Set([ROOT_ID])],
          ['Economist', new Set(['1'])],
        ]),
        errors: new ReadFieldAggregateError([
          {
            errors: [{ extensions: undefined, locations: undefined }],
            path: [
              'node',
              'asEconomist',
              'errorsClientPointerField',
              'nickname',
            ],
          },
        ]),
      });
    });

    test('reads no errors', () => {
      const store: BaseStoreLayerData = {
        Query: {
          [ROOT_ID]: {
            node____id___1: ok({
              __typename: 'Economist',
              __link: '1',
            }),
          },
        },
        Economist: {
          '1': {
            __typename: ok('Economist'),
            id: ok('1'),
            name: ok('Bob'),
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
        kind: 'Success',
        encounteredRecords: new Map([
          ['Economist', new Set(['1'])],
          ['Query', new Set([ROOT_ID])],
        ]),
        item: {
          node: {
            asEconomist: { id: '1', name: 'Bob' },
          },
        },
      });
    });
  });
});
