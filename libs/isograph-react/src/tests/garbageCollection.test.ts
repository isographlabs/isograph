import { iso } from '@iso';
import { describe, expect, test } from 'vitest';
import {
  garbageCollectEnvironment,
  retainQuery,
  type RetainedQuery,
} from '../core/garbageCollection';
import {
  ROOT_ID,
  type BaseStoreLayerData,
  type DataTypeValue,
  type WithErrorsData,
} from '../core/IsographEnvironment';
import { wrapResolvedValue } from '../core/PromiseWrapper';
import { createIsographEnvironment } from '../react/createIsographEnvironment';
import { meNameSuccessorRetainedQuery } from './meNameSuccessor';
import { nodeFieldRetainedQuery } from './nodeQuery';

function ok<T extends DataTypeValue>(value: T): WithErrorsData<T> {
  return {
    kind: 'Data',
    value,
  };
}

const getDefaultStore = (): BaseStoreLayerData => ({
  Query: {
    [ROOT_ID]: {
      me: ok({ __link: '0', __typename: 'Economist' }),
      you: ok({ __link: '1', __typename: 'Economist' }),
      node____id___0: ok({
        __link: '0',
        __typename: 'Economist',
      }),
    },
  },
  Economist: {
    0: {
      __typename: ok('Economist'),
      id: ok('0'),
      name: ok('Jeremy Bentham'),
      successor: ok({ __link: '1', __typename: 'Economist' }),
    },
    1: {
      __typename: ok('Economist'),
      id: ok('1'),
      name: ok('John Stuart Mill'),
      predecessor: ok({ __link: '0', __typename: 'Economist' }),
      successor: ok({ __link: '2', __typename: 'Economist' }),
    },
    2: {
      __typename: ok('Economist'),
      id: ok('2'),
      name: ok('Henry Sidgwick'),
      predecessor: ok({ __link: '1', __typename: 'Economist' }),
    },
  },
});

export const meNameField = iso(`
  field Query.meName {
    me {
      name
    }
  }
`)(() => {});

const meNameEntrypoint = iso(`entrypoint Query.meName`);
const meNameRetainedQuery: RetainedQuery = {
  normalizationAst: wrapResolvedValue(
    meNameEntrypoint.networkRequestInfo.normalizationAst,
  ),
  variables: {},
  root: { __link: ROOT_ID, __typename: 'Query' },
};

describe('garbage collection', () => {
  test('Unreferenced records should be garbage collected', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    expect(store.Economist?.[1]).not.toBe(undefined);

    // TODO enable babel so we don't have to do this
    retainQuery(environment, meNameRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store.Economist?.[1]).toBe(undefined);
  });

  test('Referenced records should not be garbage collected', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    expect(store.Economist?.[0]).not.toBe(undefined);

    // TODO enable babel so we don't have to do this
    retainQuery(environment, meNameRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store.Economist?.[0]).not.toBe(undefined);
  });

  test('Referenced records should not be garbage collected, and this should work with variables', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    expect(store.Economist?.[0]).not.toBe(undefined);

    retainQuery(environment, nodeFieldRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store.Economist?.[0]).not.toBe(undefined);
  });

  test('Referenced records should not be garbage collected, and this should work through multiple levels', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    retainQuery(environment, meNameSuccessorRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store.Economist?.[0]).not.toBe(undefined);
    expect(store.Economist?.[1]).not.toBe(undefined);
    expect(store.Economist?.[2]).not.toBe(undefined);
  });

  test('ROOT_ID should be garbage collected, if there are no retained queries', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );
    garbageCollectEnvironment(environment);

    expect(store.Query?.[ROOT_ID]).toBe(undefined);
    expect(store.Economist?.[0]).toBe(undefined);
  });
});
