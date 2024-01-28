import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { pet_detail_route as resolver } from '../../../components/pet_detail_route.tsx';
import Pet__pet_best_friend_card, { ReadOutType as Pet__pet_best_friend_card__outputType } from '../../Pet/pet_best_friend_card/reader.isograph';
import Pet__pet_checkins_card, { ReadOutType as Pet__pet_checkins_card__outputType } from '../../Pet/pet_checkins_card/reader.isograph';
import Pet__pet_phrase_card, { ReadOutType as Pet__pet_phrase_card__outputType } from '../../Pet/pet_phrase_card/reader.isograph';
import Pet__pet_tagline_card, { ReadOutType as Pet__pet_tagline_card__outputType } from '../../Pet/pet_tagline_card/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

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
        usedRefetchQueries: [0, 1, ],
      },
      {
        kind: "Resolver",
        alias: "pet_phrase_card",
        arguments: null,
        readerArtifact: Pet__pet_phrase_card,
        usedRefetchQueries: [],
      },
      {
        kind: "Resolver",
        alias: "pet_tagline_card",
        arguments: null,
        readerArtifact: Pet__pet_tagline_card,
        usedRefetchQueries: [],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  pet: ({
    name: string,
    pet_checkins_card: Pet__pet_checkins_card__outputType,
    pet_best_friend_card: Pet__pet_best_friend_card__outputType,
    pet_phrase_card: Pet__pet_phrase_card__outputType,
    pet_tagline_card: Pet__pet_tagline_card__outputType,
  } | null),
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.pet_detail_route" },
};

export default artifact;
