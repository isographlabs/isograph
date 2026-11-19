import { expect, test } from 'vitest';
import emptyStats from '../fixtures/store/src/__isograph/Query/EmptyStats/normalization_ast';
import feed from '../fixtures/store/src/__isograph/Query/Feed/normalization_ast';
import leaves from '../fixtures/store/src/__isograph/Query/Leaves/normalization_ast';
import nestedQuery from '../fixtures/store/src/__isograph/Query/NestedQuery/normalization_ast';
import petDetail from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import petLists from '../fixtures/store/src/__isograph/Query/PetLists/normalization_ast';
import search from '../fixtures/store/src/__isograph/Query/Search/normalization_ast';
import setTagline from '../fixtures/store/src/__isograph/Mutation/SetTagline/normalization_ast';
import { normalize } from '../src/normalize';
import type { JsonObject, JsonValue, Store, StoreLink } from '../src/store';
import {
  petDetailPet,
  petDetailResponse,
  petListsResponse,
  queryRoot,
  setTaglineResponse,
  storeOf,
} from './fixtures';

const pet = (id: string): StoreLink => ({ entityName: 'Pet', id });
const checkin = (id: string): StoreLink => ({ entityName: 'Checkin', id });
const petStats = (id: string): StoreLink => ({ entityName: 'PetStats', id });

const leavesKeys = {
  greeting: 'greeting',
  literal: 'greeting____loud___true____name___"Ada Lovelace"____times___3',
  null: 'greeting____name___null',
  variable: 'greeting____name___"Bo"',
};

function leavesResponse(tags: JsonValue, settings: JsonValue): JsonObject {
  return {
    greeting: 'hi',
    greeting____loud___l_true____name___s_Ada_Lovelace____times___l_3: 'HI ADA',
    greeting____name___v_name: 'hi Bo',
    greeting____name___l_null: null,
    settings,
    tags,
  };
}

/** The store `PetDetail` writes for `petDetailResponse()` with `skip: null`. */
function petDetailStore(): Store {
  return storeOf({
    Query: { __ROOT: { 'pet____id___"1"': pet('1') } },
    Pet: {
      '1': {
        id: '1',
        bestFriend: pet('2'),
        checkins____limit___2: [checkin('c1'), checkin('c2')],
        checkins____limit___2____skip___null: [checkin('c3')],
        name: 'Rex',
        nickname: null,
        stats: petStats('Pet:1.stats'),
        tagline: 'Good dog',
      },
      '2': { id: '2', bestFriend: pet('1'), name: 'Fido' },
    },
    PetStats: { 'Pet:1.stats': { energy: null, weight: 7 } },
    Checkin: {
      c1: { id: 'c1', location: 'Park' },
      c2: { id: 'c2', location: 'Beach' },
      c3: { id: 'c3', location: 'Vet' },
    },
  });
}

test('leaves are written into the empty store as received', () => {
  const store: Store = new Map();
  normalize(
    store,
    leaves,
    leavesResponse(['a', null], { entityName: 'Pet', id: '1' }),
    { name: 'Bo' },
  );
  expect(store).toEqual(
    storeOf({
      Query: {
        __ROOT: {
          [leavesKeys.greeting]: 'hi',
          [leavesKeys.literal]: 'HI ADA',
          [leavesKeys.null]: null,
          [leavesKeys.variable]: 'hi Bo',
          settings: { entityName: 'Pet', id: '1' },
          tags: ['a', null],
        },
      },
    }),
  );
  expect(store.has('Pet')).toBe(false);
});

test('an empty list leaf is stored as []', () => {
  const store: Store = new Map();
  normalize(store, leaves, leavesResponse([], null), { name: 'Bo' });
  expect(store.get('Query')?.get('__ROOT')?.get('tags')).toEqual([]);
});

test('a null leaf is stored as null, and its key is present', () => {
  const store: Store = new Map();
  normalize(store, leaves, leavesResponse(null, null), { name: 'Bo' });
  const root = store.get('Query')?.get('__ROOT');
  expect(root?.has('tags')).toBe(true);
  expect(root?.get('tags')).toBeNull();
  expect(root?.get('settings')).toBeNull();
});

