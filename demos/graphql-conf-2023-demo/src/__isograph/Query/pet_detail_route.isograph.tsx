import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
const resolver = (x: any) => x;
import Pet__pet_best_friend_card, { ReadOutType as Pet__pet_best_friend_card__outputType } from '../Pet/pet_best_friend_card.isograph';
import Pet__pet_checkins_card, { ReadOutType as Pet__pet_checkins_card__outputType } from '../Pet/pet_checkins_card.isograph';
import Pet__pet_phrase_card, { ReadOutType as Pet__pet_phrase_card__outputType } from '../Pet/pet_phrase_card.isograph';

import refetchQuery0 from './pet_detail_route/__refetch__0.isograph';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [{ artifact: refetchQuery0, allowedVariables: [] }, ];

const queryText = 'query pet_detail_route ($id: ID!) {\
  pet____id___id: pet(id: $id) {\
    id,\
    best_friend_relationship {\
      best_friend {\
        id,\
        name,\
        picture,\
      },\
      picture_together,\
    },\
    checkins {\
      id,\
      location,\
      time,\
    },\
    favorite_phrase,\
    name,\
    potential_new_best_friends {\
      id,\
      name,\
    },\
  },\
}';

export type ReadFromStoreType = ResolverParameterType;

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "pet",
    arguments: [
      {
        argumentName: "id",
        variableName: "id",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "best_friend_relationship",
        arguments: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "best_friend",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "name",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "picture",
                arguments: null,
              },
            ],
          },
          {
            kind: "Scalar",
            fieldName: "picture_together",
            arguments: null,
          },
        ],
      },
      {
        kind: "Linked",
        fieldName: "checkins",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "location",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "time",
            arguments: null,
          },
        ],
      },
      {
        kind: "Scalar",
        fieldName: "favorite_phrase",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "potential_new_best_friends",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            arguments: null,
          },
        ],
      },
    ],
  },
];
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "pet",
    alias: null,
    arguments: [
      {
        argumentName: "id",
        variableName: "id",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "pet_checkins_card",
        arguments: null,
        resolver: Pet__pet_checkins_card,
        variant: "Component",
        usedRefetchQueries: [],
      },
      {
        kind: "Resolver",
        alias: "pet_best_friend_card",
        arguments: null,
        resolver: Pet__pet_best_friend_card,
        variant: "Component",
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Resolver",
        alias: "pet_phrase_card",
        arguments: null,
        resolver: Pet__pet_phrase_card,
        variant: "Component",
        usedRefetchQueries: [],
      },
    ],
  },
];

export type ResolverParameterType = {
  pet: ({
    name: string,
    pet_checkins_card: Pet__pet_checkins_card__outputType,
    pet_best_friend_card: Pet__pet_best_friend_card__outputType,
    pet_phrase_card: Pet__pet_phrase_card__outputType,
  } | null),
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: ((resolver, data) => resolver(data)),
  nestedRefetchQueries,
};

export default artifact;
