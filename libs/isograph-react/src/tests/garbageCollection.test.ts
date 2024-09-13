import { describe, test, expect } from 'vitest';
import {
  ROOT_ID,
  createIsographEnvironment,
} from '../core/IsographEnvironment';
import {
  garbageCollectEnvironment,
  retainQuery,
} from '../core/garbageCollection';
import { iso } from './__isograph/iso';
import { nodeFieldRetainedQuery } from './nodeQuery';

const getDefaultStore = () => ({
  [ROOT_ID]: {
    me: { __link: '0' },
    you: { __link: '1' },
    node____id___0: {
      __link: '0',
    },
  },
  0: {
    __typename: 'Economist',
    id: '0',
    name: 'Jeremy Bentham',
    successor: { __link: '1' },
  },
  1: {
    __typename: 'Economist',
    id: '1',
    name: 'John Stuart Mill',
    predecessor: { __link: '0' },
    successor: { __link: '2' },
  },
  2: {
    __typename: 'Economist',
    id: '2',
    name: 'Henry Sidgwick',
    predecessor: { __link: '1' },
  },
});

export const meNameField = iso(`
  field Query.meName {
    me {
      name
    }
  }
`)(() => {});
import { meNameSuccessorRetainedQuery } from './meNameSuccessor';
const meNameEntrypoint = iso(`entrypoint Query.meName`);
const meNameRetainedQuery = {
  normalizationAst: meNameEntrypoint.normalizationAst,
  variables: {},
};

describe('garbage collection', () => {
  test('Unreferenced records should be garbage collected', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    expect(store[1]).not.toBe(undefined);

    // TODO enable babel so we don't have to do this
    retainQuery(environment, meNameRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store[1]).toBe(undefined);
  });

  test('Referenced records should not be garbage collected', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    expect(store[0]).not.toBe(undefined);

    // TODO enable babel so we don't have to do this
    retainQuery(environment, meNameRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store[0]).not.toBe(undefined);
  });

  test('Referenced records should not be garbage collected, and this should work with variables', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );

    expect(store[0]).not.toBe(undefined);

    retainQuery(environment, nodeFieldRetainedQuery);
    garbageCollectEnvironment(environment);

    expect(store[0]).not.toBe(undefined);
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

    expect(store[0]).not.toBe(undefined);
    expect(store[1]).not.toBe(undefined);
    expect(store[2]).not.toBe(undefined);
  });

  test('ROOT_ID should not be garbage collected, even if there are no retained queries', () => {
    const store = getDefaultStore();
    const environment = createIsographEnvironment(
      store,
      null as any,
      null as any,
    );
    garbageCollectEnvironment(environment);

    expect(store[ROOT_ID]).not.toBe(undefined);
    expect(store[0]).toBe(undefined);
  });
});
