import { expect, test } from 'vitest';
import setTagline from '../fixtures/store/src/__isograph/Mutation/SetTagline/normalization_ast';
import feed from '../fixtures/store/src/__isograph/Query/Feed/normalization_ast';
import petDetail from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import petLists from '../fixtures/store/src/__isograph/Query/PetLists/normalization_ast';
import { garbageCollect } from '../src/garbageCollection';
import { normalize } from '../src/normalize';
import type { EntityName } from '../src/artifacts';
import type { RecordId, Store } from '../src/store';
import {
  feedResponse,
  petDetailResponse,
  petListsResponse,
  setTaglineResponse,
  storeOf,
} from './fixtures';

const petDetailVariables = { id: '1', skip: null };
const setTaglineVariables = { id: '4', tagline: 'Hi' };

/** Each entity's record ids, sorted, for asserting which records remain. */
function recordIds(store: Store): {
  [entityName: EntityName]: ReadonlyArray<RecordId>;
} {
  return Object.fromEntries(
    [...store].map(([entityName, records]) => [
      entityName,
      [...records.keys()].sort(),
    ]),
  );
}

function storeWithPetDetail(): Store {
  const store: Store = new Map();
  normalize(store, petDetail, petDetailResponse(), petDetailVariables);
  return store;
}

test('retaining nothing empties the store; an empty store stays empty', () => {
  const store = storeWithPetDetail();
  garbageCollect(store, []);
  expect(store).toEqual(new Map());

  const empty: Store = new Map();
  garbageCollect(empty, [{ ast: petDetail, variables: petDetailVariables }]);
  expect(empty).toEqual(new Map());
});

test('one retained query keeps exactly what it reaches', () => {
  const store = storeWithPetDetail();
  normalize(store, petLists, petListsResponse(), {});
  garbageCollect(store, [{ ast: petDetail, variables: petDetailVariables }]);
  expect(recordIds(store)).toEqual({
    Query: ['__ROOT'],
    Pet: ['1', '2'],
    PetStats: ['Pet:1.stats'],
    Checkin: ['c1', 'c2', 'c3'],
  });
});

test('a reached record stays whole, including fields only another AST selects', () => {
  const store = storeWithPetDetail();
  normalize(store, petLists, petListsResponse(), {});
  garbageCollect(store, [{ ast: petLists, variables: {} }]);
  expect(recordIds(store)).toEqual({
    Query: ['__ROOT'],
    Pet: ['1', '3', '4', '5', '9'],
    PetStats: ['Query:__ROOT.statGrid.0.0', 'Query:__ROOT.statGrid.0.1'],
  });
  const pet1 = store.get('Pet')?.get('1');
  expect(pet1?.get('tagline')).toBe('Good dog');
  expect(pet1?.has('checkins____limit___2')).toBe(true);
  expect(store.get('Query')?.get('__ROOT')?.has('pet____id___"1"')).toBe(true);
});

test('two retained roots keep both', () => {
  const store = storeWithPetDetail();
  normalize(store, setTagline, setTaglineResponse(), setTaglineVariables);
  normalize(store, petLists, petListsResponse(), {});
  garbageCollect(store, [
    { ast: petDetail, variables: petDetailVariables },
    { ast: setTagline, variables: setTaglineVariables },
  ]);
  expect(recordIds(store)).toEqual({
    Query: ['__ROOT'],
    Pet: ['1', '2', '4'],
    PetStats: ['Pet:1.stats'],
    Checkin: ['c1', 'c2', 'c3'],
    Mutation: ['__ROOT'],
    SetTaglinePayload: [
      'Mutation:__ROOT.setTagline____input___{"id":"4","tagline":"Hi"}',
    ],
  });
});

test('variables that reach nothing keep only the root', () => {
  const store = storeWithPetDetail();
  garbageCollect(store, [
    { ast: petDetail, variables: { id: '2', skip: null } },
  ]);
  expect(recordIds(store)).toEqual({ Query: ['__ROOT'] });
});

test('conditionals: both feed items are kept, and the BlogItem walk reaches its author', () => {
  const store: Store = new Map();
  normalize(store, feed, feedResponse(), { limit: 2 });
  normalize(store, petLists, petListsResponse(), {});
  garbageCollect(store, [{ ast: feed, variables: { limit: 2 } }]);
  expect(recordIds(store)).toEqual({
    Query: ['__ROOT'],
    Pet: ['1'],
    BlogItem: ['b1'],
    AdItem: ['Query:__ROOT.feed.1____limit___2'],
  });
});

test('a conditional of another entity is not walked', () => {
  const store = storeOf({
    Query: {
      __ROOT: {
        feed____limit___2: [{ entityName: 'AdItem', id: 'a' }],
      },
    },
    AdItem: {
      a: { advertiser: 'Acme', author: { entityName: 'Pet', id: '1' } },
    },
    Pet: { '1': { id: '1', name: 'Rex' } },
  });
  garbageCollect(store, [{ ast: feed, variables: { limit: 2 } }]);
  expect(recordIds(store)).toEqual({ Query: ['__ROOT'], AdItem: ['a'] });
});

test('an absent linked record or root ends the walk without error', () => {
  const store = storeWithPetDetail();
  store.get('Pet')?.delete('2');
  garbageCollect(store, [
    { ast: petDetail, variables: petDetailVariables },
    { ast: setTagline, variables: setTaglineVariables },
  ]);
  expect(recordIds(store)).toEqual({
    Query: ['__ROOT'],
    Pet: ['1'],
    PetStats: ['Pet:1.stats'],
    Checkin: ['c1', 'c2', 'c3'],
  });
});

test('cyclic data terminates', () => {
  const store = storeOf({
    Query: { __ROOT: { 'pet____id___"1"': { entityName: 'Pet', id: '1' } } },
    Pet: {
      '1': { id: '1', bestFriend: { entityName: 'Pet', id: '2' } },
      '2': { id: '2', bestFriend: { entityName: 'Pet', id: '1' } },
    },
  });
  garbageCollect(store, [{ ast: petDetail, variables: petDetailVariables }]);
  expect(recordIds(store)).toEqual({ Query: ['__ROOT'], Pet: ['1', '2'] });
});

test('a non-link under a composite store key throws', () => {
  const store = storeOf({
    Query: { __ROOT: { pets: 'x' } },
  });
  expect(() =>
    garbageCollect(store, [{ ast: petLists, variables: {} }]),
  ).toThrow(
    'Store record Query:__ROOT holds a non-link under the composite store key "pets"; the store was written with an AST that disagrees with this one.',
  );
});
