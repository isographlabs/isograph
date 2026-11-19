import { expect, test } from 'vitest';
import leaves from '../fixtures/store/src/__isograph/Query/Leaves/normalization_ast';
import petDetail from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import search from '../fixtures/store/src/__isograph/Query/Search/normalization_ast';
import { storeKey } from '../src/storeKey';
import { stableCopy } from '../src/util';
import { selectionAt } from './fixtures';

const greeting = selectionAt(leaves.selections, 'greeting');
const literalGreeting = selectionAt(
  leaves.selections,
  'greeting____loud___l_true____name___s_Ada_Lovelace____times___l_3',
);
const nullGreeting = selectionAt(
  leaves.selections,
  'greeting____name___l_null',
);
const variableGreeting = selectionAt(
  leaves.selections,
  'greeting____name___v_name',
);
const searchSelection = selectionAt(
  search.selections,
  'search____filter___o_minAge__l_2_nameContains__v_q_c',
);
const pet = selectionAt(petDetail.selections, 'pet____id___v_id');
const pagedCheckins = selectionAt(
  petDetail.selections,
  'pet____id___v_id',
  'checkins____limit___l_2____skip___v_skip',
);

test('a selection without arguments is its selectable name', () => {
  expect(storeKey(greeting, {})).toBe('greeting');
});

test('literal arguments are chunks in the sorted order, written as JSON', () => {
  expect(storeKey(literalGreeting, {})).toBe(
    'greeting____loud___true____name___"Ada Lovelace"____times___3',
  );
});

test('a null literal is null', () => {
  expect(storeKey(nullGreeting, {})).toBe('greeting____name___null');
});

test('a variable is substituted as JSON, so null and "null" are distinct', () => {
  expect(storeKey(variableGreeting, { name: 'Bo' })).toBe(
    'greeting____name___"Bo"',
  );
  expect(storeKey(variableGreeting, { name: null })).toBe(
    'greeting____name___null',
  );
  expect(storeKey(variableGreeting, { name: null })).toBe(
    storeKey(nullGreeting, {}),
  );
  expect(storeKey(variableGreeting, { name: 'null' })).toBe(
    'greeting____name___"null"',
  );
});

test('an object argument is stable JSON with its keys sorted', () => {
  expect(storeKey(searchSelection, { q: 'Rex' })).toBe(
    'search____filter___{"minAge":2,"nameContains":"Rex"}',
  );
});

test('a string and a number id are distinct keys', () => {
  expect(storeKey(pet, { id: '1' })).toBe('pet____id___"1"');
  expect(storeKey(pet, { id: 1 })).toBe('pet____id___1');
  expect(pet.responseKey).toBe('pet____id___v_id');
});

test('a null variable is null, at the top level and inside an object', () => {
  expect(storeKey(pagedCheckins, { skip: null })).toBe(
    'checkins____limit___2____skip___null',
  );
  expect(storeKey(searchSelection, { q: null })).toBe(
    'search____filter___{"minAge":2,"nameContains":null}',
  );
});

test('a variable that is an object is sorted at every depth', () => {
  expect(
    storeKey(variableGreeting, { name: { b: 1, a: { d: 2, c: 3 } } }),
  ).toBe('greeting____name___{"a":{"c":3,"d":2},"b":1}');
});

test('a missing variable throws', () => {
  expect(() => storeKey(variableGreeting, {})).toThrow(
    'Missing variable "name": every variable of the operation is required.',
  );
});

test('stableCopy sorts keys at every depth and keeps array order', () => {
  const expected = '{"a":{"c":2,"d":[3,{"e":2,"f":1}]},"b":1}';
  expect(
    JSON.stringify(stableCopy({ b: 1, a: { d: [3, { f: 1, e: 2 }], c: 2 } })),
  ).toBe(expected);
  expect(
    JSON.stringify(stableCopy({ a: { c: 2, d: [3, { e: 2, f: 1 }] }, b: 1 })),
  ).toBe(expected);
  expect(JSON.stringify(stableCopy([2, 1]))).toBe('[2,1]');
});

test('stableCopy sorts by code unit, not locale', () => {
  expect(JSON.stringify(stableCopy({ b: 1, B: 2, a: 3, é: 4 }))).toBe(
    '{"B":2,"a":3,"b":1,"é":4}',
  );
});

test('stableCopy keeps a constructor or __proto__ key as data', () => {
  expect(
    JSON.stringify(
      stableCopy(
        Object.fromEntries([
          ['constructor', 1],
          ['__proto__', 2],
        ]),
      ),
    ),
  ).toBe('{"__proto__":2,"constructor":1}');
});

test('stableCopy of scalars and empty containers', () => {
  expect(JSON.stringify(stableCopy(null))).toBe('null');
  expect(JSON.stringify(stableCopy('x'))).toBe('"x"');
  expect(JSON.stringify(stableCopy(1.5))).toBe('1.5');
  expect(JSON.stringify(stableCopy({}))).toBe('{}');
  expect(JSON.stringify(stableCopy([]))).toBe('[]');
});

test('a list is an array of its items, in order, with variables substituted', () => {
  const tagged = {
    selectableName: 'tagged',
    arguments: [
      [
        'tags',
        {
          kind: 'List',
          value: [
            { kind: 'String', value: 'a' },
            { kind: 'Variable', name: 't' },
            { kind: 'List', value: [] },
          ],
        },
      ],
    ],
  } as const;
  expect(storeKey(tagged, { t: 'b' })).toBe('tagged____tags___["a","b",[]]');
});