test('PetDetail writes the root, the pets, stats, and checkins, without __typename', () => {
  const store: Store = new Map();
  normalize(store, petDetail, petDetailResponse(), { id: '1', skip: null });
  expect(store).toEqual(petDetailStore());
  [...store.values()].forEach((records) =>
    [...records.values()].forEach((record) =>
      expect(record.has('__typename')).toBe(false),
    ),
  );
});

test('another variable value writes a second store key beside the first', () => {
  const store: Store = new Map();
  normalize(store, petDetail, petDetailResponse(), { id: '1', skip: null });
  normalize(store, petDetail, petDetailResponse(), { id: '1', skip: 4 });
  const pet1 = store.get('Pet')?.get('1');
  expect(pet1?.get('checkins____limit___2____skip___4')).toEqual([
    checkin('c3'),
  ]);
  expect(pet1?.get('checkins____limit___2____skip___null')).toEqual([
    checkin('c3'),
  ]);
  expect(pet1?.get('checkins____limit___2')).toEqual([
    checkin('c1'),
    checkin('c2'),
  ]);
});

test('an identity-less object gets the derived id; null creates no record', () => {
  const store: Store = new Map();
  normalize(
    store,
    petDetail,
    { pet____id___v_id: { ...petDetailPet(), stats: null } },
    { id: '1', skip: null },
  );
  expect(store.get('Pet')?.get('1')?.get('stats')).toBeNull();
  expect(store.has('PetStats')).toBe(false);
});

test('cyclic data writes both depths onto one record, and both links', () => {
  const store: Store = new Map();
  normalize(store, petDetail, petDetailResponse(), { id: '1', skip: null });
  expect(store.get('Pet')?.get('1')?.get('bestFriend')).toEqual(pet('2'));
  expect(store.get('Pet')?.get('2')?.get('bestFriend')).toEqual(pet('1'));
  expect(store.get('Pet')?.size).toBe(2);
});

test('lists keep response order, nulls, and nesting; identity-less items get index ids', () => {
  const store: Store = new Map();
  normalize(store, petLists, petListsResponse(), {});
  expect(store).toEqual(
    storeOf({
      Query: {
        __ROOT: {
          maybePets: [null, pet('3')],
          petGrid: [[pet('4')], [], [null, pet('5')]],
          pets: [pet('1'), pet('9')],
          statGrid: [
            [
              petStats('Query:__ROOT.statGrid.0.0'),
              petStats('Query:__ROOT.statGrid.0.1'),
            ],
          ],
        },
      },
      Pet: {
        '3': { id: '3', name: 'Mia' },
        '4': { id: '4', name: 'Ace' },
        '5': { id: '5', name: 'Bea' },
        '1': { id: '1', name: 'Rexy' },
        '9': { id: '9', name: 'Zed' },
      },
      PetStats: {
        'Query:__ROOT.statGrid.0.0': { weight: 1 },
        'Query:__ROOT.statGrid.0.1': { weight: 2 },
      },
    }),
  );
});

test('empty and null lists', () => {
  const store: Store = new Map();
  normalize(
    store,
    petLists,
    { maybePets: null, petGrid: null, pets: [], statGrid: [] },
    {},
  );
  expect(store).toEqual(
    storeOf({
      Query: {
        __ROOT: { maybePets: null, petGrid: null, pets: [], statGrid: [] },
      },
    }),
  );
});

