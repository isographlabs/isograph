import { beforeEach, describe, expect, test, vi } from 'vitest';
import type * as PetNameModule from '../fixtures/store/src/PetName';
import type * as RecentCheckinsModule from '../fixtures/store/src/RecentCheckins';
import type * as StatsLinkModule from '../fixtures/store/src/StatsLink';
import type * as ConstantModule from '../fixtures/store/src/Constant';
import { Constant } from '../fixtures/store/src/Constant';
import { PetName } from '../fixtures/store/src/PetName';
import { RecentCheckins } from '../fixtures/store/src/RecentCheckins';
import { StatsLink } from '../fixtures/store/src/StatsLink';
import setTaglineNormalizationAst from '../fixtures/store/src/__isograph/Mutation/SetTagline/normalization_ast';
import setTaglineReaderArtifact from '../fixtures/store/src/__isograph/Mutation/SetTagline/resolver_reader';
import constantReaderArtifact from '../fixtures/store/src/__isograph/Pet/Constant/resolver_reader';
import petLinkReaderArtifact from '../fixtures/store/src/__isograph/Pet/PetLink/resolver_reader';
import petNameReaderArtifact from '../fixtures/store/src/__isograph/Pet/PetName/resolver_reader';
import constantPetNormalizationAst from '../fixtures/store/src/__isograph/Query/ConstantPet/normalization_ast';
import constantPetReaderArtifact from '../fixtures/store/src/__isograph/Query/ConstantPet/resolver_reader';
import feedNormalizationAst from '../fixtures/store/src/__isograph/Query/Feed/normalization_ast';
import feedReaderArtifact from '../fixtures/store/src/__isograph/Query/Feed/resolver_reader';
import leavesNormalizationAst from '../fixtures/store/src/__isograph/Query/Leaves/normalization_ast';
import leavesReaderArtifact from '../fixtures/store/src/__isograph/Query/Leaves/resolver_reader';
import linksEntrypoint from '../fixtures/store/src/__isograph/Query/Links/entrypoint';
import linksNormalizationAst from '../fixtures/store/src/__isograph/Query/Links/normalization_ast';
import linksReaderArtifact from '../fixtures/store/src/__isograph/Query/Links/resolver_reader';
import nestedQueryNormalizationAst from '../fixtures/store/src/__isograph/Query/NestedQuery/normalization_ast';
import nestedQueryReaderArtifact from '../fixtures/store/src/__isograph/Query/NestedQuery/resolver_reader';
import nullPassedPetNormalizationAst from '../fixtures/store/src/__isograph/Query/NullPassedPet/normalization_ast';
import nullPassedPetReaderArtifact from '../fixtures/store/src/__isograph/Query/NullPassedPet/resolver_reader';
import petDetailNormalizationAst from '../fixtures/store/src/__isograph/Query/PetDetail/normalization_ast';
import petDetailReaderArtifact from '../fixtures/store/src/__isograph/Query/PetDetail/resolver_reader';
import petListsNormalizationAst from '../fixtures/store/src/__isograph/Query/PetLists/normalization_ast';
import petListsReaderArtifact from '../fixtures/store/src/__isograph/Query/PetLists/resolver_reader';
import petSummariesNormalizationAst from '../fixtures/store/src/__isograph/Query/PetSummaries/normalization_ast';
import petSummariesReaderArtifact from '../fixtures/store/src/__isograph/Query/PetSummaries/resolver_reader';
import petSummaryEntrypoint from '../fixtures/store/src/__isograph/Query/PetSummary/entrypoint';
import petSummaryNormalizationAst from '../fixtures/store/src/__isograph/Query/PetSummary/normalization_ast';
import petSummaryReaderArtifact from '../fixtures/store/src/__isograph/Query/PetSummary/resolver_reader';
import searchNormalizationAst from '../fixtures/store/src/__isograph/Query/Search/normalization_ast';
import searchReaderArtifact from '../fixtures/store/src/__isograph/Query/Search/resolver_reader';
import { normalize } from '../src/normalize';
import {
  read,
  readButDoNotEvaluate,
  type ReadMissingData,
  type ReadResult,
} from '../src/read';
import type {
  JsonObject,
  JsonValue,
  Link,
  Store,
  Variables,
} from '../src/store';
import {
  feedResponse,
  mutationRoot,
  petDetailPet,
  petDetailResponse,
  petListsResponse,
  queryRoot,
  setTaglineResponse,
} from './fixtures';

