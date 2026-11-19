import { describe, expect, test } from 'vitest';
import petDetailNormalizationAst from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import petListsNormalizationAst from '../fixtures/store/src/__isograph/Query/PetLists/normalization_ast';
import {
  createIsographEnvironment,
  garbageCollectEnvironment,
  normalize,
  retainQuery,
  type RetainedQuery,
  type Store,
  unretainQuery,
} from '../src/index';
import {
  deferredNetwork,
  petDetailResponse,
  petListsResponse,
} from './fixtures';

const petDetailVariables = { id: '1', skip: null };

describe('createIsographEnvironment', () => {
  test('holds the store it was given and empty collections', () => {
    const store: Store = new Map();
    const environment = createIsographEnvironment(
      store,
      deferredNetwork().networkFunction,
    );
    expect(environment.store).toBe(store);
    expect(environment.retainedQueries.size).toBe(0);
    expect(environment.networkRequestCache.size).toBe(0);
    expect(environment.subscriptions.size).toBe(0);
  });
});

describe('retainQuery', () => {
  test('retains equal queries by identity', () => {
    const environment = createIsographEnvironment(
      new Map(),
      deferredNetwork().networkFunction,
    );
    const first: RetainedQuery = {
      ast: petDetailNormalizationAst,
      variables: petDetailVariables,
    };
    const second: RetainedQuery = {
      ast: petDetailNormalizationAst,
      variables: { ...petDetailVariables },
    };
    retainQuery(environment, first);
    retainQuery(environment, second);
    expect(environment.retainedQueries.size).toBe(2);
    unretainQuery(environment, first);
    expect(environment.retainedQueries.size).toBe(1);
    expect(environment.retainedQueries.has(second)).toBe(true);
  });
});

describe('garbageCollectEnvironment', () => {
  test('keeps what the retained queries reach and deletes the rest', () => {
    const environment = createIsographEnvironment(
      new Map(),
      deferredNetwork().networkFunction,
    );
    normalize(
      environment.store,
      petDetailNormalizationAst,
      petDetailResponse(),
      petDetailVariables,
    );
    normalize(
      environment.store,
      petListsNormalizationAst,
      petListsResponse(),
      {},
    );
    expect(environment.store.get('Pet')?.has('9')).toBe(true);
    const retained: RetainedQuery = {
      ast: petDetailNormalizationAst,
      variables: petDetailVariables,
    };
    retainQuery(environment, retained);

    garbageCollectEnvironment(environment);
    expect(environment.store.get('Pet')?.has('9')).toBe(false);
    expect(Array.from(environment.store.get('Pet')?.keys() ?? [])).toEqual([
      '1',
      '2',
    ]);
    expect(environment.store.get('Query')?.has('__ROOT')).toBe(true);
    expect(environment.store.get('PetStats')?.size).toBe(1);
    expect(environment.store.get('Checkin')?.size).toBe(3);

    unretainQuery(environment, retained);
    garbageCollectEnvironment(environment);
    expect(environment.store).toEqual(new Map());
  });
});
