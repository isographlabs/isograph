import { expect, test } from 'vitest';
import setTagline from '../fixtures/store/src/__isograph/Mutation/SetTagline/normalization_ast';
import emptyStats from '../fixtures/store/src/__isograph/Query/EmptyStats/normalization_ast';
import feed from '../fixtures/store/src/__isograph/Query/Feed/normalization_ast';
import leaves from '../fixtures/store/src/__isograph/Query/Leaves/normalization_ast';
import nestedQuery from '../fixtures/store/src/__isograph/Query/NestedQuery/normalization_ast';
import petDetail from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import petLists from '../fixtures/store/src/__isograph/Query/PetLists/normalization_ast';
import search from '../fixtures/store/src/__isograph/Query/Search/normalization_ast';
import type { NormalizationAst } from '../src/artifacts';
import { check } from '../src/check';
import { normalize } from '../src/normalize';
import type { JsonObject, Store, Variables } from '../src/store';
import {
  feedResponse,
  mutationRoot,
  petDetailPet,
  petDetailResponse,
  petListsResponse,
  queryRoot,
  setTaglineResponse,
  storeOf,
} from './fixtures';

const enough = { kind: 'EnoughData' };
const missing = (entityName: string, id: string) => ({
  kind: 'MissingData',
  record: { entityName, id },
});

function normalized(
  ast: NormalizationAst,
  response: JsonObject,
  variables: Variables,
): Store {
  const store: Store = new Map();
  normalize(store, ast, response, variables);
  return store;
}

const petDetailVariables = { id: '1', skip: null };

test('the empty store is missing the root record', () => {
  expect(check(new Map(), petDetail, petDetailVariables)).toEqual({
    kind: 'MissingData',
    record: queryRoot,
  });
  expect(check(new Map(), setTagline, { id: '4', tagline: 'Hi' })).toEqual({
    kind: 'MissingData',
    record: mutationRoot,
  });
});

test('every fixture entrypoint has enough data after normalizing its response', () => {
  const cases: ReadonlyArray<
    readonly [NormalizationAst, JsonObject, Variables]
  > = [
    [
      leaves,
      {
        greeting: 'hi',
        greeting____loud___l_true____name___s_Ada_Lovelace____times___l_3: 'HI',
        greeting____name___v_name: null,
        greeting____name___l_null: null,
        settings: null,
        tags: [],
      },
      { name: null },
    ],
    [nestedQuery, { greeting: 'hi', query: { tags: null } }, {}],
    [petDetail, petDetailResponse(), petDetailVariables],
    [
      emptyStats,
      {
        pet____id___v_id: {
          __typename: 'Pet',
          id: '1',
          stats: { __typename: 'PetStats' },
        },
      },
      { id: '1' },
    ],
    [petLists, petListsResponse(), {}],
    [feed, feedResponse(), { limit: 2 }],
    [
      search,
      { search____filter___o_minAge__l_2_nameContains__v_q_c: [] },
      { q: 'Rex' },
    ],
    [setTagline, setTaglineResponse(), { id: '4', tagline: 'Hi' }],
  ];
  cases.forEach(([ast, response, variables]) =>
    expect(check(normalized(ast, response, variables), ast, variables)).toEqual(
      enough,
    ),
  );
});

test('other variables are missing the root record', () => {
  const store = normalized(petDetail, petDetailResponse(), petDetailVariables);
  expect(check(store, petDetail, { id: '2', skip: null })).toEqual({
    kind: 'MissingData',
    record: queryRoot,
  });
  expect(check(store, petDetail, { id: '1', skip: 4 })).toEqual(
    missing('Pet', '1'),
  );
});

test('a null leaf is enough; an absent leaf key is missing its record', () => {
  const store = normalized(petDetail, petDetailResponse(), petDetailVariables);
  expect(check(store, petDetail, petDetailVariables)).toEqual(enough);
  store.get('Pet')?.get('1')?.delete('nickname');
  expect(check(store, petDetail, petDetailVariables)).toEqual(
    missing('Pet', '1'),
  );
});