/** Each mocked source module exports its resolver wrapped in `vi.fn`, so the generated reader
 * artifacts, which import the resolver from the source module, call the mock. A source module
 * that also holds an `entrypoint` literal is not mocked: the mock's factory loads it, it loads
 * its entrypoint artifact, and that loads the reader artifact while the mock is unfinished. A
 * top-level resolver is spied on the reader artifact object the test passes to `read`. */
vi.mock('../fixtures/store/src/PetName', async (importOriginal) => {
  const original = await importOriginal<typeof PetNameModule>();
  return { PetName: vi.fn(original.PetName) };
});
vi.mock('../fixtures/store/src/RecentCheckins', async (importOriginal) => {
  const original = await importOriginal<typeof RecentCheckinsModule>();
  return { RecentCheckins: vi.fn(original.RecentCheckins) };
});
vi.mock('../fixtures/store/src/StatsLink', async (importOriginal) => {
  const original = await importOriginal<typeof StatsLinkModule>();
  return { StatsLink: vi.fn(original.StatsLink) };
});
vi.mock('../fixtures/store/src/Constant', async (importOriginal) => {
  const original = await importOriginal<typeof ConstantModule>();
  return { Constant: vi.fn(original.Constant) };
});

beforeEach(() => {
  vi.clearAllMocks();
});

const pet = (id: string): Link<'Pet'> => ({ entityName: 'Pet', id });

/** A new store with `response` normalized by `ast` with `variables`. */
function normalized(
  ast: Parameters<typeof normalize>[1],
  response: JsonObject,
  variables: Variables,
): Store {
  const store: Store = new Map();
  normalize(store, ast, response, variables);
  return store;
}

/** `result`'s value; fails the test when it is `MissingData`. */
function okValue<T>(result: ReadResult<T>): T {
  if (result.kind === 'MissingData') {
    throw new Error(`Expected Ok, got MissingData: ${result.reason}`);
  }
  return result.value;
}

/** `result` as `MissingData`; fails the test when it is `Ok`. */
function missing<T>(result: ReadResult<T>): ReadMissingData {
  if (result.kind === 'Ok') {
    throw new Error('Expected MissingData, got Ok');
  }
  return result;
}

/** The reasons from `missing` to its innermost nested reason. */
function reasons(missingData: ReadMissingData): ReadonlyArray<string> {
  return [
    missingData.reason,
    ...(missingData.nestedReason === undefined
      ? []
      : reasons(missingData.nestedReason)),
  ];
}

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

function petSummaryResponse(): JsonObject {
  return {
    pet____id___v_id: {
      __typename: 'Pet',
      id: '1',
      checkins____limit___l_2: [
        { __typename: 'Checkin', id: 'c1', location: 'Park' },
        { __typename: 'Checkin', id: 'c2', location: 'Beach' },
      ],
      name: 'Rex',
      nickname: null,
    },
  };
}

const petSummaryValue = {
  pet: {
    id: '1',
    PetName: { name: 'Rex', nickname: null },
    RecentCheckins: {
      checkins: [
        { id: 'c1', location: 'Park' },
        { id: 'c2', location: 'Beach' },
      ],
    },
  },
};

function linksResponse(stats: JsonValue): JsonObject {
  return {
    node____id___v_id: { __typename: 'Pet', id: '1' },
    pet____id___v_id: {
      __typename: 'Pet',
      id: '1',
      bestFriend: { __typename: 'Pet', id: '2' },
      stats,
    },
  };
}

describe('leaves', () => {
  test('a leaf is the stored value whole', () => {
    const variables = { name: 'Bo' };
    const store = normalized(
      leavesNormalizationAst,
      leavesResponse(['a', null], { entityName: 'Pet', id: '1' }),
      variables,
    );
    expect(
      okValue(read(store, leavesReaderArtifact(), queryRoot, variables)),
    ).toEqual({
      greeting: 'hi',
      literalGreeting: 'HI ADA',
      nullGreeting: null,
      variableGreeting: 'hi Bo',
      tags: ['a', null],
      settings: { entityName: 'Pet', id: '1' },
    });
  });

  test('an empty list leaf is an empty list', () => {
    const store = normalized(leavesNormalizationAst, leavesResponse([], null), {
      name: 'Bo',
    });
    expect(
      okValue(read(store, leavesReaderArtifact(), queryRoot, { name: 'Bo' })),
    ).toMatchObject({ tags: [], settings: null });
  });
});

