import type {
  NormalizationSelection,
  NormalizationSelectionSet,
} from '../src/artifacts';
import type { JsonObject, Link, Store, StoreValue } from '../src/store';

/** The root records the fixture's ASTs name, for asserting on results. */
export const queryRoot: Link<'Query'> = { entityName: 'Query', id: '__ROOT' };
export const mutationRoot: Link<'Mutation'> = {
  entityName: 'Mutation',
  id: '__ROOT',
};

/** A new store whose entities, ids, store keys, and values are `records`' keys and values, in order. */
export function storeOf(records: {
  readonly [entityName: string]: {
    readonly [id: string]: { readonly [storeKey: string]: StoreValue };
  };
}): Store {
  return new Map(
    Object.entries(records).map(([entityName, byId]) => [
      entityName,
      new Map(
        Object.entries(byId).map(([id, record]) => [
          id,
          new Map(Object.entries(record)),
        ]),
      ),
    ]),
  );
}

/** The `Selection` reached from `selections` by `responseKeys`, each a response key
 * (`responseKey ?? selectableName`) among the previous selection's target's selections. */
export function selectionAt(
  selections: NormalizationSelectionSet,
  responseKey: string,
  ...rest: ReadonlyArray<string>
): NormalizationSelection {
  const found = selections.find(
    (node): node is NormalizationSelection =>
      node.kind === 'Selection' &&
      (node.responseKey ?? node.selectableName) === responseKey,
  );
  if (found === undefined) {
    throw new Error(`The fixture AST has no selection "${responseKey}".`);
  }
  const [next, ...after] = rest;
  return next === undefined
    ? found
    : selectionAt(found.target?.selections ?? [], next, ...after);
}

/** A `Pet` response object of `PetDetail`'s `pet` selection, `Pet:1` with best friend `Pet:2`
 * whose best friend is `Pet:1`. */
export function petDetailPet(): JsonObject {
  return {
    __typename: 'Pet',
    id: '1',
    bestFriend: {
      __typename: 'Pet',
      id: '2',
      bestFriend: { __typename: 'Pet', id: '1', name: 'Rex' },
      name: 'Fido',
    },
    checkins____limit___l_2: [
      { __typename: 'Checkin', id: 'c1', location: 'Park' },
      { __typename: 'Checkin', id: 'c2', location: 'Beach' },
    ],
    checkins____limit___l_2____skip___v_skip: [
      { __typename: 'Checkin', id: 'c3', location: 'Vet' },
    ],
    name: 'Rex',
    nickname: null,
    stats: { __typename: 'PetStats', energy: null, weight: 7 },
    tagline: 'Good dog',
  };
}

/** `PetDetail`'s response for `Pet:1`. */
export function petDetailResponse(): JsonObject {
  return { pet____id___v_id: petDetailPet() };
}

/** `PetLists`' response: `pets` holds `Pet:1` and `Pet:9`. */
export function petListsResponse(): JsonObject {
  return {
    maybePets: [null, { __typename: 'Pet', id: '3', name: 'Mia' }],
    petGrid: [
      [{ __typename: 'Pet', id: '4', name: 'Ace' }],
      [],
      [null, { __typename: 'Pet', id: '5', name: 'Bea' }],
    ],
    pets: [
      { __typename: 'Pet', id: '1', name: 'Rexy' },
      { __typename: 'Pet', id: '9', name: 'Zed' },
    ],
    statGrid: [
      [
        { __typename: 'PetStats', weight: 1 },
        { __typename: 'PetStats', weight: 2 },
      ],
    ],
  };
}

/** `Feed`'s response with `limit: 2`: a `BlogItem` then an `AdItem`. */
export function feedResponse(): JsonObject {
  return {
    feed____limit___v_limit: [
      {
        __typename: 'BlogItem',
        id: 'b1',
        author: { __typename: 'Pet', id: '1', name: 'Rex' },
        title: 'Hello',
      },
      { __typename: 'AdItem', advertiser: 'Acme' },
    ],
  };
}

/** `SetTagline`'s response for `Pet:4` with tagline `Hi`. */
export function setTaglineResponse(): JsonObject {
  return {
    setTagline____input___o_id__v_id_tagline__v_tagline_c: {
      __typename: 'SetTaglinePayload',
      pet: { __typename: 'Pet', id: '4', tagline: 'Hi' },
      query: { __typename: 'Query', greeting: 'hello' },
    },
  };
}