test('an absent linked record is missing that record', () => {
  const store = normalized(petDetail, petDetailResponse(), petDetailVariables);
  store.get('Pet')?.delete('2');
  expect(check(store, petDetail, petDetailVariables)).toEqual(
    missing('Pet', '2'),
  );
});

test('a null link is enough; an absent identity-less record is missing', () => {
  const nullStats = normalized(
    petDetail,
    { pet____id___v_id: { ...petDetailPet(), stats: null } },
    petDetailVariables,
  );
  expect(check(nullStats, petDetail, petDetailVariables)).toEqual(enough);

  const store = normalized(petDetail, petDetailResponse(), petDetailVariables);
  store.get('PetStats')?.delete('Pet:1.stats');
  expect(check(store, petDetail, petDetailVariables)).toEqual(
    missing('PetStats', 'Pet:1.stats'),
  );
});

test('lists: null items and empty lists are enough; an absent item record is missing', () => {
  expect(
    check(
      normalized(
        petLists,
        { maybePets: [null], petGrid: [], pets: [], statGrid: null },
        {},
      ),
      petLists,
      {},
    ),
  ).toEqual(enough);

  const store = normalized(
    petLists,
    {
      ...petListsResponse(),
      petGrid: [[], [{ __typename: 'Pet', id: '6', name: 'Ozzy' }]],
    },
    {},
  );
  store.get('Pet')?.delete('6');
  expect(check(store, petLists, {})).toEqual(missing('Pet', '6'));
});

test('conditionals apply only to their entity', () => {
  const adWithoutTitle = normalized(feed, feedResponse(), { limit: 2 });
  expect(
    adWithoutTitle
      .get('AdItem')
      ?.get('Query:__ROOT.feed.1____limit___2')
      ?.has('title'),
  ).toBe(false);
  expect(check(adWithoutTitle, feed, { limit: 2 })).toEqual(enough);

  const blogWithoutTitle = normalized(feed, feedResponse(), { limit: 2 });
  blogWithoutTitle.get('BlogItem')?.get('b1')?.delete('title');
  expect(check(blogWithoutTitle, feed, { limit: 2 })).toEqual(
    missing('BlogItem', 'b1'),
  );
});

test('the first missing position in AST order is reported', () => {
  const store = normalized(petDetail, petDetailResponse(), petDetailVariables);
  store.get('Pet')?.get('1')?.delete('tagline');
  store.get('Pet')?.delete('2');
  // `bestFriend` precedes `tagline` in the AST.
  expect(check(store, petDetail, petDetailVariables)).toEqual(
    missing('Pet', '2'),
  );
  store.get('Pet')?.get('1')?.delete('bestFriend');
  expect(check(store, petDetail, petDetailVariables)).toEqual(
    missing('Pet', '1'),
  );
});

test('an empty selection set is enough when its record exists', () => {
  const variables = { id: '1' };
  const store = normalized(
    emptyStats,
    {
      pet____id___v_id: {
        __typename: 'Pet',
        id: '1',
        stats: { __typename: 'PetStats' },
      },
    },
    variables,
  );
  expect(check(store, emptyStats, variables)).toEqual(enough);
  store.get('PetStats')?.delete('Pet:1.stats');
  expect(check(store, emptyStats, variables)).toEqual(
    missing('PetStats', 'Pet:1.stats'),
  );
});

test('a non-link under a composite store key throws', () => {
  const store = storeOf({
    Query: { __ROOT: { 'pet____id___"1"': 'x' } },
  });
  expect(() => check(store, petDetail, petDetailVariables)).toThrow(
    'Store record Query:__ROOT holds a non-link under the composite store key "pet____id___"1""; the store was written with an AST that disagrees with this one.',
  );
  const extraKey = storeOf({
    Query: {
      __ROOT: { 'pet____id___"1"': { entityName: 'Pet', id: '1', x: 1 } },
    },
  });
  expect(() => check(extraKey, petDetail, petDetailVariables)).toThrow(
    'holds a non-link',
  );
});

test('check does not change the store', () => {
  const store = normalized(petDetail, petDetailResponse(), petDetailVariables);
  const before = structuredClone(store);
  check(store, petDetail, { id: '2', skip: 4 });
  check(store, petDetail, petDetailVariables);
  expect(store).toEqual(before);
});