describe('variables', () => {
  test('a null variable reads the key normalize wrote for it', () => {
    const store = normalized(leavesNormalizationAst, leavesResponse([], null), {
      name: null,
    });
    const value = okValue(
      read(store, leavesReaderArtifact(), queryRoot, { name: null }),
    );
    const stored = store
      .get('Query')
      ?.get('__ROOT')
      ?.get('greeting____name___null');
    expect(stored).not.toBeUndefined();
    expect(value).toMatchObject({
      variableGreeting: stored,
      nullGreeting: stored,
    });
  });

  test('a variable value no normalize wrote is missing data naming the store key', () => {
    const store = normalized(leavesNormalizationAst, leavesResponse([], null), {
      name: null,
    });
    const result = missing(
      read(store, leavesReaderArtifact(), queryRoot, { name: 'Bo' }),
    );
    expect(result.reason).toContain('greeting____name___"Bo"');
    expect(result.recordLink).toEqual(queryRoot);
  });

  test('a passed null reaches the selected field and keys its store value', () => {
    const store = normalized(
      nullPassedPetNormalizationAst,
      {
        pet____id___v_id: {
          __typename: 'Pet',
          id: '1',
          checkins____limit___l_null: [{ __typename: 'Checkin', id: 'c1' }],
        },
      },
      { id: '1' },
    );
    expect(store.get('Pet')?.get('1')?.has('checkins____limit___null')).toBe(
      true,
    );
    expect(
      okValue(
        read(store, nullPassedPetReaderArtifact(), queryRoot, { id: '1' }),
      ),
    ).toEqual({ pet: { NullableCheckins: { checkins: [{ id: 'c1' }] } } });
  });
});

describe('linked', () => {
  const variables = { id: '1', skip: null };

  test('a linked node reads the records the stored links point at, cycles included', () => {
    const store = normalized(
      petDetailNormalizationAst,
      petDetailResponse(),
      variables,
    );
    expect(
      okValue(read(store, petDetailReaderArtifact(), queryRoot, variables)),
    ).toEqual({
      pet: {
        id: '1',
        name: 'Rex',
        nickname: null,
        tagline: 'Good dog',
        stats: { weight: 7, energy: null },
        bestFriend: {
          id: '2',
          name: 'Fido',
          bestFriend: { id: '1', name: 'Rex' },
        },
        checkins: [
          { id: 'c1', location: 'Park' },
          { id: 'c2', location: 'Beach' },
        ],
        pagedCheckins: [{ id: 'c3', location: 'Vet' }],
      },
    });
  });

  test('a stored null link reads null', () => {
    const store = normalized(
      petDetailNormalizationAst,
      { pet____id___v_id: { ...petDetailPet(), stats: null } },
      variables,
    );
    expect(
      okValue(read(store, petDetailReaderArtifact(), queryRoot, variables)),
    ).toMatchObject({ pet: { stats: null } });
  });

  test('a linked node over an empty list reads an empty list', () => {
    const store = normalized(
      petDetailNormalizationAst,
      {
        pet____id___v_id: {
          ...petDetailPet(),
          checkins____limit___l_2: [],
        },
      },
      variables,
    );
    expect(
      okValue(read(store, petDetailReaderArtifact(), queryRoot, variables)),
    ).toMatchObject({ pet: { checkins: [] } });
  });

  test('a stored value that is not a link under a linked node throws', () => {
    const store = normalized(
      petDetailNormalizationAst,
      petDetailResponse(),
      variables,
    );
    store.get('Pet')?.get('1')?.set('stats', 'x');
    expect(() =>
      read(store, petDetailReaderArtifact(), queryRoot, variables),
    ).toThrow(
      'Store record Pet:1 holds a non-link under the composite store key "stats"',
    );
  });
});