test('Feed writes each item by its conditional; an unknown entity gets an empty record', () => {
  const store: Store = new Map();
  normalize(
    store,
    feed,
    {
      feed____limit___v_limit: [
        {
          __typename: 'BlogItem',
          id: 'b1',
          author: { __typename: 'Pet', id: '1', name: 'Rex' },
          title: 'Hello',
        },
        { __typename: 'AdItem', advertiser: 'Acme' },
        { __typename: 'Other' },
      ],
    },
    { limit: 2 },
  );
  expect(store).toEqual(
    storeOf({
      Query: {
        __ROOT: {
          feed____limit___2: [
            { entityName: 'BlogItem', id: 'b1' },
            { entityName: 'AdItem', id: 'Query:__ROOT.feed.1____limit___2' },
            { entityName: 'Other', id: 'Query:__ROOT.feed.2____limit___2' },
          ],
        },
      },
      BlogItem: { b1: { id: 'b1', author: pet('1'), title: 'Hello' } },
      Pet: { '1': { id: '1', name: 'Rex' } },
      AdItem: { 'Query:__ROOT.feed.1____limit___2': { advertiser: 'Acme' } },
      Other: { 'Query:__ROOT.feed.2____limit___2': {} },
    }),
  );
});

test('Search links under the stable JSON key', () => {
  const store: Store = new Map();
  normalize(
    store,
    search,
    {
      search____filter___o_minAge__l_2_nameContains__v_q_c: [
        { __typename: 'Pet', id: '1', name: 'Rex' },
      ],
    },
    { q: 'Rex' },
  );
  expect(store).toEqual(
    storeOf({
      Query: {
        __ROOT: {
          'search____filter___{"minAge":2,"nameContains":"Rex"}': [pet('1')],
        },
      },
      Pet: { '1': { id: '1', name: 'Rex' } },
    }),
  );
});

test('EmptyStats creates the empty record and links it', () => {
  const store: Store = new Map();
  normalize(
    store,
    emptyStats,
    {
      pet____id___v_id: {
        __typename: 'Pet',
        id: '1',
        stats: { __typename: 'PetStats' },
      },
    },
    { id: '1' },
  );
  expect(store).toEqual(
    storeOf({
      Query: { __ROOT: { 'pet____id___"1"': pet('1') } },
      Pet: { '1': { id: '1', stats: petStats('Pet:1.stats') } },
      PetStats: { 'Pet:1.stats': {} },
    }),
  );
});

test('SetTagline writes onto Mutation.__ROOT, and its nested Query object onto Query.__ROOT', () => {
  const store: Store = new Map();
  normalize(store, setTagline, setTaglineResponse(), {
    id: '4',
    tagline: 'Hi',
  });
  const payloadId =
    'Mutation:__ROOT.setTagline____input___{"id":"4","tagline":"Hi"}';
  expect(store).toEqual(
    storeOf({
      Mutation: {
        __ROOT: {
          'setTagline____input___{"id":"4","tagline":"Hi"}': {
            entityName: 'SetTaglinePayload',
            id: payloadId,
          },
        },
      },
      SetTaglinePayload: {
        [payloadId]: { pet: pet('4'), query: queryRoot },
      },
      Pet: { '4': { id: '4', tagline: 'Hi' } },
      Query: { __ROOT: { greeting: 'hello' } },
    }),
  );
});

test('into a non-empty store, records hold the union and the later write', () => {
  const store: Store = new Map();
  normalize(store, petLists, petListsResponse(), {});
  normalize(store, petDetail, petDetailResponse(), { id: '1', skip: null });
  const pet1 = store.get('Pet')?.get('1');
  expect(pet1?.get('name')).toBe('Rex');
  expect(pet1?.get('tagline')).toBe('Good dog');
  expect(store.get('Pet')?.get('9')).toEqual(
    new Map([
      ['id', '9'],
      ['name', 'Zed'],
    ]),
  );
  expect(store.get('Query')?.get('__ROOT')?.get('pets')).toEqual([
    pet('1'),
    pet('9'),
  ]);
  expect(store.get('Query')?.get('__ROOT')?.get('pet____id___"1"')).toEqual(
    pet('1'),
  );
});

test('a field the later AST does not select is kept', () => {
  const store: Store = new Map();
  normalize(store, petDetail, petDetailResponse(), { id: '1', skip: null });
  normalize(store, petLists, petListsResponse(), {});
  const pet1 = store.get('Pet')?.get('1');
  expect(pet1?.get('tagline')).toBe('Good dog');
  expect(pet1?.get('name')).toBe('Rexy');
});

