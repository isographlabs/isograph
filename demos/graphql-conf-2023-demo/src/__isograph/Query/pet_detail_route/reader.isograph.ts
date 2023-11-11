import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = (x: any) => x;
import Pet__pet_best_friend_card, { ReadOutType as Pet__pet_best_friend_card__outputType } from '../../Pet/pet_best_friend_card/reader.isograph';
import Pet__pet_checkins_card, { ReadOutType as Pet__pet_checkins_card__outputType } from '../../Pet/pet_checkins_card/reader.isograph';
import Pet__pet_phrase_card, { ReadOutType as Pet__pet_phrase_card__outputType } from '../../Pet/pet_phrase_card/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

export type ReadFromStoreType = ResolverParameterType;

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
        readerArtifact: Pet__pet_checkins_card,
        usedRefetchQueries: [],
      },
      {
        kind: "Resolver",
        alias: "pet_best_friend_card",
        arguments: null,
        readerArtifact: Pet__pet_best_friend_card,
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Resolver",
        alias: "pet_phrase_card",
        arguments: null,
        readerArtifact: Pet__pet_phrase_card,
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

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "NonFetchableResolver",
  resolver: resolver as any,
  readerAst,
  variant: "Eager",
};

export default artifact;