describe('lists', () => {
  test('lists keep their structure at every depth', () => {
    const store = normalized(petListsNormalizationAst, petListsResponse(), {});
    expect(
      okValue(read(store, petListsReaderArtifact(), queryRoot, {})),
    ).toEqual({
      pets: [
        { id: '1', name: 'Rexy' },
        { id: '9', name: 'Zed' },
      ],
      maybePets: [null, { id: '3', name: 'Mia' }],
      petGrid: [
        [{ id: '4', name: 'Ace' }],
        [],
        [null, { id: '5', name: 'Bea' }],
      ],
      statGrid: [[{ weight: 1 }, { weight: 2 }]],
    });
  });

  test('an empty list and a null list read as stored', () => {
    const store = normalized(
      petListsNormalizationAst,
      { ...petListsResponse(), pets: [], petGrid: null },
      {},
    );
    expect(
      okValue(read(store, petListsReaderArtifact(), queryRoot, {})),
    ).toMatchObject({ pets: [], petGrid: null });
  });
});

describe('refinements', () => {
  test('a refinement reads its selections when the record is of its entity, else null', () => {
    const response = feedResponse();
    const items = response['feed____limit___v_limit'];
    const store = normalized(
      feedNormalizationAst,
      {
        feed____limit___v_limit: [
          ...(Array.isArray(items) ? items : []),
          { __typename: 'Other' },
        ],
      },
      { limit: 2 },
    );
    expect(
      okValue(read(store, feedReaderArtifact(), queryRoot, { limit: 2 })),
    ).toEqual({
      feed: [
        {
          asBlogItem: {
            id: 'b1',
            title: 'Hello',
            author: { id: '1', name: 'Rex' },
          },
          asAdItem: null,
        },
        { asBlogItem: null, asAdItem: { advertiser: 'Acme' } },
        { asBlogItem: null, asAdItem: null },
      ],
    });
  });
});

describe('object arguments', () => {
  test('an object argument reads the key of its stable JSON', () => {
    const store = normalized(
      searchNormalizationAst,
      {
        search____filter___o_minAge__l_2_nameContains__v_q_c: [
          { __typename: 'Pet', id: '1', name: 'Rex' },
        ],
      },
      { q: 'Rex' },
    );
    expect(
      okValue(read(store, searchReaderArtifact(), queryRoot, { q: 'Rex' })),
    ).toEqual({ search: [{ id: '1', name: 'Rex' }] });
  });
});

describe('client field chains', () => {
  test('a resolver node reads the selected field at the same record and returns its resolver value', () => {
    const store = normalized(petSummaryNormalizationAst, petSummaryResponse(), {
      id: '1',
    });
    expect(
      read(store, petSummaryReaderArtifact(), queryRoot, { id: '1' }),
    ).toEqual({ kind: 'Ok', value: petSummaryValue });
  });

  test('the entrypoint returns the same reader artifact', () => {
    const store = normalized(petSummaryNormalizationAst, petSummaryResponse(), {
      id: '1',
    });
    expect(
      read(
        store,
        petSummaryEntrypoint.readerArtifact(),
        petSummaryEntrypoint.normalizationAst.root,
        { id: '1' },
      ),
    ).toEqual({ kind: 'Ok', value: petSummaryValue });
  });

  test('a resolver receives its node’s arguments with the compiler-written default', () => {
    const store = normalized(petSummaryNormalizationAst, petSummaryResponse(), {
      id: '1',
    });
    read(store, petSummaryReaderArtifact(), queryRoot, { id: '1' });
    expect(vi.mocked(RecentCheckins).mock.calls[0]?.[0].parameters).toEqual({
      n: 2,
    });
    expect(vi.mocked(PetName).mock.calls[0]?.[0].parameters).toEqual({});
  });

  test('a resolver node under a list reads once per record', () => {
    const store = normalized(
      petSummariesNormalizationAst,
      {
        pets: [
          { __typename: 'Pet', id: '1', name: 'Rex', nickname: 'R' },
          { __typename: 'Pet', id: '2', name: 'Fido', nickname: null },
        ],
      },
      {},
    );
    expect(
      okValue(read(store, petSummariesReaderArtifact(), queryRoot, {})),
    ).toEqual({
      pets: [
        { PetName: { name: 'Rex', nickname: 'R' } },
        { PetName: { name: 'Fido', nickname: null } },
      ],
    });
  });
});

