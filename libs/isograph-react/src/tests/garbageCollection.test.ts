import { describe, expect, test } from 'vitest';
import {
  garbageCollectEnvironment,
  retainQuery,
} from '../core/garbageCollection';
import {
  createIsographEnvironment,
  ROOT_ID,
  type IsographStore,
} from '../core/IsographEnvironment';
import { iso } from './__isograph/iso';
import { meNameSuccessorRetainedQuery } from './meNameSuccessor';
import { nodeFieldRetainedQuery } from './nodeQuery';

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

export const meNameField = iso(`
  field Query.meName {
    me {
      name
    }
  }
`)(() => {});

const meNameEntrypoint = iso(`entrypoint Query.meName`);
const meNameRetainedQuery = {
  normalizationAst:
    meNameEntrypoint.networkRequestInfo.normalizationAst.selections,
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