test('a nested root entity object is written onto the root record', () => {
  const withTypename: Store = new Map();
  normalize(
    withTypename,
    nestedQuery,
    { greeting: 'hi', query: { __typename: 'Query', tags: ['a'] } },
    {},
  );
  const expected = storeOf({
    Query: { __ROOT: { greeting: 'hi', query: queryRoot, tags: ['a'] } },
  });
  expect(withTypename).toEqual(expected);

  const withoutTypename: Store = new Map();
  normalize(
    withoutTypename,
    nestedQuery,
    { greeting: 'hi', query: { tags: ['a'] } },
    {},
  );
  expect(withoutTypename).toEqual(expected);
});

test('an absent response key throws, naming it', () => {
  expect(() => normalize(new Map(), leaves, {}, { name: 'Bo' })).toThrow(
    'Malformed response at []: expected the response key "greeting".',
  );
});

test('a scalar where an object is selected throws', () => {
  expect(() =>
    normalize(
      new Map(),
      petDetail,
      { pet____id___v_id: 'x' },
      {
        id: '1',
        skip: null,
      },
    ),
  ).toThrow(
    'Malformed response at ["pet____id___v_id"]: expected an object, a list, or null for "pet____id___v_id", got "x".',
  );
});

test('a scalar list item at depth two throws at its index path', () => {
  expect(() =>
    normalize(
      new Map(),
      petLists,
      { ...petListsResponse(), petGrid: [[3]] },
      {},
    ),
  ).toThrow(
    'Malformed response at ["petGrid",0,0]: expected an object, a list, or null for "petGrid", got 3.',
  );
});

test('a Discriminator object without __typename throws', () => {
  expect(() =>
    normalize(
      new Map(),
      feed,
      { feed____limit___v_limit: [{ advertiser: 'Acme' }] },
      { limit: 2 },
    ),
  ).toThrow(
    'Malformed response at ["feed____limit___v_limit",0]: expected a string under "__typename".',
  );
});

test('a Known object without __typename is written', () => {
  const store: Store = new Map();
  normalize(
    store,
    search,
    {
      search____filter___o_minAge__l_2_nameContains__v_q_c: [
        { id: '1', name: 'Rex' },
      ],
    },
    { q: 'Rex' },
  );
  expect(store.get('Pet')?.get('1')?.get('name')).toBe('Rex');
});

test('a non-string id throws', () => {
  expect(() =>
    normalize(
      new Map(),
      petDetail,
      { pet____id___v_id: { ...petDetailPet(), id: 4 } },
      { id: '1', skip: null },
    ),
  ).toThrow(
    'Malformed response at ["pet____id___v_id"]: expected a string record id under "id", got 4.',
  );
});

test('an absent id throws the absent-key message', () => {
  expect(() =>
    normalize(
      new Map(),
      search,
      {
        search____filter___o_minAge__l_2_nameContains__v_q_c: [
          { __typename: 'Pet', name: 'Rex' },
        ],
      },
      { q: 'Rex' },
    ),
  ).toThrow(
    'Malformed response at ["search____filter___o_minAge__l_2_nameContains__v_q_c",0]: expected the response key "id".',
  );
});

test('a throw part-way leaves the earlier writes', () => {
  const store: Store = new Map();
  expect(() =>
    normalize(
      store,
      petLists,
      {
        ...petListsResponse(),
        pets: [{ __typename: 'Pet', id: '1', name: 'Rex' }, 'x'],
      },
      {},
    ),
  ).toThrow(
    'Malformed response at ["pets",1]: expected an object, a list, or null for "pets", got "x".',
  );
  expect(store.get('Pet')?.get('1')).toEqual(
    new Map([
      ['id', '1'],
      ['name', 'Rex'],
    ]),
  );
});

test('a record id named constructor is ordinary data', () => {
  const store: Store = new Map();
  normalize(
    store,
    search,
    {
      search____filter___o_minAge__l_2_nameContains__v_q_c: [
        { __typename: 'Pet', id: 'constructor', name: 'Rex' },
      ],
    },
    { q: 'Rex' },
  );
  expect(store.get('Pet')?.get('constructor')?.get('name')).toBe('Rex');
});