describe('roots', () => {
  test('a reader artifact reads at any record of its parent entity', () => {
    const store = normalized(
      petDetailNormalizationAst,
      { pet____id___v_id: { ...petDetailPet(), id: '4', nickname: 'Four' } },
      { id: '4', skip: null },
    );
    expect(okValue(read(store, petNameReaderArtifact(), pet('4'), {}))).toEqual(
      { name: 'Rex', nickname: 'Four' },
    );
  });

  test('a mutation reads at the mutation root, and a nested query at the query root', () => {
    const variables = { id: '4', tagline: 'Hi' };
    const store = normalized(
      setTaglineNormalizationAst,
      setTaglineResponse(),
      variables,
    );
    expect(
      okValue(read(store, setTaglineReaderArtifact(), mutationRoot, variables)),
    ).toEqual({
      setTagline: {
        pet: { id: '4', tagline: 'Hi' },
        query: { greeting: 'hello' },
      },
    });
    expect(store.get('Query')?.get('__ROOT')?.get('greeting')).toBe('hello');
  });

  test('a nested selection of the query root reads the query root itself', () => {
    const store = normalized(
      nestedQueryNormalizationAst,
      { greeting: 'hi', query: { __typename: 'Query', tags: ['t'] } },
      {},
    );
    expect(
      okValue(read(store, nestedQueryReaderArtifact(), queryRoot, {})),
    ).toEqual({ greeting: 'hi', query: { tags: ['t'] } });
  });
});

describe('missing data', () => {
  test('a read of an absent root record is missing data on the root', () => {
    expect(
      read(new Map(), petSummaryReaderArtifact(), queryRoot, { id: '1' }),
    ).toEqual({
      kind: 'MissingData',
      reason: 'No record for Query:__ROOT',
      recordLink: queryRoot,
    });
  });

  test('a missing value is wrapped once per node and names the innermost record', () => {
    const store = normalized(petSummaryNormalizationAst, petSummaryResponse(), {
      id: '1',
    });
    store.get('Pet')?.get('1')?.delete('nickname');
    const result = missing(
      read(store, petSummaryReaderArtifact(), queryRoot, { id: '1' }),
    );
    expect(result.recordLink).toEqual(pet('1'));
    expect(reasons(result)).toEqual([
      'Missing data for pet____id___"1" on Query:__ROOT',
      'Missing data for PetName on Pet:1',
      'No value for nickname on Pet:1',
    ]);
    expect(vi.mocked(PetName)).not.toHaveBeenCalled();
  });

  test('an absent linked record is missing data on that record', () => {
    const store = normalized(
      petSummariesNormalizationAst,
      {
        pets: [
          { __typename: 'Pet', id: '1', name: 'Rex', nickname: null },
          { __typename: 'Pet', id: '2', name: 'Fido', nickname: null },
        ],
      },
      {},
    );
    store.get('Pet')?.delete('2');
    expect(
      missing(read(store, petSummariesReaderArtifact(), queryRoot, {}))
        .recordLink,
    ).toEqual(pet('2'));
  });

  test('a field with an empty reader AST is missing data on an absent record', () => {
    const store = normalized(
      constantPetNormalizationAst,
      { pet____id___v_id: { __typename: 'Pet', id: '9' } },
      { id: '9' },
    );
    store.get('Pet')?.delete('9');
    const result = missing(
      read(store, constantPetReaderArtifact(), queryRoot, { id: '9' }),
    );
    expect(result.recordLink).toEqual(pet('9'));
    expect(reasons(result).at(-1)).toBe('No record for Pet:9');
    expect(vi.mocked(Constant)).not.toHaveBeenCalled();
  });

  test('an empty reader AST passes empty data and the root link', () => {
    const store = normalized(
      constantPetNormalizationAst,
      { pet____id___v_id: { __typename: 'Pet', id: '1' } },
      { id: '1' },
    );
    expect(
      okValue(read(store, constantReaderArtifact(), pet('1'), {})),
    ).toEqual({});
    expect(vi.mocked(Constant).mock.calls[0]?.[0]).toEqual({
      data: {},
      parameters: {},
      link: pet('1'),
    });
  });
});

