import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
const resolver = (x: any) => x;
import Pet__pet_summary_card, { ReadOutType as Pet__pet_summary_card__outputType } from '../Pet/pet_summary_card.isograph';

const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [];

const queryText = 'query home_route  {\
  pets {\
    id,\
    name,\
    picture,\
    tagline,\
  },\
}';

export type ReadFromStoreType = ResolverParameterType;

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "pets",
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
      {
        kind: "Scalar",
        fieldName: "tagline",
        arguments: null,
      },
    ],
  },
];
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "pets",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "pet_summary_card",
        arguments: null,
        resolver: Pet__pet_summary_card,
        variant: "Component",
        usedRefetchQueries: [],
      },
    ],
  },
];

export type ResolverParameterType = {
  pets: ({
    id: string,
    pet_summary_card: Pet__pet_summary_card__outputType,
  })[],
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
  nestedRefetchQueries,
};

export default artifact;