describe('own links', () => {
  test('each resolver receives the link of the record its reader AST was read at', () => {
    const store = normalized(
      linksNormalizationAst,
      linksResponse({ __typename: 'PetStats' }),
      { id: '1' },
    );
    const artifact = linksReaderArtifact();
    const resolver = vi.spyOn(artifact, 'resolver');
    expect(okValue(read(store, artifact, queryRoot, { id: '1' }))).toEqual({
      pet: {
        PetLink: { entityName: 'Pet', id: '1' },
        stats: { StatsLink: { entityName: 'PetStats', id: 'Pet:1.stats' } },
        bestFriend: { PetLink: { entityName: 'Pet', id: '2' } },
      },
      node: { NodeLink: { entityName: 'Pet', id: '1' } },
    });
    expect(resolver).toHaveBeenCalledOnce();
    expect(resolver.mock.calls[0]?.[0].link).toEqual(queryRoot);
  });

  test('a null record calls no resolver under it', () => {
    const store = normalized(linksNormalizationAst, linksResponse(null), {
      id: '1',
    });
    expect(
      okValue(
        read(store, linksEntrypoint.readerArtifact(), queryRoot, { id: '1' }),
      ),
    ).toMatchObject({ pet: { stats: null } });
    expect(vi.mocked(StatsLink)).not.toHaveBeenCalled();
  });

  test('a link reader rooted at a record returns that record’s link', () => {
    const store = normalized(
      petDetailNormalizationAst,
      { pet____id___v_id: { ...petDetailPet(), id: '4' } },
      { id: '4', skip: null },
    );
    expect(read(store, petLinkReaderArtifact(), pet('4'), {})).toEqual({
      kind: 'Ok',
      value: pet('4'),
    });
  });
});

test('no read changes the store', () => {
  const variables = { id: '1', skip: null };
  const store = normalized(
    petDetailNormalizationAst,
    petDetailResponse(),
    variables,
  );
  const before = structuredClone(store);
  read(store, petDetailReaderArtifact(), queryRoot, variables);
  read(store, petNameReaderArtifact(), pet('1'), {});
  read(store, petSummaryReaderArtifact(), queryRoot, { id: '1' });
  expect(store).toEqual(before);
});

/**
 * v1 has no memoization: every resolver runs on every read. When the memoized runtime lands
 * (refactors/pending/runtime/pico-runtime.md), only resolvers whose inputs changed run again,
 * and `RecentCheckins` stays at one call.
 */
test('every resolver runs again on every read', () => {
  const store = normalized(petSummaryNormalizationAst, petSummaryResponse(), {
    id: '1',
  });
  const artifact = petSummaryReaderArtifact();
  const petSummary = vi.spyOn(artifact, 'resolver');
  read(store, artifact, queryRoot, { id: '1' });
  expect(petSummary).toHaveBeenCalledTimes(1);
  expect(vi.mocked(PetName)).toHaveBeenCalledTimes(1);
  expect(vi.mocked(RecentCheckins)).toHaveBeenCalledTimes(1);
  normalize(store, petListsNormalizationAst, petListsResponse(), {});
  normalize(
    store,
    petDetailNormalizationAst,
    { pet____id___v_id: { ...petDetailPet(), name: 'Rexford' } },
    { id: '1', skip: null },
  );
  expect(okValue(read(store, artifact, queryRoot, { id: '1' }))).toMatchObject({
    pet: { PetName: { name: 'Rexford' } },
  });
  expect(petSummary).toHaveBeenCalledTimes(2);
  expect(vi.mocked(PetName)).toHaveBeenCalledTimes(2);
  expect(vi.mocked(RecentCheckins)).toHaveBeenCalledTimes(2);
});

describe('readButDoNotEvaluate', () => {
  test('returns the data read passes to the root resolver, calling nested resolvers and not the root resolver', () => {
    const store = normalized(petSummaryNormalizationAst, petSummaryResponse(), {
      id: '1',
    });
    const artifact = petSummaryReaderArtifact();
    const petSummary = vi.spyOn(artifact, 'resolver');
    expect(
      readButDoNotEvaluate(store, artifact, queryRoot, { id: '1' }),
    ).toEqual({ kind: 'Ok', value: petSummaryValue });
    expect(petSummary).not.toHaveBeenCalled();
    expect(vi.mocked(PetName)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(RecentCheckins)).toHaveBeenCalledTimes(1);
  });

  test('an absent root is missing data on the root', () => {
    expect(
      readButDoNotEvaluate(new Map(), petSummaryReaderArtifact(), queryRoot, {
        id: '1',
      }),
    ).toEqual({
      kind: 'MissingData',
      reason: 'No record for Query:__ROOT',
      recordLink: queryRoot,
    });
  });
});
